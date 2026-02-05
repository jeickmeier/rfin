//! Core Store trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    store::MAX_BATCH_SIZE,
    Error, Result, Store,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use libsql::params;
use std::collections::HashMap;

use super::store::{as_of_key, get_blob, get_string, meta_json, TursoStore};

#[async_trait]
impl Store for TursoStore {
    async fn put_market_context(
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

        let conn = self.get_conn()?;
        let sql = statements::upsert_market_context_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![market_id, as_of, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of);

        let conn = self.get_conn()?;
        let sql = statements::select_market_context_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![market_id, as_of]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                let state: MarketContextState = serde_json::from_slice(&bytes)?;
                let ctx = MarketContext::try_from(state)?;
                Ok(Some(ctx))
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
        let payload = serde_json::to_vec(instrument)?;
        let meta = meta_json(meta)?;

        let conn = self.get_conn()?;
        let sql = statements::upsert_instrument_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![instrument_id, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        let conn = self.get_conn()?;
        let sql = statements::select_instrument_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![instrument_id]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                Ok(Some(serde_json::from_slice::<InstrumentJson>(&bytes)?))
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

        // Chunk large batches to avoid query plan cache pollution and excessive IN clause sizes
        for chunk in instrument_ids.chunks(MAX_BATCH_SIZE) {
            let conn = self.get_conn()?;
            let sql = statements::select_instruments_batch_sql_with_naming(
                Backend::Sqlite,
                self.naming(),
                chunk.len(),
            );
            let mut stmt = conn.prepare(&sql).await?;

            // Build params dynamically
            let params: Vec<libsql::Value> = chunk
                .iter()
                .map(|s| libsql::Value::Text(s.clone()))
                .collect();

            let mut rows = stmt.query(params).await?;

            while let Some(row) = rows.next().await.map_err(Error::Turso)? {
                let id = get_string(&row, 0)?;
                let bytes = get_blob(&row, 1)?;
                let instrument: InstrumentJson = serde_json::from_slice(&bytes)?;
                result.insert(id, instrument);
            }
        }

        Ok(result)
    }

    async fn list_instruments(&self) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let sql = statements::list_instruments_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(()).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            out.push(get_string(&row, 0)?);
        }
        Ok(out)
    }

    async fn put_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
        spec: &PortfolioSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;
        let as_of = as_of_key(as_of);

        let conn = self.get_conn()?;
        let sql = statements::upsert_portfolio_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![portfolio_id, as_of, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of);

        let conn = self.get_conn()?;
        let sql = statements::select_portfolio_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![portfolio_id, as_of]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                Ok(Some(serde_json::from_slice::<PortfolioSpec>(&bytes)?))
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
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;

        let conn = self.get_conn()?;
        let sql = statements::upsert_scenario_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![scenario_id, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        let conn = self.get_conn()?;
        let sql = statements::select_scenario_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![scenario_id]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                Ok(Some(serde_json::from_slice::<ScenarioSpec>(&bytes)?))
            }
            None => Ok(None),
        }
    }

    async fn list_scenarios(&self) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let sql = statements::list_scenarios_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(()).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            out.push(get_string(&row, 0)?);
        }
        Ok(out)
    }

    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;

        let conn = self.get_conn()?;
        let sql =
            statements::upsert_statement_model_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![model_id, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        let conn = self.get_conn()?;
        let sql =
            statements::select_statement_model_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![model_id]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                Ok(Some(serde_json::from_slice::<FinancialModelSpec>(&bytes)?))
            }
            None => Ok(None),
        }
    }

    async fn list_statement_models(&self) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let sql = statements::list_statement_models_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(()).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            out.push(get_string(&row, 0)?);
        }
        Ok(out)
    }

    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(registry)?;
        let meta = meta_json(meta)?;

        let conn = self.get_conn()?;
        let sql =
            statements::upsert_metric_registry_sql_with_naming(Backend::Sqlite, self.naming());
        conn.execute(sql.as_ref(), params![namespace, payload, meta])
            .await?;
        Ok(())
    }

    async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        let conn = self.get_conn()?;
        let sql =
            statements::select_metric_registry_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![namespace]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let bytes = get_blob(&row, 0)?;
                Ok(Some(serde_json::from_slice::<MetricRegistry>(&bytes)?))
            }
            None => Ok(None),
        }
    }

    async fn list_metric_registries(&self) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let sql =
            statements::list_metric_registries_sql_with_naming(Backend::Sqlite, self.naming());
        let mut stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(()).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            out.push(get_string(&row, 0)?);
        }
        Ok(out)
    }

    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        let conn = self.get_conn()?;
        let sql =
            statements::delete_metric_registry_sql_with_naming(Backend::Sqlite, self.naming());
        let rows_affected = conn.execute(sql.as_ref(), params![namespace]).await?;
        Ok(rows_affected > 0)
    }
}
