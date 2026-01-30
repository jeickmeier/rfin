//! Tests for swaption core types and schedule utilities.

#![allow(clippy::unwrap_used)]

use crate::swaption::common::*;
use finstack_core::dates::{Tenor, TenorUnit};
use finstack_valuations::instruments::common::models::SABRParameters;
use finstack_valuations::instruments::pricing_overrides::VolSurfaceExtrapolation;
use finstack_valuations::instruments::rates::swaption::{
    BermudanSchedule, BermudanSwaption, SwaptionExercise, SwaptionSettlement, VolatilityModel,
};
use time::macros::date;

#[test]
fn test_swaption_settlement_and_exercise_from_str() {
    let physical: SwaptionSettlement = "physical".parse().unwrap();
    let cash: SwaptionSettlement = "CASH".parse().unwrap();
    assert_eq!(physical.to_string(), "physical");
    assert_eq!(cash.to_string(), "cash");

    let european: SwaptionExercise = "european".parse().unwrap();
    let bermudan: SwaptionExercise = "BERMUDAN".parse().unwrap();
    let american: SwaptionExercise = "american".parse().unwrap();
    assert_eq!(european.to_string(), "european");
    assert_eq!(bermudan.to_string(), "bermudan");
    assert_eq!(american.to_string(), "american");

    assert!("unknown".parse::<SwaptionSettlement>().is_err());
    assert!("invalid".parse::<SwaptionExercise>().is_err());
}

#[test]
fn test_bermudan_schedule_sort_and_lockout() {
    let d1 = date!(2026 - 06 - 01);
    let d2 = date!(2026 - 01 - 01);
    let d3 = date!(2026 - 12 - 01);
    let schedule = BermudanSchedule::new(vec![d1, d2, d3])
        .with_lockout(date!(2026 - 03 - 01))
        .with_notice_days(2);

    assert_eq!(schedule.exercise_dates, vec![d2, d1, d3]);
    assert_eq!(schedule.notice_days, 2);

    let effective = schedule.effective_dates();
    assert_eq!(effective, vec![d1, d3]);
    assert_eq!(schedule.num_exercises(), 2);
}

#[test]
fn test_bermudan_schedule_co_terminal_excludes_maturity() {
    let first_ex = date!(2026 - 01 - 01);
    let swap_end = date!(2027 - 01 - 01);
    let schedule = BermudanSchedule::co_terminal(first_ex, swap_end, Tenor::semi_annual())
        .expect("valid Bermudan schedule");

    assert!(!schedule.exercise_dates.is_empty());
    assert!(schedule.exercise_dates.first().unwrap() >= &first_ex);
    assert!(schedule.exercise_dates.last().unwrap() < &swap_end);
}

#[test]
fn test_swaption_cash_annuity_zero_forward_and_invalid_freq() {
    let (_, expiry, swap_start, swap_end) = standard_dates();
    let mut swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    swaption.settlement = SwaptionSettlement::Cash;

    let expected = swaption
        .year_fraction(swaption.swap_start, swaption.swap_end, swaption.day_count)
        .unwrap();
    let annuity = swaption.cash_annuity_par_yield(0.0).unwrap();
    assert_approx_eq(annuity, expected, 1e-8, "cash annuity zero rate");

    swaption.fixed_freq = Tenor::new(0, TenorUnit::Months);
    assert!(swaption.cash_annuity_par_yield(0.02).is_err());
}

#[test]
fn test_resolve_volatility_priority_and_greek_inputs_expired() {
    let (as_of, expiry, swap_start, swap_end) = standard_dates();
    let mut swaption = create_standard_payer_swaption(expiry, swap_start, swap_end, 0.05);
    let market = create_flat_market(as_of, 0.03, 0.2);
    let forward = swaption.forward_swap_rate(&market, as_of).unwrap();
    let t = swaption
        .year_fraction(as_of, swaption.expiry, swaption.day_count)
        .unwrap();

    let surface_vol = swaption.resolve_volatility(&market, forward, t).unwrap();
    assert_approx_eq(surface_vol, 0.2, 1e-12, "surface vol");

    swaption.pricing_overrides = swaption
        .pricing_overrides
        .clone()
        .with_implied_vol(0.35)
        .with_vol_surface_extrapolation(VolSurfaceExtrapolation::Clamp);
    let override_vol = swaption.resolve_volatility(&market, forward, t).unwrap();
    assert_approx_eq(override_vol, 0.35, 1e-12, "override vol");

    let sabr_params = SABRParameters::rates_standard(0.2, 0.5, -0.25).unwrap();
    swaption = swaption.with_sabr(sabr_params.clone());
    let sabr_vol = swaption.resolve_volatility(&market, forward, t).unwrap();
    let sabr_model = finstack_valuations::instruments::common::models::SABRModel::new(sabr_params);
    let expected_sabr = sabr_model
        .implied_volatility(forward, swaption.strike_rate, t)
        .unwrap();
    assert_approx_eq(sabr_vol, expected_sabr, 1e-10, "sabr vol");

    let expired = create_standard_payer_swaption(
        date!(2023 - 01 - 01),
        date!(2023 - 01 - 01),
        date!(2028 - 01 - 01),
        0.05,
    );
    let none_inputs = expired.greek_inputs(&market, as_of).unwrap();
    assert!(none_inputs.is_none());
}

#[test]
fn test_bermudan_swaption_schedule_and_conversion() {
    let as_of = date!(2024 - 01 - 01);
    let swap_start = date!(2025 - 01 - 01);
    let swap_end = date!(2030 - 01 - 01);
    let first_ex = date!(2026 - 01 - 01);

    let swaption = BermudanSwaption::new_payer(
        "BERM-TEST",
        finstack_core::money::Money::new(1_000_000.0, finstack_core::currency::Currency::USD),
        0.03,
        swap_start,
        swap_end,
        BermudanSchedule::co_terminal(first_ex, swap_end, Tenor::semi_annual())
            .expect("valid Bermudan schedule"),
        "USD-OIS",
        "USD-SOFR-3M",
        "USD-SWPNVOL",
    );

    let (payment_dates, accruals) = swaption.build_swap_schedule(as_of).unwrap();
    assert_eq!(payment_dates.len(), accruals.len());
    assert!(!payment_dates.is_empty());

    let payment_times = swaption.payment_times(as_of).unwrap();
    assert_eq!(payment_times.len(), payment_dates.len());

    let euro = swaption.to_european().unwrap();
    assert_eq!(euro.expiry, first_ex);
    assert_eq!(euro.swap_start, first_ex);
    assert_eq!(euro.swap_end, swap_end);
    assert_eq!(euro.vol_model, VolatilityModel::Black);

    let market = create_flat_market(as_of, 0.03, 0.2);
    let disc = market.get_discount("USD_OIS").unwrap();
    let annuity_early = swaption
        .remaining_annuity(disc.as_ref(), as_of, first_ex)
        .unwrap();
    let later_ex = date!(2027 - 01 - 01);
    let annuity_late = swaption
        .remaining_annuity(disc.as_ref(), as_of, later_ex)
        .unwrap();
    assert!(annuity_early > annuity_late);
}
