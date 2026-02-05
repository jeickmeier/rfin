//! TursoStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use finstack_core::dates::Date;
use libsql::{Builder, Connection, Database};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::OffsetDateTime;

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
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build the Turso database
        let path_str = path.to_string_lossy().into_owned();
        let db = Builder::new_local(&path_str).build().await?;

        let store = Self {
            path,
            db: Arc::new(db),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Open an in-memory Turso database (useful for testing).
    pub async fn open_in_memory() -> Result<Self> {
        let db = Builder::new_local(":memory:").build().await?;

        let store = Self {
            path: PathBuf::from(":memory:"),
            db: Arc::new(db),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get a connection from the database.
    pub(crate) fn get_conn(&self) -> Result<Connection> {
        self.db.connect().map_err(Error::Turso)
    }

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Get current schema version using PRAGMA user_version
        let mut stmt = conn.prepare("PRAGMA user_version").await?;
        let mut rows = stmt.query(()).await?;
        let current: i64 = match rows.next().await? {
            Some(row) => row.get::<i64>(0).unwrap_or(0),
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

        let migrations = migrations::migrations_for(Backend::Sqlite);

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

/// Convert metadata to JSON string.
pub(crate) fn meta_json(meta: Option<&serde_json::Value>) -> Result<String> {
    match meta {
        Some(v) => Ok(serde_json::to_string(v)?),
        None => Ok("{}".to_string()),
    }
}

/// Convert metadata to optional JSON string (for time-series where null is allowed).
pub(crate) fn meta_json_str(meta: Option<&serde_json::Value>) -> Result<Option<String>> {
    match meta {
        Some(v) => Ok(Some(serde_json::to_string(v)?)),
        None => Ok(None),
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
