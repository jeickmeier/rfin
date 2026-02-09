//! FX touch option (American binary option) instrument definition.

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Touch type: one-touch (pays if barrier is hit) or no-touch (pays if not hit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum TouchType {
    /// Pays if the spot rate touches the barrier at any time before expiry.
    OneTouch,
    /// Pays if the spot rate does NOT touch the barrier before expiry.
    NoTouch,
}

/// Barrier direction for touch options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BarrierDirection {
    /// Barrier is above current spot (spot must rise to touch).
    Up,
    /// Barrier is below current spot (spot must fall to touch).
    Down,
}

/// Payout timing for touch options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PayoutTiming {
    /// Payout occurs immediately when barrier is hit (for one-touch).
    AtHit,
    /// Payout is deferred to expiry regardless of when barrier is hit.
    AtExpiry,
}

/// FX touch option (American binary option).
///
/// Touch options pay a fixed amount if the spot rate touches a barrier
/// level at any time before expiry:
/// - One-touch: pays if barrier is touched
/// - No-touch: pays if barrier is NOT touched
///
/// # Pricing
///
/// Uses closed-form pricing for continuous monitoring (Rubinstein & Reiner 1991):
///
/// **Down-and-in one-touch (S > H, pay at expiry)**:
/// ```text
/// P = e^{-r_d T} × [(S/H)^{-(μ+λ)} × N(η·z) + (S/H)^{-(μ-λ)} × N(η·z')]
/// ```
///
/// where:
/// - μ = (r_d - r_f - σ²/2) / σ²
/// - λ = sqrt(μ² + 2r_d/σ²)
/// - z = ln(H/S)/(σ√T) + λσ√T
/// - z' = ln(H/S)/(σ√T) - λσ√T
/// - η = +1 for down barrier, -1 for up barrier
///
/// **No-touch**: P_no_touch = e^{-r_d T} × payout - P_one_touch
///
/// # References
///
/// - Rubinstein, M., & Reiner, E. (1991). "Unscrambling the Binary Code."
///   *Risk Magazine*, 4(9), 75-83.
/// - Wystup, U. (2006). *FX Options and Structured Products*. Wiley.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct FxTouchOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Base currency (foreign currency)
    pub base_currency: Currency,
    /// Quote currency (domestic currency)
    pub quote_currency: Currency,
    /// Barrier level (exchange rate that triggers the touch)
    pub barrier_level: f64,
    /// Touch type (one-touch or no-touch)
    pub touch_type: TouchType,
    /// Barrier direction (up or down)
    pub barrier_direction: BarrierDirection,
    /// Fixed payout amount
    pub payout_amount: Money,
    /// Payout timing (at hit or at expiry)
    pub payout_timing: PayoutTiming,
    /// Option expiry date
    pub expiry: Date,
    /// Day count convention
    pub day_count: DayCount,
    /// Domestic currency discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign currency discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX volatility surface ID
    pub vol_surface_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl crate::instruments::common_impl::traits::CurveDependencies for FxTouchOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .discount(self.foreign_discount_curve_id.clone())
            .build()
    }
}

impl FxTouchOption {
    /// Create a canonical example FX touch option for testing and documentation.
    ///
    /// Returns a 6-month EUR/USD down-and-in one-touch option.
    pub fn example() -> Self {
        use time::macros::date;
        Self::builder()
            .id(InstrumentId::new("FXTOUCH-EURUSD-OT"))
            .base_currency(Currency::EUR)
            .quote_currency(Currency::USD)
            .barrier_level(1.05)
            .touch_type(TouchType::OneTouch)
            .barrier_direction(BarrierDirection::Down)
            .payout_amount(Money::new(1_000_000.0, Currency::USD))
            .payout_timing(PayoutTiming::AtExpiry)
            .expiry(date!(2024 - 06 - 21))
            .day_count(DayCount::Act365F)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .vol_surface_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example FX touch option with valid constants should never fail")
            })
    }

    fn price_internal(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let calculator = super::calculator::FxTouchOptionCalculator;
        calculator.npv(self, market, as_of)
    }
}

impl crate::instruments::common_impl::traits::Instrument for FxTouchOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxTouchOption
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.price_internal(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.expiry)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        None
    }
}

// Touch options use finite-difference Greeks (barrier discontinuities make
// analytical Greeks unreliable near the barrier).

