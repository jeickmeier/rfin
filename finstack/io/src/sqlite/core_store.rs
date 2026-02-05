//! Core Store trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
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
use rusqlite::params;
use std::collections::HashMap;

use super::store::{as_of_key, meta_json, optional_row, SqliteStore};

#[async_trait]
impl Store for SqliteStore {
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
        let market_id = market_id.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_market_context_sql(Backend::Sqlite);
                conn.execute(&sql, params![market_id, as_of, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>> {
        let as_of = as_of_key(as_of);
        let market_id = market_id.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_market_context_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![market_id, as_of],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => {
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
        let instrument_id = instrument_id.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_instrument_sql(Backend::Sqlite);
                conn.execute(&sql, params![instrument_id, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        let instrument_id = instrument_id.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_instrument_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![instrument_id],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => Ok(Some(serde_json::from_slice::<InstrumentJson>(&bytes)?)),
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

        let instrument_ids = instrument_ids.to_vec();

        let rows: Vec<(String, Vec<u8>)> = self
            .conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Vec<(String, Vec<u8>)>> {
                    let sql = statements::select_instruments_batch_sql(
                        Backend::Sqlite,
                        instrument_ids.len(),
                    );
                    let mut stmt = conn.prepare(&sql)?;
                    let params: Vec<&dyn rusqlite::ToSql> = instrument_ids
                        .iter()
                        .map(|s| s as &dyn rusqlite::ToSql)
                        .collect();

                    let rows = stmt.query_map(params.as_slice(), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })?;

                    let mut result = Vec::new();
                    for row in rows {
                        result.push(row?);
                    }
                    Ok(result)
                },
            )
            .await?;

        let mut result = HashMap::new();
        for (id, bytes) in rows {
            let instrument: InstrumentJson = serde_json::from_slice(&bytes)?;
            result.insert(id, instrument);
        }
        Ok(result)
    }

    async fn list_instruments(&self) -> Result<Vec<String>> {
        let ids = self
            .conn
            .call(|conn| -> tokio_rusqlite::Result<Vec<String>> {
                let sql = statements::list_instruments_sql(Backend::Sqlite);
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map([], |row| row.get(0))?;

                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;
        Ok(ids)
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
        let portfolio_id = portfolio_id.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
                conn.execute(&sql, params![portfolio_id, as_of, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>> {
        let as_of = as_of_key(as_of);
        let portfolio_id = portfolio_id.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_portfolio_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![portfolio_id, as_of],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => Ok(Some(serde_json::from_slice::<PortfolioSpec>(&bytes)?)),
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
        let scenario_id = scenario_id.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_scenario_sql(Backend::Sqlite);
                conn.execute(&sql, params![scenario_id, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        let scenario_id = scenario_id.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_scenario_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![scenario_id],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => Ok(Some(serde_json::from_slice::<ScenarioSpec>(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn list_scenarios(&self) -> Result<Vec<String>> {
        let ids = self
            .conn
            .call(|conn| -> tokio_rusqlite::Result<Vec<String>> {
                let sql = statements::list_scenarios_sql(Backend::Sqlite);
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map([], |row| row.get(0))?;

                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;
        Ok(ids)
    }

    async fn put_statement_model(
        &self,
        model_id: &str,
        spec: &FinancialModelSpec,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(spec)?;
        let meta = meta_json(meta)?;
        let model_id = model_id.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_statement_model_sql(Backend::Sqlite);
                conn.execute(&sql, params![model_id, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        let model_id = model_id.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_statement_model_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![model_id],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => Ok(Some(serde_json::from_slice::<FinancialModelSpec>(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn list_statement_models(&self) -> Result<Vec<String>> {
        let ids = self
            .conn
            .call(|conn| -> tokio_rusqlite::Result<Vec<String>> {
                let sql = statements::list_statement_models_sql(Backend::Sqlite);
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map([], |row| row.get(0))?;

                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;
        Ok(ids)
    }

    async fn put_metric_registry(
        &self,
        namespace: &str,
        registry: &MetricRegistry,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let payload = serde_json::to_vec(registry)?;
        let meta = meta_json(meta)?;
        let namespace = namespace.to_string();

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_metric_registry_sql(Backend::Sqlite);
                conn.execute(&sql, params![namespace, payload, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        let namespace = namespace.to_string();

        let payload: Option<Vec<u8>> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<Vec<u8>>> {
                let sql = statements::select_metric_registry_sql(Backend::Sqlite);
                Ok(optional_row(conn.query_row(
                    &sql,
                    params![namespace],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        match payload {
            Some(bytes) => Ok(Some(serde_json::from_slice::<MetricRegistry>(&bytes)?)),
            None => Ok(None),
        }
    }

    async fn list_metric_registries(&self) -> Result<Vec<String>> {
        let ids = self
            .conn
            .call(|conn| -> tokio_rusqlite::Result<Vec<String>> {
                let sql = statements::list_metric_registries_sql(Backend::Sqlite);
                let mut stmt = conn.prepare(&sql)?;
                let rows = stmt.query_map([], |row| row.get(0))?;

                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;
        Ok(ids)
    }

    async fn delete_metric_registry(&self, namespace: &str) -> Result<bool> {
        let namespace = namespace.to_string();

        let rows_affected = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<usize> {
                let sql = statements::delete_metric_registry_sql(Backend::Sqlite);
                Ok(conn.execute(&sql, params![namespace])?)
            })
            .await?;
        Ok(rows_affected > 0)
    }
}
