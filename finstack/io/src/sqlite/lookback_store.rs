//! LookbackStore trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
    LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use rusqlite::params;

use super::store::{as_of_key, optional_row, parse_as_of_key, SqliteStore};

#[async_trait]
impl LookbackStore for SqliteStore {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);
        let market_id = market_id.to_string();

        let rows: Vec<(String, Vec<u8>)> = self
            .conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Vec<(String, Vec<u8>)>> {
                    let sql = statements::list_market_contexts_sql(Backend::Sqlite);
                    let mut stmt = conn.prepare(&sql)?;
                    let rows = stmt.query_map(params![market_id, start, end], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })?;

                    let mut out = Vec::new();
                    for row in rows {
                        out.push(row?);
                    }
                    Ok(out)
                },
            )
            .await?;

        let mut out = Vec::new();
        for (as_of_str, bytes) in rows {
            let as_of = parse_as_of_key(&as_of_str)?;
            let state: MarketContextState = serde_json::from_slice(&bytes)?;
            let ctx = MarketContext::try_from(state)?;
            out.push(MarketContextSnapshot {
                as_of,
                context: ctx,
            });
        }
        Ok(out)
    }

    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        let as_of = as_of_key(as_of);
        let market_id = market_id.to_string();

        let row: Option<(String, Vec<u8>)> = self
            .conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Option<(String, Vec<u8>)>> {
                    let sql = statements::latest_market_context_sql(Backend::Sqlite);
                    Ok(optional_row(conn.query_row(
                        &sql,
                        params![market_id, as_of],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    ))?)
                },
            )
            .await?;

        match row {
            Some((as_of_str, bytes)) => {
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
    }

    async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>> {
        let start = as_of_key(start);
        let end = as_of_key(end);
        let portfolio_id = portfolio_id.to_string();

        let rows: Vec<(String, Vec<u8>)> = self
            .conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Vec<(String, Vec<u8>)>> {
                    let sql = statements::list_portfolios_sql(Backend::Sqlite);
                    let mut stmt = conn.prepare(&sql)?;
                    let rows = stmt.query_map(params![portfolio_id, start, end], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
                    })?;

                    let mut out = Vec::new();
                    for row in rows {
                        out.push(row?);
                    }
                    Ok(out)
                },
            )
            .await?;

        let mut out = Vec::new();
        for (as_of_str, bytes) in rows {
            let as_of = parse_as_of_key(&as_of_str)?;
            let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
            out.push(PortfolioSnapshot { as_of, spec });
        }
        Ok(out)
    }

    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let as_of = as_of_key(as_of);
        let portfolio_id = portfolio_id.to_string();

        let row: Option<(String, Vec<u8>)> = self
            .conn
            .call(
                move |conn| -> tokio_rusqlite::Result<Option<(String, Vec<u8>)>> {
                    let sql = statements::latest_portfolio_sql(Backend::Sqlite);
                    Ok(optional_row(conn.query_row(
                        &sql,
                        params![portfolio_id, as_of],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    ))?)
                },
            )
            .await?;

        match row {
            Some((as_of_str, bytes)) => {
                let as_of = parse_as_of_key(&as_of_str)?;
                let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                Ok(Some(PortfolioSnapshot { as_of, spec }))
            }
            None => Ok(None),
        }
    }
}
