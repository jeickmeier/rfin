//! Equity pricer engine.
//!
//! Provides deterministic PV for `Equity` instruments. The PV is
//! `price_per_share * effective_shares` in the instrument's quote currency.
//!
//! All arithmetic uses the core `Money` type to respect rounding policy and
//! currency safety requirements.

use crate::instruments::common::traits::Instrument;
use crate::instruments::equity::Equity;
// (no pricer registry integration here)
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;


/// Stateless pricing engine for `Equity` instruments.
#[derive(Debug, Default, Clone, Copy)]
pub struct EquityPricer;

impl EquityPricer {
    /// Resolve price per share for the equity.
    ///
    /// Priority:
    /// 1) `inst.price_quote` if set
    /// 2) `MarketContext::price` using instrument-provided overrides and fallbacks:
    ///    explicit `price_id`, attribute hints, ticker, instrument id, `{ticker}-SPOT`, then `EQUITY-SPOT`
    ///    - If `Price`, convert to `inst.currency` via FX matrix
    ///    - If `Unitless`, treat as amount in `inst.currency`
    pub fn price_per_share(
        &self,
        inst: &Equity,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        inst.price_per_share(curves, as_of)
    }

    /// Compute present value in the instrument's currency.
    ///
    /// Parameters:
    /// - `inst`: reference to the `Equity` instrument
    /// - `curves`: market context (unused currently; placeholder for quotes)
    /// - `as_of`: valuation date (unused currently)
    pub fn pv(&self, inst: &Equity, curves: &MarketContext, as_of: Date) -> Result<Money> {
        let px = self.price_per_share(inst, curves, as_of)?;
        Ok(Money::new(
            px.amount() * inst.effective_shares(),
            inst.currency,
        ))
    }

    /// Resolve dividend yield (annualized, decimal) for the equity.
    ///
    /// Attempts to read from market context using the key format
    /// "{ticker}-DIVYIELD". When not present, defaults to 0.0.
    pub fn dividend_yield(&self, inst: &Equity, curves: &MarketContext) -> Result<f64> {
        inst.dividend_yield(curves)
    }

    /// Build forward price per share using continuous-compound approximation:
    /// F(t) = S0 × exp((r - q) × t)
    ///
    /// - S0 resolved via `price_per_share` (respects instrument overrides)
    /// - r pulled from discount curve "{CURRENCY}-OIS" zero rate
    /// - q from `dividend_yield` (0.0 when absent)
    pub fn forward_price_per_share(
        &self,
        inst: &Equity,
        curves: &MarketContext,
        as_of: Date,
        t: f64,
    ) -> Result<Money> {
        let s0 = self.price_per_share(inst, curves, as_of)?;
        let dy = self.dividend_yield(inst, curves)?;
        let discount_id = format!("{}-OIS", inst.currency);
        let disc = curves.get_discount_ref(&discount_id)?;
        let r = disc.zero(t);
        let fwd = s0.amount() * ((r - dy) * t).exp();
        Ok(Money::new(fwd, inst.currency))
    }

    /// Forward total value for the position (per-share forward × shares).
    pub fn forward_value(
        &self,
        inst: &Equity,
        curves: &MarketContext,
        as_of: Date,
        t: f64,
    ) -> Result<Money> {
        let per_share = self.forward_price_per_share(inst, curves, as_of, t)?;
        Ok(Money::new(
            per_share.amount() * inst.effective_shares(),
            inst.currency,
        ))
    }
}

// ========================= NEW SIMPLIFIED PRICER =========================

/// New simplified Equity discounting pricer (replaces macro-based version)
pub struct SimpleEquityDiscountingPricer;

impl SimpleEquityDiscountingPricer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleEquityDiscountingPricer {
    fn default() -> Self {
        Self::new()
    }
}

#[finstack_macros::register_pricer]
impl crate::pricer::Pricer for SimpleEquityDiscountingPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(
            crate::pricer::InstrumentType::Equity,
            crate::pricer::ModelKey::Discounting,
        )
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &finstack_core::market_data::MarketContext,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        // Type-safe downcasting
        let equity = instrument
            .as_any()
            .downcast_ref::<Equity>()
            .ok_or_else(|| crate::pricer::PricingError::TypeMismatch {
                expected: crate::pricer::InstrumentType::Equity,
                got: instrument.key(),
            })?;

        // Get as_of date (prefer OIS base date for the instrument currency)
        let disc_id = format!("{}-OIS", equity.currency);
        let as_of = if let Ok(disc) = market.get_discount_ref(&disc_id) {
            disc.base_date()
        } else {
            Date::from_calendar_date(1970, time::Month::January, 1).unwrap()
        };

        // Compute present value using the equity pricer
        let pv = EquityPricer
            .pv(equity, market, as_of)
            .map_err(|e| crate::pricer::PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            equity.id(),
            as_of,
            pv,
        ))
    }
}
