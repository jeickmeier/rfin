use finstack_core::config::{CurrencyScalePolicy, FinstackConfig, RoundingMode, RoundingPolicy};
use finstack_core::{Currency, Money};

#[test]
fn money_display_respects_output_scale() {
    let cfg = FinstackConfig {
        rounding: RoundingPolicy {
            mode: RoundingMode::AwayFromZero,
            // Keep ingest high so display rounding is the observable effect
            ingest_scale: CurrencyScalePolicy {
                default_scale: 6,
                overrides: Default::default(),
            },
            output_scale: CurrencyScalePolicy {
                default_scale: 3,
                overrides: Default::default(),
            },
        },
        default_currency_decimals: 2,
    };
    let m = Money::new_with_config(1.23456, Currency::USD, &cfg);
    let s = m.format_with_config(&cfg);
    assert_eq!(s, "USD 1.235");
}
