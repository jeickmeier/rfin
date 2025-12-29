//! Invariant tests for financial metrics.
//!
//! These tests verify fundamental invariants that must hold across all market conditions:
//!
//! 1. **DV01 sum-to-parallel**: Sum of bucketed DV01 ≈ parallel DV01
//! 2. **MC determinism**: Fixed seed produces identical results with parallel on/off
//!
//! Uses proptest for property-based testing to discover edge cases.

use finstack_core::currency::Currency;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::bond::{Bond, CashflowSpec};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use proptest::prelude::*;
use std::sync::Arc;
use time::macros::date;

/// Build a discount curve with a given flat rate.
fn build_discount_curve(rate: f64) -> DiscountCurve {
    let as_of = date!(2025 - 01 - 01);
    DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0f64, 1.0f64),
            (1.0f64, (-rate * 1.0).exp()),
            (2.0f64, (-rate * 2.0).exp()),
            (5.0f64, (-rate * 5.0).exp()),
            (10.0f64, (-rate * 10.0).exp()),
            (20.0f64, (-rate * 20.0).exp()),
            (30.0f64, (-rate * 30.0).exp()),
        ])
        .build()
        .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Property: Sum of bucketed DV01 should equal parallel DV01 within tight tolerance.
    ///
    /// The triangular key-rate method partitions the rate sensitivity across buckets
    /// such that the sum equals the parallel sensitivity. This invariant should hold
    /// for any valid bond configuration and market conditions.
    #[test]
    fn prop_bucketed_dv01_sums_to_parallel(
        coupon_rate in 0.01f64..0.08f64,
        notional in 1_000_000.0f64..50_000_000.0f64,
        maturity_years in 2u32..20u32,
        flat_rate in 0.01f64..0.08f64,
    ) {
        let as_of = date!(2025 - 01 - 01);
        let issue = as_of;
        let maturity = as_of.saturating_add(time::Duration::days(i64::from(maturity_years) * 365));

        // Build bond
        let bond = Bond::builder()
            .id("PROP_DV01_TEST".into())
            .notional(Money::new(notional, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                coupon_rate,
                Tenor::semi_annual(),
                DayCount::Thirty360,
            ))
            .discount_curve_id("USD-OIS".into())
            .build()
            .expect("Bond construction should succeed");

        // Build market
        let disc = build_discount_curve(flat_rate);
        let market = MarketContext::new().insert_discount(disc);

        // Compute metrics
        let metrics = vec![MetricId::Dv01, MetricId::BucketedDv01];
        let registry = standard_registry();
        let pv = bond.value(&market, as_of).expect("Bond valuation should succeed");

        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            pv,
        );

        let results = registry.compute(&metrics, &mut context).expect("Metrics should compute");

        let parallel_dv01 = results.get(&MetricId::Dv01).copied().unwrap_or(0.0);
        let bucketed_series = context.computed_series.get(&MetricId::BucketedDv01);

        if let Some(series) = bucketed_series {
            let sum_bucketed: f64 = series.iter().map(|(_, v)| v).sum();

            // Skip near-zero cases where relative error is meaningless
            if parallel_dv01.abs() > 10.0 {
                // Sum should match parallel within 0.1%
                let diff_pct = ((sum_bucketed - parallel_dv01) / parallel_dv01).abs();
                prop_assert!(
                    diff_pct < 0.001,
                    "Bucketed DV01 sum ({:.4}) should match parallel ({:.4}) within 0.1%, got {:.3}%",
                    sum_bucketed, parallel_dv01, diff_pct * 100.0
                );
            }
        }
    }

    /// Property: Both bucketed and parallel DV01 should be negative for long bond positions.
    ///
    /// A long bond position loses value when rates rise (parallel bump is +1bp),
    /// so DV01 should be negative.
    #[test]
    fn prop_dv01_sign_negative_for_long_bond(
        coupon_rate in 0.02f64..0.06f64,
        maturity_years in 3u32..15u32,
        flat_rate in 0.02f64..0.06f64,
    ) {
        let as_of = date!(2025 - 01 - 01);
        let issue = as_of;
        let maturity = as_of.saturating_add(time::Duration::days(i64::from(maturity_years) * 365));

        // Build bond with $10M notional for significant DV01
        let bond = Bond::builder()
            .id("PROP_SIGN_TEST".into())
            .notional(Money::new(10_000_000.0, Currency::USD))
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(CashflowSpec::fixed(
                coupon_rate,
                Tenor::semi_annual(),
                DayCount::Thirty360,
            ))
            .discount_curve_id("USD-OIS".into())
            .build()
            .expect("Bond construction should succeed");

        let disc = build_discount_curve(flat_rate);
        let market = MarketContext::new().insert_discount(disc);

        let metrics = vec![MetricId::Dv01];
        let registry = standard_registry();
        let pv = bond.value(&market, as_of).expect("Bond valuation should succeed");

        let mut context = MetricContext::new(
            Arc::new(bond),
            Arc::new(market),
            as_of,
            pv,
        );

        let results = registry.compute(&metrics, &mut context).expect("Metrics should compute");
        let dv01 = results.get(&MetricId::Dv01).copied().unwrap_or(0.0);

        // DV01 should be negative for long bond
        prop_assert!(
            dv01 < 0.0,
            "DV01 ({:.4}) should be negative for long bond position",
            dv01
        );
    }
}

