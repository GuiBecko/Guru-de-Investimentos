mod db;
mod error;
mod investimento;
mod perfil;

pub use db::{Db, SCHEMA_VERSION};
pub use error::CoreError;
pub use investimento::{
    Benchmark, Classe, ErroCampo, Forma, Garantia, Gravacao, INSTITUICOES, Indexador, Investimento,
    InvestimentoValido, Ir, Liquidez, NovoInvestimento, Resumo, ResumoClasse, TipoInfo, nome,
    rendimento, tipo, tipos, validar,
};

pub use perfil::{
    AVISO_FIXO, Opcao, Perfil, Pergunta, ResultadoPerfil, alocacao_sugerida, calcular, perguntas,
};

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
