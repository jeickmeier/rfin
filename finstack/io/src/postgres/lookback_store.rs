//! LookbackStore trait implementation for PostgresStore.

use crate::{
    sql::{statements, Backend},
    LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;

use super::store::{as_of_key, parse_as_of_key, PostgresStore};

#[async_trait]
impl LookbackStore for PostgresStore {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let sql = statements::list_market_contexts_sql(Backend::Postgres);
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;

        let conn = self.get_conn().await?;
        let rows = conn.query(sql, &[&market_id, &start, &end]).await?;
        let mut out = Vec::new();
        for row in rows {
            let as_of: NaiveDate = row.get(0);
            let payload: serde_json::Value = row.get(1);
            let state: MarketContextState = serde_json::from_value(payload)?;
            let ctx = MarketContext::try_from(state)?;
            out.push(MarketContextSnapshot {
                as_of: parse_as_of_key(as_of)?,
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
        let sql = statements::latest_market_context_sql(Backend::Postgres);
        let as_of = as_of_key(as_of)?;

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql, &[&market_id, &as_of]).await?;
        match row {
            Some(row) => {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let state: MarketContextState = serde_json::from_value(payload)?;
                let ctx = MarketContext::try_from(state)?;
                Ok(Some(MarketContextSnapshot {
                    as_of: parse_as_of_key(as_of)?,
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
        let sql = statements::list_portfolios_sql(Backend::Postgres);
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;

        let conn = self.get_conn().await?;
        let rows = conn.query(sql, &[&portfolio_id, &start, &end]).await?;
        let mut out = Vec::new();
        for row in rows {
            let as_of: NaiveDate = row.get(0);
            let payload: serde_json::Value = row.get(1);
            let spec: PortfolioSpec = serde_json::from_value(payload)?;
            out.push(PortfolioSnapshot {
                as_of: parse_as_of_key(as_of)?,
                spec,
            });
        }
        Ok(out)
    }

    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let sql = statements::latest_portfolio_sql(Backend::Postgres);
        let as_of = as_of_key(as_of)?;

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql, &[&portfolio_id, &as_of]).await?;
        match row {
            Some(row) => {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                let spec: PortfolioSpec = serde_json::from_value(payload)?;
                Ok(Some(PortfolioSnapshot {
                    as_of: parse_as_of_key(as_of)?,
                    spec,
                }))
            }
            None => Ok(None),
        }
    }
}