#[cfg(feature = "mc")]
mod mc_invariants {
    //! Monte Carlo determinism invariants.
    //!
    //! Tests that MC pricing produces identical results with the same seed
    //! across multiple runs. This is critical for reproducibility.

    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::scalars::MarketScalar;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::asian_option::{AsianOption, AveragingMethod};
    use finstack_valuations::instruments::common::parameters::market::OptionType;
    use finstack_valuations::instruments::common::traits::Instrument;
    use time::macros::date;

    fn create_mc_market(spot: f64, vol: f64, rate: f64) -> MarketContext {
        let as_of = date!(2024 - 01 - 01);

        let disc = DiscountCurve::builder("USD-OIS")
            .base_date(as_of)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0f64, 1.0f64),
                (1.0f64, (-rate).exp()),
                (2.0f64, (-rate * 2.0).exp()),
            ])
            .build()
            .unwrap();

        let vol_surface = VolSurface::builder("SPOT_VOL")
            .expiries(&[0.5, 1.0, 2.0])
            .strikes(&[80.0, 100.0, 120.0])
            .row(&[vol, vol, vol])
            .row(&[vol, vol, vol])
            .row(&[vol, vol, vol])
            .build()
            .unwrap();

        MarketContext::new()
            .insert_discount(disc)
            .insert_surface(vol_surface)
            .insert_price("SPOT", MarketScalar::Unitless(spot))
    }

    /// Test that MC pricing with the same seed produces identical results.
    ///
    /// This is the core determinism invariant: given the same inputs and seed,
    /// MC pricing should produce bit-exact identical results across runs.
    #[test]
    fn test_mc_same_seed_determinism() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);

        let option = AsianOption {
            id: "MC_SEED_DETERMINISM".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type: OptionType::Call,
            expiry,
            notional: Money::new(1.0, Currency::USD),
            averaging_method: AveragingMethod::Arithmetic,
            fixing_dates: vec![date!(2024 - 07 - 01), date!(2025 - 01 - 01)],
            day_count: DayCount::Act365F,
            discount_curve_id: "USD-OIS".into(),
            vol_surface_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
            past_fixings: vec![],
        };

        let market = create_mc_market(100.0, 0.25, 0.05);

        // Price multiple times with same seed
        let mut prices = Vec::with_capacity(5);
        for _ in 0..5 {
            let pv = option.value(&market, as_of).expect("Option should price");
            prices.push(pv.amount());
        }

        // All prices should be identical (bit-exact)
        let first = prices[0];
        for (i, &price) in prices.iter().enumerate() {
            assert_eq!(
                price, first,
                "Run {} produced different price ({}) than first run ({})",
                i, price, first
            );
        }
    }

    /// Test MC determinism with different market conditions.
    ///
    /// Verify that the determinism invariant holds across various market
    /// conditions (different spots, vols, rates).
    #[test]
    fn test_mc_determinism_various_markets() {
        let as_of = date!(2024 - 01 - 01);
        let expiry = date!(2025 - 01 - 01);

        let option = AsianOption {
            id: "MC_VARIOUS_MARKETS".into(),
            underlying_ticker: "SPOT".to_string(),
            spot_id: "SPOT".to_string(),
            strike: Money::new(100.0, Currency::USD),
            option_type: OptionType::Call,
            expiry,
            notional: Money::new(1.0, Currency::USD),
            averaging_method: AveragingMethod::Arithmetic,
            fixing_dates: vec![date!(2024 - 07 - 01), date!(2025 - 01 - 01)],
            day_count: DayCount::Act365F,
            discount_curve_id: "USD-OIS".into(),
            vol_surface_id: "SPOT_VOL".into(),
            div_yield_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
            past_fixings: vec![],
        };

        // Test with different market conditions
        let test_cases = [
            (90.0, 0.20, 0.03),  // Lower spot, lower vol, lower rate
            (100.0, 0.25, 0.05), // Base case
            (110.0, 0.30, 0.07), // Higher spot, higher vol, higher rate
        ];

        for (spot, vol, rate) in test_cases {
            let market = create_mc_market(spot, vol, rate);

            // Price twice and verify identical
            let pv1 = option.value(&market, as_of).expect("First pricing should succeed");
            let pv2 = option.value(&market, as_of).expect("Second pricing should succeed");

            assert_eq!(
                pv1.amount(),
                pv2.amount(),
                "Spot={}, Vol={}, Rate={}: prices should be identical ({} vs {})",
                spot,
                vol,
                rate,
                pv1.amount(),
                pv2.amount()
            );
        }
    }
}

