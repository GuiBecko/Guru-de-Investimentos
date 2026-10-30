//! Painel da carteira: histórico do patrimônio, benchmarks, números do topo e listas.

mod benchmark;
mod historico;
mod listas;

use chrono::NaiveDate;

use crate::db::Db;
use crate::error::CoreError;
use crate::investimento::Classe;

pub use benchmark::{Referencia, acumulado_benchmark, benchmark_padrao};
pub use historico::{Historico, PontoHistorico, datas_painel, historico};
pub use listas::{Instituicao, Variacao, Vencimento};

use crate::mercado::Series;
use benchmark::{fatores_benchmark, para_centesimos};
use historico::{com_aplicacao, em_centesimos, historico_com};

/// Números do topo do painel. Percentuais em centésimos de ponto percentual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Numeros {
    pub patrimonio_centavos: i64,
    pub aportado_centavos: i64,
    /// Retorno do último intervalo.
    pub mes_centesimos: Option<i64>,
    pub mes_cdi_centesimos: Option<i64>,
    /// Acumulado no último ponto, desde o começo do histórico (`Historico::inicio`).
    pub periodo_centesimos: i64,
    /// CDI do mesmo trecho do período que a carteira.
    pub periodo_cdi_centesimos: Option<i64>,
    /// `V_último − V₀ − Σ F` do período.
    pub rendeu_centavos: i64,
    /// Data do primeiro aporte considerado, só quando é posterior ao primeiro ponto: o período
    /// comparado começa nela, não 6/12 meses antes.
    pub desde: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Painel {
    pub datas: Vec<NaiveDate>,
    pub historico: Historico,
    pub benchmarks: Vec<(Referencia, Option<Vec<Option<i64>>>)>,
    pub numeros: Numeros,
    pub instituicoes: Vec<Instituicao>,
    pub vencimentos: Vec<Vencimento>,
    pub variacoes: Vec<Variacao>,
}

fn numeros(
    h: &Historico,
    periodo_cdi: Option<i64>,
    mes_cdi: Option<i64>,
) -> Result<Numeros, CoreError> {
    let (Some(primeiro), Some(ultimo)) = (h.pontos.first(), h.pontos.last()) else {
        return Err(CoreError::DbInvalid("histórico sem pontos".into()));
    };
    let fluxo = ultimo
        .aportado_centavos
        .checked_sub(primeiro.aportado_centavos);
    let rendeu = ultimo
        .patrimonio_centavos
        .checked_sub(primeiro.patrimonio_centavos)
        .zip(fluxo)
        .and_then(|(v, f)| v.checked_sub(f))
        .ok_or_else(|| CoreError::DbInvalid("rendimento da carteira excede o limite".into()))?;
    Ok(Numeros {
        patrimonio_centavos: ultimo.patrimonio_centavos,
        aportado_centavos: ultimo.aportado_centavos,
        mes_centesimos: ultimo.retorno_centesimos,
        mes_cdi_centesimos: mes_cdi,
        periodo_centesimos: ultimo.acumulado_centesimos,
        periodo_cdi_centesimos: periodo_cdi,
        rendeu_centavos: rendeu,
        desde: h.desde,
    })
}

/// Tudo que o painel mostra. Sem `classe`, os benchmarks são CDI, IPCA e BOVA11 (o gráfico
/// do protótipo); com `classe`, vêm os cinco, para o seletor do detalhe, e as listas só
/// consideram aquela classe. As séries e os investimentos são lidos uma vez só.
pub fn painel(
    db: &Db,
    hoje: NaiveDate,
    meses: u32,
    classe: Option<Classe>,
) -> Result<Painel, CoreError> {
    let datas = datas_painel(hoje, meses)?;
    let series = Series::carregar(db, hoje)?;
    let investimentos = com_aplicacao(db.listar_investimentos(hoje)?, classe)?;
    let historico = historico_com(db, &series, &investimentos, &datas)?;
    let referencias: &[Referencia] = if classe.is_some() {
        Referencia::TODOS
    } else {
        &[Referencia::Cdi, Referencia::Ipca, Referencia::Bova11]
    };
    // Os benchmarks partem do começo do histórico da carteira, para comparar o mesmo trecho.
    let base = historico.inicio.unwrap_or(0);
    let benchmarks = referencias
        .iter()
        .map(|&r| {
            let f = fatores_benchmark(db, &series, r, &datas[base..])?;
            Ok((r, para_centesimos(f, base)))
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    let periodo_cdi = benchmarks
        .iter()
        .find(|(r, _)| *r == Referencia::Cdi)
        .and_then(|(_, v)| v.as_ref()?.last().copied().flatten());
    // O CDI do último intervalo não depende de haver dado no começo do período.
    let mes_cdi = fatores_benchmark(db, &series, Referencia::Cdi, &datas[datas.len() - 2..])?
        .and_then(|f| f[1])
        .map(em_centesimos);
    let numeros = numeros(&historico, periodo_cdi, mes_cdi)?;

    let investimentos: Vec<_> = investimentos.into_iter().map(|(i, _)| i).collect();
    Ok(Painel {
        instituicoes: listas::instituicoes(&investimentos)?,
        vencimentos: listas::vencimentos(&investimentos, hoje),
        variacoes: listas::variacoes(db, &investimentos, &datas)?,
        datas,
        historico,
        benchmarks,
        numeros,
    })
}
