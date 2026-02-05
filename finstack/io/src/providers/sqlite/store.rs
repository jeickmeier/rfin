//! SqliteStore struct and helper utilities.

use crate::{
    sql::schema::TableNaming,
    sql::{migrations, Backend},
    Error, Result,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_rusqlite::Connection;

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// A SQLite-backed store using async operations.
///
/// This store wraps a single `tokio-rusqlite` connection which is thread-safe
/// and provides async access via the `call()` method. The underlying connection
/// is managed on a dedicated thread, with synchronous rusqlite operations
/// dispatched to that thread.
///
/// For concurrent access, the connection serializes requests internally.
#[derive(Clone)]
pub struct SqliteStore {
    pub(crate) path: PathBuf,
    pub(crate) conn: Arc<Connection>,
    pub(crate) naming: Arc<TableNaming>,
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path`, applying migrations.
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self> {
        Self::open_with_naming(path, TableNaming::default()).await
    }

    /// Open (or create) a SQLite database at `path`, applying migrations with custom table naming.
    pub async fn open_with_naming(path: impl Into<PathBuf>, naming: TableNaming) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open connection using tokio-rusqlite
        let conn = Connection::open(&path).await?;

        // Configure connection pragmas
        // WAL mode provides better concurrency and typically better performance for most workloads
        conn.call(|conn| -> tokio_rusqlite::Result<()> {
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch(
                "PRAGMA foreign_keys = ON;\
                 PRAGMA journal_mode = WAL;\
                 PRAGMA synchronous = NORMAL;",
            )?;
            Ok(())
        })
        .await?;

        let store = Self {
            path,
            conn: Arc::new(conn),
            naming: Arc::new(naming),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Open an in-memory SQLite database (useful for testing).
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_naming(TableNaming::default()).await
    }

    /// Open an in-memory SQLite database (useful for testing) with custom table naming.
    pub async fn open_in_memory_with_naming(naming: TableNaming) -> Result<Self> {
        let conn = Connection::open_in_memory().await?;

        // Configure connection pragmas
        conn.call(|conn| -> tokio_rusqlite::Result<()> {
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            Ok(())
        })
        .await?;

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

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()> {
        let schema_version = SCHEMA_VERSION;
        let current: i64 = self
            .conn
            .call(|conn| -> tokio_rusqlite::Result<i64> {
                Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
            })
            .await?;

        if current > schema_version {
            return Err(Error::UnsupportedSchema {
                found: current,
                expected: schema_version,
            });
        }

        if current == schema_version {
            return Ok(());
        }

        let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, self.naming());
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let tx = conn.unchecked_transaction()?;
                for (version, statements) in migrations {
                    if version <= current {
                        continue;
                    }
                    for sql in statements {
                        tx.execute_batch(&sql)?;
                    }
                }
                tx.commit()?;
                conn.pragma_update(None, "user_version", schema_version)?;
                Ok(())
            })
            .await?;
        Ok(())
    }
}

// Re-export common helpers with backend-specific names for compatibility
pub(crate) use crate::helpers::format_date_key as as_of_key;
pub(crate) use crate::helpers::format_timestamp_key as ts_key;
pub(crate) use crate::helpers::meta_json_string as meta_json;
pub(crate) use crate::helpers::parse_date_key as parse_as_of_key;
pub(crate) use crate::helpers::parse_timestamp_key as parse_ts_key;

/// Helper to convert rusqlite optional query result
pub(crate) fn optional_row<T>(result: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
