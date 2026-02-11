//! Z-spread and I-spread calculator tests.

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::{Error, InputError};
use finstack_valuations::instruments::fixed_income::bond::Bond;
use finstack_valuations::instruments::fixed_income::bond::ZSpreadCalculator;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::metrics::{MetricCalculator, MetricContext, MetricId};
use std::sync::Arc;
use time::macros::date;

#[test]
fn test_z_spread_discount_bond() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR1",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

    let curve = finstack_core::market_data::term_structures::DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (5.0, 0.80)])
        .build()
        .unwrap();
    let market = finstack_core::market_data::context::MarketContext::new().insert_discount(curve);

    let result = bond
        .price_with_metrics(&market, as_of, &[MetricId::ZSpread])
        .unwrap();
    let z = *result.measures.get("z_spread").unwrap();
    assert!(z > 0.0); // Discount bond has positive spread
}

/// Z-spread should surface a missing discount curve error instead of silently returning 0.0
/// when pricing fails inside the root-finding objective (e.g., missing discount curve).
#[test]
fn test_z_spread_missing_discount_curve_returns_error() {
    let as_of = date!(2025 - 01 - 01);
    let mut bond = Bond::fixed(
        "ZSPR-MISSING-DC",
        Money::new(100.0, Currency::USD),
        0.05,
        as_of,
        date!(2030 - 01 - 01),
        "USD-OIS",
    )
    .unwrap();
    bond.pricing_overrides = PricingOverrides::default().with_clean_price(95.0);

    // Market context with NO discount curves – any attempt to build a Z-spread PV should fail
    let market = finstack_core::market_data::context::MarketContext::new();

    // Minimal metric context: base value is arbitrary since Z-spread uses quoted clean price
    let base_value = Money::new(100.0, Currency::USD);
    let mut mctx = MetricContext::new(
        Arc::new(bond),
        Arc::new(market),
        as_of,
        base_value,
        MetricContext::default_config(),
    );

    // Pre-populate accrued to bypass the metric dependency and force the failure into
    // the Z-spread pricing helper (missing discount curve), not missing accrued.
    mctx.computed.insert(MetricId::Accrued, 0.0);

    let calc = ZSpreadCalculator::default();
    let result = calc.calculate(&mut mctx);

    // Expect a propagated input error (missing discount curve), never an apparent "perfect fit" z=0.0.
    match result {
        Err(Error::Input(InputError::MissingCurve { requested, .. })) => {
            assert!(
                requested.contains("USD-OIS"),
                "expected missing discount curve id to mention USD-OIS, got {}",
                requested
            );
        }
        Err(e) => panic!(
            "expected InputError::MissingCurve for missing discount curve, got {}",
            e
        ),
        Ok(z) => panic!(
            "expected Z-spread calculation to fail for missing discount curve, but got z={}",
            z
        ),
    }
}

/// Z-spread solver should converge for IG, HY, and distressed fixed-rate bonds
/// with realistic spreads up to ~3000 bp and maintain tight price residuals.
#[test]
fn test_z_spread_solver_convergence_across_spread_regimes() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_valuations::instruments::PricingOverrides;

    let as_of = date!(2025 - 01 - 01);
    let maturity_ig = date!(2028 - 01 - 01); // shorter IG
    let maturity_hy = date!(2032 - 01 - 01); // medium HY
    let maturity_distressed = date!(2035 - 01 - 01); // longer distressed
    let notional = Money::new(1_000_000.0, Currency::USD);

    // Simple discount curve; Z-spread will be applied as an exponential shift.
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .knots([(0.0, 1.0), (10.0, 0.7)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let market = MarketContext::new().insert_discount(disc);

    let bond_ig = Bond::fixed(
        "ZSPR-CONV-IG",
        notional,
        0.03,
        as_of,
        maturity_ig,
        "USD-OIS",
    )
    .unwrap();
    let bond_hy = Bond::fixed(
        "ZSPR-CONV-HY",
        notional,
        0.06,
        as_of,
        maturity_hy,
        "USD-OIS",
    )
    .unwrap();
    let bond_distressed = Bond::fixed(
        "ZSPR-CONV-DIST",
        notional,
        0.10,
        as_of,
        maturity_distressed,
        "USD-OIS",
    )
    .unwrap();

    // (target z-spread, bond) scenarios from IG through distressed.
    let scenarios: Vec<(f64, Bond)> = vec![
        (0.01, bond_ig),         // 100 bp IG
        (0.07, bond_hy),         // 700 bp HY
        (0.30, bond_distressed), // 3000 bp distressed
    ];

    for (target_z, base_bond) in scenarios {
        let settlement_days = base_bond.settlement_days.unwrap_or(0) as i64;
        let quote_date = as_of + time::Duration::days(settlement_days);

        // Price the bond at the target Z-spread to obtain a dirty price.
        let dirty_target =
            finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::price_from_z_spread(
                &base_bond, &market, quote_date, target_z,
            )
            .expect("pricing with target Z-spread should succeed");

        // Convert to a clean price (% of par) at the quote/settlement date
        // (dirty = clean + accrued at quote_date).
        // Accrued must be computed at the quote/settlement date, not `as_of`.
        let schedule = base_bond
            .get_full_schedule(&market)
            .expect("build full schedule");
        let accrued = finstack_valuations::cashflow::accrued_interest_amount(
            &schedule,
            quote_date,
            &base_bond.accrual_config(),
        )
        .expect("accrued at quote date");
        let clean_ccy = dirty_target - accrued;
        let clean_px = clean_ccy / notional.amount() * 100.0;

        let mut bond = base_bond.clone();
        bond.pricing_overrides = PricingOverrides::default().with_clean_price(clean_px);

        // Run Z-spread metric via the normal pipeline.
        let result = bond
            .price_with_metrics(&market, as_of, &[MetricId::ZSpread])
            .expect("Z-spread metric should converge for realistic spreads");
        let z = *result
            .measures
            .get("z_spread")
            .expect("z_spread measure should be present");

        assert!(
            (z - target_z).abs() < 5e-8,
            "Z-spread solver should recover target z (target={}, got={})",
            target_z,
            z
        );

        // Re-price with solved z and verify price residual is tiny.
        let dirty_repriced =
            finstack_valuations::instruments::fixed_income::bond::pricing::quote_engine::price_from_z_spread(
                &bond, &market, quote_date, z,
            )
            .expect("repricing with solved Z-spread should succeed");
        let price_error = (dirty_repriced - dirty_target).abs() / notional.amount();

        assert!(
            price_error < 1e-6,
            "Price residual should be < 1e-6 * notional, got {}",
            price_error
        );
    }
}
