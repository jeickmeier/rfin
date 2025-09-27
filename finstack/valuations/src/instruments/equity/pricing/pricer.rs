//! Equity pricer engine.
//!
//! Provides deterministic PV for `Equity` instruments. The PV is
//! `price_per_share * effective_shares` in the instrument's quote currency.
//!
//! All arithmetic uses the core `Money` type to respect rounding policy and
//! currency safety requirements.

use crate::instruments::equity::Equity;
use crate::instruments::common::traits::Instrument;
// (no pricer registry integration here)
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::fx::{FxConversionPolicy, FxProvider};
use finstack_core::money::Money;
use finstack_core::Result;
use finstack_core::F;

/// Stateless pricing engine for `Equity` instruments.
#[derive(Debug, Default, Clone, Copy)]
pub struct EquityPricer;

impl EquityPricer {
    /// Resolve price per share for the equity.
    ///
    /// Priority:
    /// 1) `inst.price_quote` if set
    /// 2) `MarketContext::price(inst.ticker)` using `MarketScalar::{Price,Unitless}`
    ///    - If `Price`, convert to `inst.currency` via FX matrix
    ///    - If `Unitless`, treat as amount in `inst.currency`
    pub fn price_per_share(
        &self,
        inst: &Equity,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        if let Some(px) = inst.price_quote {
            return Ok(Money::new(px, inst.currency));
        }

        let scalar = curves.price(&inst.ticker)?;
        match scalar {
            MarketScalar::Price(m) => {
                if m.currency() == inst.currency {
                    Ok(*m)
                } else {
                    // Convert via FX matrix provider
                    let matrix = curves.fx.as_ref().ok_or_else(|| {
                        finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                            id: "fx_matrix".to_string(),
                        })
                    })?;

                    struct MatrixProvider<'a> {
                        m: &'a finstack_core::money::fx::FxMatrix,
                    }
                    impl FxProvider for MatrixProvider<'_> {
                        fn rate(
                            &self,
                            from: finstack_core::currency::Currency,
                            to: finstack_core::currency::Currency,
                            on: Date,
                            policy: finstack_core::money::fx::FxConversionPolicy,
                        ) -> finstack_core::Result<finstack_core::money::fx::FxRate>
                        {
                            let r = self.m.rate(finstack_core::money::fx::FxQuery::with_policy(
                                from, to, on, policy,
                            ))?;
                            Ok(r.rate)
                        }
                    }

                    let provider = MatrixProvider { m: matrix };
                    m.convert(
                        inst.currency,
                        as_of,
                        &provider,
                        FxConversionPolicy::CashflowDate,
                    )
                }
            }
            MarketScalar::Unitless(v) => Ok(Money::new(*v, inst.currency)),
        }
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
    pub fn dividend_yield(&self, inst: &Equity, curves: &MarketContext) -> Result<F> {
        let key = format!("{}-DIVYIELD", inst.ticker);
        let dy = curves
            .price(&key)
            .map(|scalar| match scalar {
                MarketScalar::Unitless(v) => *v,
                MarketScalar::Price(_) => 0.0,
            })
            .unwrap_or(0.0);
        Ok(dy)
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
        t: F,
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
        t: F,
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

impl crate::pricer::Pricer for SimpleEquityDiscountingPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::Equity, crate::pricer::ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn crate::pricer::PriceableExt,
        market: &finstack_core::market_data::MarketContext,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        // Type-safe downcasting
        let equity = instrument.as_any()
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
        let pv = EquityPricer.pv(equity, market, as_of)
            .map_err(|e| crate::pricer::PricingError::ModelFailure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(equity.id(), as_of, pv))
    }
}
