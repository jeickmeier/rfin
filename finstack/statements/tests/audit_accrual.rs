use finstack_core::currency::Currency;
use finstack_core::dates::{build_periods, Date};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_statements::capital_structure::aggregate_instrument_cashflows;
use finstack_statements::CapitalStructureSpec;
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::instruments::Bond;
use indexmap::IndexMap;
use std::sync::Arc;
use time::Month;

mod common; // Helper module if needed, or we can just mock minimal stuff

#[test]
fn test_accrued_interest_semi_annual_bond() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Market Context (minimal)
    let market_ctx = MarketContext::new();

    // 2. Define Periods: Monthly for 1 year (2025)
    // We want to see accrual accumulate over months 1-5 and pay in month 6.
    let periods = build_periods("2025M1..M12", None)?.periods;

    // 3. Define Instrument: USD Bond, 5% coupon, Semi-annual
    // Issue: Jan 1, 2025. First Coupon: July 1, 2025.
    // Face: 1,000,000
    // Semi-annual coupon = 1,000,000 * 5% / 2 = 25,000
    let issue_date = Date::from_calendar_date(2025, Month::January, 1)?;
    let maturity_date = Date::from_calendar_date(2030, Month::January, 1)?;

    let bond = Bond::fixed(
        InstrumentId::new("BOND-AUDIT"),
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        issue_date,
        maturity_date,
        CurveId::new("USD-OIS"),
    )?;

    let mut instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> =
        IndexMap::new();
    instruments.insert("BOND-AUDIT".to_string(), Arc::new(bond));

    // 4. Run Aggregation
    let spec = CapitalStructureSpec {
        debt_instruments: vec![],
        equity_instruments: vec![],
        meta: IndexMap::new(),
        reporting_currency: None,
        fx_policy: None,
        waterfall: None,
    };
    let as_of = issue_date; // Not strictly used for contractual flows, but required API

    let cashflows =
        aggregate_instrument_cashflows(&spec, &instruments, &periods, &market_ctx, as_of)?;

    // 5. Verify Results

    // Function to get values for a specific month (1-indexed)
    let get_metrics = |month: u8| {
        let period_id = finstack_core::dates::PeriodId::month(2025, month);
        let accrued = cashflows
            .get_accrued_interest("BOND-AUDIT", &period_id)
            .expect("accrued");
        let cash = cashflows
            .get_interest_cash("BOND-AUDIT", &period_id)
            .expect("cash");
        (accrued, cash)
    };

    // Month 1 (Jan): Should have ~1 month accrued, 0 cash
    let (accrued_m1, cash_m1) = get_metrics(1);
    assert_eq!(cash_m1, 0.0, "Month 1 should have no cash payment");
    assert!(
        accrued_m1 > 4000.0 && accrued_m1 < 4300.0,
        "Month 1 accrued should be roughly 25k/6 (~4166). Got: {}",
        accrued_m1
    );

    // Month 2 (Feb): Should have ~2 months accrued
    let (accrued_m2, cash_m2) = get_metrics(2);
    assert_eq!(cash_m2, 0.0);
    assert!(
        accrued_m2 > accrued_m1,
        "Accrual should increase. M1: {}, M2: {}",
        accrued_m1,
        accrued_m2
    );
    // Rough check: M2 ~ 2 * M1
    assert!(accrued_m2 > 8000.0 && accrued_m2 < 8600.0);

    // Month 5 (May): Accumulated almost full coupon
    let (accrued_m5, _cash_m5) = get_metrics(5);
    // Total coupon is 25,000. 5/6ths is ~20,833
    assert!(
        accrued_m5 > 20000.0 && accrued_m5 < 22000.0,
        "Month 5 accrued: {}",
        accrued_m5
    );

    // Month 6 (June): Coupon payment is July 1st.
    // WAIT: The period 2025M6 ends on July 1st (exclusive? or inclusive?).
    // finstack periods are [start, end). M6 is June 1 to July 1.
    // The bond coupon is on July 1.
    // If the coupon is ON the boundary, it usually falls into the NEXT period (July aka M7) or depends on time.
    // Let's check where the cash hits.

    let (accrued_m6, cash_m6) = get_metrics(6);
    let (accrued_m7, cash_m7) = get_metrics(7);

    // If coupon falls in M7 (July), then M6 accrued should be full 6 months (~25k).
    // Let's inspect both to be robust to boundary conventions.

    println!("M6 Accrued: {}, Cash: {}", accrued_m6, cash_m6);
    println!("M7 Accrued: {}, Cash: {}", accrued_m7, cash_m7);

    if cash_m6 > 0.0 {
        // Coupon paid in June
        assert_eq!(cash_m6, 25000.0);
        // Accrued should reset
        assert!(accrued_m6 < 100.0, "Accrued should reset after payment");
    } else {
        // Coupon paid in July (more likely for July 1 date)
        assert_eq!(cash_m7, 25000.0, "Coupon should be paid in July");

        // M6 accrued should be full amount (~25k)
        assert!(
            accrued_m6 > 24000.0 && accrued_m6 <= 25000.0,
            "M6 accrued should be nearly full coupon. Got: {}",
            accrued_m6
        );

        // M7 accrued should have reset (it's essentially 1 month of new accrual now, or 0 if calculated *at* payment time)
        // If we calculate at July 1st (start of M7, end of M6), it might be 0?
        // Actually M7 period end is Aug 1. So it should be ~1 month accrued.
        assert!(
            accrued_m7 > 3000.0 && accrued_m7 < 4500.0,
            "M7 should restart accrual. Got: {}",
            accrued_m7
        );
    }

    Ok(())
}
