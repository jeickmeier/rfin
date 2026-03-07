//! Quanto option pricers.
//!
//! Only analytical pricing is supported. Monte Carlo pricing has been removed
//! because the quanto drift adjustment model cannot be correctly represented
//! in a simple 1D MC simulation without a 2D correlated equity/FX process.

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::fx::quanto_option::types::QuantoOption;
use crate::pricer::{
    InstrumentType, ModelKey, Pricer, PricerKey, PricingError, PricingErrorContext, PricingResult,
};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;

// ========================= ANALYTICAL PRICER =========================

use crate::instruments::common_impl::models::closed_form::quanto::{quanto_call, quanto_put};

/// Helper to collect inputs for quanto option pricing.
fn collect_quanto_inputs(
    inst: &QuantoOption,
    curves: &MarketContext,
    as_of: Date,
) -> finstack_core::Result<(f64, f64, f64, f64, f64, f64, f64)> {
    let t = inst
        .day_count
        .year_fraction(as_of, inst.expiry, DayCountCtx::default())?;

    let disc_curve = curves.get_discount(inst.domestic_discount_curve_id.as_str())?;
    let r_dom = disc_curve.zero(t);

    // Get foreign rate
    let for_curve = curves.get_discount(inst.foreign_discount_curve_id.as_str())?;
    let r_for = for_curve.zero(t);

    let spot_scalar = curves.price(&inst.spot_id)?;
    let spot = match spot_scalar {
        finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
        finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
    };

    let q = crate::instruments::common_impl::helpers::resolve_optional_dividend_yield(
        curves,
        inst.div_yield_id.as_ref(),
    )?;

    let vol_surface = curves.surface(inst.vol_surface_id.as_str())?;
    let sigma_equity = vol_surface.value_clamped(t, inst.equity_strike.amount());

    // Get FX volatility
    let sigma_fx = if let Some(fx_vol_id) = &inst.fx_vol_id {
        let fx_vol_surface = curves.surface(fx_vol_id.as_str())?;
        fx_vol_surface.value_clamped(t, 1.0)
    } else {
        return Err(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_vol_id".to_string(),
            },
        ));
    };

    Ok((spot, r_dom, r_for, q, sigma_equity, sigma_fx, t))
}

fn payoff_scale(inst: &QuantoOption) -> finstack_core::Result<f64> {
    let quantity = inst.underlying_quantity.ok_or_else(|| {
        finstack_core::Error::Validation(
            "QuantoOption requires `underlying_quantity`; domestic notional alone is ambiguous"
                .to_string(),
        )
    })?;
    let fx_rate = inst.payoff_fx_rate.ok_or_else(|| {
        finstack_core::Error::Validation(
            "QuantoOption requires `payoff_fx_rate`; domestic notional alone is ambiguous"
                .to_string(),
        )
    })?;

    if !quantity.is_finite() || quantity <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "QuantoOption underlying_quantity must be positive and finite; got {}",
            quantity
        )));
    }
    if !fx_rate.is_finite() || fx_rate <= 0.0 {
        return Err(finstack_core::Error::Validation(format!(
            "QuantoOption payoff_fx_rate must be positive and finite; got {}",
            fx_rate
        )));
    }

    Ok(quantity * fx_rate)
}

/// Quanto option analytical pricer.
pub struct QuantoOptionAnalyticalPricer;

impl QuantoOptionAnalyticalPricer {
    /// Create a new analytical quanto option pricer
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuantoOptionAnalyticalPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pricer for QuantoOptionAnalyticalPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::QuantoOption, ModelKey::QuantoBS)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: Date,
    ) -> PricingResult<ValuationResult> {
        let quanto = instrument
            .as_any()
            .downcast_ref::<QuantoOption>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::QuantoOption, instrument.key())
            })?;

        let (spot, r_dom, r_for, q, sigma_equity, sigma_fx, t) =
            collect_quanto_inputs(quanto, market, as_of).map_err(|e| {
                PricingError::model_failure_with_context(
                    e.to_string(),
                    PricingErrorContext::default(),
                )
            })?;

        if t <= 0.0 {
            return Ok(ValuationResult::stamped(
                quanto.id(),
                as_of,
                Money::new(0.0, quanto.quote_currency),
            ));
        }

        let price = match quanto.option_type {
            crate::instruments::OptionType::Call => quanto_call(
                spot,
                quanto.equity_strike.amount(),
                t,
                r_dom,
                r_for,
                q,
                sigma_equity,
                sigma_fx,
                quanto.correlation,
            ),
            crate::instruments::OptionType::Put => quanto_put(
                spot,
                quanto.equity_strike.amount(),
                t,
                r_dom,
                r_for,
                q,
                sigma_equity,
                sigma_fx,
                quanto.correlation,
            ),
        };

        let scale = payoff_scale(quanto).map_err(|e| {
            PricingError::model_failure_with_context(e.to_string(), PricingErrorContext::default())
        })?;
        let pv = Money::new(price * scale, quanto.quote_currency);
        Ok(ValuationResult::stamped(quanto.id(), as_of, pv))
    }
}
