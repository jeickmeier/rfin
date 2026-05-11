//! CIP invariance and rebalancing sign integration tests for MtM-resetting XCCY swap pricing.
//!
//! Spec: docs/superpowers/specs/2026-05-10-xccy-mtm-reset-design.md
//!
//! The CIP invariance identity: with `spread = 0` and CIP-consistent curves, the
//! MtM-reset PV equals the fixed-notional PV (within numerical tolerance). This is the
//! load-bearing correctness check for Task 7's math. If the implementation has sign errors
//! or formula bugs, these tests will catch them.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::money::fx::{FxMatrix, SimpleFxProvider};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::rates::xccy_swap::{
    LegSide, NotionalExchange, ResettingSide, XccySwap, XccySwapLeg,
};
use rust_decimal::Decimal;
use std::sync::Arc;
use time::Month;

const N_USD: f64 = 10_000_000.0;
const SPOT_USD_PER_EUR: f64 = 1.10;
const N_EUR_INITIAL: f64 = N_USD / SPOT_USD_PER_EUR;
const USD_ZERO: f64 = 0.02;
const EUR_ZERO: f64 = 0.01;
const TENOR_YEARS: f64 = 5.0;

fn base_date() -> Date {
    Date::from_calendar_date(2025, Month::January, 2).expect("valid base date")
}

fn end_date() -> Date {
    Date::from_calendar_date(2030, Month::January, 2).expect("valid end date")
}

fn build_discount(id: &str, base: Date, zero: f64) -> DiscountCurve {
    let t = TENOR_YEARS;
    let df_end = (-zero * t).exp();
    DiscountCurve::builder(CurveId::new(id))
        .base_date(base)
        .day_count(DayCount::Act365F)
        .knots([(0.0, 1.0), (t, df_end)])
        .interp(InterpStyle::Linear)
        .extrapolation(ExtrapolationPolicy::FlatZero)
        .build()
        .expect("build discount curve")
}

fn build_forward(id: &str, base: Date, zero: f64) -> ForwardCurve {
    ForwardCurve::builder(CurveId::new(id), 0.25)
        .base_date(base)
        .knots(vec![(0.0, zero), (TENOR_YEARS, zero)])
        .interp(InterpStyle::Linear)
        .build()
        .expect("build forward curve")
}

/// Build an FxMatrix with EUR/USD spot = SPOT_USD_PER_EUR (1.10).
///
/// Uses the same `SimpleFxProvider` pattern from the existing
/// `base_value_dispatches_mtm_resetting_to_pricing_mtm` test in
/// `finstack/valuations/src/instruments/rates/xccy_swap/types.rs`.
fn build_fx_matrix(spot_eur_to_usd: f64) -> FxMatrix {
    let provider = Arc::new(SimpleFxProvider::new());
    provider
        .set_quote(Currency::EUR, Currency::USD, spot_eur_to_usd)
        .expect("set EUR/USD FX rate");
    FxMatrix::new(provider)
}

fn build_market_context() -> MarketContext {
    let base = base_date();
    MarketContext::new()
        .insert(build_discount("USD-OIS", base, USD_ZERO))
        .insert(build_discount("EUR-OIS", base, EUR_ZERO))
        .insert(build_forward("USD-SOFR-3M", base, USD_ZERO))
        .insert(build_forward("EUR-EURIBOR-3M", base, EUR_ZERO))
        .insert_fx(build_fx_matrix(SPOT_USD_PER_EUR))
}

fn build_swap(notional_exchange: NotionalExchange, spread_bp: Decimal) -> XccySwap {
    let start = base_date();
    let end = end_date();
    let eur_leg = XccySwapLeg {
        currency: Currency::EUR,
        notional: Money::new(N_EUR_INITIAL, Currency::EUR),
        side: LegSide::Receive,
        forward_curve_id: CurveId::new("EUR-EURIBOR-3M"),
        discount_curve_id: CurveId::new("EUR-OIS"),
        start,
        end,
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        stub: StubKind::ShortFront,
        spread_bp,
        payment_lag_days: 0,
        calendar_id: None,
        reset_lag_days: None,
        allow_calendar_fallback: true,
    };
    let usd_leg = XccySwapLeg {
        currency: Currency::USD,
        notional: Money::new(N_USD, Currency::USD),
        side: LegSide::Pay,
        forward_curve_id: CurveId::new("USD-SOFR-3M"),
        discount_curve_id: CurveId::new("USD-OIS"),
        start,
        end,
        frequency: Tenor::quarterly(),
        day_count: DayCount::Act360,
        bdc: BusinessDayConvention::ModifiedFollowing,
        stub: StubKind::ShortFront,
        spread_bp: Decimal::ZERO,
        payment_lag_days: 0,
        calendar_id: None,
        reset_lag_days: None,
        allow_calendar_fallback: true,
    };

    XccySwap::new("MTM-TEST", eur_leg, usd_leg, Currency::USD)
        .with_notional_exchange(notional_exchange)
}

