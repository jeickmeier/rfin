//! Core Store trait implementation for PostgresStore.

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
use std::collections::HashMap;

use super::store::{as_of_key, meta_json, PostgresStore};

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
