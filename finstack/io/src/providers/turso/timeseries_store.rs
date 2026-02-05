//! TimeSeriesStore trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    Error, Result, SeriesKey, SeriesKind, TimeSeriesPoint, TimeSeriesStore,
};
use async_trait::async_trait;
use libsql::params;
use time::OffsetDateTime;

use super::store::{
    get_optional_f64, get_optional_string, get_string, meta_json_str, parse_ts_key, ts_key,
    TursoStore,
};

#[async_trait]
impl TimeSeriesStore for TursoStore {
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let conn = self.get_conn()?;
        let sql = statements::upsert_series_meta_sql(Backend::Sqlite);
        let meta_str = meta_json_str(meta)?;
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        conn.execute(sql, params![namespace, kind, series_id, meta_str])
            .await?;
        Ok(())
    }

    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let conn = self.get_conn()?;
        let sql = statements::select_series_meta_sql(Backend::Sqlite);
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let mut stmt = conn.prepare(sql).await?;
        let mut rows = stmt.query(params![namespace, kind, series_id]).await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let meta_str = get_optional_string(&row, 0)?;
                match meta_str {
                    Some(s) if !s.is_empty() => Ok(Some(serde_json::from_str(&s)?)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let conn = self.get_conn()?;
        let sql = statements::list_series_sql(Backend::Sqlite);
        let namespace = namespace.to_string();
        let kind_str = kind.as_str().to_string();
        let mut stmt = conn.prepare(sql).await?;
        let mut rows = stmt.query(params![namespace, kind_str]).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            out.push(get_string(&row, 0)?);
        }
        Ok(out)
    }

    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        let sql = statements::upsert_series_point_sql(Backend::Sqlite);
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();

        for point in points {
            let ts = ts_key(point.ts)?;
            let payload = match &point.payload {
                Some(value) => Some(serde_json::to_string(value)?),
                None => None,
            };
            let meta = meta_json_str(point.meta.as_ref())?;
            tx.execute(
                sql,
                params![
                    namespace.clone(),
                    kind.clone(),
                    series_id.clone(),
                    ts,
                    point.value,
                    payload,
                    meta
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
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let conn = self.get_conn()?;
        let base_sql = statements::select_points_range_sql(Backend::Sqlite);
        let sql = match limit {
            Some(max) => format!("{base_sql} LIMIT {max}"),
            None => base_sql.to_string(),
        };
        let start_ts = ts_key(start)?;
        let end_ts = ts_key(end)?;
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let mut stmt = conn.prepare(&sql).await?;
        let mut rows = stmt
            .query(params![namespace, kind, series_id, start_ts, end_ts])
            .await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(Error::Turso)? {
            let ts_str = get_string(&row, 0)?;
            let value = get_optional_f64(&row, 1)?;
            let payload_str = get_optional_string(&row, 2)?;
            let meta_str = get_optional_string(&row, 3)?;

            let payload = match payload_str {
                Some(s) if !s.is_empty() => Some(serde_json::from_str(&s)?),
                _ => None,
            };
            let meta = match meta_str {
                Some(s) if !s.is_empty() => Some(serde_json::from_str(&s)?),
                _ => None,
            };

            out.push(TimeSeriesPoint {
                ts: parse_ts_key(&ts_str)?,
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
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let conn = self.get_conn()?;
        let sql = statements::latest_point_sql(Backend::Sqlite);
        let ts_str = ts_key(ts)?;
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let mut stmt = conn.prepare(sql).await?;
        let mut rows = stmt
            .query(params![namespace, kind, series_id, ts_str])
            .await?;

        match rows.next().await.map_err(Error::Turso)? {
            Some(row) => {
                let ts_str = get_string(&row, 0)?;
                let value = get_optional_f64(&row, 1)?;
                let payload_str = get_optional_string(&row, 2)?;
                let meta_str = get_optional_string(&row, 3)?;

                let payload = match payload_str {
                    Some(s) if !s.is_empty() => Some(serde_json::from_str(&s)?),
                    _ => None,
                };
                let meta = match meta_str {
                    Some(s) if !s.is_empty() => Some(serde_json::from_str(&s)?),
                    _ => None,
                };

                Ok(Some(TimeSeriesPoint {
                    ts: parse_ts_key(&ts_str)?,
                    value,
                    payload,
                    meta,
                }))
            }
            None => Ok(None),
        }
    }
}
