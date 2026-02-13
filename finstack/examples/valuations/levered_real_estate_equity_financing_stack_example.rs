//! Example: LeveredRealEstateEquity with a multi-instrument financing stack.
//!
//! Demonstrates:
//! - Asset PV netting: PV_equity = PV_asset - sum(PV_financing)
//! - Return / coverage metrics on the levered wrapper
//! - Sensitivities: cap-rate and appraisal discount-rate bumps (finite difference)

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::CouponType;
use finstack_valuations::instruments::equity::real_estate::{
    LeveredRealEstateEquity, RealEstateAsset, RealEstateValuationMethod,
};
use finstack_valuations::instruments::fixed_income::term_loan::{
    AmortizationSpec, RateSpec, TermLoan,
};
use finstack_valuations::instruments::{Attributes, Bond, Instrument, InstrumentJson};
use finstack_valuations::metrics::MetricId;
use time::Month;

fn flat_discount_curve(id: &str, as_of: Date, rate: f64) -> DiscountCurve {
    let knots = [
        (0.0, 1.0),
        (1.0, (-rate).exp()),
        (5.0, (-rate * 5.0).exp()),
        (30.0, (-rate * 30.0).exp()),
    ];
    DiscountCurve::builder(id)
        .base_date(as_of)
        .knots(knots)
        .build()
        .expect("flat curve should build")
}

fn main() -> finstack_core::Result<()> {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let noi1 = Date::from_calendar_date(2026, Month::January, 1).unwrap();
    let noi2 = Date::from_calendar_date(2027, Month::January, 1).unwrap();

    let asset = RealEstateAsset::builder()
        .id(InstrumentId::new("RE-ASSET"))
        .currency(Currency::USD)
        .valuation_date(as_of)
        .valuation_method(RealEstateValuationMethod::Dcf)
        .noi_schedule(vec![(noi1, 1_000_000.0), (noi2, 1_050_000.0)])
        .purchase_price_opt(Some(Money::new(15_000_000.0, Currency::USD)))
        .discount_rate_opt(Some(0.09))
        .terminal_cap_rate_opt(Some(0.065))
        // Keep asset curve distinct from financing curves so discount_rate_sensitivity is defined.
        .discount_curve_id(CurveId::new("USD-RE-DISC"))
        .day_count(DayCount::Act365F)
        .attributes(Attributes::new())
        .build()?;

    let loan = TermLoan::builder()
        .id("TL-RE-001".into())
        .currency(Currency::USD)
        .notional_limit(Money::new(9_000_000.0, Currency::USD))
        .issue_date(as_of)
        .maturity(noi2)
        .rate(RateSpec::Fixed { rate_bp: 600 }) // 6%
        .frequency(Tenor::quarterly())
        .day_count(DayCount::Act360)
        .bdc(BusinessDayConvention::ModifiedFollowing)
        .calendar_id_opt(None)
        .stub(StubKind::None)
        .discount_curve_id(CurveId::from("USD-OIS"))
        .amortization(AmortizationSpec::None)
        .coupon_type(CouponType::Cash)
        .upfront_fee_opt(None)
        .ddtl_opt(None)
        .covenants_opt(None)
        .pricing_overrides(Default::default())
        .attributes(Default::default())
        .build()?;

    let bond = Bond::example();

    let levered = LeveredRealEstateEquity::builder()
        .id(InstrumentId::new("RE-EQ-L"))
        .currency(Currency::USD)
        .asset(asset.clone())
        .financing(vec![
            InstrumentJson::TermLoan(loan.clone()),
            InstrumentJson::Bond(bond.clone()),
        ])
        .exit_date_opt(Some(noi2))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .attributes(Attributes::new())
        .build()?;

    let market = MarketContext::new()
        .insert_discount(flat_discount_curve("USD-OIS", as_of, 0.05))
        .insert_discount(flat_discount_curve("USD-TREASURY", as_of, 0.05));

    let pv_asset = asset.value(&market, as_of)?;
    let pv_financing = loan.value(&market, as_of)?.amount() + bond.value(&market, as_of)?.amount();
    let pv_equity = levered.value(&market, as_of)?;

    println!("=== Levered Real Estate Equity ===");
    println!("PV_asset   = {pv_asset}");
    println!("PV_fin     = {}", Money::new(pv_financing, Currency::USD));
    println!("PV_equity  = {pv_equity}");
    println!(
        "check PV_equity == PV_asset - PV_fin: diff={}",
        pv_equity.amount() - (pv_asset.amount() - pv_financing)
    );

    let metrics = [
        MetricId::custom("real_estate::levered_irr"),
        MetricId::custom("real_estate::equity_multiple"),
        MetricId::custom("real_estate::ltv"),
        MetricId::custom("real_estate::dscr_min"),
        MetricId::custom("real_estate::debt_payoff_at_exit"),
        MetricId::custom("real_estate::cap_rate_sensitivity"),
        MetricId::custom("real_estate::discount_rate_sensitivity"),
    ];
    let priced = levered.price_with_metrics(&market, as_of, &metrics)?;
    for m in metrics {
        println!(
            "{} = {}",
            m.as_str(),
            priced.measures.get(&m).copied().unwrap_or(f64::NAN)
        );
    }

    Ok(())
}
