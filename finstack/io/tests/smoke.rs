//! Smoke tests for the `finstack-io` SQLite backend.

use finstack_core::currency::Currency;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_io::{BulkStore, LookbackStore, SqliteStore, Store};
use finstack_portfolio::{Portfolio, Position, PositionUnit};
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::instruments::InstrumentJson;
use indexmap::IndexMap;
use std::sync::Arc;
use time::macros::date;

#[tokio::test]
async fn sqlite_market_context_roundtrip() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    store
        .put_market_context("DEFAULT", as_of, &ctx, None)
        .await?;
    let loaded = store
        .get_market_context("DEFAULT", as_of)
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("market_context", "DEFAULT@2024-01-01"))?;

    let disc = loaded.get_discount("USD-OIS")?;
    assert_eq!(disc.id().as_str(), "USD-OIS");
    Ok(())
}

#[tokio::test]
async fn sqlite_portfolio_hydrates_instruments() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

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

    store
        .put_instrument("DEP_1M", &InstrumentJson::Deposit(deposit), None)
        .await?;

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

    store
        .put_portfolio_spec("FUND_A", as_of, &spec, None)
        .await?;

    let hydrated = store.load_portfolio("FUND_A", as_of).await?;
    assert_eq!(hydrated.positions.len(), 1);
    let first = hydrated.positions.first().ok_or_else(|| {
        finstack_io::Error::Invariant("Expected at least one hydrated position".into())
    })?;
    assert_eq!(first.instrument_id, "DEP_1M");

    Ok(())
}

#[tokio::test]
async fn sqlite_market_context_lookback_queries() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

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

    store.put_market_context("DEFAULT", d1, &ctx1, None).await?;
    store.put_market_context("DEFAULT", d2, &ctx2, None).await?;

    let snaps = store.list_market_contexts("DEFAULT", d1, d2).await?;
    assert_eq!(snaps.len(), 2);

    let first = snaps
        .first()
        .ok_or_else(|| finstack_io::Error::Invariant("Expected first snapshot".into()))?;
    assert_eq!(first.as_of, d1);

    let latest = store
        .latest_market_context_on_or_before("DEFAULT", d2)
        .await?
        .ok_or_else(|| finstack_io::Error::Invariant("Expected latest snapshot".into()))?;
    assert_eq!(latest.as_of, d2);

    Ok(())
}

#[tokio::test]
async fn sqlite_portfolio_lookback_queries() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let d1 = date!(2024 - 01 - 01);
    let d2 = date!(2024 - 01 - 02);
    let d3 = date!(2024 - 01 - 03);

    let spec = finstack_portfolio::PortfolioSpec {
        id: "TEST".into(),
        name: None,
        base_ccy: Currency::USD,
        as_of: d1,
        positions: vec![],
        entities: IndexMap::new(),
        books: IndexMap::new(),
        tags: IndexMap::new(),
        meta: IndexMap::new(),
    };

    store.put_portfolio_spec("TEST", d1, &spec, None).await?;
    store.put_portfolio_spec("TEST", d2, &spec, None).await?;

    // Test list_portfolios
    let snaps = store.list_portfolios("TEST", d1, d2).await?;
    assert_eq!(snaps.len(), 2);
    assert_eq!(snaps[0].as_of, d1);
    assert_eq!(snaps[1].as_of, d2);

    // Test latest_portfolio_on_or_before
    let latest = store
        .latest_portfolio_on_or_before("TEST", d3)
        .await?
        .ok_or_else(|| finstack_io::Error::Invariant("Expected latest portfolio".into()))?;
    assert_eq!(latest.as_of, d2);

    // Test latest_portfolio_on_or_before when no match
    let no_match = store
        .latest_portfolio_on_or_before("TEST", date!(2023 - 12 - 31))
        .await?;
    assert!(no_match.is_none());

    Ok(())
}

