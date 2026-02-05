//! TursoStore struct and helper utilities.

use crate::{
    sql::schema::TableNaming,
    sql::{migrations, Backend},
    Error, Result,
};
use libsql::{Builder, Connection};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// A Turso-backed store using async operations.
///
/// This store uses libsql, an in-process SQL database engine compatible with SQLite.
/// It provides native async operations without needing a blocking runtime wrapper.
///
/// Turso offers several advantages over standard SQLite:
/// - Native JSON support with built-in JSON functions
/// - Optional encryption at rest
/// - Modern async I/O (io_uring on Linux)
/// - Vector search capabilities
#[derive(Clone)]
pub struct TursoStore {
    pub(crate) path: PathBuf,
    pub(crate) conn: Arc<Connection>,
    pub(crate) naming: Arc<TableNaming>,
}

impl std::fmt::Debug for TursoStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TursoStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl TursoStore {
    /// Open (or create) a Turso database at `path`, applying migrations.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self> {
        Self::open_with_naming(path, TableNaming::default()).await
    }

    /// Open (or create) a Turso database at `path`, applying migrations with custom table naming.
    pub async fn open_with_naming(path: impl Into<PathBuf>, naming: TableNaming) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build the Turso database
        let path_str = path.to_string_lossy().into_owned();
        let db = Builder::new_local(&path_str).build().await?;
        let conn = db.connect().map_err(Error::Turso)?;

        let store = Self {
            path,
            conn: Arc::new(conn),
            naming: Arc::new(naming),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Open an in-memory Turso database (useful for testing).
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_naming(TableNaming::default()).await
    }

    /// Open an in-memory Turso database (useful for testing) with custom table naming.
    pub async fn open_in_memory_with_naming(naming: TableNaming) -> Result<Self> {
        let db = Builder::new_local(":memory:").build().await?;
        let conn = db.connect().map_err(Error::Turso)?;

        let store = Self {
            path: PathBuf::from(":memory:"),
            conn: Arc::new(conn),
            naming: Arc::new(naming),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Table naming used by this store.
    pub fn naming(&self) -> &TableNaming {
        self.naming.as_ref()
    }

    /// Get a connection from the database.
    pub(crate) fn get_conn(&self) -> Result<Arc<Connection>> {
        Ok(Arc::clone(&self.conn))
    }

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Get current schema version using PRAGMA user_version
        let mut stmt = conn.prepare("PRAGMA user_version").await?;
        let mut rows = stmt.query(()).await?;
        let current: i64 = match rows.next().await? {
            Some(row) => row.get::<i64>(0).map_err(Error::Turso)?,
            None => 0i64,
        };
        drop(rows);
        drop(stmt);

        if current > SCHEMA_VERSION {
            return Err(Error::UnsupportedSchema {
                found: current,
                expected: SCHEMA_VERSION,
            });
        }

        if current == SCHEMA_VERSION {
            return Ok(());
        }

        let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, self.naming());

        // Begin transaction
        let tx = conn.transaction().await?;

        for (version, statements) in migrations {
            if version <= current {
                continue;
            }
            for sql in statements {
                tx.execute(&sql, ()).await?;
            }
        }

        tx.commit().await?;

        // Update schema version
        let pragma_sql = format!("PRAGMA user_version = {SCHEMA_VERSION}");
        conn.execute(&pragma_sql, ()).await?;

        Ok(())
    }
}

// Re-export common helpers with backend-specific names for compatibility
pub(crate) use crate::helpers::format_date_key as as_of_key;
pub(crate) use crate::helpers::format_timestamp_key as ts_key;
pub(crate) use crate::helpers::meta_json_optional_string as meta_json_str;
pub(crate) use crate::helpers::meta_json_string as meta_json;
pub(crate) use crate::helpers::parse_date_key as parse_as_of_key;
pub(crate) use crate::helpers::parse_timestamp_key as parse_ts_key;

/// Helper to get a string from a libsql row.
pub(crate) fn get_string(row: &libsql::Row, idx: i32) -> Result<String> {
    row.get::<String>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get string at column {idx}: {e}")))
}

/// Helper to get a blob from a libsql row.
pub(crate) fn get_blob(row: &libsql::Row, idx: i32) -> Result<Vec<u8>> {
    row.get::<Vec<u8>>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get blob at column {idx}: {e}")))
}

/// Helper to get an optional f64 from a libsql row.
pub(crate) fn get_optional_f64(row: &libsql::Row, idx: i32) -> Result<Option<f64>> {
    row.get::<Option<f64>>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get optional f64 at column {idx}: {e}")))
}

/// Helper to get an optional string from a libsql row.
pub(crate) fn get_optional_string(row: &libsql::Row, idx: i32) -> Result<Option<String>> {
    row.get::<Option<String>>(idx).map_err(|e| {
        Error::Invariant(format!(
            "Failed to get optional string at column {idx}: {e}"
        ))
    })
}
