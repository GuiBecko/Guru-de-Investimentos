//! Assistente de IA: configuração, cofre da chave, montagem do contexto, cliente da Messages API
//! com streaming e as conversas gravadas.

mod aporte;
mod cliente;
mod cofre;
mod config;
mod contexto;
mod conversa;
mod gerados;
mod sse;

pub use aporte::{DIRECOES, FocoAporte, perguntar_aporte};
pub use cliente::{ClienteFixture, ClienteHttp, ClienteIa, ErroIa, corpo_insights, corpo_pedido};
pub use cofre::{Cofre, CofreMemoria, CofreSistema, validar_formato_chave};
pub use config::{ConfigIa, ConfigSalva, MODELOS};
pub use contexto::{ContextoMontado, FonteInterna, ParteContexto, estimar_tokens};
pub use conversa::{
    Conversa, ConversaResumo, MensagemGravada, UsoMes, perguntar, remover_chave, salvar_chave,
};
pub use gerados::{
    Gerados, Insight, Noticia, PainelIa, buscar_noticias, gerar_insights, ler_insights,
    ler_noticias, rotulo_insight,
};
pub use sse::{Acumulador, Fonte, Progresso};
