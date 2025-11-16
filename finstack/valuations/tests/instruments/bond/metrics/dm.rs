//! Discount margin calculator tests.

use finstack_core::currency::Currency;
use finstack_core::error::{Error, InputError};
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::bond::metrics::DiscountMarginCalculator;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_dm_fixed_bond_zero() {
    let as_of = date!(2025 - 01 - 01);
    let bond = Bond::fixed(
        "DM1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    );

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::DiscountMargin])
        .unwrap();
    let dm = *result.measures.get("discount_margin").unwrap();
    assert_eq!(dm, 0.0); // Fixed bonds return 0 DM
}

/// DM should surface a missing discount curve error instead of silently returning 0.0
/// when pricing fails inside the root-finding objective (e.g., missing discount curve).
#[test]
fn test_dm_missing_forward_curve_returns_error() {
    let as_of = date!(2025 - 01 - 01);

    // Floating-rate bond referencing a discount curve that will be missing in the market
    let bond = Bond::floating(
        "DM-FRN-MISSING-FWD",
        Money::new(100.0, Currency::USD),
        "USD-SOFR-3M",
        200.0,
        as_of,
        date!(2030 - 01 - 01),
        finstack_core::dates::Frequency::quarterly(),
        finstack_core::dates::DayCount::Act360,
        "USD-OIS",
    );

    // Market with NO discount curves – any attempt to price from DM should fail
    let market = finstack_core::market_data::context::MarketContext::new();

    // Build a minimal metric context without relying on successful base pricing;
    // base_value is arbitrary here since we're testing failure in the DM objective.
    let base_value = Money::new(100.0, Currency::USD);
    let mut mctx =
        MetricContext::new(Arc::new(bond), Arc::new(market), as_of, base_value);

    // No need to pre-compute Accrued; DM calculator will treat missing accrued as 0.
    let calc = DiscountMarginCalculator::default();
    let result = calc.calculate(&mut mctx);

    // Expect a propagated input error (missing discount curve), never an apparent "perfect fit" DM of 0.0.
    match result {
        Err(Error::Input(InputError::NotFound { id })) => {
            assert!(
                id.contains("USD-OIS"),
                "expected missing discount curve id to mention USD-OIS, got {}",
                id
            );
        }
        Err(e) => panic!("expected InputError::NotFound for missing discount curve, got {}", e),
        Ok(dm) => panic!(
            "expected DM calculation to fail for missing discount curve, but got DM={}",
            dm
        ),
    }
}

/// DM solver should converge robustly for IG, HY, and distressed FRNs with
/// realistic spread levels and maintain tight price residuals.
#[test]
fn test_dm_solver_convergence_across_spread_regimes() {
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
    use finstack_core::market_data::MarketContext;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::PricingOverrides;

    let as_of = date!(2025 - 01 - 01);
    let maturity_ig = date!(2027 - 01 - 01); // short IG
    let maturity_hy = date!(2030 - 01 - 01); // medium HY
    let maturity_distressed = date!(2035 - 01 - 01); // longer distressed
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Simple, monotonic curves suitable for FRN pricing.
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.6)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.03), (10.0, 0.03)])
        .build()
        .unwrap();
    let market = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd);

    // Base FRNs for different maturities.
    let frn_ig = Bond::floating(
        "DM-CONV-IG",
        notional,
        "USD-SOFR-3M",
        150.0,
        as_of,
        maturity_ig,
        Frequency::quarterly(),
        DayCount::Act360,
        "USD-OIS",
    );
    let frn_hy = Bond::floating(
        "DM-CONV-HY",
        notional,
        "USD-SOFR-3M",
        300.0,
        as_of,
        maturity_hy,
        Frequency::quarterly(),
        DayCount::Act360,
        "USD-OIS",
    );
    let frn_distressed = Bond::floating(
        "DM-CONV-DIST",
        notional,
        "USD-SOFR-3M",
        500.0,
        as_of,
        maturity_distressed,
        Frequency::quarterly(),
        DayCount::Act360,
        "USD-OIS",
    );

    // (target DM, bond) pairs covering IG, HY, and distressed regimes.
    let scenarios: Vec<(f64, Bond)> = vec![
        (0.01, frn_ig),          // 100 bp IG
        (0.07, frn_hy),          // 700 bp HY
        (0.20, frn_distressed),  // 2000 bp distressed
    ];

    for (target_dm, base_bond) in scenarios {
        // Price the FRN at the target DM to obtain a dirty price in currency.
        let dirty_target =
            finstack_valuations::instruments::bond::pricing::helpers::price_from_dm(
                &base_bond,
                &market,
                as_of,
                target_dm,
            )
            .expect("pricing with target DM should succeed");

        // Convert to a clean price quote (% of par) assuming valuation on a
        // coupon date (zero accrual).
        let clean_px = dirty_target / notional.amount() * 100.0;

        let mut bond = base_bond.clone();
        bond.pricing_overrides = PricingOverrides::default().with_clean_price(clean_px);

        // Run DM metric via the normal metrics pipeline.
        let result = bond
            .price_with_metrics(&market, as_of, &[MetricId::DiscountMargin])
            .expect("DM metric should converge for realistic spreads");
        let dm = *result
            .measures
            .get("discount_margin")
            .expect("discount_margin measure should be present");

        // DM should be very close to the target value.
        assert!(
            (dm - target_dm).abs() < 5e-8,
            "DM solver should recover target DM (target={}, got={})",
            target_dm,
            dm
        );

        // Re-price using the solved DM and verify price residual is tiny.
        let dirty_repriced =
            finstack_valuations::instruments::bond::pricing::helpers::price_from_dm(
                &bond,
                &market,
                as_of,
                dm,
            )
            .expect("repricing with solved DM should succeed");
        let price_error = (dirty_repriced - dirty_target).abs() / notional.amount();

        assert!(
            price_error < 1e-6,
            "Price residual should be < 1e-6 * notional, got {}",
            price_error
        );
    }
}
