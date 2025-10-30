//! Shared test utilities for FX option tests.
//!
//! Provides common fixtures, builders, and assertion helpers to reduce
//! duplication across test files.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::surfaces::VolSurface;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::fx::providers::SimpleFxProvider;
use finstack_core::money::fx::FxMatrix;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::common::parameters::market::{ExerciseStyle, OptionType};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::fx_option::FxOption;
use finstack_valuations::instruments::{PricingOverrides, SettlementType};
use std::sync::Arc;

/// Standard currency pairs for testing.
pub const BASE: Currency = Currency::EUR;
pub const QUOTE: Currency = Currency::USD;
pub const DOMESTIC_ID: &str = "USD-OIS";
pub const FOREIGN_ID: &str = "EUR-OIS";
pub const VOL_ID: &str = "EURUSD-VOL";

/// Market parameters for standard test scenarios.
#[derive(Clone, Copy, Debug)]
pub struct MarketParams {
    pub spot: f64,
    pub vol: f64,
    pub r_domestic: f64,
    pub r_foreign: f64,
}

impl Default for MarketParams {
    fn default() -> Self {
        Self {
            spot: 1.20,
            vol: 0.15,
            r_domestic: 0.03,
            r_foreign: 0.01,
        }
    }
}

impl MarketParams {
    /// ATM scenario with moderate vol.
    pub fn atm() -> Self {
        Self::default()
    }

    /// High volatility scenario.
    pub fn high_vol() -> Self {
        Self {
            vol: 0.35,
            ..Self::default()
        }
    }

    /// Low volatility scenario.
    pub fn low_vol() -> Self {
        Self {
            vol: 0.05,
            ..Self::default()
        }
    }

    /// Steep rate differential (carry).
    #[allow(dead_code)]
    pub fn steep_carry() -> Self {
        Self {
            r_domestic: 0.05,
            r_foreign: 0.01,
            ..Self::default()
        }
    }
}

