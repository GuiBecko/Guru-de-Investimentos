//! Tipos da ponte para metas.

use guru_core::{Meta, NovaMeta};
use napi_derive::napi;

#[napi(object)]
pub struct NovaMetaJs {
    pub nome: String,
    pub valor_alvo_centavos: Option<i64>,
    pub prazo: Option<String>,
}

#[napi(object)]
pub struct MetaJs {
    pub id: i64,
    pub nome: String,
    pub valor_alvo_centavos: i64,
    pub prazo: String,
    pub acumulado_centavos: i64,
    pub percentual_decimos: i64,
    pub meses_restantes: i64,
    pub aporte_necessario_centavos: Option<i64>,
    pub ritmo_centavos: Option<i64>,
    pub situacao: String,
    pub mensagem: String,
    pub investimentos: i64,
}

pub fn entrada(j: NovaMetaJs) -> NovaMeta {
    NovaMeta {
        nome: j.nome,
        valor_alvo_centavos: j.valor_alvo_centavos,
        prazo: j.prazo,
    }
}

pub fn meta(m: Meta) -> MetaJs {
    MetaJs {
        id: m.id,
        nome: m.nome,
        valor_alvo_centavos: m.valor_alvo_centavos,
        prazo: m.prazo,
        acumulado_centavos: m.acumulado_centavos,
        percentual_decimos: m.percentual_decimos,
        meses_restantes: m.meses_restantes,
        aporte_necessario_centavos: m.aporte_necessario_centavos,
        ritmo_centavos: m.ritmo_centavos,
        situacao: m.situacao.codigo().to_string(),
        mensagem: m.mensagem,
        investimentos: m.investimentos,
    }
}
