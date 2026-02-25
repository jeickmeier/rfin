//! TimeSeriesStore trait implementation for SqliteStore.

use crate::{
    sql::{statements, Backend},
    Result, SeriesKey, SeriesKind, TimeSeriesPoint, TimeSeriesStore,
};
use async_trait::async_trait;
use rusqlite::params;
use time::OffsetDateTime;

use super::store::{optional_row, ts_key, SqliteStore};

type SeriesRow = (String, Option<f64>, Option<String>, Option<String>);

#[async_trait]
impl TimeSeriesStore for SqliteStore {
    async fn put_series_meta(
        &self,
        key: &SeriesKey,
        meta: Option<&serde_json::Value>,
    ) -> Result<()> {
        let meta = match meta {
            Some(value) => Some(serde_json::to_string(value)?),
            None => None,
        };
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let naming = std::sync::Arc::clone(&self.naming);

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let sql = statements::upsert_series_meta_sql_with_naming(Backend::Sqlite, &naming);
                conn.execute(sql.as_ref(), params![namespace, kind, series_id, meta])?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let naming = std::sync::Arc::clone(&self.naming);

        let meta: Option<String> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<String>> {
                let sql = statements::select_series_meta_sql_with_naming(Backend::Sqlite, &naming);
                Ok(optional_row(conn.query_row(
                    sql.as_ref(),
                    params![namespace, kind, series_id],
                    |row| row.get(0),
                ))?)
            })
            .await?;

        crate::helpers::optional_json_string_to_value(meta)
    }

    async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let namespace = namespace.to_string();
        let kind_str = kind.as_str().to_string();
        let naming = std::sync::Arc::clone(&self.naming);

        let ids = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<String>> {
                let sql = statements::list_series_sql_with_naming(Backend::Sqlite, &naming);
                let mut stmt = conn.prepare(sql.as_ref())?;
                let rows = stmt.query_map(params![namespace, kind_str], |row| row.get(0))?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;
        Ok(ids)
    }

    async fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        // Pre-serialize all points before entering the closure
        let serialized: Vec<(String, Option<f64>, Option<String>, Option<String>)> = points
            .iter()
            .map(|point| {
                let ts = ts_key(point.ts)?;
                let payload = match &point.payload {
                    Some(value) => Some(serde_json::to_string(value)?),
                    None => None,
                };
                let meta = match &point.meta {
                    Some(value) => Some(serde_json::to_string(value)?),
                    None => None,
                };
                Ok((ts, point.value, payload, meta))
            })
            .collect::<Result<Vec<_>>>()?;

        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let naming = std::sync::Arc::clone(&self.naming);

        self.conn
            .call(move |conn| -> tokio_rusqlite::Result<()> {
                let tx = conn.unchecked_transaction()?;
                {
                    let sql =
                        statements::upsert_series_point_sql_with_naming(Backend::Sqlite, &naming);
                    let mut stmt = tx.prepare(sql.as_ref())?;
                    for (ts, value, payload, meta) in &serialized {
                        stmt.execute(params![
                            namespace, kind, series_id, ts, value, payload, meta
                        ])?;
                    }
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    async fn get_points_range(
        &self,
        key: &SeriesKey,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let start = ts_key(start)?;
        let end = ts_key(end)?;
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let naming = std::sync::Arc::clone(&self.naming);

        let rows: Vec<SeriesRow> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Vec<SeriesRow>> {
                let base_sql =
                    statements::select_points_range_sql_with_naming(Backend::Sqlite, &naming);
                let sql = match limit {
                    Some(max) => format!("{} LIMIT {max}", base_sql.as_ref()),
                    None => base_sql.as_ref().to_string(),
                };
                let mut stmt = conn.prepare(&sql)?;
                let rows =
                    stmt.query_map(params![namespace, kind, series_id, start, end], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, Option<f64>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                        ))
                    })?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                Ok(out)
            })
            .await?;

        let mut out = Vec::new();
        for (ts_str, value, payload, meta) in rows {
            out.push(crate::helpers::time_series_point_from_row(
                ts_str, value, payload, meta,
            )?);
        }
        Ok(out)
    }

    async fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let ts = ts_key(ts)?;
        let namespace = key.namespace.clone();
        let kind = key.kind.as_str().to_string();
        let series_id = key.series_id.clone();
        let naming = std::sync::Arc::clone(&self.naming);

        let row: Option<SeriesRow> = self
            .conn
            .call(move |conn| -> tokio_rusqlite::Result<Option<SeriesRow>> {
                let sql = statements::latest_point_sql_with_naming(Backend::Sqlite, &naming);
                Ok(optional_row(conn.query_row(
                    sql.as_ref(),
                    params![namespace, kind, series_id, ts],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                ))?)
            })
            .await?;

        match row {
            Some((ts_str, value, payload, meta)) => Ok(Some(
                crate::helpers::time_series_point_from_row(ts_str, value, payload, meta)?,
            )),
            None => Ok(None),
        }
    }
}
