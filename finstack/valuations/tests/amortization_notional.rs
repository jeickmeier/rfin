//! Tests for amortization and notional functionality.

use finstack_valuations::cashflow::amortization_notional::{AmortizationSpec, Notional};
use finstack_core::{currency::Currency, money::Money, dates::Date};

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
            items: vec![
                (date1, Money::new(100_000.0, Currency::USD)),
            ],
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
