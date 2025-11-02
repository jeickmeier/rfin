//! Integration tests validating analytical pricer registration and availability.
//!
//! These tests ensure that all analytical/semi-analytical pricers are properly
//! registered and accessible via the pricer registry.

use finstack_valuations::pricer::{create_standard_registry, InstrumentType, ModelKey};

#[test]
fn test_registry_has_all_analytical_pricers() {
    let registry = create_standard_registry();

    // Test that all analytical pricers are registered
    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::AsianOption,
                ModelKey::AsianGeometricBS
            ))
            .is_some(),
        "AsianGeometricBS pricer should be registered"
    );

    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::AsianOption,
                ModelKey::AsianTurnbullWakeman
            ))
            .is_some(),
        "AsianTurnbullWakeman pricer should be registered"
    );

    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::BarrierOption,
                ModelKey::BarrierBSContinuous
            ))
            .is_some(),
        "BarrierBSContinuous pricer should be registered"
    );

    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::LookbackOption,
                ModelKey::LookbackBSContinuous
            ))
            .is_some(),
        "LookbackBSContinuous pricer should be registered"
    );

    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::QuantoOption,
                ModelKey::QuantoBS
            ))
            .is_some(),
        "QuantoBS pricer should be registered"
    );

    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::FxBarrierOption,
                ModelKey::FxBarrierBSContinuous
            ))
            .is_some(),
        "FxBarrierBSContinuous pricer should be registered"
    );

    #[cfg(feature = "mc")]
    assert!(
        registry
            .get_pricer(finstack_valuations::pricer::PricerKey::new(
                InstrumentType::EquityOption,
                ModelKey::HestonFourier
            ))
            .is_some(),
        "HestonFourier pricer should be registered"
    );
}

#[test]
fn test_model_key_parsing() {
    use std::str::FromStr;

    // Test that all new ModelKey variants can be parsed from strings
    assert_eq!(
        ModelKey::from_str("asian_geometric_bs").unwrap(),
        ModelKey::AsianGeometricBS
    );

    assert_eq!(
        ModelKey::from_str("asian_turnbull_wakeman").unwrap(),
        ModelKey::AsianTurnbullWakeman
    );

    assert_eq!(
        ModelKey::from_str("barrier_bs_continuous").unwrap(),
        ModelKey::BarrierBSContinuous
    );

    assert_eq!(
        ModelKey::from_str("lookback_bs_continuous").unwrap(),
        ModelKey::LookbackBSContinuous
    );

    assert_eq!(ModelKey::from_str("quanto_bs").unwrap(), ModelKey::QuantoBS);

    assert_eq!(
        ModelKey::from_str("fx_barrier_bs_continuous").unwrap(),
        ModelKey::FxBarrierBSContinuous
    );

    assert_eq!(
        ModelKey::from_str("heston_fourier").unwrap(),
        ModelKey::HestonFourier
    );
}
