//! Capital structure integration tests for spec builders.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_statements::capital_structure::integration::{
    build_bond_from_spec, build_swap_from_spec,
};
use finstack_statements::types::DebtInstrumentSpec;
use finstack_valuations::instruments::common::parameters::legs::{FixedLegSpec, FloatLegSpec};
use finstack_valuations::instruments::rates::irs::{FloatingLegCompounding, InterestRateSwap};
use finstack_valuations::instruments::{fixed_income::bond::Bond, PayReceive};
use rust_decimal::Decimal;
use time::Month;

fn usd_irs_swap(
    id: InstrumentId,
    notional: Money,
    fixed_rate: f64,
    start: Date,
    end: Date,
    side: PayReceive,
) -> finstack_core::Result<InterestRateSwap> {
    let rate_decimal = Decimal::try_from(fixed_rate).map_err(|_| {
        finstack_core::Error::Validation(format!(
            "Invalid fixed rate: {} cannot be converted to Decimal. \
             Check for NaN, infinity, or values exceeding Decimal range.",
            fixed_rate
        ))
    })?;

    let fixed = FixedLegSpec {
        discount_curve_id: CurveId::new("USD-OIS"),
        rate: rate_decimal,
        freq: Tenor::semi_annual(),
        dc: DayCount::Thirty360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: Some("usny".to_string()),
        stub: StubKind::None,
        start,
        end,
        par_method: None,
        compounding_simple: true,
        payment_delay_days: 0,
    };

    let float = FloatLegSpec {
        discount_curve_id: CurveId::new("USD-OIS"),
        forward_curve_id: CurveId::new("USD-SOFR-3M"),
        spread_bp: Decimal::ZERO,
        freq: Tenor::quarterly(),
        dc: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        calendar_id: Some("usny".to_string()),
        stub: StubKind::None,
        reset_lag_days: 0,
        fixing_calendar_id: None,
        start,
        end,
        compounding: FloatingLegCompounding::Simple,
        payment_delay_days: 0,
    };

    let swap = InterestRateSwap::builder()
        .id(id)
        .notional(notional)
        .side(side)
        .fixed(fixed)
        .float(float)
        .build()?;

    swap.validate()?;
    Ok(swap)
}

#[test]
fn test_build_bond_from_spec() {
    let bond = Bond::fixed(
        InstrumentId::new("BOND-001"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        Date::from_calendar_date(2025, Month::January, 15).expect("valid date"),
        Date::from_calendar_date(2030, Month::January, 15).expect("valid date"),
        CurveId::new("USD-OIS"),
    )
    .expect("Bond::fixed should succeed with valid parameters");

    let spec = DebtInstrumentSpec::Bond {
        id: "BOND-001".to_string(),
        spec: serde_json::to_value(&bond).expect("bond should serialize"),
    };

    let deserialized_bond = build_bond_from_spec(&spec).expect("bond should deserialize");
    assert_eq!(deserialized_bond.id.as_str(), "BOND-001");
    assert_eq!(deserialized_bond.notional.currency(), Currency::USD);

    use finstack_valuations::instruments::fixed_income::bond::CashflowSpec;
    if let CashflowSpec::Fixed(spec) = &deserialized_bond.cashflow_spec {
        assert_eq!(spec.rate.to_string(), "0.05");
    } else {
        panic!("Expected fixed cashflow spec");
    }
}

#[test]
fn test_build_swap_from_spec() {
    let swap = usd_irs_swap(
        InstrumentId::new("SWAP-001"),
        Money::new(5_000_000.0, Currency::USD),
        0.04,
        Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
        Date::from_calendar_date(2030, Month::January, 1).expect("valid date"),
        PayReceive::PayFixed,
    )
    .expect("swap should build");

    let spec = DebtInstrumentSpec::Swap {
        id: "SWAP-001".to_string(),
        spec: serde_json::to_value(&swap).expect("swap should serialize"),
    };

    let deserialized_swap = build_swap_from_spec(&spec).expect("swap should deserialize");
    assert_eq!(deserialized_swap.id.as_str(), "SWAP-001");
    assert_eq!(deserialized_swap.notional.currency(), Currency::USD);
}
