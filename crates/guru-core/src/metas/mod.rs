//! Metas financeiras: validação, progresso contra o alvo e repositório.

mod modelo;
mod progresso;
mod repositorio;
mod validacao;

pub use modelo::{Meta, NovaMeta, Situacao};
pub use progresso::{Progresso, formatar_reais, progresso};