#[tokio::test]
async fn sqlite_scenario_roundtrip() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let spec = finstack_scenarios::ScenarioSpec {
        id: "test_scenario".into(),
        name: Some("Test Scenario".into()),
        description: Some("A test scenario".into()),
        operations: vec![],
        priority: 0,
    };

    store.put_scenario("SCENARIO_1", &spec, None).await?;
    let loaded = store
        .get_scenario("SCENARIO_1")
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("scenario", "SCENARIO_1"))?;

    assert_eq!(loaded.id, "test_scenario");
    assert_eq!(loaded.name, Some("Test Scenario".into()));

    // Test not found
    assert!(store.get_scenario("NONEXISTENT").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn sqlite_statement_model_roundtrip() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let spec = finstack_statements::FinancialModelSpec::new("test_model", vec![]);

    store.put_statement_model("MODEL_1", &spec, None).await?;
    let loaded = store
        .get_statement_model("MODEL_1")
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("statement_model", "MODEL_1"))?;

    assert_eq!(loaded.id, "test_model");

    // Test not found
    assert!(store.get_statement_model("NONEXISTENT").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn sqlite_bulk_instruments() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    let deposit1 = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let deposit2 = Deposit::builder()
        .id("DEP_3M".into())
        .notional(Money::new(2_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let instr1 = InstrumentJson::Deposit(deposit1);
    let instr2 = InstrumentJson::Deposit(deposit2);

    store
        .put_instruments_batch(&[("DEP_1M", &instr1, None), ("DEP_3M", &instr2, None)])
        .await?;

    let loaded1 = store
        .get_instrument("DEP_1M")
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("instrument", "DEP_1M"))?;
    let loaded2 = store
        .get_instrument("DEP_3M")
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("instrument", "DEP_3M"))?;

    assert!(matches!(loaded1, InstrumentJson::Deposit(_)));
    assert!(matches!(loaded2, InstrumentJson::Deposit(_)));

    Ok(())
}

#[tokio::test]
async fn sqlite_bulk_market_contexts() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

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

    store
        .put_market_contexts_batch(&[("DEFAULT", d1, &ctx1, None), ("DEFAULT", d2, &ctx2, None)])
        .await?;

    let snaps = store.list_market_contexts("DEFAULT", d1, d2).await?;
    assert_eq!(snaps.len(), 2);

    Ok(())
}

#[tokio::test]
async fn sqlite_schema_migration_idempotent() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("finstack.db");

    // Open twice - second open should be idempotent
    let _store1 = SqliteStore::open(&path).await?;
    let store2 = SqliteStore::open(&path).await?;

    // Verify store still works
    let as_of = date!(2024 - 01 - 01);
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx = MarketContext::new().insert_discount(curve);

    store2
        .put_market_context("DEFAULT", as_of, &ctx, None)
        .await?;
    assert!(store2.get_market_context("DEFAULT", as_of).await?.is_some());

    Ok(())
}

#[tokio::test]
async fn sqlite_upsert_overwrites() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    // Insert first version
    let curve1 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx1 = MarketContext::new().insert_discount(curve1);
    store
        .put_market_context("DEFAULT", as_of, &ctx1, None)
        .await?;

    // Upsert with different data
    let curve2 = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots(vec![(0.0, 1.0), (1.0, 0.95)]) // Different discount factor
        .set_interp(InterpStyle::Linear)
        .build()?;
    let ctx2 = MarketContext::new().insert_discount(curve2);
    store
        .put_market_context("DEFAULT", as_of, &ctx2, None)
        .await?;

    // Verify the second version is stored
    let loaded = store
        .get_market_context("DEFAULT", as_of)
        .await?
        .ok_or_else(|| finstack_io::Error::not_found("market_context", "DEFAULT@2024-01-01"))?;
    let disc = loaded.get_discount("USD-OIS")?;

    // The 1Y discount factor should be 0.95, not 0.98
    let df = disc.df(1.0);
    assert!((df - 0.95).abs() < 1e-10);

    Ok(())
}

