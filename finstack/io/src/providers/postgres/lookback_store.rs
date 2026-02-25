//! LookbackStore trait implementation for PostgresStore.

use super::store::{as_of_key, PostgresStore};
use crate::{
    sql::{statements, Backend},
    LookbackStore, MarketContextSnapshot, PortfolioSnapshot, Result,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use finstack_core::dates::Date;

#[async_trait]
impl LookbackStore for PostgresStore {
    async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        let sql =
            statements::list_market_contexts_sql_with_naming(Backend::Postgres, self.naming());
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;

        let conn = self.get_conn().await?;
        let rows = conn
            .query(sql.as_ref(), &[&market_id, &start, &end])
            .await?;
        let mut out = Vec::new();
        for row in rows {
            let as_of: NaiveDate = row.get(0);
            let payload: serde_json::Value = row.get(1);
            out.push(crate::helpers::market_context_snapshot_from_postgres_row(
                as_of, payload,
            )?);
        }
        Ok(out)
    }

    async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        let sql =
            statements::latest_market_context_sql_with_naming(Backend::Postgres, self.naming());
        let as_of = as_of_key(as_of)?;

        let conn = self.get_conn().await?;
        let row = conn.query_opt(sql.as_ref(), &[&market_id, &as_of]).await?;
        match row {
            Some(row) => {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                Ok(Some(
                    crate::helpers::market_context_snapshot_from_postgres_row(as_of, payload)?,
                ))
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
        let sql = statements::list_portfolios_sql_with_naming(Backend::Postgres, self.naming());
        let start = as_of_key(start)?;
        let end = as_of_key(end)?;

        let conn = self.get_conn().await?;
        let rows = conn
            .query(sql.as_ref(), &[&portfolio_id, &start, &end])
            .await?;
        let mut out = Vec::new();
        for row in rows {
            let as_of: NaiveDate = row.get(0);
            let payload: serde_json::Value = row.get(1);
            out.push(crate::helpers::portfolio_snapshot_from_postgres_row(
                as_of, payload,
            )?);
        }
        Ok(out)
    }

    async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        let sql = statements::latest_portfolio_sql_with_naming(Backend::Postgres, self.naming());
        let as_of = as_of_key(as_of)?;

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(sql.as_ref(), &[&portfolio_id, &as_of])
            .await?;
        match row {
            Some(row) => {
                let as_of: NaiveDate = row.get(0);
                let payload: serde_json::Value = row.get(1);
                Ok(Some(crate::helpers::portfolio_snapshot_from_postgres_row(
                    as_of, payload,
                )?))
            }
            None => Ok(None),
        }
    }
}