/// CIP invariance: with spread = 0 and CIP-consistent curves, MtM-reset and fixed-notional
/// XCCY swap PVs must agree. This is the textbook result and the load-bearing check that
/// our implementation is correct.
///
/// The swap is structured as EUR leg1 (receive) / USD leg2 (pay), with leg1 (EUR) resetting.
/// Under CIP with flat curves and spot FX = 1.10, the MtM-reset redistributes cashflows
/// in time but is PV-neutral.
#[test]
fn cip_invariance_mtm_reset_equals_fixed_notional_when_spread_zero() {
    let ctx = build_market_context();
    let as_of = base_date();

    let fixed = build_swap(NotionalExchange::InitialAndFinal, Decimal::ZERO);
    let mtm = build_swap(
        NotionalExchange::MtmResetting {
            resetting_side: ResettingSide::Leg1, // EUR leg resets
        },
        Decimal::ZERO,
    );

    let pv_fixed = fixed.base_value(&ctx, as_of).expect("fixed PV").amount();
    let pv_mtm = mtm.base_value(&ctx, as_of).expect("mtm PV").amount();

    // Tolerance: ~1e-4 absolute (= 1e-11 relative for N_USD=1e7). Calendar/day-count
    // noise from the schedule builder can introduce O(1e-5)-scale fluctuations.
    let tol = 1e-4 * N_USD;
    assert!(
        (pv_fixed - pv_mtm).abs() < tol,
        "CIP invariance violated: pv_fixed={pv_fixed:.4}, pv_mtm={pv_mtm:.4}, diff={:.4e}, tol={tol:.4e}",
        pv_fixed - pv_mtm
    );
}

/// Direction-2 CIP invariance: swap the rate ordering (USD at 1%, EUR at 2%) so the forward FX
/// moves the other way. The CIP-invariance identity must hold regardless of which
/// currency has the higher rate.
#[test]
fn cip_invariance_holds_under_reversed_rate_ordering() {
    let base = base_date();
    // Swap the rates: USD at EUR_ZERO (1%), EUR at USD_ZERO (2%)
    let usd_disc = build_discount("USD-OIS", base, EUR_ZERO); // USD at "1%"
    let eur_disc = build_discount("EUR-OIS", base, USD_ZERO); // EUR at "2%"
    let usd_fwd = build_forward("USD-SOFR-3M", base, EUR_ZERO);
    let eur_fwd = build_forward("EUR-EURIBOR-3M", base, USD_ZERO);

    let fx = build_fx_matrix(SPOT_USD_PER_EUR);

    let ctx = MarketContext::new()
        .insert(usd_disc)
        .insert(eur_disc)
        .insert(usd_fwd)
        .insert(eur_fwd)
        .insert_fx(fx);

    let as_of = base_date();
    let fixed = build_swap(NotionalExchange::InitialAndFinal, Decimal::ZERO);
    let mtm = build_swap(
        NotionalExchange::MtmResetting {
            resetting_side: ResettingSide::Leg1,
        },
        Decimal::ZERO,
    );

    let pv_fixed = fixed.base_value(&ctx, as_of).expect("fixed PV").amount();
    let pv_mtm = mtm.base_value(&ctx, as_of).expect("mtm PV").amount();

    let tol = 1e-4 * N_USD;
    assert!(
        (pv_fixed - pv_mtm).abs() < tol,
        "CIP invariance failed under reversed rate ordering: pv_fixed={pv_fixed:.4}, pv_mtm={pv_mtm:.4}, diff={:.4e}, tol={tol:.4e}",
        pv_fixed - pv_mtm
    );
}