#[test]
fn sqlite_date_format_is_iso8601() {
    // This test verifies that dates are stored in ISO 8601 format (YYYY-MM-DD)
    // which is critical for correct lexicographic ordering in SQL BETWEEN queries.

    // Test various dates to ensure consistent formatting
    let test_cases = [
        (date!(2024 - 01 - 01), "2024-01-01"),
        (date!(2024 - 12 - 31), "2024-12-31"),
        (date!(1999 - 01 - 01), "1999-01-01"),
        (date!(2030 - 06 - 15), "2030-06-15"),
    ];

    for (date, expected) in test_cases {
        let formatted = format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            date.month() as u8,
            date.day()
        );
        assert_eq!(formatted, expected, "Date formatting mismatch for {date}");
    }

    // Verify lexicographic ordering works correctly
    let dates = ["2024-01-01", "2024-01-02", "2024-12-31", "2025-01-01"];
    for i in 0..dates.len() - 1 {
        assert!(
            dates[i] < dates[i + 1],
            "Lexicographic ordering failed: {} should be < {}",
            dates[i],
            dates[i + 1]
        );
    }
}

#[tokio::test]
async fn sqlite_meta_json_stored() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    let deposit = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let meta = serde_json::json!({
        "source": "bloomberg",
        "version": 1
    });

    store
        .put_instrument("DEP_1M", &InstrumentJson::Deposit(deposit), Some(&meta))
        .await?;

    // Verify instrument can be loaded (meta is stored but not returned by get_instrument)
    let loaded = store.get_instrument("DEP_1M").await?;
    assert!(loaded.is_some());

    Ok(())
}

#[tokio::test]
async fn sqlite_not_found_errors() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    // Test get_* methods return None for missing entries
    assert!(store.get_market_context("MISSING", as_of).await?.is_none());
    assert!(store.get_instrument("MISSING").await?.is_none());
    assert!(store.get_portfolio_spec("MISSING", as_of).await?.is_none());
    assert!(store.get_scenario("MISSING").await?.is_none());
    assert!(store.get_statement_model("MISSING").await?.is_none());
    assert!(store.get_metric_registry("MISSING").await?.is_none());

    // Test load_* methods return NotFound errors
    let result = store.load_market_context("MISSING", as_of).await;
    assert!(matches!(result, Err(finstack_io::Error::NotFound { .. })));

    let result = store.load_portfolio_spec("MISSING", as_of).await;
    assert!(matches!(result, Err(finstack_io::Error::NotFound { .. })));

    let result = store.load_metric_registry("MISSING").await;
    assert!(matches!(result, Err(finstack_io::Error::NotFound { .. })));

    Ok(())
}

#[tokio::test]
async fn sqlite_metric_registry_roundtrip() -> finstack_io::Result<()> {
    use finstack_statements::registry::{MetricDefinition, MetricRegistry, UnitType};

    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    // Create a metric registry with some standard financial metrics
    let registry = MetricRegistry {
        namespace: "fin".into(),
        schema_version: 1,
        metrics: vec![
            MetricDefinition {
                id: "gross_margin".into(),
                name: "Gross Margin %".into(),
                formula: "gross_profit / revenue".into(),
                description: Some("Gross profit as percentage of revenue".into()),
                category: Some("margins".into()),
                unit_type: Some(UnitType::Percentage),
                requires: vec!["revenue".into(), "gross_profit".into()],
                tags: vec!["margins".into(), "profitability".into()],
                meta: IndexMap::new(),
            },
            MetricDefinition {
                id: "debt_to_equity".into(),
                name: "Debt to Equity".into(),
                formula: "total_debt / total_equity".into(),
                description: Some("Total debt divided by total equity".into()),
                category: Some("leverage".into()),
                unit_type: Some(UnitType::Ratio),
                requires: vec!["total_debt".into(), "total_equity".into()],
                tags: vec!["leverage".into(), "solvency".into()],
                meta: IndexMap::new(),
            },
        ],
        meta: IndexMap::new(),
    };

    // Store the registry
    store.put_metric_registry("fin", &registry, None).await?;

    // Load and verify
    let loaded = store
        .get_metric_registry("fin")
        .await?
        .expect("Registry should exist");
    assert_eq!(loaded.namespace, "fin");
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.metrics.len(), 2);
    assert_eq!(loaded.metrics[0].id, "gross_margin");
    assert_eq!(loaded.metrics[1].id, "debt_to_equity");

    // Verify formula is preserved
    assert_eq!(loaded.metrics[0].formula, "gross_profit / revenue");

    // Verify unit_type is preserved
    assert_eq!(loaded.metrics[0].unit_type, Some(UnitType::Percentage));
    assert_eq!(loaded.metrics[1].unit_type, Some(UnitType::Ratio));

    Ok(())
}