/// Build a flat discount curve.
pub fn build_flat_discount_curve(rate: f64, base_date: Date, curve_id: &str) -> DiscountCurve {
    DiscountCurve::builder(curve_id)
        .base_date(base_date)
        .day_count(DayCount::Act365F)
        .knots([
            (0.0, 1.0),
            (0.25, (-rate * 0.25).exp()),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

/// Build a flat volatility surface.
pub fn build_flat_vol_surface(vol: f64, surface_id: &str) -> VolSurface {
    VolSurface::builder(surface_id)
        .expiries(&[0.25, 0.5, 1.0, 2.0, 5.0])
        .strikes(&[0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4])
        .row(&[vol; 7])
        .row(&[vol; 7])
        .row(&[vol; 7])
        .row(&[vol; 7])
        .row(&[vol; 7])
        .build()
        .unwrap()
}

/// Create FX matrix with a given spot rate.
pub fn create_fx_matrix(eur_usd_rate: f64) -> FxMatrix {
    let provider = SimpleFxProvider::new();
    provider.set_quote(BASE, QUOTE, eur_usd_rate);
    FxMatrix::new(Arc::new(provider))
}

/// Build complete market context from parameters.
pub fn build_market_context(as_of: Date, params: MarketParams) -> MarketContext {
    let disc_curve_usd = build_flat_discount_curve(params.r_domestic, as_of, DOMESTIC_ID);
    let disc_curve_eur = build_flat_discount_curve(params.r_foreign, as_of, FOREIGN_ID);
    let vol_surface = build_flat_vol_surface(params.vol, VOL_ID);
    let fx_matrix = create_fx_matrix(params.spot);

    MarketContext::new()
        .insert_discount(disc_curve_usd)
        .insert_discount(disc_curve_eur)
        .insert_surface(vol_surface)
        .insert_fx(fx_matrix)
}

/// Build a standard European call option.
pub fn build_call_option(_as_of: Date, expiry: Date, strike: f64, notional: f64) -> FxOption {
    FxOption::builder()
        .id(InstrumentId::new("FX_CALL_TEST"))
        .base_currency(BASE)
        .quote_currency(QUOTE)
        .strike(strike)
        .option_type(OptionType::Call)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .day_count(DayCount::Act365F)
        .notional(Money::new(notional, BASE))
        .settlement(SettlementType::Cash)
        .domestic_disc_id(CurveId::new(DOMESTIC_ID))
        .foreign_disc_id(CurveId::new(FOREIGN_ID))
        .vol_id(CurveId::new(VOL_ID))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .unwrap()
}

/// Build a standard European put option.
pub fn build_put_option(_as_of: Date, expiry: Date, strike: f64, notional: f64) -> FxOption {
    FxOption::builder()
        .id(InstrumentId::new("FX_PUT_TEST"))
        .base_currency(BASE)
        .quote_currency(QUOTE)
        .strike(strike)
        .option_type(OptionType::Put)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .day_count(DayCount::Act365F)
        .notional(Money::new(notional, BASE))
        .settlement(SettlementType::Cash)
        .domestic_disc_id(CurveId::new(DOMESTIC_ID))
        .foreign_disc_id(CurveId::new(FOREIGN_ID))
        .vol_id(CurveId::new(VOL_ID))
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .unwrap()
}

/// Assert two floats are approximately equal with relative and absolute tolerance.
pub fn assert_approx_eq(actual: f64, expected: f64, rel_tol: f64, abs_tol: f64, msg: &str) {
    let diff = (actual - expected).abs();
    let rel_diff = if expected.abs() > 1e-12 {
        diff / expected.abs()
    } else {
        diff
    };

    let passes = diff <= abs_tol || rel_diff <= rel_tol;
    assert!(
        passes,
        "{}: expected {}, got {} (abs_diff={:.6e}, rel_diff={:.6e})",
        msg, expected, actual, diff, rel_diff
    );
}

/// Assert value is within a range (inclusive).
pub fn assert_in_range(value: f64, min: f64, max: f64, msg: &str) {
    assert!(
        value >= min && value <= max,
        "{}: expected value in [{}, {}], got {}",
        msg,
        min,
        max,
        value
    );
}

/// Finite difference delta approximation for validation.
pub fn finite_diff_delta(
    option: &FxOption,
    market: &MarketContext,
    as_of: Date,
    bump: f64,
) -> finstack_core::Result<f64> {
    let spot = market
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            option.base_currency,
            option.quote_currency,
            as_of,
        ))?
        .rate;

    // Bump spot up
    let mut market_up = market.clone();
    let fx_up = create_fx_matrix(spot + bump);
    market_up = market_up.insert_fx(fx_up);
    let pv_up = option.value(&market_up, as_of)?;

    // Bump spot down
    let mut market_down = market.clone();
    let fx_down = create_fx_matrix(spot - bump);
    market_down = market_down.insert_fx(fx_down);
    let pv_down = option.value(&market_down, as_of)?;

    Ok((pv_up.amount() - pv_down.amount()) / (2.0 * bump))
}

/// Finite difference gamma approximation for validation.
pub fn finite_diff_gamma(
    option: &FxOption,
    market: &MarketContext,
    as_of: Date,
    bump: f64,
) -> finstack_core::Result<f64> {
    let spot = market
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            option.base_currency,
            option.quote_currency,
            as_of,
        ))?
        .rate;

    let pv_mid = option.value(market, as_of)?;

    let mut market_up = market.clone();
    let fx_up = create_fx_matrix(spot + bump);
    market_up = market_up.insert_fx(fx_up);
    let pv_up = option.value(&market_up, as_of)?;

    let mut market_down = market.clone();
    let fx_down = create_fx_matrix(spot - bump);
    market_down = market_down.insert_fx(fx_down);
    let pv_down = option.value(&market_down, as_of)?;

    Ok((pv_up.amount() - 2.0 * pv_mid.amount() + pv_down.amount()) / (bump * bump))
}

/// Finite difference vega approximation for validation.
pub fn finite_diff_vega(
    option: &FxOption,
    market: &MarketContext,
    as_of: Date,
    bump: f64,
) -> finstack_core::Result<f64> {
    // Bump volatility surface
    let vol_surface = market.surface_ref(option.vol_id.clone())?;
    let base_vol = vol_surface.value_clamped(0.5, option.strike);

    let vol_surface_up = build_flat_vol_surface(base_vol + bump, VOL_ID);
    let mut market_up = market.clone();
    market_up = market_up.insert_surface(vol_surface_up);
    let pv_up = option.value(&market_up, as_of)?;

    let vol_surface_down = build_flat_vol_surface(base_vol - bump, VOL_ID);
    let mut market_down = market.clone();
    market_down = market_down.insert_surface(vol_surface_down);
    let pv_down = option.value(&market_down, as_of)?;

    // Vega is per 1% vol move, so scale by 100
    Ok((pv_up.amount() - pv_down.amount()) / (2.0 * bump) / 100.0)
}
