mod catalogo;
mod modelo;
mod texto;
mod validacao;

pub use catalogo::{
    Benchmark, Classe, Forma, Garantia, INSTITUICOES, Indexador, Ir, Liquidez, TipoInfo, tipo,
    tipos,
};
pub use modelo::{ErroCampo, InvestimentoValido, NovoInvestimento};
pub use texto::{nome, rendimento};
pub use validacao::validar;
