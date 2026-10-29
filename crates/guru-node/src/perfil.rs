//! Tipos da ponte para perfil, alocação-alvo e objetivo.
//! A interface nunca recebe a pontuação das opções.

use guru_core::{AVISO_FIXO, Classe, Perfil, PerfilGravado, perguntas};
use napi_derive::napi;

#[napi(object)]
pub struct PerguntaJs {
    pub numero: u32,
    pub bloco: String,
    pub texto: String,
    pub porque: String,
    pub opcoes: Vec<String>,
}

#[napi(object)]
pub struct QuestionarioJs {
    pub perguntas: Vec<PerguntaJs>,
    pub avisos_fixos: Vec<String>,
}

#[napi(object)]
pub struct AlocacaoClasseJs {
    pub classe: String,
    pub rotulo: String,
    pub percentual: u32,
}

#[napi(object)]
pub struct PercentualClasseJs {
    pub classe: String,
    pub percentual: i64,
}

#[napi(object)]
pub struct PerfilJs {
    pub perfil: String,
    pub perfil_rotulo: String,
    pub descricao: String,
    pub avisos: Vec<String>,
    pub feito_em: String,
    pub vencido: bool,
    pub alocacao_sugerida: Vec<AlocacaoClasseJs>,
}

#[napi(object)]
pub struct ResultadoPerfilJs {
    pub perfil: Option<PerfilJs>,
    pub erros: Vec<crate::investimentos::ErroCampoJs>,
}

pub fn questionario() -> QuestionarioJs {
    QuestionarioJs {
        perguntas: perguntas()
            .iter()
            .map(|p| PerguntaJs {
                numero: p.numero.into(),
                bloco: p.bloco.to_string(),
                texto: p.texto.to_string(),
                porque: p.porque.to_string(),
                opcoes: p.opcoes.iter().map(|o| o.texto.to_string()).collect(),
            })
            .collect(),
        avisos_fixos: AVISO_FIXO.iter().map(|s| s.to_string()).collect(),
    }
}

pub fn alocacao(lista: Vec<(Classe, u8)>) -> Vec<AlocacaoClasseJs> {
    lista
        .into_iter()
        .map(|(c, p)| AlocacaoClasseJs {
            classe: c.codigo().to_string(),
            rotulo: c.rotulo().to_string(),
            percentual: p.into(),
        })
        .collect()
}

pub fn perfil(g: PerfilGravado) -> PerfilJs {
    let p: Perfil = g.perfil;
    PerfilJs {
        perfil: p.codigo().to_string(),
        perfil_rotulo: p.rotulo().to_string(),
        descricao: p.descricao().to_string(),
        avisos: g.avisos,
        feito_em: g.feito_em,
        vencido: g.vencido,
        alocacao_sugerida: alocacao(g.alocacao_sugerida),
    }
}
