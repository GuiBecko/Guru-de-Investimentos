//! Tipos da ponte para o painel e a tarefa assíncrona que o calcula. Os campos `Option`
//! chegam ao JS como `null` (`use_nullable`), para o renderer checar com `=== null`.

use std::path::PathBuf;

use chrono::NaiveDate;
use guru_core::{Classe, CoreError, Db, Painel};
use napi::JsValue;
use napi::bindgen_prelude::{AsyncTask, Env, Task};
use napi_derive::napi;

#[napi(object, use_nullable = true)]
pub struct PontoPainelJs {
    pub patrimonio_centavos: i64,
    pub aportado_centavos: i64,
    pub retorno_centesimos: Option<i64>,
    pub acumulado_centesimos: i64,
}

#[napi(object)]
pub struct SerieClasseJs {
    pub classe: String,
    pub rotulo: String,
    pub patrimonio: Vec<i64>,
}

#[napi(object, use_nullable = true)]
pub struct SerieBenchmarkJs {
    pub codigo: String,
    pub rotulo: String,
    pub valores: Option<Vec<Option<i64>>>,
}

#[napi(object, use_nullable = true)]
pub struct NumerosPainelJs {
    pub patrimonio_centavos: i64,
    pub aportado_centavos: i64,
    pub mes_centesimos: Option<i64>,
    pub mes_cdi_centesimos: Option<i64>,
    pub periodo_centesimos: i64,
    pub periodo_cdi_centesimos: Option<i64>,
    pub rendeu_centavos: i64,
    /// `AAAA-MM-DD` do primeiro aporte quando ele é posterior ao primeiro ponto; senão `null`.
    pub desde: Option<String>,
}

#[napi(object)]
pub struct InstituicaoJs {
    pub nome: String,
    pub atual_centavos: i64,
    pub percentual_decimos: i64,
}

#[napi(object)]
pub struct VencimentoJs {
    pub id: i64,
    pub nome: String,
    pub vencimento: String,
    pub atual_centavos: i64,
}

#[napi(object)]
pub struct VariacaoJs {
    pub ticker: String,
    pub variacao_decimos: i64,
}

/// `benchmark_padrao` só existe com classe; sem ela o campo some (`undefined`).
#[napi(object)]
pub struct PainelJs {
    pub datas: Vec<String>,
    pub pontos: Vec<PontoPainelJs>,
    pub por_classe: Vec<SerieClasseJs>,
    pub benchmarks: Vec<SerieBenchmarkJs>,
    pub numeros: NumerosPainelJs,
    pub instituicoes: Vec<InstituicaoJs>,
    pub vencimentos: Vec<VencimentoJs>,
    pub variacoes: Vec<VariacaoJs>,
    pub benchmark_padrao: Option<String>,
}

fn para_js(p: Painel, classe: Option<Classe>) -> PainelJs {
    PainelJs {
        datas: p.datas.iter().map(|d| d.to_string()).collect(),
        pontos: p
            .historico
            .pontos
            .into_iter()
            .map(|x| PontoPainelJs {
                patrimonio_centavos: x.patrimonio_centavos,
                aportado_centavos: x.aportado_centavos,
                retorno_centesimos: x.retorno_centesimos,
                acumulado_centesimos: x.acumulado_centesimos,
            })
            .collect(),
        por_classe: p
            .historico
            .por_classe
            .into_iter()
            .map(|(c, patrimonio)| SerieClasseJs {
                classe: c.codigo().to_string(),
                rotulo: c.rotulo().to_string(),
                patrimonio,
            })
            .collect(),
        benchmarks: p
            .benchmarks
            .into_iter()
            .map(|(r, valores)| SerieBenchmarkJs {
                codigo: r.codigo().to_string(),
                rotulo: r.rotulo().to_string(),
                valores,
            })
            .collect(),
        numeros: NumerosPainelJs {
            patrimonio_centavos: p.numeros.patrimonio_centavos,
            aportado_centavos: p.numeros.aportado_centavos,
            mes_centesimos: p.numeros.mes_centesimos,
            mes_cdi_centesimos: p.numeros.mes_cdi_centesimos,
            periodo_centesimos: p.numeros.periodo_centesimos,
            periodo_cdi_centesimos: p.numeros.periodo_cdi_centesimos,
            rendeu_centavos: p.numeros.rendeu_centavos,
            desde: p.numeros.desde.map(|d| d.to_string()),
        },
        instituicoes: p
            .instituicoes
            .into_iter()
            .map(|i| InstituicaoJs {
                nome: i.nome,
                atual_centavos: i.atual_centavos,
                percentual_decimos: i.percentual_decimos,
            })
            .collect(),
        vencimentos: p
            .vencimentos
            .into_iter()
            .map(|v| VencimentoJs {
                id: v.id,
                nome: v.nome,
                vencimento: v.vencimento.to_string(),
                atual_centavos: v.atual_centavos,
            })
            .collect(),
        variacoes: p
            .variacoes
            .into_iter()
            .map(|v| VariacaoJs {
                ticker: v.ticker,
                variacao_decimos: v.variacao_decimos,
            })
            .collect(),
        benchmark_padrao: classe.map(|c| guru_core::benchmark_padrao(c).codigo().to_string()),
    }
}

/// Cálculo do painel na threadpool do libuv, como `AtualizarMercado`: com a carteira grande
/// ele leva dezenas de milissegundos, que não podem travar o processo principal. A tarefa abre
/// a própria conexão (`Db::open`), então o `Mutex` de `Core` não fica preso enquanto calcula;
/// a data de mercado é capturada antes, na thread do JS.
pub struct CalcularPainel {
    pub pasta_dados: PathBuf,
    pub hoje: NaiveDate,
    pub meses: u32,
    pub classe: Option<String>,
}

impl CalcularPainel {
    fn classe(&self) -> Result<Option<Classe>, (String, String)> {
        self.classe
            .as_deref()
            .map(|c| {
                Classe::de_codigo(c).ok_or_else(|| {
                    (
                        "NAO_ENCONTRADO".to_string(),
                        format!("Classe desconhecida: {c}."),
                    )
                })
            })
            .transpose()
    }

    fn executar(&self, classe: Option<Classe>) -> Result<Painel, CoreError> {
        let db = Db::open(&self.pasta_dados)?;
        guru_core::painel(&db, self.hoje, self.meses, classe)
    }
}

impl Task for CalcularPainel {
    type Output = Result<(Painel, Option<Classe>), (String, String)>;
    type JsValue = PainelJs;

    fn compute(&mut self) -> napi::Result<Self::Output> {
        Ok(self.classe().and_then(|classe| {
            self.executar(classe)
                .map(|p| (p, classe))
                .map_err(|e| (e.code().to_string(), e.to_string()))
        }))
    }

    // Mesma rejeição com `code` de `AtualizarMercado::resolve`.
    fn resolve(&mut self, env: Env, saida: Self::Output) -> napi::Result<Self::JsValue> {
        match saida {
            Ok((p, classe)) => Ok(para_js(p, classe)),
            Err((code, mensagem)) => {
                let mut erro =
                    env.create_error(napi::Error::new(napi::Status::GenericFailure, mensagem))?;
                erro.set("code", code)?;
                Err(napi::Error::from(erro.to_unknown()))
            }
        }
    }
}

pub fn tarefa(
    pasta_dados: PathBuf,
    hoje: NaiveDate,
    meses: u32,
    classe: Option<String>,
) -> AsyncTask<CalcularPainel> {
    AsyncTask::new(CalcularPainel {
        pasta_dados,
        hoje,
        meses,
        classe,
    })
}
