//! Assistente de IA: configuração, cofre da chave e montagem do contexto enviado ao Claude.

mod cofre;
mod config;
mod contexto;

pub use cofre::{Cofre, CofreMemoria, CofreSistema, validar_formato_chave};
pub use config::{ConfigIa, MODELOS};
pub use contexto::{ContextoMontado, FonteInterna, ParteContexto, estimar_tokens};
