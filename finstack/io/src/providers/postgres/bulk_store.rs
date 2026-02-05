//! BulkStore trait implementation for PostgresStore.
//!
//! This implementation pre-serializes all data before opening the transaction,
//! matching the pattern used by SQLite and Turso backends. This provides:
//! - Early error detection (serialization errors before transaction starts)
//! - Reduced transaction hold time (no serialization inside the transaction)
//! - Consistent behavior across all backends

use crate::{
    sql::{statements, Backend},
    BulkStore, Result,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;

use super::store::{as_of_key, meta_json, PostgresStore};

#[async_trait]
impl BulkStore for PostgresStore {
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, serde_json::Value, serde_json::Value)> = instruments
            .iter()
            .map(|(id, instrument, meta)| {
                let payload = serde_json::to_value(instrument)?;
                let meta = meta_json(*meta);
                Ok((id.to_string(), payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let sql = statements::upsert_instrument_sql(Backend::Postgres);

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;
        for (instrument_id, payload, meta) in &serialized {
            tx.execute(sql, &[instrument_id, payload, meta]).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, NaiveDate, serde_json::Value, serde_json::Value)> = contexts
            .iter()
            .map(|(market_id, as_of, context, meta)| {
                let state: MarketContextState = (*context).into();
                let payload = serde_json::to_value(&state)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                Ok((market_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let sql = statements::upsert_market_context_sql(Backend::Postgres);

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;
        for (market_id, as_of, payload, meta) in &serialized {
            tx.execute(sql, &[market_id, as_of, payload, meta]).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, NaiveDate, serde_json::Value, serde_json::Value)> = portfolios
            .iter()
            .map(|(portfolio_id, as_of, spec, meta)| {
                let payload = serde_json::to_value(spec)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                Ok((portfolio_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let sql = statements::upsert_portfolio_sql(Backend::Postgres);

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;
        for (portfolio_id, as_of, payload, meta) in &serialized {
            tx.execute(sql, &[portfolio_id, as_of, payload, meta])
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