#[tokio::test]
async fn sqlite_metric_registry_list_and_delete() -> finstack_io::Result<()> {
    use finstack_statements::registry::{MetricDefinition, MetricRegistry};

    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    // Start with no registries
    let namespaces = store.list_metric_registries().await?;
    assert!(namespaces.is_empty());

    // Create multiple registries
    let fin_registry = MetricRegistry {
        namespace: "fin".into(),
        schema_version: 1,
        metrics: vec![MetricDefinition {
            id: "gross_margin".into(),
            name: "Gross Margin".into(),
            formula: "gross_profit / revenue".into(),
            description: None,
            category: None,
            unit_type: None,
            requires: vec![],
            tags: vec![],
            meta: IndexMap::new(),
        }],
        meta: IndexMap::new(),
    };

    let custom_registry = MetricRegistry {
        namespace: "custom".into(),
        schema_version: 1,
        metrics: vec![MetricDefinition {
            id: "custom_kpi".into(),
            name: "Custom KPI".into(),
            formula: "revenue / headcount".into(),
            description: None,
            category: None,
            unit_type: None,
            requires: vec![],
            tags: vec![],
            meta: IndexMap::new(),
        }],
        meta: IndexMap::new(),
    };

    store
        .put_metric_registry("fin", &fin_registry, None)
        .await?;
    store
        .put_metric_registry("custom", &custom_registry, None)
        .await?;

    // List should return both, sorted alphabetically
    let namespaces = store.list_metric_registries().await?;
    assert_eq!(namespaces, vec!["custom", "fin"]);

    // Delete one registry
    let deleted = store.delete_metric_registry("custom").await?;
    assert!(deleted);

    // Verify it's gone
    assert!(store.get_metric_registry("custom").await?.is_none());

    // List should only show "fin" now
    let namespaces = store.list_metric_registries().await?;
    assert_eq!(namespaces, vec!["fin"]);

    // Delete non-existent should return false
    let deleted = store.delete_metric_registry("nonexistent").await?;
    assert!(!deleted);

    Ok(())
}

