//! Postgres backend for `finstack-io`.

use crate::{
    sql::{migrations, statements, Backend},
    BulkStore, Error, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result, SeriesKey,
    SeriesKind, Store, TimeSeriesPoint, TimeSeriesStore,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use postgres::{Client, NoTls};
use std::collections::HashMap;

/// A Postgres-backed store.
///
/// This store is `Send + Sync` because it opens a new connection per call.
#[derive(Clone, Debug)]
pub struct PostgresStore {
    url: String,
}

impl PostgresStore {
    /// Connect to a Postgres database at `url`, applying migrations.
    pub fn connect(url: &str) -> Result<Self> {
        let store = Self {
            url: url.to_string(),
        };
        store.with_conn(migrate)?;
        Ok(store)
    }

    /// Connection URL used by this store.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn with_conn<R>(&self, f: impl FnOnce(&mut Client) -> Result<R>) -> Result<R> {
        let mut client = Client::connect(&self.url, NoTls)?;
        client.batch_execute("SET statement_timeout = 5000")?;
        f(&mut client)
    }
}

fn migrate(client: &mut Client) -> Result<()> {
    client.batch_execute(&migrations::schema_migrations_table_sql(Backend::Postgres))?;

    let row = client.query_opt("SELECT MAX(version) FROM finstack_schema_migrations", &[])?;
    let current: i64 = match row {
        Some(row) => row.get::<_, Option<i64>>(0).unwrap_or(0),
        None => 0,
    };

    if current > migrations::LATEST_VERSION {
        return Err(Error::UnsupportedSchema {
            found: current,
            expected: migrations::LATEST_VERSION,
        });
    }

    if current == migrations::LATEST_VERSION {
        return Ok(());
    }

    let migrations = migrations::migrations_for(Backend::Postgres);
    let mut tx = client.transaction()?;
    for (version, statements) in migrations {
        if version <= current {
            continue;
        }
        for sql in statements {
            tx.batch_execute(&sql)?;
        }
        tx.execute(
            "INSERT INTO finstack_schema_migrations (version, applied_at) VALUES ($1, now())",
            &[&version],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn meta_json(meta: Option<&serde_json::Value>) -> serde_json::Value {
    meta.cloned().unwrap_or_else(|| serde_json::json!({}))
}

fn as_of_key(as_of: Date) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(as_of.year(), as_of.month() as u32, as_of.day() as u32)
        .ok_or_else(|| Error::Invariant("Invalid date".into()))
}

fn parse_as_of_key(value: NaiveDate) -> Result<Date> {
    let month = u8::try_from(value.month())
        .map_err(|_| Error::Invariant("Invalid month from database".into()))?;
    let day = u8::try_from(value.day())
        .map_err(|_| Error::Invariant("Invalid day from database".into()))?;
    Date::from_calendar_date(
        value.year(),
        month
            .try_into()
            .map_err(|_| Error::Invariant("Invalid month from database".into()))?,
        day,
    )
    .map_err(|e| Error::Invariant(format!("Invalid date in database: {e}")))
}

fn ts_key(ts: time::OffsetDateTime) -> Result<DateTime<Utc>> {
    let seconds = ts.unix_timestamp();
    let nanos = ts.nanosecond();
    DateTime::<Utc>::from_timestamp(seconds, nanos)
        .ok_or_else(|| Error::Invariant("Invalid timestamp".into()))
}

fn parse_ts_key(ts: DateTime<Utc>) -> Result<time::OffsetDateTime> {
    let secs = ts.timestamp();
    let nanos = ts.timestamp_subsec_nanos();
    let base = time::OffsetDateTime::from_unix_timestamp(secs)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp in database: {e}")))?;
    base.replace_nanosecond(nanos)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp in database: {e}")))
}

impl Store for PostgresStore {
    fn put_market_context(
        &self,
        market_id: &str,
        as_of: Date,
        context: &MarketContext,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let state: MarketContextState = context.into();
        let payload = serde_json::to_value(&state)?;
        let meta = meta_json(meta);
        let as_of = as_of_key(as_of)?;
        let sql = statements::upsert_market_context_sql(Backend::Postgres);

        self.with_conn(|client| {
            client.execute(&sql, &[&market_id, &as_of, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_market_context(&self, market_id: &str, as_of: Date) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of)?;
        let sql = statements::select_market_context_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&market_id, &as_of])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    let state: MarketContextState = serde_json::from_value(payload)?;
                    Ok(Some(MarketContext::try_from(state)?))
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
        let payload = serde_json::to_value(instrument)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_instrument_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(&sql, &[&instrument_id, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        let sql = statements::select_instrument_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&instrument_id])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    Ok(Some(serde_json::from_value(payload)?))
                }
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
        let sql = statements::select_instruments_batch_sql(Backend::Postgres, instrument_ids.len());
        self.with_conn(|client| {
            let params: Vec<&(dyn postgres::types::ToSql + Sync)> = instrument_ids
                .iter()
                .map(|s| s as &(dyn postgres::types::ToSql + Sync))
                .collect();
            let rows = client.query(&sql, &params)?;
            let mut out = HashMap::new();
            for row in rows {
                let id: String = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let instrument: InstrumentJson = serde_json::from_value(payload)?;
                out.insert(id, instrument);
            }
            Ok(out)
        })
    }

    fn list_instruments(&self) -> Result<Vec<String>> {
        let sql = statements::list_instruments_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows = client.query(&sql, &[])?;
            Ok(rows.into_iter().map(|row| row.get(0)).collect())
        })
    }

    fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let as_of = as_of_key(as_of)?;
        let sql = statements::upsert_portfolio_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(&sql, &[&portfolio_id, &as_of, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_portfolio_spec(&self, portfolio_id: &str, as_of: Date) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of)?;
        let sql = statements::select_portfolio_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&portfolio_id, &as_of])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    Ok(Some(serde_json::from_value(payload)?))
                }
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
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_scenario_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(&sql, &[&scenario_id, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        let sql = statements::select_scenario_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&scenario_id])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    Ok(Some(serde_json::from_value(payload)?))
                }
                None => Ok(None),
            }
        })
    }

    fn list_scenarios(&self) -> Result<Vec<String>> {
        let sql = statements::list_scenarios_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows = client.query(&sql, &[])?;
            Ok(rows.into_iter().map(|row| row.get(0)).collect())
        })
    }

    fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_statement_model_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(&sql, &[&model_id, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        let sql = statements::select_statement_model_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&model_id])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    Ok(Some(serde_json::from_value(payload)?))
                }
                None => Ok(None),
            }
        })
    }

    fn list_statement_models(&self) -> Result<Vec<String>> {
        let sql = statements::list_statement_models_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows = client.query(&sql, &[])?;
            Ok(rows.into_iter().map(|row| row.get(0)).collect())
        })
    }

    fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(registry)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_metric_registry_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(&sql, &[&namespace, &payload, &meta])?;
            Ok(())
        })
    }

    fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        let sql = statements::select_metric_registry_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&namespace])?;
            match row {
                Some(row) => {
                    let payload: serde_json::Value = row.get(0);
                    Ok(Some(serde_json::from_value(payload)?))
                }
                None => Ok(None),
            }
        })
    }

    fn list_metric_registries(&self) -> Result<Vec<String>> {
        let sql = statements::list_metric_registries_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows = client.query(&sql, &[])?;
            Ok(rows.into_iter().map(|row| row.get(0)).collect())
        })
    }

    fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        let sql = statements::delete_metric_registry_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows_affected = client.execute(&sql, &[&namespace])?;
            Ok(rows_affected > 0)
        })
    }
}

