//! BulkStore trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
    BulkStore, Result,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;
use rusqlite::params;

use super::store::{as_of_key, meta_json, SqliteStore};

#[async_trait]
impl BulkStore for SqliteStore {
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before entering the closure
        let serialized: Vec<(String, Vec<u8>, String)> = instruments
            .iter()
            .map(|(id, instrument, meta)| {
                let payload = serde_json::to_vec(instrument)?;
                let meta = meta_json(*meta)?;
                Ok((id.to_string(), payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let tx = conn.unchecked_transaction()?;
                {
                    let sql = statements::upsert_instrument_sql(Backend::Sqlite);
                    let mut stmt = tx.prepare(sql)?;

                    for (instrument_id, payload, meta) in &serialized {
                        stmt.execute(params![instrument_id, payload, meta])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before entering the closure
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

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let tx = conn.unchecked_transaction()?;
                {
                    let sql = statements::upsert_market_context_sql(Backend::Sqlite);
                    let mut stmt = tx.prepare(sql)?;

                    for (market_id, as_of, payload, meta) in &serialized {
                        stmt.execute(params![market_id, as_of, payload, meta])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        // Pre-serialize all data before entering the closure
        let serialized: Vec<(String, String, Vec<u8>, String)> = portfolios
            .iter()
            .map(|(portfolio_id, as_of, spec, meta)| {
                let payload = serde_json::to_vec(spec)?;
                let meta = meta_json(*meta)?;
                let as_of = as_of_key(*as_of);
                Ok((portfolio_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let tx = conn.unchecked_transaction()?;
                {
                    let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
                    let mut stmt = tx.prepare(sql)?;

                    for (portfolio_id, as_of, payload, meta) in &serialized {
                        stmt.execute(params![portfolio_id, as_of, payload, meta])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }
}
