#![cfg(all(target_arch = "wasm32", feature = "wasm_benchmarks"))]

//! WASM micro-benchmarks for bridge overhead.
//!
//! These are intentionally lightweight and non-flaky:
//! - They do not enforce hard performance thresholds.
//! - They validate the APIs execute and report timings to the console.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

/// Benchmark the overhead of `PricerRegistry.priceInstrument` through WASM bindings.
#[wasm_bindgen_test]
fn bench_price_instrument_roundtrip() {
    use finstack_wasm::*;
    use wasm_bindgen::JsValue;

    let as_of = FsDate::new(2024, 1, 2).expect("Valid date");
    let market = MarketContext::new();

    let usd = Currency::new("USD").expect("Currency constructor should succeed");
    let notional = Money::new(1_000_000.0, &usd);
    let issue = FsDate::new(2024, 1, 1).expect("Valid date");
    let maturity = FsDate::new(2026, 1, 1).expect("Valid date");
    let bond = Bond::new(
        "bond1",
        &notional,
        &issue,
        &maturity,
        "USD-OIS",
        Some(0.05),
        Some(Frequency::semi_annual()),
        Some(DayCount::thirty_360()),
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("Bond constructor should succeed");
    let bond_js: JsValue = bond.into();

    // Use an empty registry to benchmark the WASM bridge overhead without
    // incurring memory-heavy registry initialization in the test runner.
    // This call is expected to return an error due to missing pricer registration.
    let registry = PricerRegistry::new_empty();

    // Warmup
    let _ = registry.price_instrument(&bond_js, "discounting", &market, &as_of, None);

    let iters = 50;
    let t0 = now_ms();
    for _ in 0..iters {
        let _ = registry.price_instrument(&bond_js, "discounting", &market, &as_of, None);
    }
    let dt = now_ms() - t0;

    // Log only; don't assert hard thresholds.
    web_sys::console::log_1(
        &format!(
            "bench_price_instrument_roundtrip: {iters} iters, {:.3}ms total",
            dt
        )
        .into(),
    );

    assert!(dt >= 0.0, "timing should be non-negative");
}

/// Benchmark portfolio valuation result materialization and repeated `positionValues` access.
#[wasm_bindgen_test]
fn bench_portfolio_position_values_access() {
    use finstack_wasm::*;
    use wasm_bindgen::JsValue;

    let as_of = FsDate::new(2024, 1, 2).expect("Valid date");

    let curve = DiscountCurve::new(
        "USD-OIS",
        &as_of,
        vec![0.0, 1.0, 2.0, 5.0],
        vec![1.0, 0.98, 0.955, 0.88],
        JsValue::from_str("act_365f"),
        JsValue::from_str("log_linear"),
        JsValue::from_str("flat_forward"),
        true,
    )
    .expect("DiscountCurve should build");

    let mut market = MarketContext::new();
    market.insert(&curve);

    // Build a portfolio with many similar bond positions to exercise
    // result materialization + JS object creation for `positionValues`.
    let entity = Entity::new("ENTITY_A".to_string());

    let mut builder = PortfolioBuilder::new("PORT_1".to_string())
        .base_ccy(Currency::new("USD").expect("USD"))
        .as_of(&as_of)
        .expect("asOf should set")
        .entity(&entity);

    let usd = Currency::new("USD").expect("Currency constructor should succeed");
    let notional = Money::new(1_000_000.0, &usd);
    let issue = FsDate::new(2024, 1, 1).expect("Valid date");
    let maturity = FsDate::new(2026, 1, 1).expect("Valid date");
    let bond = Bond::new(
        "bond1",
        &notional,
        &issue,
        &maturity,
        "USD-OIS",
        Some(0.05),
        Some(Frequency::semi_annual()),
        Some(DayCount::thirty_360()),
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("Bond constructor should succeed");

    let unit = PositionUnit::face_value();
    let n_positions = 250usize;
    for i in 0..n_positions {
        let pos =
            createPositionFromBond(format!("POS_{i}"), "ENTITY_A".to_string(), &bond, 1.0, unit);
        builder = builder.position(&pos);
    }

    let portfolio = builder.build().expect("Portfolio should build");

    let valuation =
        valuePortfolio(&portfolio, &market, None).expect("valuePortfolio should succeed");

    // First access builds and caches the JS object.
    let obj0 = valuation
        .position_values()
        .expect("positionValues should return an object");
    assert!(
        !obj0.is_undefined(),
        "positionValues should not be undefined"
    );

    let iters = 100;
    let t0 = now_ms();
    for _ in 0..iters {
        let _ = valuation
            .position_values()
            .expect("positionValues should succeed");
    }
    let dt = now_ms() - t0;

    web_sys::console::log_1(
        &format!(
            "bench_portfolio_position_values_access: {n_positions} positions, {iters} reads, {:.3}ms total",
            dt
        )
        .into(),
    );

    assert!(dt >= 0.0, "timing should be non-negative");
}
