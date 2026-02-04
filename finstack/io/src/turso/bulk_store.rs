//! BulkStore trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    BulkStore, Error, Result,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;
use turso::params;

use super::store::{as_of_key, meta_json, TursoStore};

impl BulkStore for TursoStore {
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let tx = conn.transaction().await.map_err(Error::Turso)?;

                let sql = statements::upsert_instrument_sql(Backend::Sqlite);

                for (instrument_id, instrument, meta) in instruments {
                    let payload = serde_json::to_vec(instrument)?;
                    let meta = meta_json(*meta)?;
                    tx.execute(&sql, params![*instrument_id, payload, meta])
                        .await
                        .map_err(Error::Turso)?;
                }

                tx.commit().await.map_err(Error::Turso)?;
                Ok(())
            })
        })
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let tx = conn.transaction().await.map_err(Error::Turso)?;

                let sql = statements::upsert_market_context_sql(Backend::Sqlite);

                for (market_id, as_of, context, meta) in contexts {
                    let state: MarketContextState = (*context).into();
                    let payload = serde_json::to_vec(&state)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    tx.execute(&sql, params![*market_id, as_of, payload, meta])
                        .await
                        .map_err(Error::Turso)?;
                }

                tx.commit().await.map_err(Error::Turso)?;
                Ok(())
            })
        })
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let tx = conn.transaction().await.map_err(Error::Turso)?;

                let sql = statements::upsert_portfolio_sql(Backend::Sqlite);

                for (portfolio_id, as_of, spec, meta) in portfolios {
                    let payload = serde_json::to_vec(spec)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    tx.execute(&sql, params![*portfolio_id, as_of, payload, meta])
                        .await
                        .map_err(Error::Turso)?;
                }

                tx.commit().await.map_err(Error::Turso)?;
                Ok(())
            })
        })
    }
}
