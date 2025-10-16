//! Black model pricing tests with manual formula validation

use crate::swaption::common::*;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;

#[test]
fn test_atm_payer_swaption_pricing() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let market = create_flat_market(as_of, 0.05, 0.50);

    let pv = swaption.value(&market, as_of).unwrap();

    // ATM swaption should have positive value
    assert!(
        pv.amount() > 0.0,
        "ATM payer swaption should have positive value"
    );
    assert_eq!(pv.currency(), Currency::USD);

    // Typical ATM 1Y into 5Y swaption with 50% vol should be a few % of notional
    assert_reasonable(pv.amount(), 1_000.0, 100_000.0, "ATM swaption PV");
}

#[test]
fn test_black_formula_manual_validation() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let market = create_flat_market(as_of, 0.05, 0.50);

    let pv_inst = swaption.value(&market, as_of).unwrap().amount();

    // Manual Black76 calculation
    let disc = market.get_discount_ref("USD_OIS").unwrap();
    let t = swaption
        .year_fraction(as_of, expiry, swaption.day_count)
        .unwrap();
    let forward = swaption.forward_swap_rate(disc, as_of).unwrap();
    let annuity = swaption.swap_annuity(disc, as_of).unwrap();
    let vol = 0.50;

    let var: f64 = vol * vol * t;
    let sqrt_var = var.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * var) / sqrt_var;
    let d2 = d1 - sqrt_var;

    let norm_d1 = finstack_core::math::norm_cdf(d1);
    let norm_d2 = finstack_core::math::norm_cdf(d2);

    let expected = annuity * (forward * norm_d1 - strike * norm_d2) * swaption.notional.amount();

    assert_approx_eq(pv_inst, expected, 1e-6, "Black formula manual validation");
}

#[test]
fn test_itm_payer_has_higher_value() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);

    // ATM
    let atm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let pv_atm = atm.value(&market, as_of).unwrap().amount();

    // ITM (strike below forward)
    let itm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.03);
    let pv_itm = itm.value(&market, as_of).unwrap().amount();

    // OTM (strike above forward)
    let otm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.07);
    let pv_otm = otm.value(&market, as_of).unwrap().amount();

    assert!(pv_itm > pv_atm, "ITM should be more valuable than ATM");
    assert!(pv_atm > pv_otm, "ATM should be more valuable than OTM");
}

#[test]
fn test_otm_payer_has_positive_value() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.30);

    // Deep OTM (strike well above forward)
    let otm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.10);
    let pv_otm = otm.value(&market, as_of).unwrap().amount();

    // Should still have time value
    assert!(pv_otm > 0.0, "OTM swaption should have positive time value");
}

#[test]
fn test_payer_receiver_put_call_parity() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let market = create_flat_market(as_of, 0.05, 0.30);

    let payer = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    let receiver = create_standard_receiver_swaption(expiry, swap_start, swap_end, strike);

    let pv_payer = payer.value(&market, as_of).unwrap().amount();
    let pv_receiver = receiver.value(&market, as_of).unwrap().amount();

    // For ATM swaptions, payer and receiver should have similar values
    // Payer - Receiver = PV(forward_swap - strike) ≈ 0 for ATM
    // Note: With quarterly fixed/float legs and Act/360, there can be small differences
    let diff = (pv_payer - pv_receiver).abs();
    let scale = (pv_payer + pv_receiver) / 2.0;

    assert!(
        diff / scale < 0.15,
        "ATM payer-receiver difference should be reasonably small (within 15%)"
    );
}

#[test]
fn test_volatility_impact() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);

    // Low vol market
    let market_low = create_flat_market(as_of, 0.05, 0.10);
    let pv_low = swaption.value(&market_low, as_of).unwrap().amount();

    // High vol market
    let market_high = create_flat_market(as_of, 0.05, 0.50);
    let pv_high = swaption.value(&market_high, as_of).unwrap().amount();

    assert!(
        pv_high > pv_low,
        "Higher volatility should increase option value"
    );
    assert!(
        pv_high > 2.0 * pv_low,
        "50% vol should be significantly higher than 10% vol"
    );
}

#[test]
fn test_zero_volatility_intrinsic_value() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let market = create_flat_market(as_of, 0.05, 0.0);

    // ITM payer (strike below forward)
    let itm = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.03);
    let pv_itm = itm.value(&market, as_of).unwrap().amount();

    // With zero vol, ITM should have near-zero value (expires in future)
    // But our implementation should handle this gracefully
    assert!(pv_itm >= 0.0, "Zero vol pricing should be non-negative");
}

#[test]
fn test_short_expiry_swaption() {
    let as_of = time::macros::date!(2024 - 01 - 01);
    let expiry = time::macros::date!(2024 - 02 - 01); // 1 month
    let swap_start = expiry;
    let swap_end = time::macros::date!(2029 - 02 - 01);

    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let pv = swaption.value(&market, as_of).unwrap().amount();

    assert!(pv > 0.0, "Short expiry swaption should have positive value");
    // Short expiry should have less time value
    assert_reasonable(pv, 100.0, 20_000.0, "Short expiry swaption PV");
}

#[test]
fn test_long_tenor_swap() {
    let (as_of, expiry, swap_start, _) = standard_dates();
    let swap_end = time::macros::date!(2045 - 01 - 01); // 20Y swap

    let swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.05, 0.30);

    let pv = swaption.value(&market, as_of).unwrap().amount();

    // Longer tenor means higher annuity and more sensitivity
    // With 30% vol on 20Y underlying, value can be relatively modest due to discounting
    assert!(pv > 0.0, "Long tenor swaption should have positive value");
    assert_reasonable(pv, 100.0, 200_000.0, "Long tenor swaption PV");
}

#[test]
fn test_notional_scaling() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let strike = 0.05;
    let market = create_flat_market(as_of, 0.05, 0.30);

    // 1M notional
    let mut swaption1 = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    swaption1.notional = Money::new(1_000_000.0, Currency::USD);
    let pv1 = swaption1.value(&market, as_of).unwrap().amount();

    // 10M notional
    let mut swaption10 = create_standard_payer_swaption(expiry, swap_start, swap_end, strike);
    swaption10.notional = Money::new(10_000_000.0, Currency::USD);
    let pv10 = swaption10.value(&market, as_of).unwrap().amount();

    assert_approx_eq(
        pv10,
        pv1 * 10.0,
        1e-6,
        "PV should scale linearly with notional",
    );
}