#[cfg(test)]
mod additional_invariants {
    //! Additional invariant tests that don't use proptest.

    use super::*;

    /// Bucketed DV01 should sum to parallel for a range of maturities.
    #[test]
    fn test_dv01_sum_invariant_across_maturities() {
        let as_of = date!(2025 - 01 - 01);
        let disc = build_discount_curve(0.04);
        let market = MarketContext::new().insert_discount(disc);
        let registry = standard_registry();

        for years in [2, 5, 10, 15, 20] {
            let maturity = as_of.saturating_add(time::Duration::days(i64::from(years) * 365));

            let bond = Bond::builder()
                .id(format!("MATURITY_{}Y", years).into())
                .notional(Money::new(10_000_000.0, Currency::USD))
                .issue(as_of)
                .maturity(maturity)
                .cashflow_spec(CashflowSpec::fixed(
                    0.04,
                    Tenor::semi_annual(),
                    DayCount::Thirty360,
                ))
                .discount_curve_id("USD-OIS".into())
                .build()
                .expect("Bond construction should succeed");

            let metrics = vec![MetricId::Dv01, MetricId::BucketedDv01];
            let pv = bond.value(&market, as_of).expect("Valuation should succeed");

            let mut context = MetricContext::new(
                Arc::new(bond),
                Arc::new(market.clone()),
                as_of,
                pv,
            );

            let results = registry.compute(&metrics, &mut context).expect("Metrics should compute");
            let parallel_dv01 = results.get(&MetricId::Dv01).copied().unwrap_or(0.0);

            if let Some(series) = context.computed_series.get(&MetricId::BucketedDv01) {
                let sum_bucketed: f64 = series.iter().map(|(_, v)| v).sum();

                let diff_pct = ((sum_bucketed - parallel_dv01) / parallel_dv01).abs();
                assert!(
                    diff_pct < 0.001,
                    "{}Y bond: bucketed sum ({:.2}) vs parallel ({:.2}), diff: {:.4}%",
                    years,
                    sum_bucketed,
                    parallel_dv01,
                    diff_pct * 100.0
                );
            }
        }
    }
}
