//! TimeSeriesStore trait implementation for TursoStore.

use crate::{
    sql::{statements, Backend},
    Error, Result, SeriesKey, SeriesKind, TimeSeriesPoint, TimeSeriesStore,
};
use time::OffsetDateTime;
use turso::params;

use super::store::{parse_ts_key, ts_key, TursoStore};

impl TimeSeriesStore for TursoStore {
    fn put_series_meta(&self, key: &SeriesKey, meta: Option<&serde_json::Value>) -> Result<()> {
        let meta = match meta {
            Some(value) => Some(serde_json::to_string(value)?),
            None => None,
        };
        let sql = statements::upsert_series_meta_sql(Backend::Sqlite);
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                conn.execute(
                    &sql,
                    params![
                        key.namespace.clone(),
                        key.kind.as_str(),
                        key.series_id.clone(),
                        meta
                    ],
                )
                .await
                .map_err(Error::Turso)?;
                Ok(())
            })
        })
    }

    fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let sql = statements::select_series_meta_sql(Backend::Sqlite);
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![
                        key.namespace.clone(),
                        key.kind.as_str(),
                        key.series_id.clone()
                    ])
                    .await
                    .map_err(Error::Turso)?;

                match rows.next().await.map_err(Error::Turso)? {
                    Some(row) => {
                        let meta = get_optional_string(&row, 0)?;
                        match meta {
                            Some(value) => Ok(Some(serde_json::from_str(&value)?)),
                            None => Ok(None),
                        }
                    }
                    None => Ok(None),
                }
            })
        })
    }

    fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        let sql = statements::list_series_sql(Backend::Sqlite);
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![namespace, kind.as_str()])
                    .await
                    .map_err(Error::Turso)?;

                let mut out = Vec::new();
                while let Some(row) = rows.next().await.map_err(Error::Turso)? {
                    out.push(get_string(&row, 0)?);
                }
                Ok(out)
            })
        })
    }

    fn put_points_batch(&self, key: &SeriesKey, points: &[TimeSeriesPoint]) -> Result<()> {
        let sql = statements::upsert_series_point_sql(Backend::Sqlite);
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let tx = conn.transaction().await.map_err(Error::Turso)?;

                for point in points {
                    let ts = ts_key(point.ts)?;
                    let payload = match &point.payload {
                        Some(value) => Some(serde_json::to_string(value)?),
                        None => None,
                    };
                    let meta = match &point.meta {
                        Some(value) => Some(serde_json::to_string(value)?),
                        None => None,
                    };
                    tx.execute(
                        &sql,
                        params![
                            key.namespace.clone(),
                            key.kind.as_str(),
                            key.series_id.clone(),
                            ts,
                            point.value,
                            payload,
                            meta
                        ],
                    )
                    .await
                    .map_err(Error::Turso)?;
                }

                tx.commit().await.map_err(Error::Turso)?;
                Ok(())
            })
        })
    }

    fn get_points_range(
        &self,
        key: &SeriesKey,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let mut sql = statements::select_points_range_sql(Backend::Sqlite);
        if let Some(max) = limit {
            sql = format!("{sql} LIMIT {max}");
        }
        let start = ts_key(start)?;
        let end = ts_key(end)?;
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![
                        key.namespace.clone(),
                        key.kind.as_str(),
                        key.series_id.clone(),
                        start,
                        end
                    ])
                    .await
                    .map_err(Error::Turso)?;

                let mut out = Vec::new();
                while let Some(row) = rows.next().await.map_err(Error::Turso)? {
                    let ts_str = get_string(&row, 0)?;
                    let value = get_optional_f64(&row, 1)?;
                    let payload_str = get_optional_string(&row, 2)?;
                    let meta_str = get_optional_string(&row, 3)?;

                    let payload = match payload_str {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    };
                    let meta = match meta_str {
                        Some(value) => Some(serde_json::from_str(&value)?),
                        None => None,
                    };
                    out.push(TimeSeriesPoint {
                        ts: parse_ts_key(&ts_str)?,
                        value,
                        payload,
                        meta,
                    });
                }
                Ok(out)
            })
        })
    }

    fn latest_point_on_or_before(
        &self,
        key: &SeriesKey,
        ts: OffsetDateTime,
    ) -> Result<Option<TimeSeriesPoint>> {
        let sql = statements::latest_point_sql(Backend::Sqlite);
        let ts = ts_key(ts)?;
        self.with_conn(|conn, runtime| {
            runtime.block_on(async move {
                let mut stmt = conn.prepare(&sql).await.map_err(Error::Turso)?;
                let mut rows = stmt
                    .query(params![
                        key.namespace.clone(),
                        key.kind.as_str(),
                        key.series_id.clone(),
                        ts
                    ])
                    .await
                    .map_err(Error::Turso)?;

                match rows.next().await.map_err(Error::Turso)? {
                    Some(row) => {
                        let ts_str = get_string(&row, 0)?;
                        let value = get_optional_f64(&row, 1)?;
                        let payload_str = get_optional_string(&row, 2)?;
                        let meta_str = get_optional_string(&row, 3)?;

                        let payload = match payload_str {
                            Some(value) => Some(serde_json::from_str(&value)?),
                            None => None,
                        };
                        let meta = match meta_str {
                            Some(value) => Some(serde_json::from_str(&value)?),
                            None => None,
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

fn get_optional_string(row: &turso::Row, idx: usize) -> Result<Option<String>> {
    match row.get_value(idx).map_err(Error::Turso)? {
        turso::value::Value::Text(s) => Ok(Some(s)),
        turso::value::Value::Null => Ok(None),
        other => Err(Error::Invariant(format!(
            "Expected text or null at column {idx}, got {:?}",
            other
        ))),
    }
}

fn get_optional_f64(row: &turso::Row, idx: usize) -> Result<Option<f64>> {
    match row.get_value(idx).map_err(Error::Turso)? {
        turso::value::Value::Real(f) => Ok(Some(f)),
        turso::value::Value::Integer(i) => Ok(Some(i as f64)),
        turso::value::Value::Null => Ok(None),
        other => Err(Error::Invariant(format!(
            "Expected real or null at column {idx}, got {:?}",
            other
        ))),
    }
}
