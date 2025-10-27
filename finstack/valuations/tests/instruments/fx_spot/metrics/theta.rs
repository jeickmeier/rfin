//! Theta metric tests for FX Spot.

use super::super::common::*;
use finstack_core::{currency::Currency, dates::Date, money::Money};
use finstack_valuations::{
    instruments::{
        common::{metrics::GenericTheta, traits::Instrument},
        fx_spot::FxSpot,
    },
    metrics::{traits::MetricCalculator, MetricContext},
};
use std::sync::Arc;

fn create_context(fx: FxSpot, as_of: Date) -> MetricContext {
    let market = market_full();
    let base_value = fx.npv(&market, as_of).unwrap();
    let instrument: Arc<dyn Instrument> = Arc::new(fx);
    MetricContext::new(instrument, Arc::new(market), as_of, base_value)
}

#[test]
fn test_theta_basic() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = GenericTheta::<FxSpot>::default();

    let theta = calc.calculate(&mut ctx).unwrap();

    // For FX spot with explicit rate, theta should be very small or zero
    // since value doesn't change with time when rate is fixed
    assert!(theta.abs() < 100.0, "Theta should be small for fixed rate");
}

#[test]
fn test_theta_settled_position() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 10)); // Past
    let mut ctx = create_context(fx, test_date());
    let calc = GenericTheta::<FxSpot>::default();

    let theta = calc.calculate(&mut ctx).unwrap();

    // Settled position has zero theta
    assert_approx_eq(theta, 0.0, EPSILON, "Theta zero for settled position");
}

#[test]
fn test_theta_dependencies() {
    let calc = GenericTheta::<FxSpot>::default();
    let deps = calc.dependencies();

    // Theta should have no additional dependencies
    assert_eq!(deps.len(), 0, "Theta has no dependencies");
}

#[test]
fn test_theta_with_future_settlement() {
    let fx = eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 2, 15)); // 1 month out
    let mut ctx = create_context(fx, test_date());
    let calc = GenericTheta::<FxSpot>::default();

    // Should not panic
    let _theta = calc.calculate(&mut ctx).unwrap();
}

#[test]
fn test_theta_zero_notional() {
    let fx = sample_eurusd()
        .try_with_notional(Money::new(0.0, Currency::EUR))
        .unwrap()
        .with_rate(1.20)
        .with_settlement(d(2025, 1, 17));
    let mut ctx = create_context(fx, test_date());
    let calc = GenericTheta::<FxSpot>::default();

    let theta = calc.calculate(&mut ctx).unwrap();
    assert_approx_eq(theta, 0.0, EPSILON, "Theta zero for zero notional");
}

#[test]
fn test_theta_calculation_completes() {
    // Regression test: ensure theta calculation completes without error
    let calc = GenericTheta::<FxSpot>::default();
    let test_cases = vec![
        eurusd_with_notional(1_000_000.0, 1.20).with_settlement(d(2025, 1, 17)),
        eurusd_with_notional(5_000_000.0, 1.25).with_settlement(d(2025, 2, 15)),
        sample_gbpusd()
            .try_with_notional(Money::new(2_000_000.0, Currency::GBP))
            .unwrap()
            .with_rate(1.40)
            .with_settlement(d(2025, 3, 15)),
    ];

    for fx in test_cases {
        let mut ctx = create_context(fx, test_date());
        let result = calc.calculate(&mut ctx);
        assert!(result.is_ok(), "Theta calculation should complete");
    }
}
