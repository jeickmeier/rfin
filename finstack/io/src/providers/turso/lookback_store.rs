//! LookbackStore trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    Error, LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result,
};
use async_trait::async_trait;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use libsql::params;

use super::store::{as_of_key, get_blob, get_string, parse_as_of_key, TursoStore};

#[async_trait]
impl LookbackStore for TursoStore {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let start_key = as_of_key(start);
        let end_key = as_of_key(end);
        let conn = self.get_conn()?;
        let sql = statements::list_market_contexts_sql_with_naming(Backend::Sqlite, self.naming());
        let stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![market_id, start_key, end_key]).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let as_of_str = get_string(&row, 0)?;
            let bytes = get_blob(&row, 1)?;
            let as_of = parse_as_of_key(&as_of_str)?;
            let state: MarketContextState = serde_json::from_slice(&bytes)?;
            let context = MarketContext::try_from(state)?;
            out.push(MarketContextSnapshot { as_of, context });
        }
        Ok(out)
    }

    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        let as_of_key_str = as_of_key(as_of);
        let conn = self.get_conn()?;
        let sql = statements::latest_market_context_sql_with_naming(Backend::Sqlite, self.naming());
        let stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![market_id, as_of_key_str]).await?;

        match rows.next().await.map_err(Error::from)? {
            Some(row) => {
                let as_of_str = get_string(&row, 0)?;
                let bytes = get_blob(&row, 1)?;
                let as_of = parse_as_of_key(&as_of_str)?;
                let state: MarketContextState = serde_json::from_slice(&bytes)?;
                let context = MarketContext::try_from(state)?;
                Ok(Some(MarketContextSnapshot { as_of, context }))
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
        let start_key = as_of_key(start);
        let end_key = as_of_key(end);
        let conn = self.get_conn()?;
        let sql = statements::list_portfolios_sql_with_naming(Backend::Sqlite, self.naming());
        let stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt
            .query(params![portfolio_id, start_key, end_key])
            .await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::from)? {
            let as_of_str = get_string(&row, 0)?;
            let bytes = get_blob(&row, 1)?;
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
        let as_of_key_str = as_of_key(as_of);
        let conn = self.get_conn()?;
        let sql = statements::latest_portfolio_sql_with_naming(Backend::Sqlite, self.naming());
        let stmt = conn.prepare(sql.as_ref()).await?;
        let mut rows = stmt.query(params![portfolio_id, as_of_key_str]).await?;

        match rows.next().await.map_err(Error::from)? {
            Some(row) => {
                let as_of_str = get_string(&row, 0)?;
                let bytes = get_blob(&row, 1)?;
                let as_of = parse_as_of_key(&as_of_str)?;
                let spec: PortfolioSpec = serde_json::from_slice(&bytes)?;
                Ok(Some(PortfolioSnapshot { as_of, spec }))
            }
            None => Ok(None),
        }
    }
}
