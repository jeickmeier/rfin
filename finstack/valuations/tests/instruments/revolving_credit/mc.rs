//! Monte Carlo pricing tests for revolving credit facilities.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::HazardCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::revolving_credit::{
    BaseRateSpec, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees, StochasticUtilizationSpec,
    UtilizationProcess,
};
use time::macros::date;

use crate::common::test_helpers::flat_discount_curve;

fn build_flat_discount_curve(
    rate: f64,
    base_date: Date,
    curve_id: &str,
) -> finstack_core::market_data::term_structures::DiscountCurve {
    flat_discount_curve(rate, base_date, curve_id)
}

#[test]
fn test_mc_pricer_stochastic_utilization() {
    // Setup dates
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    // Create a revolving credit facility with stochastic utilization
    let facility = RevolvingCredit::builder()
        .id("RC-MC-001".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(5_000_000.0, Currency::USD)) // 50% initial utilization
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 }) // 5% interest
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 10.0, 5.0))
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.6, // Mean-revert to 60% utilization
                    speed: 0.5,       // Moderate mean reversion
                    volatility: 0.15, // 15% volatility
                },
                num_paths: 10000, // 10k paths for reasonable convergence
                seed: Some(42),   // Fixed seed for reproducibility
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: None,
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    // Create a flat discount curve at 3%
    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");

    // Build market context
    let market = MarketContext::new().insert_discount(disc_curve);

    // Price using MC
    let pv = facility.value(&market, val_date).unwrap();

    // Expected value should be positive (we're receiving fees and interest)
    assert!(pv.amount() > 0.0, "PV should be positive");

    // Rough sanity check: PV should be in a reasonable range INCLUDING principal repayment
    // With 1 year maturity, 10M commitment, ~50-60% utilization, principal ~5-6M repaid at maturity
    // Discounted principal at 3%: ~ 4.85M - 5.8M, plus interest/fees ~ 250-300k
    // Allow a wide band due to stochastic utilization
    assert!(
        pv.amount() > 4_500_000.0 && pv.amount() < 6_500_000.0,
        "PV should be in reasonable range, got {}",
        pv.amount()
    );
}

#[test]
#[cfg(feature = "mc")]
fn test_mc_pricer_market_anchored_zero_vol_and_vol_sensitivity() {
    use finstack_valuations::instruments::revolving_credit::types::{
        CreditSpreadProcessSpec, McConfig,
    };

    // Setup dates
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2028 - 01 - 01); // 3 years

    // Flat discount at 3%
    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");

    // Simple hazard curve with constant hazard 2% (implies ~1.2% spread at R=40%)
    let hazard = HazardCurve::builder("BORROWER-HZD")
        .base_date(val_date)
        .recovery_rate(0.40)
        .day_count(DayCount::Act365F)
        .knots([(1.0, 0.02), (5.0, 0.02)])
        .build()
        .unwrap();

    // Build market context with discount + hazard
    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_hazard(hazard);

    // Zero vol (should collapse toward deterministic behavior wrt default randomness off)
    let facility_zero_vol = RevolvingCredit::builder()
        .id("RC-MC-ANCHOR".into())
        .commitment_amount(Money::new(5_000_000.0, Currency::USD))
        .drawn_amount(Money::new(2_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.055 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 50.0, 0.0))
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.4,
                    speed: 0.5,
                    volatility: 1e-8, // near-zero, but positive to satisfy model constraints
                },
                num_paths: 5000,
                seed: Some(42),
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: Some(McConfig {
                    correlation_matrix: None,
                    recovery_rate: 0.40,
                    credit_spread_process: CreditSpreadProcessSpec::MarketAnchored {
                        hazard_curve_id: "BORROWER-HZD".into(),
                        kappa: 0.5,
                        implied_vol: 1e-8, // near-zero, but positive to satisfy CIR constraints
                        tenor_years: None,
                    },
                    interest_rate_process: None,
                    util_credit_corr: Some(0.8),
                }),
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let pv_zero = facility_zero_vol.value(&market, val_date).unwrap();
    assert!(pv_zero.amount().is_finite());

    // Higher vol should not increase PV for the lender on average (default risk)
    let facility_high_vol = RevolvingCredit::builder()
        .id("RC-MC-ANCHOR-HV".into())
        .commitment_amount(Money::new(5_000_000.0, Currency::USD))
        .drawn_amount(Money::new(2_000_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.055 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(25.0, 50.0, 0.0))
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.4,
                    speed: 0.5,
                    volatility: 0.20,
                },
                num_paths: 5000,
                seed: Some(42),
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: Some(McConfig {
                    correlation_matrix: None,
                    recovery_rate: 0.40,
                    credit_spread_process: CreditSpreadProcessSpec::MarketAnchored {
                        hazard_curve_id: "BORROWER-HZD".into(),
                        kappa: 0.5,
                        implied_vol: 0.30,
                        tenor_years: None,
                    },
                    interest_rate_process: None,
                    util_credit_corr: Some(0.8),
                }),
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let pv_high_vol = facility_high_vol.value(&market, val_date).unwrap();

    // Allow tolerance due to path randomness and competing utilization effect
    assert!(pv_high_vol.amount() <= pv_zero.amount() * 1.02);
}