impl LookbackStore for PostgresStore {
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let sql = statements::list_market_contexts_sql(Backend::Postgres);
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;

        self.with_conn(|client| {
            let rows = client.query(&sql, &[&market_id, &start, &end])?;
            let mut out = Vec::new();
            for row in rows {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let state: MarketContextState = serde_json::from_value(payload)?;
                let ctx = MarketContext::try_from(state)?;
                out.push(MarketContextSnapshot {
                    as_of: parse_as_of_key(as_of)?,
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
        let sql = statements::latest_market_context_sql(Backend::Postgres);
        let as_of = as_of_key(as_of)?;
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&market_id, &as_of])?;
            match row {
                Some(row) => {
                    let as_of: NaiveDate = row.get(0);
                    let payload: serde_json::Value = row.get(1);
                    let state: MarketContextState = serde_json::from_value(payload)?;
                    let ctx = MarketContext::try_from(state)?;
                    Ok(Some(MarketContextSnapshot {
                        as_of: parse_as_of_key(as_of)?,
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
        let sql = statements::list_portfolios_sql(Backend::Postgres);
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;
        self.with_conn(|client| {
            let rows = client.query(&sql, &[&portfolio_id, &start, &end])?;
            let mut out = Vec::new();
            for row in rows {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let spec: PortfolioSpec = serde_json::from_value(payload)?;
                out.push(PortfolioSnapshot {
                    as_of: parse_as_of_key(as_of)?,
                    spec,
                });
            }
            Ok(out)
        })
    }

    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let sql = statements::latest_portfolio_sql(Backend::Postgres);
        let as_of = as_of_key(as_of)?;
        self.with_conn(|client| {
            let row = client.query_opt(&sql, &[&portfolio_id, &as_of])?;
            match row {
                Some(row) => {
                    let as_of: NaiveDate = row.get(0);
                    let payload: serde_json::Value = row.get(1);
                    let spec: PortfolioSpec = serde_json::from_value(payload)?;
                    Ok(Some(PortfolioSnapshot {
                        as_of: parse_as_of_key(as_of)?,
                        spec,
                    }))
                }
                None => Ok(None),
            }
        })
    }
}

impl TimeSeriesStore for PostgresStore {
    fn put_series_meta(&self, key: &SeriesKey, meta: Option<&serde_json::Value>) -> Result<()> {
        let meta = meta.cloned();
        let sql = statements::upsert_series_meta_sql(Backend::Postgres);
        self.with_conn(|client| {
            client.execute(
                &sql,
                &[&key.namespace, &key.kind.as_str(), &key.series_id, &meta],
            )?;
            Ok(())
        })
    }

    fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let sql = statements::select_series_meta_sql(Backend::Postgres);
        self.with_conn(|client| {
            let row =
                client.query_opt(&sql, &[&key.namespace, &key.kind.as_str(), &key.series_id])?;
            match row {
                Some(row) => {
                    let meta: Option<serde_json::Value> = row.get(0);
                    Ok(meta)
                }
                None => Ok(None),
            }
        })
    }

    fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let sql = statements::list_series_sql(Backend::Postgres);
        self.with_conn(|client| {
            let rows = client.query(&sql, &[&namespace, &kind.as_str()])?;
            Ok(rows.into_iter().map(|row| row.get(0)).collect())
        })
    }

    fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        let sql = statements::upsert_series_point_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for point in points {
                let ts = ts_key(point.ts)?;
                let payload = point.payload.clone();
                let meta = point.meta.clone();
                tx.execute(
                    &sql,
                    &[
                        &key.namespace,
                        &key.kind.as_str(),
                        &key.series_id,
                        &ts,
                        &point.value,
                        &payload,
                        &meta,
                    ],
                )?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn get_points_range(
        &self,
        key: &SeriesKey,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let mut sql = statements::select_points_range_sql(Backend::Postgres);
        if let Some(max) = limit {
            sql = format!("{sql} LIMIT {max}");
        }
        let start = ts_key(start)?;
        let end = ts_key(end)?;
        self.with_conn(|client| {
            let rows = client.query(
                &sql,
                &[
                    &key.namespace,
                    &key.kind.as_str(),
                    &key.series_id,
                    &start,
                    &end,
                ],
            )?;
            let mut out = Vec::new();
            for row in rows {
                let ts: DateTime<Utc> = row.get(0);
                let value: Option<f64> = row.get(1);
                let payload: Option<serde_json::Value> = row.get(2);
                let meta: Option<serde_json::Value> = row.get(3);
                out.push(TimeSeriesPoint {
                    ts: parse_ts_key(ts)?,
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
        ts: time::OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let sql = statements::latest_point_sql(Backend::Postgres);
        let ts = ts_key(ts)?;
        self.with_conn(|client| {
            let row = client.query_opt(
                &sql,
                &[&key.namespace, &key.kind.as_str(), &key.series_id, &ts],
            )?;
            match row {
                Some(row) => {
                    let ts: DateTime<Utc> = row.get(0);
                    let value: Option<f64> = row.get(1);
                    let payload: Option<serde_json::Value> = row.get(2);
                    let meta: Option<serde_json::Value> = row.get(3);
                    Ok(Some(TimeSeriesPoint {
                        ts: parse_ts_key(ts)?,
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

impl BulkStore for PostgresStore {
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_instrument_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (instrument_id, instrument, meta) in instruments {
                let payload = serde_json::to_value(instrument)?;
                let meta = meta_json(*meta);
                tx.execute(&sql, &[instrument_id, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_market_context_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (market_id, as_of, context, meta) in contexts {
                let state: MarketContextState = (*context).into();
                let payload = serde_json::to_value(&state)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                tx.execute(&sql, &[market_id, &as_of, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_portfolio_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (portfolio_id, as_of, spec, meta) in portfolios {
                let payload = serde_json::to_value(spec)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                tx.execute(&sql, &[portfolio_id, &as_of, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }
}
