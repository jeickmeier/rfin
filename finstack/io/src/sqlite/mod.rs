//! SQLite backend for `finstack-io`.
//!
//! This module provides a minimal, predictable schema with JSON payload blobs
//! for domain objects, indexed by `(id, as_of)` where applicable.

use crate::{
    sql::{migrations, statements, Backend},
    BulkStore, Error, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result, SeriesKey,
    SeriesKind, Store, TimeSeriesPoint, TimeSeriesStore,
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
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

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

fn ts_key(ts: OffsetDateTime) -> Result<String> {
    ts.format(&Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format: {e}")))
}

fn parse_ts_key(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp format in database: {s} ({e})")))
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
            let sql = statements::upsert_market_context_sql(Backend::Sqlite);
            conn.execute(&sql, params![market_id, as_of, payload, meta])?;
            Ok(())
        })
    }

    fn get_market_context(&self, market_id: &str, as_of: Date) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let sql = statements::select_market_context_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![market_id, as_of], |row| row.get(0))
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
            let sql = statements::upsert_instrument_sql(Backend::Sqlite);
            conn.execute(&sql, params![instrument_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        self.with_conn(|conn| {
            let sql = statements::select_instrument_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![instrument_id], |row| row.get(0))
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
            let sql =
                statements::select_instruments_batch_sql(Backend::Sqlite, instrument_ids.len());
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
            let sql = statements::list_instruments_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
            conn.execute(&sql, params![portfolio_id, as_of, payload, meta])?;
            Ok(())
        })
    }

    fn get_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn| {
            let sql = statements::select_portfolio_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![portfolio_id, as_of], |row| row.get(0))
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
            let sql = statements::upsert_scenario_sql(Backend::Sqlite);
            conn.execute(&sql, params![scenario_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        self.with_conn(|conn| {
            let sql = statements::select_scenario_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![scenario_id], |row| row.get(0))
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<ScenarioSpec>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_scenarios(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let sql = statements::list_scenarios_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::upsert_statement_model_sql(Backend::Sqlite);
            conn.execute(&sql, params![model_id, payload, meta])?;
            Ok(())
        })
    }

    fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        self.with_conn(|conn| {
            let sql = statements::select_statement_model_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![model_id], |row| row.get(0))
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<FinancialModelSpec>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_statement_models(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let sql = statements::list_statement_models_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::upsert_metric_registry_sql(Backend::Sqlite);
            conn.execute(&sql, params![namespace, payload, meta])?;
            Ok(())
        })
    }

    fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        self.with_conn(|conn| {
            let sql = statements::select_metric_registry_sql(Backend::Sqlite);
            let payload: Option<Vec<u8>> = conn
                .query_row(&sql, params![namespace], |row| row.get(0))
                .optional()?;

            match payload {
                Some(bytes) => Ok(Some(serde_json::from_slice::<MetricRegistry>(&bytes)?)),
                None => Ok(None),
            }
        })
    }

    fn list_metric_registries(&self) -> Result<Vec<String>> {
        self.with_conn(|conn| {
            let sql = statements::list_metric_registries_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::delete_metric_registry_sql(Backend::Sqlite);
            let rows_affected = conn.execute(&sql, params![namespace])?;
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
            let sql = statements::list_market_contexts_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::latest_market_context_sql(Backend::Sqlite);
            let row: Option<(String, Vec<u8>)> = conn
                .query_row(&sql, params![market_id, as_of], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
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
            let sql = statements::list_portfolios_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
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
            let sql = statements::latest_portfolio_sql(Backend::Sqlite);
            let row: Option<(String, Vec<u8>)> = conn
                .query_row(&sql, params![portfolio_id, as_of], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
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

type SeriesRow = (String, Option<f64>, Option<String>, Option<String>);

impl TimeSeriesStore for SqliteStore {
    fn put_series_meta(&self, key: &SeriesKey, meta: Option<&serde_json::Value>) -> Result<()> {
        let meta = match meta {
            Some(value) => Some(serde_json::to_string(value)?),
            None => None,
        };
        let sql = statements::upsert_series_meta_sql(Backend::Sqlite);
        self.with_conn(|conn| {
            conn.execute(
                &sql,
                params![key.namespace, key.kind.as_str(), key.series_id, meta],
            )?;
            Ok(())
        })
    }

    fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let sql = statements::select_series_meta_sql(Backend::Sqlite);
        self.with_conn(|conn| {
            let meta: Option<String> = conn
                .query_row(
                    &sql,
                    params![key.namespace, key.kind.as_str(), key.series_id],
                    |row| row.get(0),
                )
                .optional()?;

            match meta {
                Some(value) => Ok(Some(serde_json::from_str(&value)?)),
                None => Ok(None),
            }
        })
    }

    fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let sql = statements::list_series_sql(Backend::Sqlite);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![namespace, kind.as_str()], |row| row.get(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }

    fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        let sql = statements::upsert_series_point_sql(Backend::Sqlite);
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(&sql)?;
                for point in points {
                    let ts = ts_key(point.ts)?;
                    let payload = match &point.payload {
                        Some(value) => Some(serde_json::to_string(value)?),
                        None => None,
                    };
                    let meta = match &point.meta {
                        Some(value) => Some(serde_json::to_string(value)?),
                        None => None,
                    };
                    stmt.execute(params![
                        key.namespace,
                        key.kind.as_str(),
                        key.series_id,
                        ts,
                        point.value,
                        payload,
                        meta
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn get_points_range(
        &self,
        key: &SeriesKey,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let mut sql = statements::select_points_range_sql(Backend::Sqlite);
        if let Some(max) = limit {
            sql = format!("{sql} LIMIT {max}");
        }
        let start = ts_key(start)?;
        let end = ts_key(end)?;
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(
                params![key.namespace, key.kind.as_str(), key.series_id, start, end],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<f64>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )?;
            let mut out = Vec::new();
            for row in rows {
                let (ts_str, value, payload, meta) = row?;
                let payload = match payload {
                    Some(value) => Some(serde_json::from_str(&value)?),
                    None => None,
                };
                let meta = match meta {
                    Some(value) => Some(serde_json::from_str(&value)?),
                    None => None,
                };
                out.push(TimeSeriesPoint {
                    ts: parse_ts_key(&ts_str)?,
                    value,
                    payload,
                    meta,
                });
            }
            Ok(out)
        })
    }

    fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let sql = statements::latest_point_sql(Backend::Sqlite);
        let ts = ts_key(ts)?;
        self.with_conn(|conn| {
            let row: Option<SeriesRow> = conn
                .query_row(
                    &sql,
                    params![key.namespace, key.kind.as_str(), key.series_id, ts],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?;

            match row {
                Some((ts_str, value, payload, meta)) => {
                    let payload = match payload {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    };
                    let meta = match meta {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    };
                    Ok(Some(TimeSeriesPoint {
                        ts: parse_ts_key(&ts_str)?,
                        value,
                        payload,
                        meta,
                    }))
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
                let sql = statements::upsert_instrument_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

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
                let sql = statements::upsert_market_context_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

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
                let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

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
