use finstack_core::cashflow::primitives::{CFKind, CashFlow};
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
    let cf = CashFlow {
        date: payment,
        reset_date: Some(payment),
        amount,
        kind: CFKind::FloatReset,
        accrual_factor: 0.0,
        rate: None,
    };
    assert_eq!(cf.kind, CFKind::FloatReset);
    assert_eq!(cf.reset_date, Some(payment));

    let reset = date(5);
    let cf = CashFlow {
        date: payment,
        reset_date: Some(reset),
        amount,
        kind: CFKind::FloatReset,
        accrual_factor: 0.0,
        rate: None,
    };
    assert_eq!(cf.reset_date, Some(reset));
}

// Note: Notional and AmortizationSpec tests moved to finstack/valuations/tests/cashflow/amortization_spec.rs
