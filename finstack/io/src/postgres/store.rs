//! PostgresStore struct and helper utilities.

use crate::{
    sql::{migrations, Backend},
    Error, Result,
};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use finstack_core::dates::Date;
use postgres::{Client, NoTls};

/// A Postgres-backed store.
///
/// This store is `Send + Sync` because it opens a new connection per call.
#[derive(Clone, Debug)]
pub struct PostgresStore {
    pub(crate) url: String,
}

impl PostgresStore {
    /// Connect to a Postgres database at `url`, applying migrations.
    pub fn connect(url: &str) -> Result<Self> {
        let store = Self {
            url: url.to_string(),
        };
        store.with_conn(migrate)?;
        Ok(store)
    }

    /// Connection URL used by this store.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn with_conn<R>(&self, f: impl FnOnce(&mut Client) -> Result<R>) -> Result<R> {
        let mut client = Client::connect(&self.url, NoTls)?;
        client.batch_execute("SET statement_timeout = 5000")?;
        f(&mut client)
    }
}

pub(crate) fn migrate(client: &mut Client) -> Result<()> {
    client.batch_execute(&migrations::schema_migrations_table_sql(Backend::Postgres))?;

    let row = client.query_opt("SELECT MAX(version) FROM finstack_schema_migrations", &[])?;
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
    let mut tx = client.transaction()?;
    for (version, statements) in migrations {
        if version <= current {
            continue;
        }
        for sql in statements {
            tx.batch_execute(&sql)?;
        }
        tx.execute(
            "INSERT INTO finstack_schema_migrations (version, applied_at) VALUES ($1, now())",
            &[&version],
        )?;
    }
    tx.commit()?;
    Ok(())
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