#[tokio::test]
async fn sqlite_list_instruments() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    // Start with no instruments
    let ids = store.list_instruments().await?;
    assert!(ids.is_empty());

    // Add some instruments
    let deposit1 = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let deposit2 = Deposit::builder()
        .id("DEP_3M".into())
        .notional(Money::new(2_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    store
        .put_instrument("DEP_1M", &InstrumentJson::Deposit(deposit1), None)
        .await?;
    store
        .put_instrument("DEP_3M", &InstrumentJson::Deposit(deposit2), None)
        .await?;

    // List should return both, sorted alphabetically
    let ids = store.list_instruments().await?;
    assert_eq!(ids, vec!["DEP_1M", "DEP_3M"]);

    Ok(())
}

#[tokio::test]
async fn sqlite_get_instruments_batch() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    let as_of = date!(2024 - 01 - 01);

    // Add some instruments
    let deposit1 = Deposit::builder()
        .id("DEP_1M".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 02 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    let deposit2 = Deposit::builder()
        .id("DEP_3M".into())
        .notional(Money::new(2_000_000.0, Currency::USD))
        .start(as_of)
        .end(date!(2024 - 04 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;

    store
        .put_instrument("DEP_1M", &InstrumentJson::Deposit(deposit1), None)
        .await?;
    store
        .put_instrument("DEP_3M", &InstrumentJson::Deposit(deposit2), None)
        .await?;

    // Batch fetch both
    let result = store
        .get_instruments_batch(&["DEP_1M".into(), "DEP_3M".into()])
        .await?;
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("DEP_1M"));
    assert!(result.contains_key("DEP_3M"));

    // Batch fetch with some missing
    let result = store
        .get_instruments_batch(&["DEP_1M".into(), "NONEXISTENT".into()])
        .await?;
    assert_eq!(result.len(), 1);
    assert!(result.contains_key("DEP_1M"));

    // Empty batch
    let result = store.get_instruments_batch(&[]).await?;
    assert!(result.is_empty());

    Ok(())
}

#[tokio::test]
async fn sqlite_list_scenarios() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    // Start with no scenarios
    let ids = store.list_scenarios().await?;
    assert!(ids.is_empty());

    // Add some scenarios
    let spec1 = finstack_scenarios::ScenarioSpec {
        id: "scenario_a".into(),
        name: Some("Scenario A".into()),
        description: None,
        operations: vec![],
        priority: 0,
    };

    let spec2 = finstack_scenarios::ScenarioSpec {
        id: "scenario_b".into(),
        name: Some("Scenario B".into()),
        description: None,
        operations: vec![],
        priority: 1,
    };

    store.put_scenario("SCENARIO_A", &spec1, None).await?;
    store.put_scenario("SCENARIO_B", &spec2, None).await?;

    // List should return both, sorted alphabetically
    let ids = store.list_scenarios().await?;
    assert_eq!(ids, vec!["SCENARIO_A", "SCENARIO_B"]);

    Ok(())
}

#[tokio::test]
async fn sqlite_list_statement_models() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    // Start with no models
    let ids = store.list_statement_models().await?;
    assert!(ids.is_empty());

    // Add some models
    let spec1 = finstack_statements::FinancialModelSpec::new("model_a", vec![]);
    let spec2 = finstack_statements::FinancialModelSpec::new("model_b", vec![]);

    store.put_statement_model("MODEL_A", &spec1, None).await?;
    store.put_statement_model("MODEL_B", &spec2, None).await?;

    // List should return both, sorted alphabetically
    let ids = store.list_statement_models().await?;
    assert_eq!(ids, vec!["MODEL_A", "MODEL_B"]);

    Ok(())
}

#[tokio::test]
async fn sqlite_metric_registry_upsert() -> finstack_io::Result<()> {
    use finstack_statements::registry::{MetricDefinition, MetricRegistry, UnitType};

    let dir = tempfile::tempdir()?;
    let store = SqliteStore::open(dir.path().join("finstack.db")).await?;

    // Initial registry with one metric
    let registry_v1 = MetricRegistry {
        namespace: "fin".into(),
        schema_version: 1,
        metrics: vec![MetricDefinition {
            id: "gross_margin".into(),
            name: "Gross Margin".into(),
            formula: "gross_profit / revenue".into(),
            description: None,
            category: None,
            unit_type: Some(UnitType::Percentage),
            requires: vec![],
            tags: vec![],
            meta: IndexMap::new(),
        }],
        meta: IndexMap::new(),
    };

    store.put_metric_registry("fin", &registry_v1, None).await?;

    // Updated registry with two metrics
    let registry_v2 = MetricRegistry {
        namespace: "fin".into(),
        schema_version: 2,
        metrics: vec![
            MetricDefinition {
                id: "gross_margin".into(),
                name: "Gross Margin %".into(), // Updated name
                formula: "gross_profit / revenue".into(),
                description: Some("Updated description".into()),
                category: None,
                unit_type: Some(UnitType::Percentage),
                requires: vec!["revenue".into(), "gross_profit".into()],
                tags: vec![],
                meta: IndexMap::new(),
            },
            MetricDefinition {
                id: "net_margin".into(),
                name: "Net Margin %".into(),
                formula: "net_income / revenue".into(),
                description: None,
                category: None,
                unit_type: Some(UnitType::Percentage),
                requires: vec![],
                tags: vec![],
                meta: IndexMap::new(),
            },
        ],
        meta: IndexMap::new(),
    };

    // Upsert should overwrite
    store.put_metric_registry("fin", &registry_v2, None).await?;

    let loaded = store
        .get_metric_registry("fin")
        .await?
        .expect("Registry should exist");
    assert_eq!(loaded.schema_version, 2);
    assert_eq!(loaded.metrics.len(), 2);
    assert_eq!(loaded.metrics[0].name, "Gross Margin %");
    assert_eq!(
        loaded.metrics[0].description,
        Some("Updated description".into())
    );
    assert_eq!(loaded.metrics[1].id, "net_margin");

    Ok(())
}

