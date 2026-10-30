//! Listas do painel: instituições, vencimentos e variação mensal dos tickers.

use chrono::NaiveDate;

use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{Investimento, decimos};

use super::historico::dividir_arredondando;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instituicao {
    pub nome: String,
    pub atual_centavos: i64,
    pub percentual_decimos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vencimento {
    pub id: i64,
    pub nome: String,
    pub vencimento: NaiveDate,
    pub atual_centavos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variacao {
    pub ticker: String,
    pub variacao_decimos: i64,
}

/// `round((p1 / p0 − 1) × 1000)` meio para cima; `None` com preço inicial não positivo.
fn variacao_decimos(p0: i64, p1: i64) -> Option<i64> {
    if p0 <= 0 {
        return None;
    }
    let r = dividir_arredondando(
        2 * (i128::from(p1) - i128::from(p0)) * 1000,
        2 * i128::from(p0),
    );
    i64::try_from(r).ok()
}

fn somar(a: i64, b: i64) -> Result<i64, CoreError> {
    a.checked_add(b)
        .ok_or_else(|| CoreError::DbInvalid("soma da carteira excede o limite".into()))
}

/// Valor atual por instituição, do maior para o menor; o empate vai por nome.
pub(super) fn instituicoes(inv: &[Investimento]) -> Result<Vec<Instituicao>, CoreError> {
    let mut lista: Vec<(String, i64)> = Vec::new();
    for i in inv {
        match lista.iter_mut().find(|(n, _)| *n == i.instituicao) {
            Some((_, total)) => *total = somar(*total, i.valor_atual_centavos)?,
            None => lista.push((i.instituicao.clone(), i.valor_atual_centavos)),
        }
    }
    let total = lista.iter().try_fold(0i64, |a, (_, v)| somar(a, *v))?;
    lista.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(lista
        .into_iter()
        .map(|(nome, atual)| Instituicao {
            nome,
            atual_centavos: atual,
            percentual_decimos: decimos(atual, total),
        })
        .collect())
}

/// Os cinco próximos vencimentos a partir de `hoje` (inclusive), em ordem crescente.
pub(super) fn vencimentos(inv: &[Investimento], hoje: NaiveDate) -> Vec<Vencimento> {
    let mut lista: Vec<Vencimento> = inv
        .iter()
        .filter_map(|i| {
            let v = NaiveDate::parse_from_str(i.vencimento.as_deref()?, "%Y-%m-%d").ok()?;
            (v >= hoje).then(|| Vencimento {
                id: i.id,
                nome: i.nome.clone(),
                vencimento: v,
                atual_centavos: i.valor_atual_centavos,
            })
        })
        .collect();
    lista.sort_by(|a, b| {
        a.vencimento
            .cmp(&b.vencimento)
            .then_with(|| a.nome.cmp(&b.nome))
            .then_with(|| a.id.cmp(&b.id))
    });
    lista.truncate(5);
    lista
}

/// Variação de cada ticker de renda variável entre o último fim de mês (penúltima data do
/// painel) e `hoje` (a última), do maior para o menor. Ticker sem cotação em alguma das duas
/// datas, ou com a mesma data de cotação nas duas, fica de fora.
pub(super) fn variacoes(
    db: &Db,
    inv: &[Investimento],
    datas: &[NaiveDate],
) -> Result<Vec<Variacao>, CoreError> {
    let [.., fim_do_mes, hoje] = datas else {
        return Ok(Vec::new());
    };
    let mut tickers: Vec<&str> = inv
        .iter()
        .filter(|i| i.tipo.starts_with("rv-"))
        .filter_map(|i| i.ticker.as_deref())
        .collect();
    tickers.sort_unstable();
    tickers.dedup();
    let mut out = Vec::new();
    for t in tickers {
        let (Some(antes), Some(agora)) = (db.cotacao_em(t, *fim_do_mes)?, db.cotacao_em(t, *hoje)?)
        else {
            continue;
        };
        if antes.data == agora.data {
            continue;
        }
        if let Some(v) = variacao_decimos(antes.fechamento_centavos, agora.fechamento_centavos) {
            out.push(Variacao {
                ticker: t.to_string(),
                variacao_decimos: v,
            });
        }
    }
    out.sort_by(|a, b| {
        b.variacao_decimos
            .cmp(&a.variacao_decimos)
            .then_with(|| a.ticker.cmp(&b.ticker))
    });
    Ok(out)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn variacao_em_decimos_arredonda_meio_para_cima() {
        assert_eq!(variacao_decimos(3000, 3300), Some(100));
        assert_eq!(variacao_decimos(3000, 2700), Some(-100));
        // +0,05 ponto: 0,5 décimo sobe para 1; -0,05 ponto: -0,5 sobe para 0.
        assert_eq!(variacao_decimos(2000, 2001), Some(1));
        assert_eq!(variacao_decimos(2000, 1999), Some(0));
        assert_eq!(variacao_decimos(0, 100), None);
    }
}
