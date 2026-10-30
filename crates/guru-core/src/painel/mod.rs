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

use benchmark::{Fatores, fatores_benchmark, para_centesimos};
use historico::em_centesimos;

/// Números do topo do painel. Percentuais em centésimos de ponto percentual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Numeros {
    pub patrimonio_centavos: i64,
    pub aportado_centavos: i64,
    /// Retorno do último intervalo.
    pub mes_centesimos: Option<i64>,
    pub mes_cdi_centesimos: Option<i64>,
    /// Acumulado no último ponto.
    pub periodo_centesimos: i64,
    pub periodo_cdi_centesimos: Option<i64>,
    /// `V_último − V₀ − Σ F` do período.
    pub rendeu_centavos: i64,
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
    cdi: Option<&Fatores>,
    cdi_ultimo_mes: Option<&Fatores>,
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
    let periodo_cdi = cdi
        .and_then(|f| f.last().copied().flatten())
        .map(em_centesimos);
    let mes_cdi = cdi_ultimo_mes.and_then(|f| f[1]).map(em_centesimos);
    Ok(Numeros {
        patrimonio_centavos: ultimo.patrimonio_centavos,
        aportado_centavos: ultimo.aportado_centavos,
        mes_centesimos: ultimo.retorno_centesimos,
        mes_cdi_centesimos: mes_cdi,
        periodo_centesimos: ultimo.acumulado_centesimos,
        periodo_cdi_centesimos: periodo_cdi,
        rendeu_centavos: rendeu,
    })
}

/// Tudo que o painel mostra. Sem `classe`, os benchmarks são CDI, IPCA e BOVA11 (o gráfico
/// do protótipo); com `classe`, vêm os cinco, para o seletor do detalhe, e as listas só
/// consideram aquela classe.
pub fn painel(
    db: &Db,
    hoje: NaiveDate,
    meses: u32,
    classe: Option<Classe>,
) -> Result<Painel, CoreError> {
    let datas = datas_painel(hoje, meses)?;
    let historico = historico(db, hoje, meses, classe)?;
    let referencias: &[Referencia] = if classe.is_some() {
        Referencia::TODOS
    } else {
        &[Referencia::Cdi, Referencia::Ipca, Referencia::Bova11]
    };
    let benchmarks = referencias
        .iter()
        .map(|&r| Ok((r, para_centesimos(fatores_benchmark(db, r, &datas)?))))
        .collect::<Result<Vec<_>, CoreError>>()?;
    let cdi = fatores_benchmark(db, Referencia::Cdi, &datas)?;
    // O CDI do último intervalo não depende de haver dado no primeiro ponto do período.
    let cdi_ultimo_mes = fatores_benchmark(db, Referencia::Cdi, &datas[datas.len() - 2..])?;
    let numeros = numeros(&historico, cdi.as_ref(), cdi_ultimo_mes.as_ref())?;

    let investimentos: Vec<_> = db
        .listar_investimentos(hoje)?
        .into_iter()
        .filter(|i| classe.is_none_or(|c| i.classe == c))
        .collect();
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
