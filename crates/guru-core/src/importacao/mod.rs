//! Leitura da planilha de posição exportada pela B3 (formato `.xlsx`).

mod aplicacao;
mod comparacao;
mod posicao;
mod xlsx;

pub use aplicacao::{Decisao, ResultadoImportacao, aplicar_importacao, ultima_importacao};
pub use comparacao::{Acao, Comparacao, Estado, Item, comparar};
pub use posicao::{AbaRv, LinhaB3, Posicao, ler_posicao, ler_posicao_abas};
pub use xlsx::{Aba, Celula, ler_xlsx};
