use finstack_core::{Currency, Money};
use finstack_core::config::{with_temp_config, FinstackConfig, RoundingMode, RoundingPolicy, CurrencyScalePolicy};

#[test]
fn money_display_respects_output_scale() {
    let cfg = FinstackConfig {
        rounding_mode: RoundingMode::AwayFromZero,
        rounding: RoundingPolicy {
            mode: RoundingMode::AwayFromZero,
            // Keep ingest high so display rounding is the observable effect
            ingest_scale: CurrencyScalePolicy { default_scale: 6, overrides: Default::default() },
            output_scale: CurrencyScalePolicy { default_scale: 3, overrides: Default::default() },
        },
    };
    let s = with_temp_config(cfg, || format!("{}", Money::new(1.23456, Currency::USD)));
    assert_eq!(s, "USD 1.235");
}