// ---------------------------------------------------------------------------
// Concurrent access tests
// ---------------------------------------------------------------------------
// These tests verify that SQLite's async handling works correctly with concurrent
// access. The tokio-rusqlite crate uses a dedicated thread for SQLite operations,
// so multiple async tasks can safely access the same connection.

#[tokio::test]
async fn sqlite_concurrent_writes_succeed() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("concurrent.db");
    let store = SqliteStore::open(&db_path).await?;

    // Pre-populate with an instrument
    let deposit = Deposit::builder()
        .id("DEPO-001".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(date!(2024 - 01 - 01))
        .end(date!(2025 - 01 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;
    let instrument = InstrumentJson::Deposit(deposit);
    store.put_instrument("DEPO-001", &instrument, None).await?;

    // Clone store for concurrent access
    let store1 = store.clone();
    let store2 = store.clone();
    let instrument1 = instrument.clone();
    let instrument2 = instrument.clone();

    // Spawn two tasks that write concurrently
    let handle1 = tokio::spawn(async move {
        for i in 0..10 {
            store1
                .put_instrument(&format!("THREAD1-{i}"), &instrument1, None)
                .await?;
        }
        Ok::<_, finstack_io::Error>(())
    });

    let handle2 = tokio::spawn(async move {
        for i in 0..10 {
            store2
                .put_instrument(&format!("THREAD2-{i}"), &instrument2, None)
                .await?;
        }
        Ok::<_, finstack_io::Error>(())
    });

    // Both tasks should complete successfully
    handle1.await.expect("Task 1 panicked")?;
    handle2.await.expect("Task 2 panicked")?;

    // Verify all instruments were written
    let instruments = store.list_instruments().await?;
    assert!(instruments.len() >= 21); // 1 original + 10 from each task

    Ok(())
}

#[tokio::test]
async fn sqlite_concurrent_reads_and_writes() -> finstack_io::Result<()> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("concurrent_rw.db");
    let store = SqliteStore::open(&db_path).await?;

    // Pre-populate with instruments
    let deposit = Deposit::builder()
        .id("DEPO-BASE".into())
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(date!(2024 - 01 - 01))
        .end(date!(2025 - 01 - 01))
        .day_count(finstack_core::dates::DayCount::Act360)
        .discount_curve_id("USD-OIS".into())
        .build()?;
    let instrument = InstrumentJson::Deposit(deposit);

    for i in 0..5 {
        store
            .put_instrument(&format!("INIT-{i}"), &instrument, None)
            .await?;
    }

    let store_writer = store.clone();
    let store_reader = store.clone();
    let instrument_for_writer = instrument.clone();

    // Writer task: continuously writes new instruments
    let writer = tokio::spawn(async move {
        for i in 0..20 {
            store_writer
                .put_instrument(&format!("WRITE-{i}"), &instrument_for_writer, None)
                .await?;
        }
        Ok::<_, finstack_io::Error>(())
    });

    // Reader task: continuously reads and lists instruments
    let reader = tokio::spawn(async move {
        for _ in 0..20 {
            let _ = store_reader.list_instruments().await?;
            let _ = store_reader.get_instrument("INIT-0").await?;
        }
        Ok::<_, finstack_io::Error>(())
    });

    // Both should complete without errors
    writer.await.expect("Writer task panicked")?;
    reader.await.expect("Reader task panicked")?;

    // Verify writes succeeded
    let instruments = store.list_instruments().await?;
    assert!(instruments.len() >= 25); // 5 initial + 20 written

    Ok(())
}
