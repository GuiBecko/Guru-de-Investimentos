//! Importação de posições: planilha da B3 (`.xlsx`) e extrato da BTG (PDF).

mod aplicacao;
mod btg;
mod comparacao;
mod comparacao_btg;
mod instituicao;
pub mod pdf;
mod posicao;
mod xlsx;

pub use aplicacao::{Decisao, ResultadoImportacao, aplicar_importacao, ultima_importacao};
pub use btg::{AtivoBtg, ExtratoBtg, ler_extrato, ler_extrato_linhas};
pub use comparacao::{Acao, Comparacao, Estado, Fonte, Item, comparar};
pub use comparacao_btg::comparar_btg;
pub use posicao::{AbaRv, LinhaB3, Posicao, ler_posicao, ler_posicao_abas};
pub use xlsx::{Aba, Celula, ler_xlsx};
