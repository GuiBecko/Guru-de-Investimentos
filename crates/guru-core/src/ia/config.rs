//! Configuração da IA: modelo, instruções e chaves de privacidade.

use rusqlite::params;

use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::ErroCampo;

/// (código, rótulo), na ordem em que a tela mostra. O primeiro é o padrão.
pub const MODELOS: &[(&str, &str)] = &[
    ("claude-opus-5-5", "Opus 5.5 · mais capaz"),
    ("claude-sonnet-5-5", "Sonnet 5.5 · equilíbrio"),
    ("claude-haiku-5-5", "Haiku 5.5 · mais barato"),
];

const MAX_INSTRUCOES: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIa {
    pub modelo: String,
    pub instrucoes: String,
    pub apenas_percentuais: bool,
    pub nomes_instituicoes: bool,
    pub busca_web: bool,
    /// Só os 4 últimos caracteres da chave, para a tela mostrar `sk-ant-…4f2a`.
    pub chave_final: Option<String>,
}

/// O resultado de `salvar_config_ia`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigSalva {
    pub erros: Vec<ErroCampo>,
    /// Alguma parte editada voltou ao automático porque a privacidade ficou mais restrita.
    pub edicoes_restauradas: bool,
}

impl Db {
    pub fn config_ia(&self) -> Result<ConfigIa, CoreError> {
        self.conn
            .query_row(
                "SELECT modelo, instrucoes, apenas_percentuais, nomes_instituicoes, busca_web, \
                 chave_final FROM ia_config WHERE id = 1",
                [],
                |r| {
                    Ok(ConfigIa {
                        modelo: r.get(0)?,
                        instrucoes: r.get(1)?,
                        apenas_percentuais: r.get(2)?,
                        nomes_instituicoes: r.get(3)?,
                        busca_web: r.get(4)?,
                        chave_final: r.get(5)?,
                    })
                },
            )
            .map_err(classifica)
    }

    /// Nada é gravado se houver erro. Se a privacidade ficar mais restrita (ligar "Apenas
    /// percentuais" ou desligar "Nome das instituições"), o texto editado das partes que mostram
    /// valores ou instituições é apagado na mesma transação, para a edição não furar a chave.
    pub fn salvar_config_ia(
        &self,
        modelo: &str,
        instrucoes: &str,
        apenas_percentuais: bool,
        nomes_instituicoes: bool,
        busca_web: bool,
    ) -> Result<ConfigSalva, CoreError> {
        let mut erros = Vec::new();
        if !MODELOS.iter().any(|(codigo, _)| *codigo == modelo) {
            erros.push(ErroCampo {
                campo: "modelo".into(),
                mensagem: "Escolha um dos modelos da lista.".into(),
            });
        }
        if instrucoes.chars().count() > MAX_INSTRUCOES {
            erros.push(ErroCampo {
                campo: "instrucoes".into(),
                mensagem: "Use no máximo 4.000 caracteres.".into(),
            });
        }
        if !erros.is_empty() {
            return Ok(ConfigSalva {
                erros,
                edicoes_restauradas: false,
            });
        }
        let tx = self.conn.unchecked_transaction().map_err(classifica)?;
        let (antes_percentuais, antes_nomes): (bool, bool) = tx
            .query_row(
                "SELECT apenas_percentuais, nomes_instituicoes FROM ia_config WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(classifica)?;
        tx.execute(
            "UPDATE ia_config SET modelo = ?1, instrucoes = ?2, apenas_percentuais = ?3, \
                 nomes_instituicoes = ?4, busca_web = ?5 WHERE id = 1",
            params![
                modelo,
                instrucoes,
                apenas_percentuais,
                nomes_instituicoes,
                busca_web
            ],
        )
        .map_err(classifica)?;
        // Valores em R$ aparecem na carteira e nas metas; instituições, só na carteira.
        let mut partes: Vec<&str> = Vec::new();
        if apenas_percentuais && !antes_percentuais {
            partes.extend(["carteira", "metas"]);
        }
        if !nomes_instituicoes && antes_nomes && !partes.contains(&"carteira") {
            partes.push("carteira");
        }
        let mut restauradas = 0;
        for parte in partes {
            restauradas += tx
                .execute(
                    "UPDATE ia_contexto_parte SET texto_editado = NULL \
                     WHERE parte = ?1 AND texto_editado IS NOT NULL",
                    [parte],
                )
                .map_err(classifica)?;
        }
        tx.commit().map_err(classifica)?;
        Ok(ConfigSalva {
            erros: Vec::new(),
            edicoes_restauradas: restauradas > 0,
        })
    }

    /// Guarda só os 4 últimos caracteres, mesmo que quem chama passe mais.
    pub(crate) fn gravar_chave_final(&self, final4: Option<&str>) -> Result<(), CoreError> {
        let final4: Option<String> = final4.map(|c| {
            let todos: Vec<char> = c.chars().collect();
            todos[todos.len().saturating_sub(4)..].iter().collect()
        });
        self.conn
            .execute(
                "UPDATE ia_config SET chave_final = ?1 WHERE id = 1",
                [final4],
            )
            .map_err(classifica)?;
        Ok(())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chave_final_guarda_so_os_4_ultimos_caracteres() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        db.gravar_chave_final(Some("sk-ant-segredo-4f2a")).unwrap();
        assert_eq!(db.config_ia().unwrap().chave_final.as_deref(), Some("4f2a"));
        db.gravar_chave_final(Some("ab")).unwrap();
        assert_eq!(db.config_ia().unwrap().chave_final.as_deref(), Some("ab"));
        db.gravar_chave_final(None).unwrap();
        assert_eq!(db.config_ia().unwrap().chave_final, None);
    }
}
