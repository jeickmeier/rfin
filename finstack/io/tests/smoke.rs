//! Smoke tests for the `finstack-io` SQLite backend.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_io::{LookbackStore, SqliteStore, Store};
use finstack_portfolio::{Portfolio, Position, PositionUnit};
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::instruments::InstrumentJson;
use std::sync::Arc;
use time::macros::date;

#[test]
fn sqlite_market_context_roundtrip() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db"))?;

    let as_of = date!(2024 - 01 - 01);
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    store.put_market_context("DEFAULT", as_of, &ctx, None)?;
    let loaded = store
        .get_market_context("DEFAULT", as_of)?
        .ok_or_else(|| finstack_io::Error::not_found("market_context", "DEFAULT@2024-01-01"))?;

    let disc = loaded.get_discount("USD-OIS")?;
    assert_eq!(disc.id().as_str(), "USD-OIS");
    Ok(())
}

#[test]
fn sqlite_portfolio_hydrates_instruments() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db"))?;

    let as_of = date!(2024 - 01 - 01);

    // Store instrument definition.
    let deposit = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    store.put_instrument("DEP_1M", &InstrumentJson::Deposit(deposit), None)?;

    // Create a portfolio spec with missing inline instrument_spec (should be resolved from registry).
    let mut portfolio = Portfolio::new("FUND_A", Currency::USD, as_of);
    portfolio.entities.insert(
        finstack_portfolio::EntityId::new("FUND_A"),
        finstack_portfolio::Entity::new("FUND_A"),
    );

    let instrument = Arc::from(
        InstrumentJson::Deposit(
            Deposit::builder()
                .id("DEP_1M".into())
                .notional(Money::new(1_000_000.0, Currency::USD))
                .start(as_of)
                .end(date!(2024 - 02 - 01))
                .day_count(finstack_core::dates::DayCount::Act360)
                .discount_curve_id("USD-OIS".into())
                .build()?,
        )
        .into_boxed()?,
    );

    let position = Position::new(
        "POS_1",
        "FUND_A",
        "DEP_1M",
        instrument,
        1.0,
        PositionUnit::Units,
    )?;
    portfolio.positions.push(position);

    let mut spec = portfolio.to_spec();
    if let Some(pos) = spec.positions.get_mut(0) {
        pos.instrument_spec = None;
    } else {
        return Err(finstack_io::Error::Invariant(
            "Expected at least one position in portfolio spec".into(),
        ));
    }

    store.put_portfolio_spec("FUND_A", as_of, &spec, None)?;

    let hydrated = store.load_portfolio("FUND_A", as_of)?;
    assert_eq!(hydrated.positions.len(), 1);
    let first = hydrated.positions.first().ok_or_else(|| {
        finstack_io::Error::Invariant("Expected at least one hydrated position".into())
    })?;
    assert_eq!(first.instrument_id, "DEP_1M");

    Ok(())
}

#[test]
fn sqlite_market_context_lookback_queries() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db"))?;

    let d1 = date!(2024 - 01 - 01);
    let d2 = date!(2024 - 01 - 02);

    let curve1 = DiscountCurve::builder("USD-OIS")
        .base_date(d1)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx1 = MarketContext::new().insert_discount(curve1);

    let curve2 = DiscountCurve::builder("USD-OIS")
        .base_date(d2)
        .knots(vec![(0.0, 1.0), (1.0, 0.97)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx2 = MarketContext::new().insert_discount(curve2);

    store.put_market_context("DEFAULT", d1, &ctx1, None)?;
    store.put_market_context("DEFAULT", d2, &ctx2, None)?;

    let snaps = store.list_market_contexts("DEFAULT", d1, d2)?;
    assert_eq!(snaps.len(), 2);

    let first = snaps
        .first()
        .ok_or_else(|| finstack_io::Error::Invariant("Expected first snapshot".into()))?;
    assert_eq!(first.as_of, "2024-01-01");

    let latest = store
        .latest_market_context_on_or_before("DEFAULT", d2)?
        .ok_or_else(|| finstack_io::Error::Invariant("Expected latest snapshot".into()))?;
    assert_eq!(latest.as_of, "2024-01-02");

    Ok(())
}
