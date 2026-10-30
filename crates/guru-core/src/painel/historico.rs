//! Patrimônio e aportado nos fins de mês, mais hoje, e os retornos entre eles.
//! Regras em "Global Constraints" do plano da fase 3b.

use chrono::{Datelike, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::{Classe, Investimento};
use crate::mercado::{Series, valorizar_com};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PontoHistorico {
    pub data: NaiveDate,
    pub patrimonio_centavos: i64,
    pub aportado_centavos: i64,
    /// Retorno do intervalo que termina neste ponto, em centésimos de ponto percentual;
    /// `None` no ponto 0 e quando não havia patrimônio no começo do intervalo.
    pub retorno_centesimos: Option<i64>,
    /// Retorno encadeado desde o ponto 0, em centésimos; o ponto 0 vale 0.
    pub acumulado_centesimos: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Historico {
    pub pontos: Vec<PontoHistorico>,
    /// Patrimônio por ponto, só das classes com algum investimento, na ordem de `Classe`.
    pub por_classe: Vec<(Classe, Vec<i64>)>,
}

/// O último dia de cada um dos `meses` meses anteriores ao de `hoje`, em ordem crescente,
/// mais `hoje`. `meses` só pode ser 6 ou 12.
pub fn datas_painel(hoje: NaiveDate, meses: u32) -> Result<Vec<NaiveDate>, CoreError> {
    if meses != 6 && meses != 12 {
        return Err(CoreError::PeriodoInvalido(meses));
    }
    let primeiro = hoje.with_day(1).ok_or(CoreError::PeriodoInvalido(meses))?;
    let mut datas = Vec::with_capacity(meses as usize + 1);
    for k in (1..=meses).rev() {
        let fim = primeiro
            .checked_sub_months(Months::new(k - 1))
            .and_then(|d| d.pred_opt())
            .ok_or(CoreError::PeriodoInvalido(meses))?;
        datas.push(fim);
    }
    datas.push(hoje);
    Ok(datas)
}

fn data_aplicacao(inv: &Investimento) -> Result<NaiveDate, CoreError> {
    NaiveDate::parse_from_str(&inv.data_aplicacao, "%Y-%m-%d")
        .map_err(|_| CoreError::DbInvalid("investimento com data inválida no banco".into()))
}

fn somar(a: i64, b: i64) -> Result<i64, CoreError> {
    a.checked_add(b)
        .ok_or_else(|| CoreError::DbInvalid("soma da carteira excede o limite".into()))
}

/// `round(num / den)` meio para cima, com `den > 0`.
fn dividir_arredondando(num: i128, den: i128) -> i128 {
    (2 * num + den).div_euclid(2 * den)
}

/// Dietz simples: `r = (V1 − V0 − F) / (V0 + F/2)` em centésimos. `None` sem patrimônio
/// no começo ou com denominador não positivo.
fn retorno_dietz(v0: i64, fluxo: i64, v1: i64) -> Option<i64> {
    if v0 <= 0 {
        return None;
    }
    let (v0, f, v1) = (i128::from(v0), i128::from(fluxo), i128::from(v1));
    let den = 2 * v0 + f;
    if den <= 0 {
        return None;
    }
    let r = dividir_arredondando(2 * (v1 - v0 - f) * 10_000, den);
    i64::try_from(r).ok()
}

/// `Π(1 + r_k) − 1` em centésimos; retorno ausente conta como fator 1.
fn encadear(acumulado: Decimal, retorno: Option<i64>) -> Option<Decimal> {
    let r = Decimal::from(retorno.unwrap_or(0)) / Decimal::from(10_000);
    acumulado.checked_mul(Decimal::ONE + r)
}

fn em_centesimos(fator: Decimal) -> i64 {
    let c = (fator - Decimal::ONE) * Decimal::from(10_000) + Decimal::new(5, 1);
    c.floor().to_i64().unwrap_or(0)
}

/// Patrimônio e aportado em cada data do painel, e os retornos entre elas. Com `classe`,
/// só entram os investimentos daquela classe.
pub fn historico(
    db: &Db,
    hoje: NaiveDate,
    meses: u32,
    classe: Option<Classe>,
) -> Result<Historico, CoreError> {
    let datas = datas_painel(hoje, meses)?;
    let series = Series::carregar(db, hoje)?;
    let investimentos: Vec<(Investimento, NaiveDate)> = db
        .listar_investimentos(hoje)?
        .into_iter()
        .filter(|i| classe.is_none_or(|c| i.classe == c))
        .map(|i| data_aplicacao(&i).map(|d| (i, d)))
        .collect::<Result<_, _>>()?;

    let classes: Vec<Classe> = Classe::TODOS
        .iter()
        .copied()
        .filter(|c| investimentos.iter().any(|(i, _)| i.classe == *c))
        .collect();
    let mut por_classe: Vec<(Classe, Vec<i64>)> =
        classes.iter().map(|c| (*c, Vec::new())).collect();

    let mut pontos: Vec<PontoHistorico> = Vec::with_capacity(datas.len());
    let mut acumulado = Decimal::ONE;
    for (idx, &d) in datas.iter().enumerate() {
        let (mut patrimonio, mut aportado, mut fluxo) = (0i64, 0i64, 0i64);
        for (_, v) in &mut por_classe {
            v.push(0);
        }
        for (inv, aplicacao) in &investimentos {
            if *aplicacao > d {
                continue;
            }
            let atual = valorizar_com(db, &series, inv, d)?.atual_centavos;
            patrimonio = somar(patrimonio, atual)?;
            aportado = somar(aportado, inv.valor_aplicado_centavos)?;
            if idx > 0 && *aplicacao > datas[idx - 1] {
                fluxo = somar(fluxo, inv.valor_aplicado_centavos)?;
            }
            if let Some(ultimo) = por_classe
                .iter_mut()
                .find(|(c, _)| *c == inv.classe)
                .and_then(|(_, v)| v.last_mut())
            {
                *ultimo = somar(*ultimo, atual)?;
            }
        }
        let retorno = match pontos.last() {
            Some(anterior) => retorno_dietz(anterior.patrimonio_centavos, fluxo, patrimonio),
            None => None,
        };
        if idx > 0 {
            acumulado = encadear(acumulado, retorno)
                .ok_or_else(|| CoreError::DbInvalid("retorno acumulado excede o limite".into()))?;
        }
        pontos.push(PontoHistorico {
            data: d,
            patrimonio_centavos: patrimonio,
            aportado_centavos: aportado,
            retorno_centesimos: retorno,
            acumulado_centesimos: em_centesimos(acumulado),
        });
    }
    Ok(Historico { pontos, por_classe })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn data(a: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(a, m, d).unwrap()
    }

    #[test]
    fn datas_de_doze_meses() {
        let d = datas_painel(data(2026, 10, 28), 12).unwrap();
        assert_eq!(d.len(), 13);
        assert_eq!(d[0], data(2025, 10, 31));
        assert_eq!(d[1], data(2025, 11, 30));
        assert_eq!(d[11], data(2026, 9, 30));
        assert_eq!(d[12], data(2026, 10, 28));
    }

    #[test]
    fn datas_de_seis_meses_cruzando_o_ano() {
        let d = datas_painel(data(2026, 3, 15), 6).unwrap();
        assert_eq!(d.len(), 7);
        assert_eq!(d[0], data(2025, 9, 30));
        assert_eq!(d[5], data(2026, 2, 28));
        assert_eq!(d[6], data(2026, 3, 15));
    }

    #[test]
    fn periodo_invalido() {
        let e = datas_painel(data(2026, 10, 28), 7).unwrap_err();
        assert_eq!(e.code(), "PERIODO_INVALIDO");
    }

    #[test]
    fn dietz_do_caso_sintetico() {
        assert_eq!(retorno_dietz(100_000, 20_000, 121_000), Some(91));
    }

    #[test]
    fn dietz_ausente_sem_patrimonio_ou_denominador() {
        assert_eq!(retorno_dietz(0, 20_000, 21_000), None);
        assert_eq!(retorno_dietz(100, -500, 0), None);
    }

    #[test]
    fn dietz_negativo_arredonda_meio_para_cima() {
        // -0,005 → -0,5 centésimo: meio para cima dá 0 (não -1).
        assert_eq!(retorno_dietz(20_000, 0, 19_999), Some(0));
        assert_eq!(retorno_dietz(100_000, 0, 90_000), Some(-1000));
    }

    #[test]
    fn acumulado_encadeia_dois_retornos_de_um_por_cento() {
        let a = encadear(Decimal::ONE, Some(100)).unwrap();
        let a = encadear(a, Some(100)).unwrap();
        assert_eq!(em_centesimos(a), 201);
        let a = encadear(a, None).unwrap();
        assert_eq!(em_centesimos(a), 201);
    }
}
