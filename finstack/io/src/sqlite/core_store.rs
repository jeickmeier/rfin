//! Core Store trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
    Result, Store,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::store::{as_of_key, meta_json, SqliteStore};

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
