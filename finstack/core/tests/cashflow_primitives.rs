use finstack_core::cashflow::primitives::{AmortizationSpec, CFKind, CashFlow, Notional};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use time::Month;

fn date(day: u8) -> Date {
    Date::from_calendar_date(2025, Month::January, day).unwrap()
}

#[test]
fn floating_cf_defaults_reset_date() {
    let payment = date(10);
    let amount = Money::new(50.0, Currency::USD);
    let cf = CashFlow::floating_cf(payment, amount, None).unwrap();
    assert_eq!(cf.kind, CFKind::FloatReset);
    assert_eq!(cf.reset_date, Some(payment));

    let reset = date(5);
    let cf = CashFlow::floating_cf(payment, amount, Some(reset)).unwrap();
    assert_eq!(cf.reset_date, Some(reset));
}

#[test]
fn notional_validate_linear_and_percent() {
    let mut notional = Notional::par(1_000_000.0, Currency::USD);
    notional.amort = AmortizationSpec::LinearTo {
        final_notional: Money::new(500_000.0, Currency::USD),
    };
    assert!(notional.validate().is_ok());

    notional.amort = AmortizationSpec::LinearTo {
        final_notional: Money::new(1_500_000.0, Currency::USD),
    };
    assert!(notional.validate().is_err());

    notional.amort = AmortizationSpec::PercentPerPeriod { pct: 0.05 };
    assert!(notional.validate().is_ok());
}

#[test]
fn notional_validate_step_and_custom_rules() {
    let mut notional = Notional::par(1_000_000.0, Currency::USD);
    notional.amort = AmortizationSpec::StepRemaining {
        schedule: vec![
            (date(15), Money::new(800_000.0, Currency::USD)),
            (date(30), Money::new(600_000.0, Currency::USD)),
        ],
    };
    assert!(notional.validate().is_ok());

    notional.amort = AmortizationSpec::StepRemaining {
        schedule: vec![
            (date(30), Money::new(700_000.0, Currency::USD)),
            (date(15), Money::new(800_000.0, Currency::USD)),
        ],
    };
    assert!(notional.validate().is_err());

    notional.amort = AmortizationSpec::CustomPrincipal {
        items: vec![(date(10), Money::new(10_000.0, Currency::EUR))],
    };
    assert!(notional.validate().is_err());
}
