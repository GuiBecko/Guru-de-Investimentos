use chrono::{Datelike, NaiveDate, SecondsFormat, Utc};
use rusqlite::named_params;

use super::modelo::{Meta, NovaMeta};
use super::progresso::progresso;
use super::validacao::validar_meta;
use crate::db::{Db, classifica};
use crate::error::CoreError;
use crate::investimento::ErroCampo;

fn agora() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn corrompido(detalhe: &str) -> CoreError {
    CoreError::DbInvalid(format!("meta com dado inválido no banco ({detalhe})"))
}

struct Linha {
    id: i64,
    nome: String,
    valor_alvo: i64,
    prazo: String,
}

/// `YYYY-MM` gravado pela validação.
fn ano_mes(prazo: &str) -> Result<(i32, u32), CoreError> {
    NaiveDate::parse_from_str(&format!("{prazo}-01"), "%Y-%m-%d")
        .map(|d| (d.year(), d.month()))
        .map_err(|_| corrompido("prazo"))
}

type Vinculo = (i64, i64, NaiveDate);

impl Db {
    pub fn criar_meta(
        &self,
        nova: &NovaMeta,
        hoje: NaiveDate,
    ) -> Result<Result<i64, Vec<ErroCampo>>, CoreError> {
        let v = match validar_meta(nova, hoje, false) {
            Ok(v) => v,
            Err(erros) => return Ok(Err(erros)),
        };
        let agora = agora();
        self.conn
            .execute(
                "INSERT INTO metas (nome, valor_alvo_centavos, prazo, criado_em, atualizado_em) \
                 VALUES (:nome, :valor, :prazo, :agora, :agora)",
                named_params! {
                    ":nome": v.nome,
                    ":valor": v.valor_alvo_centavos,
                    ":prazo": v.prazo,
                    ":agora": agora,
                },
            )
            .map_err(classifica)?;
        Ok(Ok(self.conn.last_insert_rowid()))
    }

    pub fn atualizar_meta(
        &self,
        id: i64,
        nova: &NovaMeta,
        hoje: NaiveDate,
    ) -> Result<Result<i64, Vec<ErroCampo>>, CoreError> {
        let v = match validar_meta(nova, hoje, true) {
            Ok(v) => v,
            Err(erros) => return Ok(Err(erros)),
        };
        let n = self
            .conn
            .execute(
                "UPDATE metas SET nome = :nome, valor_alvo_centavos = :valor, prazo = :prazo, \
                 atualizado_em = :agora WHERE id = :id",
                named_params! {
                    ":nome": v.nome,
                    ":valor": v.valor_alvo_centavos,
                    ":prazo": v.prazo,
                    ":agora": agora(),
                    ":id": id,
                },
            )
            .map_err(classifica)?;
        if n == 0 {
            return Err(CoreError::MetaNaoEncontrada(id));
        }
        Ok(Ok(id))
    }

    /// Os investimentos vinculados continuam; só perdem o vínculo.
    pub fn apagar_meta(&self, id: i64) -> Result<(), CoreError> {
        let n = self
            .conn
            .execute(
                "DELETE FROM metas WHERE id = :id",
                named_params! {":id": id},
            )
            .map_err(classifica)?;
        if n == 0 {
            return Err(CoreError::MetaNaoEncontrada(id));
        }
        Ok(())
    }

    /// Ordenadas por prazo e depois por nome (sem diferenciar maiúsculas).
    pub fn listar_metas(&self, hoje: NaiveDate) -> Result<Vec<Meta>, CoreError> {
        let linhas = self.linhas_de_metas(None)?;
        let vinculos = self.vinculos()?;
        let mut metas = linhas
            .into_iter()
            .map(|l| montar(l, &vinculos, hoje))
            .collect::<Result<Vec<_>, _>>()?;
        metas.sort_by_cached_key(|m| (m.prazo.clone(), m.nome.to_lowercase(), m.id));
        Ok(metas)
    }

    pub fn meta(&self, id: i64, hoje: NaiveDate) -> Result<Meta, CoreError> {
        let l = self
            .linhas_de_metas(Some(id))?
            .into_iter()
            .next()
            .ok_or(CoreError::MetaNaoEncontrada(id))?;
        montar(l, &self.vinculos()?, hoje)
    }

    fn linhas_de_metas(&self, id: Option<i64>) -> Result<Vec<Linha>, CoreError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, nome, valor_alvo_centavos, prazo FROM metas \
                 WHERE :id IS NULL OR id = :id",
            )
            .map_err(classifica)?;
        stmt.query_map(named_params! {":id": id}, |r| {
            Ok(Linha {
                id: r.get(0)?,
                nome: r.get(1)?,
                valor_alvo: r.get(2)?,
                prazo: r.get(3)?,
            })
        })
        .map_err(classifica)?
        .collect::<Result<_, _>>()
        .map_err(classifica)
    }

    /// (meta, valor aplicado, data da aplicação) de todo investimento vinculado.
    fn vinculos(&self) -> Result<Vec<Vinculo>, CoreError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT meta_id, valor_aplicado_centavos, data_aplicacao FROM investimentos \
                 WHERE meta_id IS NOT NULL",
            )
            .map_err(classifica)?;
        let linhas: Vec<(i64, i64, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(classifica)?
            .collect::<Result<_, _>>()
            .map_err(classifica)?;
        linhas
            .into_iter()
            .map(|(meta, valor, data)| {
                let d = NaiveDate::parse_from_str(&data, "%Y-%m-%d")
                    .map_err(|_| corrompido("data da aplicação"))?;
                Ok((meta, valor, d))
            })
            .collect()
    }
}

fn montar(l: Linha, vinculos: &[Vinculo], hoje: NaiveDate) -> Result<Meta, CoreError> {
    let meus: Vec<(i64, NaiveDate)> = vinculos
        .iter()
        .filter(|(meta, _, _)| *meta == l.id)
        .map(|(_, valor, data)| (*valor, *data))
        .collect();
    let p = progresso(l.valor_alvo, ano_mes(&l.prazo)?, &meus, hoje);
    Ok(Meta {
        id: l.id,
        nome: l.nome,
        valor_alvo_centavos: l.valor_alvo,
        prazo: l.prazo,
        acumulado_centavos: p.acumulado_centavos,
        percentual_decimos: p.percentual_decimos,
        meses_restantes: p.meses_restantes,
        aporte_necessario_centavos: p.aporte_necessario_centavos,
        ritmo_centavos: p.ritmo_centavos,
        situacao: p.situacao,
        mensagem: p.mensagem,
        investimentos: meus.len() as i64,
    })
}
