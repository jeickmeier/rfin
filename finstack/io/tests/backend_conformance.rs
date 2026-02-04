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
use time::macros::date;
use time::OffsetDateTime;

fn run_conformance<S: Store + BulkStore + LookbackStore + TimeSeriesStore>(
    store: &S,
    prefix: &str,
) -> finstack_io::Result<()> {
    let as_of = date!(2024 - 01 - 01);

    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    let market_id = format!("{prefix}_MARKET");
    store.put_market_context(&market_id, as_of, &ctx, None)?;
    let loaded = store
        .get_market_context(&market_id, as_of)?
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
    store.put_instrument(
        &format!("{prefix}_DEP_1M"),
        &InstrumentJson::Deposit(deposit),
        None,
    )?;

    let series_key = SeriesKey::new(
        format!("{prefix}_ns"),
        format!("{prefix}_series"),
        SeriesKind::Quote,
    );
    store.put_series_meta(&series_key, Some(&serde_json::json!({"source": "test"})))?;

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
    store.put_points_batch(&series_key, &points)?;

    let range = store.get_points_range(
        &series_key,
        now - time::Duration::hours(1),
        now + time::Duration::hours(2),
        None,
    )?;
    assert_eq!(range.len(), 2);

    let latest = store.latest_point_on_or_before(&series_key, now + time::Duration::hours(2))?;
    assert!(latest.is_some());

    Ok(())
}

#[test]
fn sqlite_conformance() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = finstack_io::SqliteStore::open(dir.path().join("finstack.db"))?;
    run_conformance(&store, "sqlite")
}

#[cfg(feature = "postgres")]
#[test]
fn postgres_conformance() -> finstack_io::Result<()> {
    let url = match std::env::var("POSTGRES_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("POSTGRES_URL not set; skipping postgres_conformance");
            return Ok(());
        }
    };
    let store = finstack_io::PostgresStore::connect(&url)?;
    run_conformance(&store, "postgres")
}
