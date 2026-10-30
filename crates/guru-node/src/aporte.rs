//! Tipos da ponte para o próximo aporte.

use guru_core::{Aporte, ErroCampo, ParteAporte, ResultadoAporte};
use napi_derive::napi;

use crate::investimentos::{ErroCampoJs, erros};

#[napi(object, use_nullable = true)]
pub struct ParteAporteJs {
    pub classe: String,
    pub rotulo: String,
    pub atual_centavos: i64,
    pub alvo_percentual: u32,
    pub sugerido_centavos: i64,
    pub percentual_atual_decimos: i64,
    pub percentual_depois_decimos: i64,
    pub situacao: String,
}

#[napi(object, use_nullable = true)]
pub struct AporteJs {
    pub valor_centavos: i64,
    pub patrimonio_centavos: i64,
    pub partes: Vec<ParteAporteJs>,
}

#[napi(object, use_nullable = true)]
pub struct ResultadoAporteJs {
    /// "sem_alvo" | "invalido" | "dividido"
    pub estado: String,
    pub erros: Vec<ErroCampoJs>,
    pub aporte: Option<AporteJs>,
}

fn parte(p: ParteAporte) -> ParteAporteJs {
    ParteAporteJs {
        classe: p.classe.codigo().to_string(),
        rotulo: p.classe.rotulo().to_string(),
        atual_centavos: p.atual_centavos,
        alvo_percentual: u32::from(p.alvo_percentual),
        sugerido_centavos: p.sugerido_centavos,
        percentual_atual_decimos: p.percentual_atual_decimos,
        percentual_depois_decimos: p.percentual_depois_decimos,
        situacao: p.situacao.codigo().to_string(),
    }
}

fn aporte(a: Aporte) -> AporteJs {
    AporteJs {
        valor_centavos: a.valor_centavos,
        patrimonio_centavos: a.patrimonio_centavos,
        partes: a.partes.into_iter().map(parte).collect(),
    }
}

pub fn resultado(r: ResultadoAporte) -> ResultadoAporteJs {
    let (estado, lista, ap): (&str, Vec<ErroCampo>, Option<AporteJs>) = match r {
        ResultadoAporte::SemAlvo => ("sem_alvo", vec![], None),
        ResultadoAporte::Invalido(e) => ("invalido", e, None),
        ResultadoAporte::Dividido(a) => ("dividido", vec![], Some(aporte(a))),
    };
    ResultadoAporteJs {
        estado: estado.to_string(),
        erros: erros(lista),
        aporte: ap,
    }
}
