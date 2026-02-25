//! Core Store trait implementation for PostgresStore.

use crate::{
    sql::{statements, Backend},
    store::MAX_BATCH_SIZE,
    Result, Store,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use std::collections::HashMap;

use super::store::{as_of_key, meta_json, quote_ident, PostgresStore};

#[async_trait]
impl Store for PostgresStore {
    async fn put_market_context(
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
        let sql =
            statements::upsert_market_context_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&market_id, &as_of, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of)?;
        let sql =
            statements::select_market_context_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&market_id, &as_of]).await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                let state: MarketContextState = crate::helpers::json_from_value(payload)?;
                Ok(Some(MarketContext::try_from(state)?))
            }
            None => Ok(None),
        }
    }

    async fn put_instrument(
        &self,
        instrument_id: &str,
        instrument: &InstrumentJson,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(instrument)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_instrument_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&instrument_id, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        let sql = statements::select_instrument_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&instrument_id]).await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                Ok(Some(crate::helpers::json_from_value(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn get_instruments_batch(
        &self,
        instrument_ids: &[String],
    ) -> Result<HashMap<String, InstrumentJson>> {
        if instrument_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut result = HashMap::with_capacity(instrument_ids.len());

        // Chunk large batches to cap payload size; use `ANY($1)` to avoid dynamic placeholder
        // counts (better plan cache reuse, fewer allocations).
        let instruments_table = quote_ident(&self.naming().resolve("instruments"));
        let select_any_sql =
            format!("SELECT id, payload FROM {instruments_table} WHERE id = ANY($1)");
        let conn = self.get_conn().await?;
        for chunk in instrument_ids.chunks(MAX_BATCH_SIZE) {
            // Use `Vec<&str>` to reliably bind as `text[]`.
            let ids: Vec<&str> = chunk.iter().map(String::as_str).collect();
            let rows = conn.query(&select_any_sql, &[&ids]).await?;

            for row in rows {
                let id: String = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let instrument: InstrumentJson = crate::helpers::json_from_value(payload)?;
                result.insert(id, instrument);
            }
        }

        Ok(result)
    }

    async fn list_instruments(&self) -> Result<Vec<String>> {
        let sql = statements::list_instruments_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows = conn.query(sql.as_ref(), &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let as_of = as_of_key(as_of)?;
        let sql = statements::upsert_portfolio_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&portfolio_id, &as_of, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of)?;
        let sql = statements::select_portfolio_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(sql.as_ref(), &[&portfolio_id, &as_of])
            .await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                Ok(Some(crate::helpers::json_from_value(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn put_scenario(
        &self,
        scenario_id: &str,
        spec: &ScenarioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let sql = statements::upsert_scenario_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&scenario_id, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        let sql = statements::select_scenario_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&scenario_id]).await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                Ok(Some(crate::helpers::json_from_value(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn list_scenarios(&self) -> Result<Vec<String>> {
        let sql = statements::list_scenarios_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows = conn.query(sql.as_ref(), &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(spec)?;
        let meta = meta_json(meta);
        let sql =
            statements::upsert_statement_model_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&model_id, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        let sql =
            statements::select_statement_model_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&model_id]).await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                Ok(Some(crate::helpers::json_from_value(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn list_statement_models(&self) -> Result<Vec<String>> {
        let sql =
            statements::list_statement_models_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows = conn.query(sql.as_ref(), &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_value(registry)?;
        let meta = meta_json(meta);
        let sql =
            statements::upsert_metric_registry_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(sql.as_ref(), &[&namespace, &payload, &meta])
            .await?;
        Ok(())
    }

    async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        let sql =
            statements::select_metric_registry_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&namespace]).await?;
        match row {
            Some(row) => {
                let payload: serde_json::Value = row.get(0);
                Ok(Some(crate::helpers::json_from_value(payload)?))
            }
            None => Ok(None),
        }
    }

    async fn list_metric_registries(&self) -> Result<Vec<String>> {
        let sql =
            statements::list_metric_registries_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows = conn.query(sql.as_ref(), &[]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        let sql =
            statements::delete_metric_registry_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows_affected = conn.execute(sql.as_ref(), &[&namespace]).await?;
        Ok(rows_affected > 0)
    }
}
