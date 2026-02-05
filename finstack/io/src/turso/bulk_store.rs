//! BulkStore trait implementation for TursoStore.

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
        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_instrument_sql(Backend::Sqlite);
        for (instrument_id, instrument, meta) in instruments {
            let payload = serde_json::to_vec(instrument)?;
            let meta = meta_json(*meta)?;
            tx.execute(&sql, params![*instrument_id, payload, meta])
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_market_context_sql(Backend::Sqlite);
        for (market_id, as_of, context, meta) in contexts {
            let state: MarketContextState = (*context).into();
            let payload = serde_json::to_vec(&state)?;
            let meta = meta_json(*meta)?;
            let as_of = as_of_key(*as_of);
            tx.execute(&sql, params![*market_id, as_of, payload, meta])
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
        for (portfolio_id, as_of, spec, meta) in portfolios {
            let payload = serde_json::to_vec(spec)?;
            let meta = meta_json(*meta)?;
            let as_of = as_of_key(*as_of);
            tx.execute(&sql, params![*portfolio_id, as_of, payload, meta])
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
