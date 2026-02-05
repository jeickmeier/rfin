//! BulkStore trait implementation for PostgresStore.
//!
//! This implementation pre-serializes all data before opening the transaction,
//! matching the pattern used by SQLite and Turso backends. This provides:
//! - Early error detection (serialization errors before transaction starts)
//! - Reduced transaction hold time (no serialization inside the transaction)
//! - Consistent behavior across all backends

use crate::{BulkStore, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{MarketContext, MarketContextState};
use finstack_portfolio::PortfolioSpec;
use finstack_valuations::instruments::InstrumentJson;

use super::store::{as_of_key, meta_json, quote_ident, PostgresStore};

#[async_trait]
impl BulkStore for PostgresStore {
    async fn put_instruments_batch(
        &self,
        instruments: &[(&str, &InstrumentJson, Option<&serde_json::Value>)],
    ) -> Result<()> {
        if instruments.is_empty() {
            return Ok(());
        }

        // Pre-serialize all data before opening the transaction.
        let serialized: Vec<(String, serde_json::Value, serde_json::Value)> = instruments
            .iter()
            .map(|(id, instrument, meta)| {
                let payload = serde_json::to_value(instrument)?;
                let meta = meta_json(*meta);
                Ok((id.to_string(), payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        // Use UNNEST to upsert many rows per round-trip (much faster than row-by-row executes).
        let instruments_table = quote_ident(&self.naming().resolve("instruments"));
        let upsert_sql = format!(
            "INSERT INTO {instruments_table} (id, payload, meta)\n\
SELECT t.id, t.payload, t.meta\n\
FROM UNNEST($1::text[], $2::jsonb[], $3::jsonb[]) AS t(id, payload, meta)\n\
ON CONFLICT (id)\n\
DO UPDATE SET\n\
  payload = EXCLUDED.payload,\n\
  meta = EXCLUDED.meta,\n\
  updated_at = now()"
        );

        let mut ids = Vec::with_capacity(serialized.len());
        let mut payloads = Vec::with_capacity(serialized.len());
        let mut metas = Vec::with_capacity(serialized.len());
        for (id, payload, meta) in serialized {
            ids.push(id);
            payloads.push(payload);
            metas.push(meta);
        }

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;

        // Chunk to cap parameter payload sizes.
        const CHUNK_SIZE: usize = 2_000;
        for start in (0..ids.len()).step_by(CHUNK_SIZE) {
            let end = (start + CHUNK_SIZE).min(ids.len());
            let ids_slice = &ids[start..end];
            let payloads_slice = &payloads[start..end];
            let metas_slice = &metas[start..end];
            tx.execute(&upsert_sql, &[&ids_slice, &payloads_slice, &metas_slice])
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn put_market_contexts_batch(
        &self,
        contexts: &[(&str, Date, &MarketContext, Option<&serde_json::Value>)],
    ) -> Result<()> {
        if contexts.is_empty() {
            return Ok(());
        }

        // Pre-serialize all data before opening the transaction.
        let serialized: Vec<(String, NaiveDate, serde_json::Value, serde_json::Value)> = contexts
            .iter()
            .map(|(market_id, as_of, context, meta)| {
                let state: MarketContextState = (*context).into();
                let payload = serde_json::to_value(&state)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                Ok((market_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let market_contexts_table = quote_ident(&self.naming().resolve("market_contexts"));
        let upsert_sql = format!(
            "INSERT INTO {market_contexts_table} (id, as_of, payload, meta)\n\
SELECT t.id, t.as_of, t.payload, t.meta\n\
FROM UNNEST($1::text[], $2::date[], $3::jsonb[], $4::jsonb[]) AS t(id, as_of, payload, meta)\n\
ON CONFLICT (id, as_of)\n\
DO UPDATE SET\n\
  payload = EXCLUDED.payload,\n\
  meta = EXCLUDED.meta,\n\
  updated_at = now()"
        );

        let mut ids = Vec::with_capacity(serialized.len());
        let mut as_ofs = Vec::with_capacity(serialized.len());
        let mut payloads = Vec::with_capacity(serialized.len());
        let mut metas = Vec::with_capacity(serialized.len());
        for (id, as_of, payload, meta) in serialized {
            ids.push(id);
            as_ofs.push(as_of);
            payloads.push(payload);
            metas.push(meta);
        }

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;

        const CHUNK_SIZE: usize = 1_000;
        for start in (0..ids.len()).step_by(CHUNK_SIZE) {
            let end = (start + CHUNK_SIZE).min(ids.len());
            let ids_slice = &ids[start..end];
            let as_ofs_slice = &as_ofs[start..end];
            let payloads_slice = &payloads[start..end];
            let metas_slice = &metas[start..end];
            tx.execute(
                &upsert_sql,
                &[&ids_slice, &as_ofs_slice, &payloads_slice, &metas_slice],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn put_portfolios_batch(
        &self,
        portfolios: &[(&str, Date, &PortfolioSpec, Option<&serde_json::Value>)],
    ) -> Result<()> {
        if portfolios.is_empty() {
            return Ok(());
        }

        // Pre-serialize all data before opening the transaction.
        let serialized: Vec<(String, NaiveDate, serde_json::Value, serde_json::Value)> = portfolios
            .iter()
            .map(|(portfolio_id, as_of, spec, meta)| {
                let payload = serde_json::to_value(spec)?;
                let meta = meta_json(*meta);
                let as_of = as_of_key(*as_of)?;
                Ok((portfolio_id.to_string(), as_of, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let portfolios_table = quote_ident(&self.naming().resolve("portfolios"));
        let upsert_sql = format!(
            "INSERT INTO {portfolios_table} (id, as_of, payload, meta)\n\
SELECT t.id, t.as_of, t.payload, t.meta\n\
FROM UNNEST($1::text[], $2::date[], $3::jsonb[], $4::jsonb[]) AS t(id, as_of, payload, meta)\n\
ON CONFLICT (id, as_of)\n\
DO UPDATE SET\n\
  payload = EXCLUDED.payload,\n\
  meta = EXCLUDED.meta,\n\
  updated_at = now()"
        );

        let mut ids = Vec::with_capacity(serialized.len());
        let mut as_ofs = Vec::with_capacity(serialized.len());
        let mut payloads = Vec::with_capacity(serialized.len());
        let mut metas = Vec::with_capacity(serialized.len());
        for (id, as_of, payload, meta) in serialized {
            ids.push(id);
            as_ofs.push(as_of);
            payloads.push(payload);
            metas.push(meta);
        }

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;

        const CHUNK_SIZE: usize = 1_000;
        for start in (0..ids.len()).step_by(CHUNK_SIZE) {
            let end = (start + CHUNK_SIZE).min(ids.len());
            let ids_slice = &ids[start..end];
            let as_ofs_slice = &as_ofs[start..end];
            let payloads_slice = &payloads[start..end];
            let metas_slice = &metas[start..end];
            tx.execute(
                &upsert_sql,
                &[&ids_slice, &as_ofs_slice, &payloads_slice, &metas_slice],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
