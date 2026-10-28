mod db;
mod error;
mod investimento;

pub use db::{Db, SCHEMA_VERSION};
pub use error::CoreError;
pub use investimento::{
    Benchmark, Classe, ErroCampo, Forma, Garantia, INSTITUICOES, Indexador, InvestimentoValido, Ir,
    Liquidez, NovoInvestimento, TipoInfo, nome, rendimento, tipo, tipos, validar,
};

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
