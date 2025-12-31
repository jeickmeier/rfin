use super::fixtures::*;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::rates::xccy_swap::{NotionalExchange, XccySwap};

#[test]
fn requires_fx_matrix_when_reporting_currency_differs() {
    let base = d(2025, 1, 2);
    let maturity = d(2026, 1, 2);
    let swap = XccySwap::new(
        "XCCY-TEST",
        base,
        maturity,
        leg_usd_receive(),
        leg_eur_pay(),
        Currency::USD,
    )
    .with_notional_exchange(NotionalExchange::None);

    let market = market_without_fx();
    let err = swap.npv(&market, base).unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("fx_matrix"));
}

#[test]
fn prices_with_fx_and_curves() {
    let base = d(2025, 1, 2);
    let maturity = d(2026, 1, 2);
    let swap = XccySwap::new(
        "XCCY-TEST",
        base,
        maturity,
        leg_usd_receive(),
        leg_eur_pay(),
        Currency::USD,
    );

    let market = market_with_fx();
    let pv = swap.npv(&market, base).unwrap();
    assert_eq!(pv.currency(), Currency::USD);
    assert!(pv.amount().is_finite());
}

#[test]
fn notional_exchange_changes_pv() {
    let base = d(2025, 1, 2);
    let maturity = d(2026, 1, 2);
    let swap_none = XccySwap::new(
        "XCCY-TEST-NONE",
        base,
        maturity,
        leg_usd_receive(),
        leg_eur_pay(),
        Currency::USD,
    )
    .with_notional_exchange(NotionalExchange::None);

    let swap_ex = XccySwap::new(
        "XCCY-TEST-EX",
        base,
        maturity,
        leg_usd_receive(),
        leg_eur_pay(),
        Currency::USD,
    )
    .with_notional_exchange(NotionalExchange::InitialAndFinal);

    let market = market_with_fx();
    let pv_none = swap_none.npv(&market, base).unwrap().amount();
    let pv_ex = swap_ex.npv(&market, base).unwrap().amount();
    assert!((pv_none - pv_ex).abs() > 1e-10);
}
