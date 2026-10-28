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
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::SchemaTooNew { .. } => "SCHEMA_TOO_NEW",
            CoreError::DbInvalid(_) => "DB_INVALID",
            CoreError::Io(_) => "IO",
            CoreError::Sqlite(_) => "SQLITE",
        }
    }
}
