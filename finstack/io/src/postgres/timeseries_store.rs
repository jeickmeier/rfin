//! TimeSeriesStore trait implementation for PostgresStore.

use crate::{
    sql::{statements, Backend},
    Result, SeriesKey, SeriesKind, TimeSeriesPoint, TimeSeriesStore,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::store::{parse_ts_key, ts_key, PostgresStore};

#[async_trait]
impl TimeSeriesStore for PostgresStore {
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let meta = meta.cloned();
        let sql = statements::upsert_series_meta_sql(Backend::Postgres);

        let conn = self.get_conn().await?;
        conn.execute(
            &sql,
            &[&key.namespace, &key.kind.as_str(), &key.series_id, &meta],
        )
        .await?;
        Ok(())
    }

    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let sql = statements::select_series_meta_sql(Backend::Postgres);

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(&sql, &[&key.namespace, &key.kind.as_str(), &key.series_id])
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
        let sql = statements::list_series_sql(Backend::Postgres);

        let conn = self.get_conn().await?;
        let rows = conn.query(&sql, &[&namespace, &kind.as_str()]).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        let sql = statements::upsert_series_point_sql(Backend::Postgres);

        let mut conn = self.get_conn().await?;
        let tx = conn.transaction().await?;
        for point in points {
            let ts = ts_key(point.ts)?;
            let payload = point.payload.clone();
            let meta = point.meta.clone();
            tx.execute(
                &sql,
                &[
                    &key.namespace,
                    &key.kind.as_str(),
                    &key.series_id,
                    &ts,
                    &point.value,
                    &payload,
                    &meta,
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
        let mut sql = statements::select_points_range_sql(Backend::Postgres);
        if let Some(max) = limit {
            sql = format!("{sql} LIMIT {max}");
        }
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
        let sql = statements::latest_point_sql(Backend::Postgres);
        let ts = ts_key(ts)?;

        let conn = self.get_conn().await?;
        let row = conn
            .query_opt(
                &sql,
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
