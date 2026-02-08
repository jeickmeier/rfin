//! Backend conformance tests for `finstack-io`.
//!
//! These tests verify that all storage backends (SQLite, Postgres, Turso)
//! implement the storage traits consistently.
//!
//! # Running Tests
//!
//! ## SQLite (default)
//!
//! SQLite tests run automatically with no configuration:
//!
//! ```bash
//! cargo test -p finstack-io
//! ```
//!
//! ## Postgres
//!
//! Postgres tests require:
//! 1. The `postgres` feature enabled
//! 2. A running Postgres instance
//! 3. The `POSTGRES_URL` environment variable set
//!
//! ```bash
//! # Start a local Postgres (example using Docker)
//! docker run -d --name finstack-pg \
//!     -e POSTGRES_PASSWORD=test \
//!     -e POSTGRES_DB=finstack_test \
//!     -p 5432:5432 \
//!     postgres:15
//!
//! # Run tests with Postgres
//! POSTGRES_URL="postgres://postgres:test@localhost:5432/finstack_test" \
//!     cargo test -p finstack-io --features postgres
//! ```
//!
//! If `POSTGRES_URL` is not set, Postgres tests are skipped with a message.
//!
//! # CI Considerations
//!
//! In CI environments, you may want to:
//! - Always run SQLite tests (no external dependencies)
//! - Optionally run Postgres tests when a Postgres service is available
//! - Use GitHub Actions services or similar to provision Postgres

use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_core::{currency::Currency, dates::DayCount};
use finstack_io::{
    BulkStore, LookbackStore, SeriesKey, SeriesKind, Store, TimeSeriesPoint, TimeSeriesStore,
};
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::instruments::InstrumentJson;
#[cfg(feature = "postgres")]
use indexmap::IndexMap;
use time::macros::date;
use time::OffsetDateTime;

async fn run_conformance<S: Store + BulkStore + LookbackStore + TimeSeriesStore>(
    store: &S,
    prefix: &str,
) -> finstack_io::Result<()> {
    let as_of = date!(2024 - 01 - 01);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    let market_id = format!("{prefix}_MARKET");
    store
        .put_market_context(&market_id, as_of, &ctx, None)
        .await?;
    let loaded = store
        .get_market_context(&market_id, as_of)
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("market_context", &market_id))?;
    let disc = loaded.get_discount("USD-OIS")?;
    assert_eq!(disc.id().as_str(), "USD-OIS");

    let deposit = Deposit::builder()
        .id(format!("{prefix}_DEP_1M").into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;
    store
        .put_instrument(
            &format!("{prefix}_DEP_1M"),
            &InstrumentJson::Deposit(deposit),
            None,
        )
        .await?;

    let series_key = SeriesKey::new(
        format!("{prefix}_ns"),
        format!("{prefix}_series"),
        SeriesKind::Quote,
    );
    store
        .put_series_meta(&series_key, Some(&serde_json::json!({"source": "test"})))
        .await?;

    let now = OffsetDateTime::now_utc();
    let points = vec![
        TimeSeriesPoint {
            ts: now,
            value: Some(100.0),
            payload: Some(serde_json::json!({"mid": 100.0})),
            meta: None,
        },
        TimeSeriesPoint {
            ts: now + time::Duration::hours(1),
            value: Some(101.0),
            payload: Some(serde_json::json!({"mid": 101.0})),
            meta: None,
        },
    ];
    store.put_points_batch(&series_key, &points).await?;

    let range = store
        .get_points_range(
            &series_key,
            now - time::Duration::hours(1),
            now + time::Duration::hours(2),
            None,
        )
        .await?;
    assert_eq!(range.len(), 2);

    let latest = store
        .latest_point_on_or_before(&series_key, now + time::Duration::hours(2))
        .await?;
    assert!(latest.is_some());

    Ok(())
}

#[tokio::test]
async fn sqlite_conformance() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = finstack_io::SqliteStore::open(dir.path().join("finstack.db")).await?;
    run_conformance(&store, "sqlite").await
}

