//! SqliteStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use finstack_core::dates::Date;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
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
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open connection using tokio-rusqlite
        let conn = Connection::open(&path).await?;

        // Configure connection pragmas
        conn.call(|conn| -> tokio_rusqlite::Result<()> {
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            Ok(())
        })
        .await?;

        let store = Self {
            path,
            conn: Arc::new(conn),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Open an in-memory SQLite database (useful for testing).
    pub async fn open_in_memory() -> Result<Self> {
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
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()> {
        let schema_version = SCHEMA_VERSION;
        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let current: i64 =
                    conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
                if current > schema_version {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Unsupported schema version: found={current}, expected={schema_version}"
                    ))
                    .into());
                }

                if current == schema_version {
                    return Ok(());
                }

                let migrations = migrations::migrations_for(Backend::Sqlite);
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
    // Explicitly format as ISO 8601 to ensure lexicographic ordering works correctly.
    // Do not change this format without updating all existing data and SQL queries.
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
///
/// # Examples
/// - `2024-01-01T12:00:00.000000Z` (no fractional seconds)
/// - `2024-01-01T12:00:00.500000Z` (500ms)
/// - `2024-01-01T12:00:00.123456Z` (with microseconds)
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
    // Try RFC3339 parsing which handles both our fixed format and standard RFC3339
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format in database: {s} ({e})")))
}

/// Helper to convert rusqlite optional query result
pub(crate) fn optional_row<T>(result: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
