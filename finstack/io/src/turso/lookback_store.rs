//! LookbackStore trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    Error, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use turso::params;

use super::store::{as_of_key, parse_as_of_key, TursoStore};

impl LookbackStore for TursoStore {
    fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);

        self.with_conn(|conn, runtime| {
            let sql = statements::list_market_contexts_sql(Backend::Sqlite);
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![market_id, start, end])
                    .await
                    .map_err(Error::Turso)?;

                let mut out = Vec::new();
                while let Some(row) = rows.next().await.map_err(Error::Turso)? {
                    let as_of_str = get_string(&row, 0)?;
                    let bytes = get_blob(&row, 1)?;
                    let as_of = parse_as_of_key(&as_of_str)?;
                    let state: MarketContextState = serde_json::from_slice(&bytes)?;
                    let ctx = MarketContext::try_from(state)?;
                    out.push(MarketContextSnapshot {
                        as_of,
                        context: ctx,
                    });
                }
                Ok(out)
            })
        })
    }

    fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn, runtime| {
            let sql = statements::latest_market_context_sql(Backend::Sqlite);
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![market_id, as_of])
                    .await
                    .map_err(Error::Turso)?;

                match rows.next().await.map_err(Error::Turso)? {
                    Some(row) => {
                        let as_of_str = get_string(&row, 0)?;
                        let bytes = get_blob(&row, 1)?;
                        let as_of = parse_as_of_key(&as_of_str)?;
                        let state: MarketContextState = serde_json::from_slice(&bytes)?;
                        let ctx = MarketContext::try_from(state)?;
                        Ok(Some(MarketContextSnapshot {
                            as_of,
                            context: ctx,
                        }))
                    }
                    None => Ok(None),
                }
            })
        })
    }

    fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);

        self.with_conn(|conn, runtime| {
            let sql = statements::list_portfolios_sql(Backend::Sqlite);
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![portfolio_id, start, end])
                    .await
                    .map_err(Error::Turso)?;

                let mut out = Vec::new();
                while let Some(row) = rows.next().await.map_err(Error::Turso)? {
                    let as_of_str = get_string(&row, 0)?;
                    let bytes = get_blob(&row, 1)?;
                    let as_of = parse_as_of_key(&as_of_str)?;
                    let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                    out.push(PortfolioSnapshot { as_of, spec });
                }
                Ok(out)
            })
        })
    }

    fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let as_of = as_of_key(as_of);
        self.with_conn(|conn, runtime| {
            let sql = statements::latest_portfolio_sql(Backend::Sqlite);
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![portfolio_id, as_of])
                    .await
                    .map_err(Error::Turso)?;

                match rows.next().await.map_err(Error::Turso)? {
                    Some(row) => {
                        let as_of_str = get_string(&row, 0)?;
                        let bytes = get_blob(&row, 1)?;
                        let as_of = parse_as_of_key(&as_of_str)?;
                        let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                        Ok(Some(PortfolioSnapshot { as_of, spec }))
                    }
                    None => Ok(None),
                }
            })
        })
    }
}

// Helper functions to extract values from Turso rows

fn get_string(row: &turso::Row, idx: usize) -> Result<String> {
    match row.get_value(idx).map_err(Error::Turso)? {
        turso::value::Value::Text(s) => Ok(s),
        other => Err(Error::Invariant(format!(
            "Expected text at column {idx}, got {:?}",
            other
        ))),
    }
}

fn get_blob(row: &turso::Row, idx: usize) -> Result<Vec<u8>> {
    match row.get_value(idx).map_err(Error::Turso)? {
        turso::value::Value::Blob(b) => Ok(b),
        other => Err(Error::Invariant(format!(
            "Expected blob at column {idx}, got {:?}",
            other
        ))),
    }
}
