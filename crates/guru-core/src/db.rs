use std::path::{Path, PathBuf};

use rusqlite::{Connection, ErrorCode};
use rusqlite_migration::{M, Migrations};

use crate::error::CoreError;

const MIGRATIONS_SQL: &[&str] = &[include_str!("migrations/0001_inicial.sql")];

/// Versão do esquema: número de migrações conhecidas por este app.
pub const SCHEMA_VERSION: u32 = MIGRATIONS_SQL.len() as u32;

pub struct Db {
    conn: Connection,
    path: PathBuf,
}

impl Db {
    pub fn open(data_dir: &Path) -> Result<Db, CoreError> {
        std::fs::create_dir_all(data_dir)?;
        let path = data_dir.join("guru.db");
        let mut conn = Connection::open(&path)?;

        let versao_arquivo = user_version(&conn)?;
        if versao_arquivo > SCHEMA_VERSION {
            return Err(CoreError::SchemaTooNew {
                banco: versao_arquivo,
                app: SCHEMA_VERSION,
            });
        }

        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))
            .map_err(classifica)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let migrations = Migrations::new(MIGRATIONS_SQL.iter().map(|sql| M::up(sql)).collect());
        migrations
            .to_latest(&mut conn)
            .map_err(|e| CoreError::DbInvalid(e.to_string()))?;

        Ok(Db { conn, path })
    }

    pub fn schema_version(&self) -> Result<u32, CoreError> {
        user_version(&self.conn)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn user_version(conn: &Connection) -> Result<u32, CoreError> {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
        .map_err(classifica)
}

/// Arquivo que não é SQLite vira `DbInvalid`; o resto fica como `Sqlite`.
fn classifica(e: rusqlite::Error) -> CoreError {
    match e.sqlite_error_code() {
        Some(ErrorCode::NotADatabase) => CoreError::DbInvalid(
            "o arquivo guru.db não é um banco SQLite (pode estar corrompido)".to_string(),
        ),
        _ => CoreError::Sqlite(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usa_wal_e_chaves_estrangeiras() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(dir.path()).unwrap();
        let modo: String = db
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        let fk: i64 = db
            .conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(modo, "wal");
        assert_eq!(fk, 1);
    }
}