impl crate::instruments::common_impl::traits::OptionDeltaProvider for FxTouchOption {
    fn option_delta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common_impl::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Use FX spot bump via FX matrix
        let fx_matrix = market.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let current_spot = fx_matrix
            .rate(finstack_core::money::fx::FxQuery::new(
                self.base_currency,
                self.quote_currency,
                as_of,
            ))?
            .rate;
        let bump_size = current_spot * crate::metrics::bump_sizes::SPOT;
        if bump_size <= 0.0 {
            return Ok(0.0);
        }

        // Bump spot up/down via vol override trick: use pricing overrides for spot bump
        // Instead, we bump the FX matrix
        let up_fx = {
            let fx_up = finstack_core::money::fx::FxMatrix::new(fx_matrix.provider().clone());
            fx_up.set_quote(
                self.base_currency,
                self.quote_currency,
                current_spot * (1.0 + crate::metrics::bump_sizes::SPOT),
            );
            market.clone().insert_fx(fx_up)
        };
        let dn_fx = {
            let fx_dn = finstack_core::money::fx::FxMatrix::new(fx_matrix.provider().clone());
            fx_dn.set_quote(
                self.base_currency,
                self.quote_currency,
                current_spot * (1.0 - crate::metrics::bump_sizes::SPOT),
            );
            market.clone().insert_fx(fx_dn)
        };

        let pv_up = self.value(&up_fx, as_of)?.amount();
        let pv_dn = self.value(&dn_fx, as_of)?.amount();

        Ok((pv_up - pv_dn) / (2.0 * bump_size))
    }
}

impl crate::instruments::common_impl::traits::OptionGammaProvider for FxTouchOption {
    fn option_gamma(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common_impl::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();

        let fx_matrix = market.fx().ok_or(finstack_core::Error::from(
            finstack_core::InputError::NotFound {
                id: "fx_matrix".to_string(),
            },
        ))?;
        let current_spot = fx_matrix
            .rate(finstack_core::money::fx::FxQuery::new(
                self.base_currency,
                self.quote_currency,
                as_of,
            ))?
            .rate;
        let bump_size = current_spot * crate::metrics::bump_sizes::SPOT;
        if bump_size <= 0.0 {
            return Ok(0.0);
        }

        let up_fx = {
            let fx_up = finstack_core::money::fx::FxMatrix::new(fx_matrix.provider().clone());
            fx_up.set_quote(
                self.base_currency,
                self.quote_currency,
                current_spot * (1.0 + crate::metrics::bump_sizes::SPOT),
            );
            market.clone().insert_fx(fx_up)
        };
        let dn_fx = {
            let fx_dn = finstack_core::money::fx::FxMatrix::new(fx_matrix.provider().clone());
            fx_dn.set_quote(
                self.base_currency,
                self.quote_currency,
                current_spot * (1.0 - crate::metrics::bump_sizes::SPOT),
            );
            market.clone().insert_fx(fx_dn)
        };

        let pv_up = self.value(&up_fx, as_of)?.amount();
        let pv_dn = self.value(&dn_fx, as_of)?.amount();

        Ok((pv_up - 2.0 * base_pv + pv_dn) / (bump_size * bump_size))
    }
}

impl crate::instruments::common_impl::traits::OptionVegaProvider for FxTouchOption {
    fn option_vega(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common_impl::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();
        let bumped = crate::metrics::bump_surface_vol_absolute(
            market,
            self.vol_surface_id.as_str(),
            crate::metrics::bump_sizes::VOLATILITY,
        )?;
        let pv_bumped = self.value(&bumped, as_of)?.amount();
        Ok((pv_bumped - base_pv) / crate::metrics::bump_sizes::VOLATILITY)
    }
}

impl crate::instruments::common_impl::traits::OptionRhoProvider for FxTouchOption {
    fn option_rho_bp(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common_impl::traits::Instrument;

        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        let base_pv = self.value(market, as_of)?.amount();
        let bump_bp = self.pricing_overrides.rho_bump_bp();
        let bumped = crate::metrics::bump_discount_curve_parallel(
            market,
            &self.domestic_discount_curve_id,
            bump_bp,
        )?;
        let pv_bumped = self.value(&bumped, as_of)?.amount();
        Ok(pv_bumped - base_pv)
    }
}
