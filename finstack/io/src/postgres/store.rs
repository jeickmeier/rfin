//! PostgresStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use finstack_core::dates::Date;
use tokio_postgres::NoTls;

/// A Postgres-backed store using async connection pooling.
///
/// This store uses `deadpool-postgres` for connection pooling, providing
/// efficient async access to Postgres with automatic connection management.
#[derive(Clone)]
pub struct PostgresStore {
    pub(crate) pool: Pool,
    pub(crate) url: String,
}

impl std::fmt::Debug for PostgresStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStore")
            .field("url", &self.url)
            .finish_non_exhaustive()
    }
}

impl PostgresStore {
    /// Connect to a Postgres database at `url`, applying migrations.
    ///
    /// This creates a connection pool with default settings:
    /// - Max connections: 16
    /// - Connection recycling: Fast (check connection on borrow)
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_pool_size(url, 16).await
    }

    /// Connect with a custom pool size.
    pub async fn connect_with_pool_size(url: &str, max_size: usize) -> Result<Self> {
        let mut config = Config::new();
        config.url = Some(url.to_string());
        config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)?;

        // Set max pool size
        pool.resize(max_size);

        let store = Self {
            pool,
            url: url.to_string(),
        };

        // Run migrations
        store.migrate().await?;

        Ok(store)
    }

    /// Connection URL used by this store.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get a connection from the pool.
    pub(crate) async fn get_conn(&self) -> Result<deadpool_postgres::Object> {
        let conn = self.pool.get().await?;
        conn.execute("SET statement_timeout = 5000", &[]).await?;
        Ok(conn)
    }

    /// Run schema migrations.
    async fn migrate(&self) -> Result<()> {
        let mut conn = self.get_conn().await?;

        conn.batch_execute(&migrations::schema_migrations_table_sql(Backend::Postgres))
            .await?;

        let row = conn
            .query_opt("SELECT MAX(version) FROM finstack_schema_migrations", &[])
            .await?;
        // MAX() returns NULL when the table is empty (no migrations applied yet).
        // In that case, we treat it as version 0 to apply all migrations.
        let current: i64 = row.and_then(|r| r.get::<_, Option<i64>>(0)).unwrap_or(0);

        if current > migrations::LATEST_VERSION {
            return Err(Error::UnsupportedSchema {
                found: current,
                expected: migrations::LATEST_VERSION,
            });
        }

        if current == migrations::LATEST_VERSION {
            return Ok(());
        }

        let migrations = migrations::migrations_for(Backend::Postgres);
        let tx = conn.transaction().await?;
        for (version, statements) in migrations {
            if version <= current {
                continue;
            }
            for sql in statements {
                tx.batch_execute(&sql).await?;
            }
            tx.execute(
                "INSERT INTO finstack_schema_migrations (version, applied_at) VALUES ($1, now())",
                &[&version],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

pub(crate) fn meta_json(meta: Option<&serde_json::Value>) -> serde_json::Value {
    meta.cloned().unwrap_or_else(|| serde_json::json!({}))
}

pub(crate) fn as_of_key(as_of: Date) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(as_of.year(), as_of.month() as u32, as_of.day() as u32)
        .ok_or_else(|| Error::Invariant("Invalid date".into()))
}

pub(crate) fn parse_as_of_key(value: NaiveDate) -> Result<Date> {
    let month = u8::try_from(value.month())
        .map_err(|_| Error::Invariant("Invalid month from database".into()))?;
    let day = u8::try_from(value.day())
        .map_err(|_| Error::Invariant("Invalid day from database".into()))?;
    Date::from_calendar_date(
        value.year(),
        month
            .try_into()
            .map_err(|_| Error::Invariant("Invalid month from database".into()))?,
        day,
    )
    .map_err(|e| Error::Invariant(format!("Invalid date in database: {e}")))
}

pub(crate) fn ts_key(ts: time::OffsetDateTime) -> Result<DateTime<Utc>> {
    let seconds = ts.unix_timestamp();
    let nanos = ts.nanosecond();
    DateTime::<Utc>::from_timestamp(seconds, nanos)
        .ok_or_else(|| Error::Invariant("Invalid timestamp".into()))
}

pub(crate) fn parse_ts_key(ts: DateTime<Utc>) -> Result<time::OffsetDateTime> {
    let secs = ts.timestamp();
    let nanos = ts.timestamp_subsec_nanos();
    let base = time::OffsetDateTime::from_unix_timestamp(secs)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp in database: {e}")))?;
    base.replace_nanosecond(nanos)
        .map_err(|e| Error::Invariant(format!("Invalid timestamp in database: {e}")))
}
