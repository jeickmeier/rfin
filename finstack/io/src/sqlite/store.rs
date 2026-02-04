//! SqliteStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use finstack_core::dates::Date;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// A SQLite-backed store.
///
/// This store is `Send + Sync` because it opens a new SQLite connection per call.
/// For bulk operations, prefer adding higher-level methods that reuse a single
/// connection internally.
#[derive(Clone, Debug)]
pub struct SqliteStore {
    pub(crate) path: PathBuf,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path`, applying migrations.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let store = Self { path };
        store.with_conn(migrate)?;
        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn with_conn<R>(&self, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
        let conn = open_conn(&self.path)?;
        f(&conn)
    }
}

pub(crate) fn open_conn(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
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
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
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

pub(crate) fn ts_key(ts: OffsetDateTime) -> Result<String> {
    ts.format(&Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format: {e}")))
}

pub(crate) fn parse_ts_key(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format in database: {s} ({e})")))
}
