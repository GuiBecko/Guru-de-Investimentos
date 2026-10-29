mod catalogo;
mod modelo;
mod repositorio;
mod texto;
mod validacao;

pub(crate) use catalogo::enum_catalogo;
pub use catalogo::{
    Benchmark, Classe, Forma, Garantia, INSTITUICOES, Indexador, Ir, Liquidez, TipoInfo, tipo,
    tipos,
};
pub use modelo::{ErroCampo, InvestimentoValido, NovoInvestimento};
pub use repositorio::{Gravacao, Investimento, Resumo, ResumoClasse};
pub use texto::{nome, rendimento};
pub use validacao::validar;
