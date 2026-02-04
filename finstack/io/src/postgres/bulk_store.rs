//! BulkStore trait implementation for PostgresStore.

use crate::{
    sql::{statements, Backend},
    BulkStore, Result,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;

use super::store::{as_of_key, meta_json, PostgresStore};

impl BulkStore for PostgresStore {
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_instrument_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (instrument_id, instrument, meta) in instruments {
                let payload = serde_json::to_value(instrument)?;
                let meta = meta_json(*meta);
                tx.execute(&sql, &[instrument_id, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_market_context_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (market_id, as_of, context, meta) in contexts {
                let state: MarketContextState = (*context).into();
                let payload = serde_json::to_value(&state)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                tx.execute(&sql, &[market_id, &as_of, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        let sql = statements::upsert_portfolio_sql(Backend::Postgres);
        self.with_conn(|client| {
            let mut tx = client.transaction()?;
            for (portfolio_id, as_of, spec, meta) in portfolios {
                let payload = serde_json::to_value(spec)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                tx.execute(&sql, &[portfolio_id, &as_of, &payload, &meta])?;
            }
            tx.commit()?;
            Ok(())
        })
    }
}
