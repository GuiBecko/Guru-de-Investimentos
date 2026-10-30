//! Benchmarks acumulados nas datas do painel. Regras em "Global Constraints" do plano da 3b.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::Classe;
use crate::mercado::{PontoDiario, Series};

use super::historico::em_centesimos;

/// Referência de comparação do gráfico (`Benchmark` já nomeia o campo do investimento).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Referencia {
    Cdi,
    Selic,
    Ipca,
    Bova11,
    Xfix11,
}

impl Referencia {
    /// Opções do seletor, na ordem em que aparecem.
    pub const TODOS: &'static [Self] = &[
        Self::Cdi,
        Self::Selic,
        Self::Ipca,
        Self::Bova11,
        Self::Xfix11,
    ];

    pub fn codigo(&self) -> &'static str {
        match self {
            Self::Cdi => "cdi",
            Self::Selic => "selic",
            Self::Ipca => "ipca",
            Self::Bova11 => "bova11",
            Self::Xfix11 => "xfix11",
        }
    }

    pub fn rotulo(&self) -> &'static str {
        match self {
            Self::Cdi => "CDI",
            Self::Selic => "Selic",
            Self::Ipca => "IPCA",
            Self::Bova11 => "BOVA11 (Ibovespa)",
            Self::Xfix11 => "XFIX11 (IFIX)",
        }
    }
}

/// Benchmark escolhido de início no detalhe de cada classe.
pub fn benchmark_padrao(classe: Classe) -> Referencia {
    match classe {
        Classe::Acoes => Referencia::Bova11,
        Classe::Fii => Referencia::Xfix11,
        Classe::Rf | Classe::Ext | Classe::Outros => Referencia::Cdi,
    }
}

/// Fator acumulado (`1 + retorno`) por data; `None` em uma data sem dado.
pub(super) type Fatores = Vec<Option<Decimal>>;

fn estouro() -> CoreError {
    CoreError::DbInvalid("benchmark acumulado excede o limite".into())
}

/// Fatores de `r` de `datas[0]` até cada data. CDI, Selic e IPCA saem das séries já
/// carregadas (até a última data, pelo menos); as cotações vêm do banco.
pub(super) fn fatores_benchmark(
    db: &Db,
    series: &Series,
    r: Referencia,
    datas: &[NaiveDate],
) -> Result<Option<Fatores>, CoreError> {
    let Some(&inicio) = datas.first() else {
        return Ok(None);
    };
    match r {
        Referencia::Cdi => diario(series.diaria("cdi"), datas, inicio),
        Referencia::Selic => diario(series.diaria("selic"), datas, inicio),
        Referencia::Ipca => ipca(series.ipca(), datas, inicio),
        Referencia::Bova11 => cotacoes(db, "BOVA11", datas, inicio),
        Referencia::Xfix11 => cotacoes(db, "XFIX11", datas, inicio),
    }
}

/// `Π(1 + t_d/100)` para d em `(D₀, Dᵢ]`. Sem dado em `D₀` (a série começa depois dele), ausente.
fn diario(
    pontos: &[PontoDiario],
    datas: &[NaiveDate],
    inicio: NaiveDate,
) -> Result<Option<Fatores>, CoreError> {
    if pontos.first().is_none_or(|p| p.data > inicio) {
        return Ok(None);
    }
    let cem = Decimal::from(100);
    let mut fator = Decimal::ONE;
    let mut proximo = pontos.partition_point(|p| p.data <= inicio);
    let mut out = Vec::with_capacity(datas.len());
    for &d in datas {
        while let Some(p) = pontos.get(proximo).filter(|p| p.data <= d) {
            fator = fator
                .checked_mul(Decimal::ONE + p.valor / cem)
                .ok_or_else(estouro)?;
            proximo += 1;
        }
        out.push(Some(fator));
    }
    Ok(Some(out))
}

fn mes(d: NaiveDate) -> String {
    d.format("%Y-%m").to_string()
}

/// Último mês cujo último dia já passou em `d`: o próprio mês se `d` o encerra.
fn ultimo_mes_fechado(d: NaiveDate) -> Option<String> {
    if d.succ_opt().is_some_and(|s| s.month() != d.month()) {
        Some(mes(d))
    } else {
        d.with_day(1)?.pred_opt().map(mes)
    }
}

