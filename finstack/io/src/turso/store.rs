//! TursoStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use finstack_core::dates::Date;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::runtime::Runtime;
use turso::{Builder, Connection, Database};

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// A Turso-backed store.
///
/// This store uses Turso, an in-process SQL database engine compatible with SQLite.
/// It wraps the async Turso API with a blocking runtime for compatibility with the
/// synchronous `Store` trait.
///
/// Turso offers several advantages over standard SQLite:
/// - Native JSON support with built-in JSON functions
/// - Optional encryption at rest
/// - Modern async I/O (io_uring on Linux)
/// - Vector search capabilities
#[derive(Clone)]
pub struct TursoStore {
    pub(crate) path: PathBuf,
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) db: Arc<Database>,
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
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create a tokio runtime for blocking on async operations
        let runtime = Runtime::new()
            .map_err(|e| Error::Invariant(format!("Failed to create tokio runtime: {e}")))?;

        // Build the Turso database
        let path_str = path.to_string_lossy();
        let db = runtime.block_on(async move {
            Builder::new_local(&path_str)
                .build()
                .await
                .map_err(Error::Turso)
        })?;

        let store = Self {
            path,
            runtime: Arc::new(runtime),
            db: Arc::new(db),
        };

        // Run migrations
        store.with_conn(migrate)?;

        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Execute a function with a database connection.
    pub(crate) fn with_conn<R>(
        &self,
        f: impl FnOnce(&mut Connection, &Runtime) -> Result<R>,
    ) -> Result<R> {
        let mut conn = self.db.connect().map_err(Error::Turso)?;
        f(&mut conn, &self.runtime)
    }
}

/// Run migrations on the database.
pub(crate) fn migrate(conn: &mut Connection, runtime: &Runtime) -> Result<()> {
    // Run everything in a single async block to avoid ownership issues
    let migrations_list = migrations::migrations_for(Backend::Sqlite);

    runtime.block_on(async move {
        // Get current schema version using PRAGMA user_version
        let mut stmt = conn
            .prepare("PRAGMA user_version")
            .await
            .map_err(Error::Turso)?;
        let mut rows = stmt.query(()).await.map_err(Error::Turso)?;
        let current: i64 = match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let val = row.get_value(0).map_err(Error::Turso)?;
                match val {
                    turso::value::Value::Integer(i) => i,
                    _ => 0i64,
                }
            }
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

        // Begin transaction
        let tx = conn.transaction().await.map_err(Error::Turso)?;

        for (version, statements) in migrations_list {
            if version <= current {
                continue;
            }
            for sql in statements {
                tx.execute(&sql, ()).await.map_err(Error::Turso)?;
            }
        }

        tx.commit().await.map_err(Error::Turso)?;

        // Update schema version
        let pragma_sql = format!("PRAGMA user_version = {SCHEMA_VERSION}");
        conn.execute(&pragma_sql, ()).await.map_err(Error::Turso)?;

        Ok(())
    })
}

/// Convert metadata to JSON string.
pub(crate) fn meta_json(meta: Option<&serde_json::Value>) -> Result<String> {
    match meta {
        Some(v) => Ok(serde_json::to_string(v)?),
        None => Ok("{}".to_string()),
    }
}

/// Format a date as ISO 8601 (YYYY-MM-DD) for use as a database key.
///
/// This format is critical for correct lexicographic ordering in SQL `BETWEEN` queries.
pub(crate) fn as_of_key(as_of: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        as_of.year(),
        as_of.month() as u8,
        as_of.day()
    )
}

/// Parse a date from ISO 8601 (YYYY-MM-DD) format.
pub(crate) fn parse_as_of_key(s: &str) -> Result<Date> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DATE)
        .map_err(|e| Error::Invariant(format!("Invalid date format in database: {s} ({e})")))
}

/// Format a timestamp as a fixed-width ISO 8601 string for use as a database key.
///
/// Uses format: `YYYY-MM-DDTHH:MM:SS.ffffffZ` (always 27 characters)
///
/// This fixed-width format is critical for correct lexicographic ordering in SQL queries.
/// Unlike RFC3339, which omits fractional seconds when they are zero, this format always
/// includes 6 decimal places for microseconds, ensuring consistent string width and
/// correct chronological ordering when sorted lexicographically.
pub(crate) fn ts_key(ts: OffsetDateTime) -> Result<String> {
    // Convert to UTC for consistent storage
    let ts_utc = ts.to_offset(time::UtcOffset::UTC);
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:06}Z",
        ts_utc.year(),
        ts_utc.month() as u8,
        ts_utc.day(),
        ts_utc.hour(),
        ts_utc.minute(),
        ts_utc.second(),
        ts_utc.microsecond(),
    ))
}

/// Parse a timestamp from a fixed-width ISO 8601 string.
///
/// Accepts format: `YYYY-MM-DDTHH:MM:SS.ffffffZ`
///
/// Also accepts RFC3339 format for backwards compatibility with existing data.
pub(crate) fn parse_ts_key(s: &str) -> Result<OffsetDateTime> {
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format in database: {s} ({e})")))
}
