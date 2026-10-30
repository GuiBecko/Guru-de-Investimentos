use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error(
        "O banco de dados foi criado por uma versão mais nova do Guru de Investimentos (esquema {banco}, este app entende até {app}). Atualize o aplicativo."
    )]
    SchemaTooNew { banco: u32, app: u32 },
    #[error("O arquivo do banco de dados não é válido: {0}")]
    DbInvalid(String),
    #[error("Erro de leitura ou gravação em disco: {0}")]
    Io(#[from] std::io::Error),
    #[error("Erro no banco de dados: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Este investimento não existe mais. Atualize a lista.")]
    NaoEncontrado(i64),
    #[error("Esta meta não existe mais. Atualize a lista.")]
    MetaNaoEncontrada(i64),
    #[error("Período inválido: use 6 ou 12 meses.")]
    PeriodoInvalido(u32),
    #[error("{0}")]
    ArquivoInvalido(String),
    #[error("{0}")]
    DecisaoInvalida(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::SchemaTooNew { .. } => "SCHEMA_TOO_NEW",
            CoreError::DbInvalid(_) => "DB_INVALID",
            CoreError::Io(_) => "IO",
            CoreError::Sqlite(_) => "SQLITE",
            CoreError::NaoEncontrado(_) | CoreError::MetaNaoEncontrada(_) => "NAO_ENCONTRADO",
            CoreError::PeriodoInvalido(_) => "PERIODO_INVALIDO",
            CoreError::ArquivoInvalido(_) => "ARQUIVO_INVALIDO",
            CoreError::DecisaoInvalida(_) => "DECISAO_INVALIDA",
        }
    }
}