#[cfg(feature = "turso")]
#[tokio::test]
async fn turso_conformance() -> finstack_io::Result<()> {
    let store = finstack_io::TursoStore::open_in_memory().await?;
    run_conformance(&store, "turso").await
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_conformance() -> finstack_io::Result<()> {
    let url = match std::env::var("POSTGRES_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("POSTGRES_URL not set; skipping postgres_conformance");
            return Ok(());
        }
    };
    let store = finstack_io::PostgresStore::connect(&url).await?;
    run_conformance(&store, "postgres").await
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres_bulk_store_roundtrip() -> finstack_io::Result<()> {
    let url = match std::env::var("POSTGRES_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("POSTGRES_URL not set; skipping postgres_bulk_store_roundtrip");
            return Ok(());
        }
    };

    let store = finstack_io::PostgresStore::connect(&url).await?;
    let as_of = date!(2024 - 01 - 01);
    let prefix = format!("bulk_{}", OffsetDateTime::now_utc().unix_timestamp_nanos());

    // ---------------------------------------------------------------------
    // Instruments: large batch to cross chunk boundary
    // ---------------------------------------------------------------------
    let count = 2_001; // > CHUNK_SIZE (2_000)
    let mut ids = Vec::with_capacity(count);
    let mut instruments = Vec::with_capacity(count);
    for i in 0..count {
        let id = format!("{prefix}_DEP_{i}");
        let deposit = Deposit::builder()
            .id(id.clone().into())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .start(as_of)
            .end(date!(2024 - 02 - 01))
            .day_count(DayCount::Act360)
            .discount_curve_id("USD-OIS".into())
            .build()?;
        ids.push(id);
        instruments.push(InstrumentJson::Deposit(deposit));
    }

    let instrument_batch: Vec<(&str, &InstrumentJson, Option<&serde_json::Value>)> = ids
        .iter()
        .zip(instruments.iter())
        .map(|(id, instr)| (id.as_str(), instr, None))
        .collect();
    store.put_instruments_batch(&instrument_batch).await?;

    let loaded_first = store
        .get_instrument(&ids[0])
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("instrument", &ids[0]))?;
    assert!(matches!(loaded_first, InstrumentJson::Deposit(_)));

    let loaded_last = store
        .get_instrument(&ids[count - 1])
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("instrument", &ids[count - 1]))?;
    assert!(matches!(loaded_last, InstrumentJson::Deposit(_)));

    // ---------------------------------------------------------------------
    // Market contexts: small batch
    // ---------------------------------------------------------------------
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    let market_id = format!("{prefix}_MARKET");
    let d1 = date!(2024 - 01 - 01);
    let d2 = date!(2024 - 01 - 02);
    store
        .put_market_contexts_batch(&[(&market_id, d1, &ctx, None), (&market_id, d2, &ctx, None)])
        .await?;

    let loaded = store
        .get_market_context(&market_id, d2)
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("market_context", &market_id))?;
    let disc = loaded.get_discount("USD-OIS")?;
    assert_eq!(disc.id().as_str(), "USD-OIS");

    // ---------------------------------------------------------------------
    // Portfolios: small batch
    // ---------------------------------------------------------------------
    let portfolio_id = format!("{prefix}_PORT");
    let spec = finstack_portfolio::PortfolioSpec {
        id: portfolio_id.clone(),
        name: None,
        base_ccy: Currency::USD,
        as_of: d1,
        positions: vec![],
        entities: IndexMap::new(),
        books: IndexMap::new(),
        tags: IndexMap::new(),
        meta: IndexMap::new(),
    };

    store
        .put_portfolios_batch(&[
            (&portfolio_id, d1, &spec, None),
            (&portfolio_id, d2, &spec, None),
        ])
        .await?;
    let loaded = store
        .get_portfolio_spec(&portfolio_id, d2)
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("portfolio", &portfolio_id))?;
    assert_eq!(loaded.id, portfolio_id);

    Ok(())
}
