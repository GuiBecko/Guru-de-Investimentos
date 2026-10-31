//! Assistente de IA: configuração, cofre da chave, montagem do contexto, cliente da Messages API
//! com streaming e as conversas gravadas.

mod cliente;
mod cofre;
mod config;
mod contexto;
mod conversa;
mod sse;

pub use cliente::{ClienteFixture, ClienteHttp, ClienteIa, ErroIa, corpo_pedido};
pub use cofre::{Cofre, CofreMemoria, CofreSistema, validar_formato_chave};
pub use config::{ConfigIa, MODELOS};
pub use contexto::{ContextoMontado, FonteInterna, ParteContexto, estimar_tokens};
pub use conversa::{
    Conversa, ConversaResumo, MensagemGravada, UsoMes, perguntar, remover_chave, salvar_chave,
};
pub use sse::{Acumulador, Fonte, Progresso};
