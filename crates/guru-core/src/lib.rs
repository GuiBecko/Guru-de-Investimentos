mod db;
mod error;
mod investimento;
mod mercado;
mod metas;
mod perfil;

pub use db::{Db, SCHEMA_VERSION};
pub use error::CoreError;
pub use investimento::{
    Benchmark, Classe, ErroCampo, Forma, Garantia, Gravacao, INSTITUICOES, Indexador, Investimento,
    InvestimentoValido, Ir, Liquidez, MAX_CENTAVOS, NovoInvestimento, Resumo, ResumoClasse,
    TipoInfo, nome, rendimento, tipo, tipos, validar,
};

pub use perfil::{
    AVISO_FIXO, Opcao, Perfil, PerfilGravado, Pergunta, ResultadoPerfil, alocacao_sugerida,
    calcular, perguntas,
};

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use metas::{Meta, NovaMeta, Progresso, Situacao, formatar_reais, progresso};

pub use mercado::{
    Cotacao, ErroFonte, PontoDiario, PrecoTesouro, ler_cotahist_txt, ler_cotahist_zip, ler_sgs,
    ler_tesouro,
};
