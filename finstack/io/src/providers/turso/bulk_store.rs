//! BulkStore trait implementation for TursoStore.
//!
//! This implementation pre-serializes all data before opening the transaction,
//! matching the pattern used by SQLite and Postgres backends. This provides:
//! - Early error detection (serialization errors before transaction starts)
//! - Reduced transaction hold time (no serialization inside the transaction)
//! - Consistent behavior across all backends

use crate::{
    sql::{statements, Backend},
    BulkStore, Result,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;
use libsql::params;

use super::store::{as_of_key, meta_json, TursoStore};

#[async_trait]
impl BulkStore for TursoStore {
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, Vec<u8>, String)> = instruments
            .iter()
            .map(|(id, instrument, meta)| {
                let payload = serde_json::to_vec(instrument)?;
                let meta = meta_json(*meta)?;
                Ok((id.to_string(), payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_instrument_sql_with_naming(Backend::Sqlite, self.naming());
        for (instrument_id, payload, meta) in &serialized {
            tx.execute(
                sql.as_ref(),
                params![instrument_id.as_str(), payload.clone(), meta.as_str()],
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, String, Vec<u8>, String)> = contexts
            .iter()
            .map(|(market_id, as_of, context, meta)| {
                let state: MarketContextState = (*context).into();
                let payload = serde_json::to_vec(&state)?;
                let meta = meta_json(*meta)?;
                let as_of = as_of_key(*as_of);
                Ok((market_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_market_context_sql_with_naming(Backend::Sqlite, self.naming());
        for (market_id, as_of, payload, meta) in &serialized {
            tx.execute(
                sql.as_ref(),
                params![
                    market_id.as_str(),
                    as_of.as_str(),
                    payload.clone(),
                    meta.as_str()
                ],
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before opening the transaction
        let serialized: Vec<(String, String, Vec<u8>, String)> = portfolios
            .iter()
            .map(|(portfolio_id, as_of, spec, meta)| {
                let payload = serde_json::to_vec(spec)?;
                let meta = meta_json(*meta)?;
                let as_of = as_of_key(*as_of);
                Ok((portfolio_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_portfolio_sql_with_naming(Backend::Sqlite, self.naming());
        for (portfolio_id, as_of, payload, meta) in &serialized {
            tx.execute(
                sql.as_ref(),
                params![
                    portfolio_id.as_str(),
                    as_of.as_str(),
                    payload.clone(),
                    meta.as_str()
                ],
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
