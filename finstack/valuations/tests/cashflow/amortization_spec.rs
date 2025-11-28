//! Tests for amortization and notional functionality.
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::{AmortizationSpec, Notional};

#[test]
fn test_notional_par() {
    let notional = Notional::par(1_000_000.0, Currency::USD);

    assert_eq!(notional.initial.amount(), 1_000_000.0);
    assert_eq!(notional.initial.currency(), Currency::USD);
    assert!(matches!(notional.amort, AmortizationSpec::None));
}

#[test]
fn test_notional_currency() {
    let notional = Notional::par(500_000.0, Currency::EUR);
    assert_eq!(notional.currency(), Currency::EUR);
}

#[test]
fn test_amortization_spec_none_validation() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::None,
    };

    let result = notional.validate();
    assert!(result.is_ok());
}

#[test]
fn test_amortization_spec_linear_to_validation_ok() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::LinearTo {
            final_notional: Money::new(500_000.0, Currency::USD),
        },
    };

    let result = notional.validate();
    assert!(result.is_ok());
}

#[test]
fn test_amortization_spec_linear_to_validation_currency_mismatch() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::LinearTo {
            final_notional: Money::new(500_000.0, Currency::EUR), // Different currency
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_linear_to_validation_final_exceeds_initial() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::LinearTo {
            final_notional: Money::new(1_500_000.0, Currency::USD), // Exceeds initial
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_step_remaining_validation_ok() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();

    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining {
            schedule: vec![
                (date1, Money::new(750_000.0, Currency::USD)),
                (date2, Money::new(500_000.0, Currency::USD)),
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_ok());
}

#[test]
fn test_amortization_spec_step_remaining_validation_unsorted_dates_rejected() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();

    // Intentionally unsorted input (later date first)
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining {
            schedule: vec![
                (date2, Money::new(500_000.0, Currency::USD)),
                (date1, Money::new(750_000.0, Currency::USD)),
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_step_remaining_validation_duplicate_dates_rejected() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();

    // Duplicate same date
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining {
            schedule: vec![
                (date1, Money::new(800_000.0, Currency::USD)),
                (date1, Money::new(700_000.0, Currency::USD)),
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_step_remaining_validation_currency_mismatch() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();

    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining {
            schedule: vec![
                (date1, Money::new(750_000.0, Currency::EUR)), // Different currency
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_step_remaining_validation_increasing() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();
    let date2 = Date::from_calendar_date(2025, time::Month::June, 1).unwrap();

    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining {
            schedule: vec![
                (date1, Money::new(500_000.0, Currency::USD)),
                (date2, Money::new(750_000.0, Currency::USD)), // Increasing - invalid
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_percent_per_period_validation() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: 0.05 },
    };

    let result = notional.validate();
    assert!(result.is_ok());
}

#[test]
fn test_amortization_spec_custom_principal_validation_ok() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();

    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::CustomPrincipal {
            items: vec![(date1, Money::new(100_000.0, Currency::USD))],
        },
    };

    let result = notional.validate();
    assert!(result.is_ok());
}

#[test]
fn test_amortization_spec_custom_principal_validation_currency_mismatch() {
    let date1 = Date::from_calendar_date(2025, time::Month::March, 1).unwrap();

    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::CustomPrincipal {
            items: vec![
                (date1, Money::new(100_000.0, Currency::EUR)), // Different currency
            ],
        },
    };

    let result = notional.validate();
    assert!(result.is_err());
}

#[test]
fn test_amortization_spec_default() {
    let default_spec = AmortizationSpec::default();
    assert!(matches!(default_spec, AmortizationSpec::None));
}

// =============================================================================
// Edge Case Tests (Market Standards Review - Safety)
// =============================================================================

#[test]
fn test_amortization_spec_percent_nan_rejected() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: f64::NAN },
    };

    let result = notional.validate();
    assert!(result.is_err(), "NaN percentage should be rejected");
}

#[test]
fn test_amortization_spec_percent_infinity_rejected() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: f64::INFINITY },
    };

    let result = notional.validate();
    assert!(result.is_err(), "Infinite percentage should be rejected");
}

#[test]
fn test_amortization_spec_percent_neg_infinity_rejected() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod {
            pct: f64::NEG_INFINITY,
        },
    };

    let result = notional.validate();
    assert!(
        result.is_err(),
        "Negative infinite percentage should be rejected"
    );
}

#[test]
fn test_amortization_spec_percent_negative_rejected() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: -0.05 },
    };

    let result = notional.validate();
    assert!(result.is_err(), "Negative percentage should be rejected");
}

#[test]
fn test_amortization_spec_percent_over_100_rejected() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: 1.5 },
    };

    let result = notional.validate();
    assert!(result.is_err(), "Percentage over 100% should be rejected");
}

#[test]
fn test_amortization_spec_percent_zero_ok() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: 0.0 },
    };

    let result = notional.validate();
    assert!(result.is_ok(), "0% percentage should be valid (edge case)");
}

#[test]
fn test_amortization_spec_percent_100_ok() {
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::PercentPerPeriod { pct: 1.0 },
    };

    let result = notional.validate();
    assert!(
        result.is_ok(),
        "100% percentage should be valid (edge case)"
    );
}

#[test]
fn test_amortization_spec_step_remaining_empty_ok() {
    // Empty schedule means no amortization events - should be valid
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::StepRemaining { schedule: vec![] },
    };

    let result = notional.validate();
    assert!(
        result.is_ok(),
        "Empty step schedule should be valid (no amortization)"
    );
}

#[test]
fn test_amortization_spec_custom_principal_empty_ok() {
    // Empty custom principal means no exchanges - should be valid
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::CustomPrincipal { items: vec![] },
    };

    let result = notional.validate();
    assert!(
        result.is_ok(),
        "Empty custom principal should be valid (no exchanges)"
    );
}

#[test]
fn test_amortization_spec_linear_to_zero_ok() {
    // Full amortization to zero is a valid scenario (e.g., fully amortizing loan)
    let notional = Notional {
        initial: Money::new(1_000_000.0, Currency::USD),
        amort: AmortizationSpec::LinearTo {
            final_notional: Money::new(0.0, Currency::USD),
        },
    };

    let result = notional.validate();
    assert!(
        result.is_ok(),
        "Linear amortization to zero should be valid"
    );
}