#[test]
fn test_mc_pricer_deterministic_reproducibility() {
    // Test that MC pricer is deterministic with fixed seed
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let facility = RevolvingCredit::builder()
        .id("RC-MC-002".into())
        .commitment_amount(Money::new(5_000_000.0, Currency::USD))
        .drawn_amount(Money::new(2_500_000.0, Currency::USD))
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.04 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::flat(20.0, 5.0, 3.0))
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.5,
                    speed: 0.3,
                    volatility: 0.10,
                },
                num_paths: 1000,
                seed: Some(12345),
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: None,
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.02, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Price twice with same seed
    let pv1 = facility.value(&market, val_date).unwrap();
    let pv2 = facility.value(&market, val_date).unwrap();

    // Should be exactly the same due to fixed seed
    assert_eq!(
        pv1.amount(),
        pv2.amount(),
        "MC pricer should be deterministic with fixed seed"
    );
}

#[test]
fn test_mc_pricer_convergence() {
    // Test that more paths lead to better estimates (less variance)
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2026 - 01 - 01);

    let disc_curve = build_flat_discount_curve(0.04, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    // Test with different number of paths
    let num_paths_list = vec![100, 1000, 5000];
    let mut results = Vec::new();

    for &num_paths in &num_paths_list {
        let facility = RevolvingCredit::builder()
            .id(format!("RC-MC-003-{}", num_paths).into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(6_000_000.0, Currency::USD))
            .commitment_date(commitment_date)
            .maturity_date(maturity_date)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.06 })
            .day_count(DayCount::Act360)
            .payment_frequency(Tenor::quarterly())
            .fees(RevolvingCreditFees::flat(30.0, 15.0, 10.0))
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                StochasticUtilizationSpec {
                    utilization_process: UtilizationProcess::MeanReverting {
                        target_rate: 0.7,
                        speed: 0.4,
                        volatility: 0.20,
                    },
                    num_paths,
                    seed: Some(99999),
                    antithetic: false,
                    use_sobol_qmc: false,
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let pv = facility.value(&market, val_date).unwrap();
        results.push(pv.amount());
    }

    // Results should be relatively stable (within reasonable range)
    let mean = results.iter().sum::<f64>() / results.len() as f64;
    for &result in &results {
        let relative_diff = (result - mean).abs() / mean;
        assert!(
            relative_diff < 0.1,
            "Results should converge: {:?}, mean: {}, diff: {}",
            results,
            mean,
            relative_diff
        );
    }
}

#[test]
fn test_mc_utilization_mean_reversion() {
    // Test that the mean-reverting process behaves correctly
    let val_date = date!(2025 - 01 - 01);
    let commitment_date = date!(2025 - 01 - 01);
    let maturity_date = date!(2027 - 01 - 01); // 2 years

    // Start with very low utilization (10%), target 80%
    let facility = RevolvingCredit::builder()
        .id("RC-MC-004".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(1_000_000.0, Currency::USD)) // 10% initial
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.8, // Should drift toward 80%
                    speed: 1.0,       // Fast mean reversion
                    volatility: 0.05, // Low volatility
                },
                num_paths: 5000,
                seed: Some(54321),
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: None,
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let disc_curve = build_flat_discount_curve(0.03, val_date, "USD-OIS");
    let market = MarketContext::new().insert_discount(disc_curve);

    let pv = facility.value(&market, val_date).unwrap();

    // With mean reversion to higher utilization, PV should reflect
    // increasing interest payments over time
    assert!(pv.amount() > 0.0);

    // Compare to a facility with constant high utilization
    let high_util_facility = RevolvingCredit::builder()
        .id("RC-MC-005".into())
        .commitment_amount(Money::new(10_000_000.0, Currency::USD))
        .drawn_amount(Money::new(8_000_000.0, Currency::USD)) // 80% constant
        .commitment_date(commitment_date)
        .maturity_date(maturity_date)
        .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
        .day_count(DayCount::Act360)
        .payment_frequency(Tenor::quarterly())
        .fees(RevolvingCreditFees::default())
        .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
            StochasticUtilizationSpec {
                utilization_process: UtilizationProcess::MeanReverting {
                    target_rate: 0.8, // Already at target
                    speed: 1.0,
                    volatility: 0.05,
                },
                num_paths: 5000,
                seed: Some(54321),
                antithetic: false,
                use_sobol_qmc: false,
                mc_config: None,
            },
        )))
        .discount_curve_id("USD-OIS".into())
        .build()
        .unwrap();

    let pv_high = high_util_facility.value(&market, val_date).unwrap();

    // The facility starting at low utilization should have lower PV
    // than the one starting at high utilization (due to path-dependence)
    assert!(
        pv.amount() < pv_high.amount(),
        "Lower initial utilization should result in lower PV"
    );
}
