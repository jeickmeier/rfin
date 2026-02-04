//! BulkStore trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
    BulkStore, Result,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;
use rusqlite::params;

use super::store::{as_of_key, meta_json, SqliteStore};

impl BulkStore for SqliteStore {
    fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let sql = statements::upsert_instrument_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

                for (instrument_id, instrument, meta) in instruments {
                    let payload = serde_json::to_vec(instrument)?;
                    let meta = meta_json(*meta)?;
                    stmt.execute(params![instrument_id, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let sql = statements::upsert_market_context_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

                for (market_id, as_of, context, meta) in contexts {
                    let state: MarketContextState = (*context).into();
                    let payload = serde_json::to_vec(&state)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    stmt.execute(params![market_id, as_of, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            {
                let sql = statements::upsert_portfolio_sql(Backend::Sqlite);
                let mut stmt = tx.prepare(&sql)?;

                for (portfolio_id, as_of, spec, meta) in portfolios {
                    let payload = serde_json::to_vec(spec)?;
                    let meta = meta_json(*meta)?;
                    let as_of = as_of_key(*as_of);
                    stmt.execute(params![portfolio_id, as_of, payload, meta])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }
}
