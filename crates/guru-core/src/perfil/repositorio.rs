use chrono::{Months, NaiveDate, SecondsFormat, Utc};

use super::{Perfil, calcular};
use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::{Classe, ErroCampo};

const MESES_DE_VALIDADE: u32 = 24;
const MAX_OBJETIVO: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfilGravado {
    pub perfil: Perfil,
    pub avisos: Vec<String>,
    pub feito_em: String,
    pub vencido: bool,
    pub alocacao_sugerida: Vec<(Classe, u8)>,
}

fn agora() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn corrompido(detalhe: &str) -> CoreError {
    CoreError::DbInvalid(format!("perfil com dado inválido no banco ({detalhe})"))
}

/// Array JSON de strings, escrito à mão para não depender de mais uma biblioteca.
fn avisos_em_json(avisos: &[String]) -> String {
    let itens: Vec<String> = avisos
        .iter()
        .map(|a| {
            let mut s = String::from("\"");
            for c in a.chars() {
                match c {
                    '"' => s.push_str("\\\""),
                    '\\' => s.push_str("\\\\"),
                    '\n' => s.push_str("\\n"),
                    c if (c as u32) < 0x20 => s.push_str(&format!("\\u{:04x}", c as u32)),
                    c => s.push(c),
                }
            }
            s.push('"');
            s
        })
        .collect();
    format!("[{}]", itens.join(","))
}

fn avisos_de_json(json: &str) -> Option<Vec<String>> {
    let mut chars = json.trim().chars();
    if chars.next()? != '[' {
        return None;
    }
    let mut avisos = Vec::new();
    loop {
        match chars.next()? {
            ']' => return Some(avisos),
            ',' | ' ' => {}
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => match chars.next()? {
                            'n' => s.push('\n'),
                            'u' => {
                                let hex: String = chars.by_ref().take(4).collect();
                                s.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                            }
                            outro => s.push(outro),
                        },
                        c => s.push(c),
                    }
                }
                avisos.push(s);
            }
            _ => return None,
        }
    }
}

fn vencido(feito_em: NaiveDate, hoje: NaiveDate) -> bool {
    feito_em
        .checked_add_months(Months::new(MESES_DE_VALIDADE))
        .is_some_and(|limite| hoje > limite)
}

impl Db {
    /// Calcula, grava o perfil e, se ainda não há alocação-alvo, grava a sugerida.
    pub fn registrar_perfil(
        &self,
        respostas: &[u8],
        hoje: NaiveDate,
    ) -> Result<Result<PerfilGravado, ErroCampo>, CoreError> {
        let resultado = match calcular(respostas) {
            Ok(r) => r,
            Err(e) => return Ok(Err(e)),
        };
        let feito_em = hoje.format("%Y-%m-%d").to_string();
        let texto_respostas = respostas
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let tx = self.conn.unchecked_transaction().map_err(classifica)?;
        tx.execute(
            "INSERT INTO perfis (perfil, pontos, respostas, avisos, feito_em) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                resultado.perfil.codigo(),
                resultado.pontos,
                texto_respostas,
                avisos_em_json(&resultado.avisos),
                feito_em
            ],
        )
        .map_err(classifica)?;
        let ja_tem: i64 = tx
            .query_row("SELECT COUNT(*) FROM alocacao_alvo", [], |r| r.get(0))
            .map_err(classifica)?;
        if ja_tem == 0 {
            for (classe, pct) in &resultado.alocacao_sugerida {
                tx.execute(
                    "INSERT INTO alocacao_alvo (classe, percentual) VALUES (?1, ?2)",
                    rusqlite::params![classe.codigo(), pct],
                )
                .map_err(classifica)?;
            }
        }
        tx.commit().map_err(classifica)?;

