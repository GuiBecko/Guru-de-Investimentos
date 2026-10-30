//! Leitura da planilha de posição exportada pela B3 (formato `.xlsx`).

mod xlsx;

pub use xlsx::{Aba, Celula, ler_xlsx};
