mod catalogo;
mod modelo;
mod repositorio;
mod texto;
mod validacao;

pub use catalogo::{
    Benchmark, Classe, Forma, Garantia, INSTITUICOES, Indexador, Ir, Liquidez, TipoInfo, tipo,
    tipos,
};
pub use modelo::{ErroCampo, InvestimentoValido, NovoInvestimento};
pub use repositorio::{Gravacao, Investimento, Resumo, ResumoClasse};
pub use texto::{nome, rendimento};
pub use validacao::validar;