        Ok(Ok(PerfilGravado {
            perfil: resultado.perfil,
            avisos: resultado.avisos,
            feito_em,
            vencido: false,
            alocacao_sugerida: resultado.alocacao_sugerida,
        }
        .com_validade(hoje)))
    }

    pub fn perfil_atual(&self, hoje: NaiveDate) -> Result<Option<PerfilGravado>, CoreError> {
        let linha: Option<(String, String, String)> = match self.conn.query_row(
            "SELECT perfil, avisos, feito_em FROM perfis ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ) {
            Ok(l) => Some(l),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(classifica(e)),
        };
        let Some((codigo, avisos, feito_em)) = linha else {
            return Ok(None);
        };
        let perfil = Perfil::de_codigo(&codigo).ok_or_else(|| corrompido("perfil"))?;
        let avisos = avisos_de_json(&avisos).ok_or_else(|| corrompido("avisos"))?;
        Ok(Some(
            PerfilGravado {
                perfil,
                avisos,
                feito_em,
                vencido: false,
                alocacao_sugerida: super::alocacao_sugerida(perfil),
            }
            .com_validade(hoje),
        ))
    }

    /// As cinco classes na ordem de `Classe::TODOS`, ou `None` se nunca foi definida.
    pub fn alocacao_alvo(&self) -> Result<Option<Vec<(Classe, u8)>>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT classe, percentual FROM alocacao_alvo")
            .map_err(classifica)?;
        let linhas: Vec<(String, u8)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(classifica)?
            .collect::<Result<_, _>>()
            .map_err(classifica)?;
        if linhas.is_empty() {
            return Ok(None);
        }
        let mut alvo = Vec::new();
        for classe in Classe::TODOS {
            // Classe sem linha (por exemplo, criada depois) vale 0.
            let pct = linhas
                .iter()
                .find(|(c, _)| c == classe.codigo())
                .map_or(0, |(_, pct)| *pct);
            alvo.push((*classe, pct));
        }
        Ok(Some(alvo))
    }

    /// Exige as cinco classes, inteiros de 0 a 100 somando 100. Nada é gravado se houver erro.
    pub fn salvar_alocacao_alvo(
        &self,
        valores: &[(String, i64)],
    ) -> Result<Vec<ErroCampo>, CoreError> {
        let mut erros = Vec::new();
        let mut escolhidos = Vec::new();
        for classe in Classe::TODOS {
            let valor = valores
                .iter()
                .find(|(c, _)| c == classe.codigo())
                .map(|(_, v)| *v)
                .filter(|v| (0..=100).contains(v));
            match valor {
                Some(v) => escolhidos.push((classe.codigo(), v)),
                None => erros.push(ErroCampo {
                    campo: classe.codigo().into(),
                    mensagem: "Use um número inteiro de 0 a 100.".into(),
                }),
            }
        }
        if erros.is_empty() {
            let soma: i64 = escolhidos.iter().map(|(_, v)| v).sum();
            if soma != 100 {
                erros.push(ErroCampo {
                    campo: "alocacao".into(),
                    mensagem: format!("A soma precisa dar 100%. Hoje dá {soma}%."),
                });
            }
        }
        if !erros.is_empty() {
            return Ok(erros);
        }
        let tx = self.conn.unchecked_transaction().map_err(classifica)?;
        tx.execute("DELETE FROM alocacao_alvo", [])
            .map_err(classifica)?;
        for (classe, pct) in escolhidos {
            tx.execute(
                "INSERT INTO alocacao_alvo (classe, percentual) VALUES (?1, ?2)",
                rusqlite::params![classe, pct],
            )
            .map_err(classifica)?;
        }
        tx.commit().map_err(classifica)?;
        Ok(Vec::new())
    }

    pub fn objetivo(&self) -> Result<Option<String>, CoreError> {
        match self
            .conn
            .query_row("SELECT texto FROM objetivo WHERE id = 1", [], |r| r.get(0))
        {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(classifica(e)),
        }
    }

    /// Texto vazio (após `trim`) apaga o objetivo.
    pub fn salvar_objetivo(&self, texto: &str) -> Result<Vec<ErroCampo>, CoreError> {
        let texto = texto.trim();
        if texto.chars().count() > MAX_OBJETIVO {
            return Ok(vec![ErroCampo {
                campo: "objetivo".into(),
                mensagem: "Use até 2.000 caracteres.".into(),
            }]);
        }
        if texto.is_empty() {
            self.conn
                .execute("DELETE FROM objetivo", [])
                .map_err(classifica)?;
        } else {
            self.conn
                .execute(
                    "INSERT INTO objetivo (id, texto, atualizado_em) VALUES (1, ?1, ?2) \
                     ON CONFLICT (id) DO UPDATE SET texto = excluded.texto, \
                     atualizado_em = excluded.atualizado_em",
                    rusqlite::params![texto, agora()],
                )
                .map_err(classifica)?;
        }
        Ok(Vec::new())
    }
}

impl PerfilGravado {
    fn com_validade(mut self, hoje: NaiveDate) -> Self {
        self.vencido = NaiveDate::parse_from_str(&self.feito_em, "%Y-%m-%d")
            .map(|d| vencido(d, hoje))
            .unwrap_or(false);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avisos_ida_e_volta_com_aspas_e_acentos() {
        let avisos = vec!["Ela disse \"não\" \\ ok".to_string(), "Segundo".to_string()];
        assert_eq!(avisos_de_json(&avisos_em_json(&avisos)), Some(avisos));
        assert_eq!(avisos_de_json("[]"), Some(vec![]));
    }

    #[test]
    fn alocacao_alvo_com_classe_ausente_vale_zero() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let valores: Vec<(String, i64)> = [
            ("rf", 50),
            ("acoes", 25),
            ("fii", 10),
            ("ext", 10),
            ("outros", 5),
        ]
        .iter()
        .map(|(c, v)| (c.to_string(), *v))
        .collect();
        assert!(db.salvar_alocacao_alvo(&valores).unwrap().is_empty());
        db.conn
            .execute("DELETE FROM alocacao_alvo WHERE classe = 'outros'", [])
            .unwrap();
        let alvo = db.alocacao_alvo().unwrap().unwrap();
        assert_eq!(alvo.len(), 5);
        assert_eq!(alvo[4], (Classe::Outros, 0));
        assert_eq!(alvo[0], (Classe::Rf, 50));
    }

    #[test]
    fn apagar_meta_desvincula_investimento_sem_apagar() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        db.conn
            .execute(
                "INSERT INTO metas (nome, valor_alvo_centavos, prazo, criado_em, atualizado_em) \
                 VALUES ('Casa', 100, '2030-01', 'x', 'x')",
                [],
            )
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO investimentos (tipo, instituicao, data_aplicacao, \
                 valor_aplicado_centavos, criado_em, atualizado_em, meta_id) \
                 VALUES ('rf-cdb', 'A', '2025-01-01', 100, 'x', 'x', 1)",
                [],
            )
            .unwrap();
        db.conn
            .execute("DELETE FROM metas WHERE id = 1", [])
            .unwrap();
        let (n, meta): (i64, Option<i64>) = db
            .conn
            .query_row(
                "SELECT COUNT(*), MAX(meta_id) FROM investimentos",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((n, meta), (1, None));
    }
}