/// `Π(1 + ipca_m/100)` dos meses cujo último dia está em `(D₀, Dᵢ]` e estão guardados.
/// Mês ainda não publicado ou faltando simplesmente não entra no produto. Sem nenhum mês
/// guardado até o de `D₀`, o benchmark inteiro fica ausente.
fn ipca(
    meses: &[(String, Decimal)],
    datas: &[NaiveDate],
    inicio: NaiveDate,
) -> Result<Option<Fatores>, CoreError> {
    let mes_inicio = mes(inicio);
    if !meses.iter().any(|(m, _)| *m <= mes_inicio) {
        return Ok(None);
    }
    let cem = Decimal::from(100);
    let mut out = Vec::with_capacity(datas.len());
    for &d in datas {
        let Some(ate) = ultimo_mes_fechado(d) else {
            out.push(None);
            continue;
        };
        let mut fator = Decimal::ONE;
        for (m, v) in meses {
            if *m > mes_inicio && *m <= ate {
                fator = fator
                    .checked_mul(Decimal::ONE + *v / cem)
                    .ok_or_else(estouro)?;
            }
        }
        out.push(Some(fator));
    }
    Ok(Some(out))
}

/// `cotacao(Dᵢ) / cotacao(D₀)`. Sem cotação em `D₀`, ausente.
fn cotacoes(
    db: &Db,
    ticker: &str,
    datas: &[NaiveDate],
    inicio: NaiveDate,
) -> Result<Option<Fatores>, CoreError> {
    let Some(base) = db
        .cotacao_em(ticker, inicio)?
        .map(|c| c.fechamento_centavos)
        .filter(|&c| c > 0)
    else {
        return Ok(None);
    };
    let base = Decimal::from(base);
    let mut out = Vec::with_capacity(datas.len());
    for &d in datas {
        out.push(
            db.cotacao_em(ticker, d)?
                .map(|c| Decimal::from(c.fechamento_centavos) / base),
        );
    }
    Ok(Some(out))
}

/// Em centésimos, com `antes` pontos `None` na frente: o benchmark começa no ponto `antes`
/// do painel (o começo do histórico da carteira).
pub(super) fn para_centesimos(f: Option<Fatores>, antes: usize) -> Option<Vec<Option<i64>>> {
    f.map(|v| {
        std::iter::repeat_n(None, antes)
            .chain(v.into_iter().map(|x| x.map(em_centesimos)))
            .collect()
    })
}

/// Retorno acumulado do benchmark de `datas[0]` até cada data, em centésimos de ponto
/// percentual. `None` quando falta o dado em `datas[0]`; sem dado numa data, só ela fica `None`.
pub fn acumulado_benchmark(
    db: &Db,
    r: Referencia,
    datas: &[NaiveDate],
) -> Result<Option<Vec<Option<i64>>>, CoreError> {
    let Some(&fim) = datas.iter().max() else {
        return Ok(None);
    };
    let series = Series::carregar(db, fim)?;
    Ok(para_centesimos(
        fatores_benchmark(db, &series, r, datas)?,
        0,
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn codigos_e_rotulos() {
        let c: Vec<_> = Referencia::TODOS.iter().map(|r| r.codigo()).collect();
        assert_eq!(c, ["cdi", "selic", "ipca", "bova11", "xfix11"]);
        let r: Vec<_> = Referencia::TODOS.iter().map(|r| r.rotulo()).collect();
        assert_eq!(
            r,
            ["CDI", "Selic", "IPCA", "BOVA11 (Ibovespa)", "XFIX11 (IFIX)"]
        );
    }

    #[test]
    fn padrao_por_classe() {
        assert_eq!(benchmark_padrao(Classe::Rf), Referencia::Cdi);
        assert_eq!(benchmark_padrao(Classe::Acoes), Referencia::Bova11);
        assert_eq!(benchmark_padrao(Classe::Fii), Referencia::Xfix11);
        assert_eq!(benchmark_padrao(Classe::Ext), Referencia::Cdi);
        assert_eq!(benchmark_padrao(Classe::Outros), Referencia::Cdi);
    }
}
