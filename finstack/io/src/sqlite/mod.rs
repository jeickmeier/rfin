//! SQLite backend for `finstack-io`.
//!
//! This module provides a minimal, predictable schema with JSON payload blobs
//! for domain objects, indexed by `(id, as_of)` where applicable.

use crate::{
    BulkStore, Error, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result, Store,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SCHEMA_VERSION: i64 = 2;

// Pre-defined SQL statements to avoid runtime format! allocations.
// Timestamp format: RFC3339-ish without timezone offsets (UTC 'Z').
const PUT_MARKET_CONTEXT_SQL: &str = concat!(
    "INSERT INTO market_contexts (id, as_of, payload, meta) VALUES (?1, ?2, ?3, ?4) ",
    "ON CONFLICT(id, as_of) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const PUT_INSTRUMENT_SQL: &str = concat!(
    "INSERT INTO instruments (id, payload, meta) VALUES (?1, ?2, ?3) ",
    "ON CONFLICT(id) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const PUT_PORTFOLIO_SQL: &str = concat!(
    "INSERT INTO portfolios (id, as_of, payload, meta) VALUES (?1, ?2, ?3, ?4) ",
    "ON CONFLICT(id, as_of) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const PUT_SCENARIO_SQL: &str = concat!(
    "INSERT INTO scenarios (id, payload, meta) VALUES (?1, ?2, ?3) ",
    "ON CONFLICT(id) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const PUT_STATEMENT_MODEL_SQL: &str = concat!(
    "INSERT INTO statement_models (id, payload, meta) VALUES (?1, ?2, ?3) ",
    "ON CONFLICT(id) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const PUT_METRIC_REGISTRY_SQL: &str = concat!(
    "INSERT INTO metric_registries (namespace, payload, meta) VALUES (?1, ?2, ?3) ",
    "ON CONFLICT(namespace) DO UPDATE SET ",
    "payload = excluded.payload, ",
    "meta = excluded.meta, ",
    "updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')"
);

const SCHEMA_SQL_V1: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS instruments (
  id TEXT PRIMARY KEY NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS portfolios (
  id TEXT NOT NULL,
  as_of TEXT NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY (id, as_of)
);

CREATE INDEX IF NOT EXISTS idx_portfolios_as_of ON portfolios(as_of);

CREATE TABLE IF NOT EXISTS market_contexts (
  id TEXT NOT NULL,
  as_of TEXT NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  PRIMARY KEY (id, as_of)
);

CREATE INDEX IF NOT EXISTS idx_market_contexts_as_of ON market_contexts(as_of);

CREATE TABLE IF NOT EXISTS scenarios (
  id TEXT PRIMARY KEY NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS statement_models (
  id TEXT PRIMARY KEY NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS metric_registries (
  namespace TEXT PRIMARY KEY NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

COMMIT;
"#;

const SCHEMA_SQL_V2_UPGRADE: &str = r#"
BEGIN;

CREATE TABLE IF NOT EXISTS metric_registries (
  namespace TEXT PRIMARY KEY NOT NULL,
  payload BLOB NOT NULL,
  meta TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

COMMIT;
"#;

/// A SQLite-backed store.
///
/// This store is `Send + Sync` because it opens a new SQLite connection per call.
/// For bulk operations, prefer adding higher-level methods that reuse a single
/// connection internally.
#[derive(Clone, Debug)]
pub struct SqliteStore {
    path: PathBuf,
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

    fn with_conn<R>(&self, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
        let conn = open_conn(&self.path)?;
        f(&conn)
    }
}

fn open_conn(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match current {
        0 => {
            // Fresh database: apply latest schema (includes all tables)
            conn.execute_batch(SCHEMA_SQL_V1)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            Ok(())
        }
        1 => {
            // Upgrade from v1 to v2: add metric_registries table
            conn.execute_batch(SCHEMA_SQL_V2_UPGRADE)?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            Ok(())
        }
        SCHEMA_VERSION => Ok(()),
        found => Err(Error::UnsupportedSchema {
            found,
            expected: SCHEMA_VERSION,
        }),
    }
}

fn meta_json(meta: Option<&serde_json::Value>) -> Result<String> {
    match meta {
        Some(v) => Ok(serde_json::to_string(v)?),
        None => Ok("{}".to_string()),
    }
}

/// Format a date as ISO 8601 (YYYY-MM-DD) for use as a database key.
///
/// This format is critical for correct lexicographic ordering in SQL `BETWEEN` queries.
fn as_of_key(as_of: Date) -> String {
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
fn parse_as_of_key(s: &str) -> Result<Date> {
    Date::parse(s, &time::format_description::well_known::Iso8601::DATE)
        .map_err(|e| Error::Invariant(format!("Invalid date format in database: {s} ({e})")))
}

impl Store for SqliteStore {
    fn put_market_context(
        &self,
        market_id: &str,
        as_of: Date,
        context: &MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let state: MarketContextState = context.into();
        let payload = serde_json::to_vec(&state)?;
        let meta = meta_json(meta)?;
        let as_of = as_of_key(as_of);

        self.with_conn(|conn| {
            conn.execute(
                PUT_MARKET_CONTEXT_SQL,
                params![market_id, as_of, payload, meta],
            )?;
            Ok(())
        })
    }

    fn get_market_context(&self, market_id: &str, as_of: Date) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM market_contexts WHERE id = ?1 AND as_of = ?2",
                    params![market_id, as_of],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => {
                    let state: MarketContextState = serde_json::from_slice(&bytes)?;
                    let ctx = MarketContext::try_from(state)?;
                    Ok(Some(ctx))
                }
                None => Ok(None),
            }
        })
    }

    fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(instrument)?;
        let meta = meta_json(meta)?;

        self.with_conn(|conn| {
            conn.execute(PUT_INSTRUMENT_SQL, params![instrument_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM instruments WHERE id = ?1",
                    params![instrument_id],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<InstrumentJson>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<HashMap<String, InstrumentJson>> {
        if instrument_ids.is_empty() {
            return Ok(HashMap::new());
        }

        self.with_conn(|conn| {
            // Build a parameterized query with the right number of placeholders
            let placeholders: Vec<&str> = (0..instrument_ids.len()).map(|_| "?").collect();
            let sql = format!(
                "SELECT id, payload FROM instruments WHERE id IN ({})",
                placeholders.join(", ")
            );

            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::ToSql> = instrument_ids
                .iter()
                .map(|s| s as &dyn rusqlite::ToSql)
                .collect();

            let rows = stmt.query_map(params.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;

            let mut result = HashMap::new();
            for row in rows {
                let (id, bytes) = row?;
                let instrument: InstrumentJson = serde_json::from_slice(&bytes)?;
                result.insert(id, instrument);
            }
            Ok(result)
        })
    }

    fn list_instruments(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM instruments ORDER BY id ASC")?;
            let rows = stmt.query_map([], |row| row.get(0))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;
        let as_of = as_of_key(as_of);

        self.with_conn(|conn| {
            conn.execute(
                PUT_PORTFOLIO_SQL,
                params![portfolio_id, as_of, payload, meta],
            )?;
            Ok(())
        })
    }

    fn get_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM portfolios WHERE id = ?1 AND as_of = ?2",
                    params![portfolio_id, as_of],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<PortfolioSpec>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;

        self.with_conn(|conn| {
            conn.execute(PUT_SCENARIO_SQL, params![scenario_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM scenarios WHERE id = ?1",
                    params![scenario_id],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<ScenarioSpec>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_scenarios(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM scenarios ORDER BY id ASC")?;
            let rows = stmt.query_map([], |row| row.get(0))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;

        self.with_conn(|conn| {
            conn.execute(PUT_STATEMENT_MODEL_SQL, params![model_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM statement_models WHERE id = ?1",
                    params![model_id],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<FinancialModelSpec>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_statement_models(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT id FROM statement_models ORDER BY id ASC")?;
            let rows = stmt.query_map([], |row| row.get(0))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(registry)?;
        let meta = meta_json(meta)?;

        self.with_conn(|conn| {
            conn.execute(PUT_METRIC_REGISTRY_SQL, params![namespace, payload, meta])?;
            Ok(())
        })
    }

    fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT payload FROM metric_registries WHERE namespace = ?1",
                    params![namespace],
                    |row| row.get(0),
                )
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<MetricRegistry>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_metric_registries(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT namespace FROM metric_registries ORDER BY namespace ASC")?;
            let rows = stmt.query_map([], |row| row.get(0))?;

            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let rows_affected = conn.execute(
                "DELETE FROM metric_registries WHERE namespace = ?1",
                params![namespace],
            )?;
            Ok(rows_affected > 0)
        })
    }
}

impl LookbackStore for SqliteStore {
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);

        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT as_of, payload FROM market_contexts
                 WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3
                 ORDER BY as_of ASC",
            )?;
            let rows = stmt.query_map(params![market_id, start, end], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;

            let mut out = Vec::new();
            for row in rows {
                let (as_of_str, bytes) = row?;
                let as_of = parse_as_of_key(&as_of_str)?;
                let state: MarketContextState = serde_json::from_slice(&bytes)?;
                let ctx = MarketContext::try_from(state)?;
                out.push(MarketContextSnapshot {
                    as_of,
                    context: ctx,
                });
            }
            Ok(out)
        })
    }

    fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let row: Option<(String, Vec<u8>)> = conn
                .query_row(
                    "SELECT as_of, payload FROM market_contexts
                     WHERE id = ?1 AND as_of <= ?2
                     ORDER BY as_of DESC
                     LIMIT 1",
                    params![market_id, as_of],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            match row {
                Some((as_of_str, bytes)) => {
                    let as_of = parse_as_of_key(&as_of_str)?;
                    let state: MarketContextState = serde_json::from_slice(&bytes)?;
                    let ctx = MarketContext::try_from(state)?;
                    Ok(Some(MarketContextSnapshot {
                        as_of,
                        context: ctx,
                    }))
                }
                None => Ok(None),
            }
        })
    }

    fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);

        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT as_of, payload FROM portfolios
                 WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3
                 ORDER BY as_of ASC",
            )?;
            let rows = stmt.query_map(params![portfolio_id, start, end], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;

            let mut out = Vec::new();
            for row in rows {
                let (as_of_str, bytes) = row?;
                let as_of = parse_as_of_key(&as_of_str)?;
                let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                out.push(PortfolioSnapshot { as_of, spec });
            }
            Ok(out)
        })
    }

    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let row: Option<(String, Vec<u8>)> = conn
                .query_row(
                    "SELECT as_of, payload FROM portfolios
                     WHERE id = ?1 AND as_of <= ?2
                     ORDER BY as_of DESC
                     LIMIT 1",
                    params![portfolio_id, as_of],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            match row {
                Some((as_of_str, bytes)) => {
                    let as_of = parse_as_of_key(&as_of_str)?;
                    let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                    Ok(Some(PortfolioSnapshot { as_of, spec }))
                }
                None => Ok(None),
            }
        })
    }
}

impl BulkStore for SqliteStore {
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(PUT_INSTRUMENT_SQL)?;

                for (instrument_id, instrument, meta) in instruments {
                    let payload = serde_json::to_vec(instrument)?;
                    let meta = meta_json(*meta)?;
                    stmt.execute(params![instrument_id, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(PUT_MARKET_CONTEXT_SQL)?;

                for (market_id, as_of, context, meta) in contexts {
                    let state: MarketContextState = (*context).into();
                    let payload = serde_json::to_vec(&state)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    stmt.execute(params![market_id, as_of, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(PUT_PORTFOLIO_SQL)?;

                for (portfolio_id, as_of, spec, meta) in portfolios {
                    let payload = serde_json::to_vec(spec)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    stmt.execute(params![portfolio_id, as_of, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }
}
