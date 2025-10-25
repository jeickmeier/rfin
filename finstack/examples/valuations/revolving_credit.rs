//! Example demonstrating construction and pricing of a revolving credit facility.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::{
    term_structures::{discount_curve::DiscountCurve, forward_curve::ForwardCurve},
    MarketContext,
};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::revolving_credit::{
    InterestRateSpec, RcfFeeSpec, RevolvingCreditFacility, TransactionType,
    ResetConvention,
};
use finstack_valuations::instruments::common::traits::Instrument;
use time::Month;

fn main() -> finstack_core::Result<()> {
    let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let end = Date::from_calendar_date(2028, Month::January, 1).unwrap();
    let facility = RevolvingCreditFacility::builder()
        .id(InstrumentId::new("RCF-DEMO"))
        .credit_limit(Money::new(25_000_000.0, Currency::USD))
        .initial_drawn(Money::new(5_000_000.0, Currency::USD))
        .start_date(start)
        .maturity_date(end)
        .interest(InterestRateSpec::Floating {
            fwd_id: CurveId::new("USD-SOFR"),
            spread_bp: 200.0,
            reset_lag_days: 2,
            reset_frequency: None,
            reset_calendar_id: None,
            reset_convention: ResetConvention::InAdvance,
        })
        .fees(RcfFeeSpec {
            commitment_fee_bp: 35.0,
            utilization_fee_bp: 25.0,
            upfront_fee: Some(Money::new(125_000.0, Currency::USD)),
        })
        .disc_id(CurveId::new("USD-OIS"))
        .day_count(finstack_core::dates::DayCount::Act360)
        .transactions(vec![
            finstack_valuations::instruments::revolving_credit::RcfTransaction {
                date: Date::from_calendar_date(2025, Month::April, 1).unwrap(),
                amount: Money::new(2_000_000.0, Currency::USD),
                transaction_type: TransactionType::Drawdown,
            },
            finstack_valuations::instruments::revolving_credit::RcfTransaction {
                date: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
                amount: Money::new(1_000_000.0, Currency::USD),
                transaction_type: TransactionType::Repayment,
            },
        ])
        .build()
        .expect("RCF builder");

    let discount = DiscountCurve::builder("USD-OIS")
        .base_date(start)
        .knots(vec![(0.0, 1.0), (3.0, 0.95), (5.0, 0.90)])
        .build()
        .expect("discount curve");

    let forward = ForwardCurve::builder("USD-SOFR", 0.25)
        .base_date(start)
        .knots(vec![(0.0, 0.05), (3.0, 0.055), (5.0, 0.0575)])
        .build()
        .expect("forward curve");

    let market = MarketContext::new()
        .insert_discount(discount)
        .insert_forward(forward);
    let pv = facility.value(&market, start)?;
    println!("Revolving credit PV: {}", pv);

    let schedule = facility.build_full_schedule(&market, start)?;
    println!("Cashflow schedule (date, kind, amount):");
    for cf in &schedule.flows {
        println!("  {} | {:?} | {}", cf.date, cf.kind, cf.amount);
    }


    Ok(())
}

