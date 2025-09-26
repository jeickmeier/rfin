use finstack_core::config::{CurrencyScalePolicy, FinstackConfig, RoundingMode, RoundingPolicy};
use finstack_core::money::Money;
use finstack_core::prelude::Currency;

#[test]
fn money_display_respects_output_scale() {
    let cfg = FinstackConfig {
        rounding: RoundingPolicy {
            mode: RoundingMode::AwayFromZero,
            // Keep ingest high so display rounding is the observable effect
            ingest_scale: CurrencyScalePolicy {
                overrides: Default::default(),
            },
            output_scale: CurrencyScalePolicy {
                overrides: Default::default(),
            },
        },
    };
    let m = Money::new_with_config(1.23456, Currency::USD, &cfg);
    // default USD decimals is 2 by ISO; override output to 3 for the test
    let cfg = FinstackConfig {
        rounding: RoundingPolicy {
            mode: RoundingMode::AwayFromZero,
            ingest_scale: CurrencyScalePolicy { overrides: Default::default() },
            output_scale: CurrencyScalePolicy {
                overrides: std::iter::once((Currency::USD, 3)).collect(),
            },
        },
    };
    let s = m.format_with_config(&cfg);
    assert_eq!(s, "USD 1.235");
}
