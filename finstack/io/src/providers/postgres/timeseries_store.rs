//! TimeSeriesStore trait implementation for PostgresStore.

use crate::{
    sql::{statements, Backend},
    Result, SeriesKey, SeriesKind, TimeSeriesPoint, TimeSeriesStore,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::store::{parse_ts_key, quote_ident, ts_key, PostgresStore};

#[async_trait]
impl TimeSeriesStore for PostgresStore {
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let meta = meta.cloned();
        let sql = statements::upsert_series_meta_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        conn.execute(
            sql.as_ref(),
            &[&key.namespace, &key.kind.as_str(), &key.series_id, &meta],
        )
        .await?;
        Ok(())
    }

    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let sql = statements::select_series_meta_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(
                sql.as_ref(),
                &[&key.namespace, &key.kind.as_str(), &key.series_id],
            )
            .await?;
        match row {
            Some(row) => {
                let meta: Option<serde_json::Value> = row.get(0);
                Ok(meta)
            }
            None => Ok(None),
        }
    }

    async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let sql = statements::list_series_sql_with_naming(Backend::Postgres, self.naming());

        let conn = self.get_conn().await?;
        let rows = conn
            .query(sql.as_ref(), &[&namespace, &kind.as_str()])
            .await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let series_points_table = quote_ident(&self.naming().resolve("series_points"));
        let upsert_points_unnest_sql = format!(
            "INSERT INTO {series_points_table} (namespace, kind, series_id, ts, value, payload, meta)\n\
SELECT $1, $2, $3, t.ts, t.value, t.payload, t.meta\n\
FROM UNNEST(\n\
  $4::timestamptz[],\n\
  $5::double precision[],\n\
  $6::jsonb[],\n\
  $7::jsonb[]\n\
) AS t(ts, value, payload, meta)\n\
ON CONFLICT (namespace, kind, series_id, ts)\n\
DO UPDATE SET\n\
  value = EXCLUDED.value,\n\
  payload = EXCLUDED.payload,\n\
  meta = EXCLUDED.meta,\n\
  updated_at = now()"
        );

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;

        // Chunk to avoid oversized parameter payloads.
        // Note: this is independent of `MAX_BATCH_SIZE` (which is about query shape / plan cache).
        const POINTS_CHUNK_SIZE: usize = 10_000;
        for chunk in points.chunks(POINTS_CHUNK_SIZE) {
            let mut ts = Vec::with_capacity(chunk.len());
            let mut values = Vec::with_capacity(chunk.len());
            let mut payloads: Vec<Option<serde_json::Value>> = Vec::with_capacity(chunk.len());
            let mut metas: Vec<Option<serde_json::Value>> = Vec::with_capacity(chunk.len());

            for point in chunk {
                ts.push(ts_key(point.ts)?);
                values.push(point.value);
                payloads.push(point.payload.clone());
                metas.push(point.meta.clone());
            }

            tx.execute(
                &upsert_points_unnest_sql,
                &[
                    &key.namespace,
                    &key.kind.as_str(),
                    &key.series_id,
                    &ts,
                    &values,
                    &payloads,
                    &metas,
                ],
            )
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_points_range(
        &self,
        key: &SeriesKey,
        start: time::OffsetDateTime,
        end: time::OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let base_sql =
            statements::select_points_range_sql_with_naming(Backend::Postgres, self.naming());
        let sql = match limit {
            Some(max) => format!("{} LIMIT {max}", base_sql.as_ref()),
            None => base_sql.as_ref().to_string(),
        };
        let start = ts_key(start)?;
        let end = ts_key(end)?;

        let conn = self.get_conn().await?;
        let rows = conn
            .query(
                &sql,
                &[
                    &key.namespace,
                    &key.kind.as_str(),
                    &key.series_id,
                    &start,
                    &end,
                ],
            )
            .await?;
        let mut out = Vec::new();
        for row in rows {
            let ts: DateTime<Utc> = row.get(0);
            let value: Option<f64> = row.get(1);
            let payload: Option<serde_json::Value> = row.get(2);
            let meta: Option<serde_json::Value> = row.get(3);
            out.push(TimeSeriesPoint {
                ts: parse_ts_key(ts)?,
                value,
                payload,
                meta,
            });
        }
        Ok(out)
    }

    async fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: time::OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let sql = statements::latest_point_sql_with_naming(Backend::Postgres, self.naming());
        let ts = ts_key(ts)?;

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(
                sql.as_ref(),
                &[&key.namespace, &key.kind.as_str(), &key.series_id, &ts],
            )
            .await?;
        match row {
            Some(row) => {
                let ts: DateTime<Utc> = row.get(0);
                let value: Option<f64> = row.get(1);
                let payload: Option<serde_json::Value> = row.get(2);
                let meta: Option<serde_json::Value> = row.get(3);
                Ok(Some(TimeSeriesPoint {
                    ts: parse_ts_key(ts)?,
                    value,
                    payload,
                    meta,
                }))
            }
            None => Ok(None),
        }
    }
}
