//! Equity option instrument definition and Black–Scholes helpers.
//!
//! # Dividend Handling
//!
//! This implementation uses a **continuous dividend yield** model (parameter `q` in BSM).
//! The dividend yield is provided via `div_yield_id` as a unitless scalar representing
//! the annualized continuous dividend yield.
//!
//! ## Continuous vs Discrete Dividends
//!
//! **Continuous dividend yield** (implemented here) is appropriate for:
//! - Index options (e.g., SPX, NDX) where dividend yield is well-defined
//! - Long-dated options where discrete effects average out
//! - Options on indices with many constituents and frequent ex-dates
//!
//! **Discrete dividends** are important for:
//! - Single-stock options near ex-dividend dates
//! - Short-dated options where discrete jumps are material
//! - High-yield stocks with large dividend payments
//!
//! ## Discrete Dividend Adjustment (Not Yet Implemented)
//!
//! For stocks with known discrete dividends, use the **spot adjustment method**:
//! ```text
//! S_adj = S - Σ D_i × e^{-r × t_i}  (for all dividends D_i at times t_i before expiry)
//! ```
//!
//! This is the QuantLib default approach. Until discrete dividend support is added,
//! users pricing single-stock options near ex-dividend dates should:
//!
//! 1. **Pre-adjust spot externally**: Calculate `S_adj` and pass as the spot price
//! 2. **Use implied dividend yield**: Back-solve for `q` from market option prices
//! 3. **Use terminal forward**: If forward prices are available, use `F = S × e^{(r-q)T}`
//!
//! ## Example: Manual Discrete Dividend Adjustment
//!
//! ```rust,ignore
//! // Stock at $100, dividend of $2 in 0.25 years, r = 5%, T = 0.5 years
//! let spot = 100.0;
//! let dividend = 2.0;
//! let t_div = 0.25;
//! let r = 0.05;
//!
//! // Adjusted spot for BSM pricing
//! let s_adj = spot - dividend * (-r * t_div).exp();
//! // s_adj ≈ 98.01
//! ```
//!
//! # Market Data Validation
//!
//! When `div_yield_id` is set, the lookup **must succeed**. A failed lookup returns
//! an error rather than silently defaulting to zero. This prevents market data
//! configuration errors from affecting P&L calculations.
//!
//! # References
//!
//! - Hull, J. C. (2018). *Options, Futures, and Other Derivatives* (10th ed.). Chapter 17.
//! - Haug, E. G. (2007). *The Complete Guide to Option Pricing Formulas* (2nd ed.). Section 2.8.
//! - QuantLib: `AnalyticEuropeanEngine` with `DividendVanillaOption`

// pricing formulas are implemented in the pricing engine; keep this module free of direct math imports
use crate::instruments::common::parameters::underlying::EquityUnderlyingParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use time::macros::date;
//
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::EquityOptionParams;

/// Equity option instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct EquityOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying equity ticker symbol
    pub underlying_ticker: String,
    /// Strike price
    pub strike: Money,
    /// Option type (call or put)
    pub option_type: OptionType,
    /// Exercise style (European or American)
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Contract size (number of shares per contract)
    pub contract_size: f64,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Settlement type (physical or cash)
    pub settlement: SettlementType,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Equity spot price identifier
    pub spot_id: String,
    /// Equity volatility surface ID
    pub vol_surface_id: CurveId,
    /// Optional continuous dividend yield identifier.
    ///
    /// The dividend yield should be a unitless scalar representing the annualized
    /// continuous dividend yield (e.g., 0.02 for 2%). This is used in the BSM model
    /// as the `q` parameter: `d1 = (ln(S/K) + (r - q + σ²/2)T) / (σ√T)`.
    ///
    /// **Important**: If this field is set, the lookup must succeed. A failed lookup
    /// will return an error rather than silently defaulting to zero, preventing
    /// market data configuration errors from affecting P&L.
    pub div_yield_id: Option<String>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::instruments::common::pricing::HasDiscountCurve for EquityOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for EquityOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl EquityOption {
    /// Create a canonical example equity option for testing and documentation.
    ///
    /// Returns an at-the-money SPX call option with 6 months to expiry.
    pub fn example() -> Self {
        let notional = Money::new(100_000.0, Currency::USD);
        let underlying = EquityUnderlyingParams::new("SPX", "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(100.0);

        // SAFETY: All inputs are compile-time validated constants
        Self::builder()
            .id(InstrumentId::new("SPX-CALL-4500"))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(4500.0, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(date!(2024 - 06 - 21))
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example equity option with valid constants should never fail")
            })
    }

    /// Create a European call option with standard conventions.
    ///
    /// This convenience constructor eliminates the builder for the most common case.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    #[deprecated(
        since = "0.4.0",
        note = "Use `EquityOption::builder()` and set `id/underlying/strike/expiry/option_type/exercise_style/curves/ids` explicitly before calling `.build()`."
    )]
    pub fn european_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> finstack_core::Result<Self> {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        // Build directly using derive-generated builder setters
        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
    }

    /// Create a European put option with standard conventions.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    #[deprecated(
        since = "0.4.0",
        note = "Use `EquityOption::builder()` and set `id/underlying/strike/expiry/option_type/exercise_style/curves/ids` explicitly before calling `.build()`."
    )]
    pub fn european_put(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> finstack_core::Result<Self> {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Put)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
    }

    /// Create an American call option with standard conventions.
    ///
    /// # Errors
    ///
    /// Returns an error if the builder fails validation.
    #[deprecated(
        since = "0.4.0",
        note = "Use `EquityOption::builder()` and set `id/underlying/strike/expiry/option_type/exercise_style/curves/ids` explicitly before calling `.build()`."
    )]
    pub fn american_call(
        id: impl Into<String>,
        ticker: impl Into<String>,
        strike: f64,
        expiry: Date,
        notional: Money,
        contract_size: f64,
    ) -> finstack_core::Result<Self> {
        let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
            .with_dividend_yield("EQUITY-DIVYIELD")
            .with_contract_size(contract_size);

        Self::builder()
            .id(InstrumentId::new(id.into()))
            .underlying_ticker(underlying.ticker)
            .strike(Money::new(strike, notional.currency()))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::American)
            .expiry(expiry)
            .contract_size(underlying.contract_size)
            .day_count(finstack_core::dates::DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new("EQUITY-VOL"))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
    }

    /// Create a new equity option using parameter structs
    pub fn new(
        id: impl Into<String>,
        option_params: &EquityOptionParams,
        underlying_params: &EquityUnderlyingParams,
        discount_curve_id: CurveId,
        vol_surface_id: CurveId,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            underlying_ticker: underlying_params.ticker.clone(),
            strike: option_params.strike,
            option_type: option_params.option_type,
            exercise_style: option_params.exercise_style,
            expiry: option_params.expiry,
            contract_size: option_params.contract_size,
            day_count: finstack_core::dates::DayCount::Act365F,
            settlement: option_params.settlement,
            discount_curve_id,
            spot_id: underlying_params.spot_id.to_owned(),
            vol_surface_id,
            div_yield_id: underlying_params.div_yield_id.to_owned(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Calculate Greeks for this equity option
    pub fn greeks(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<crate::instruments::equity_option::pricer::EquityOptionGreeks> {
        use crate::instruments::equity_option::pricer;
        pricer::compute_greeks(self, curves, as_of)
    }

    /// Calculate delta of this equity option
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.delta)
    }

    /// Calculate gamma of this equity option
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.gamma)
    }

    /// Calculate vega of this equity option
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.vega)
    }

    /// Calculate theta of this equity option
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.theta)
    }

    /// Calculate rho of this equity option
    pub fn rho(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, as_of)?;
        Ok(greeks.rho)
    }

    /// Calculate implied volatility of this equity option
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        market_price: f64,
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }
        if market_price <= 0.0 {
            return Ok(0.0);
        }
        if self.contract_size <= 0.0 {
            return Ok(0.0);
        }

        // Collect inputs except vol
        let (spot, r, q, _sigma, _t) = {
            use crate::instruments::equity_option::pricer;
            let (spot, r, q, sigma, t) = pricer::collect_inputs(self, curves, as_of)?;
            (spot, r, q, sigma, t)
        };
        let k = self.strike.amount();
        let target_unit = market_price / self.contract_size;
        Ok(crate::instruments::common::models::bs_implied_vol(
            spot,
            k,
            r,
            q,
            t,
            self.option_type,
            target_unit,
        ))
    }
}

impl crate::instruments::common::traits::OptionDeltaProvider for EquityOption {
    fn option_delta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks(market, as_of)?.delta)
    }
}

impl crate::instruments::common::traits::OptionGammaProvider for EquityOption {
    fn option_gamma(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks(market, as_of)?.gamma)
    }
}

impl crate::instruments::common::traits::OptionVegaProvider for EquityOption {
    fn option_vega(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks(market, as_of)?.vega)
    }
}

impl crate::instruments::common::traits::OptionThetaProvider for EquityOption {
    fn option_theta(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        Ok(self.greeks(market, as_of)?.theta)
    }
}

impl crate::instruments::common::traits::OptionRhoProvider for EquityOption {
    fn option_rho_bp(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        // EquityOptionGreeks::rho is per 1% rate move; metrics expose per 1bp.
        Ok(self.greeks(market, as_of)?.rho / 100.0)
    }
}

impl crate::instruments::common::traits::OptionVannaProvider for EquityOption {
    fn option_vanna(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        // Match the public metric test/reference conventions:
        // - Spot bump: ±1% (relative, on the spot scalar)
        // - Vol bump: ±1 vol point (absolute, parallel surface bump)
        let spot_scalar = market.price(&self.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
            finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
        };
        let spot_bump_abs = spot * crate::metrics::bump_sizes::SPOT;
        if spot_bump_abs <= 0.0 {
            return Ok(0.0);
        }

        let vol_bump_abs = crate::metrics::bump_sizes::VOLATILITY;

        let curves_vol_up = crate::metrics::bump_surface_vol_absolute(
            market,
            self.vol_surface_id.as_str(),
            vol_bump_abs,
        )?;
        let curves_vol_dn = crate::metrics::bump_surface_vol_absolute(
            market,
            self.vol_surface_id.as_str(),
            -vol_bump_abs,
        )?;

        // Delta at sigma+:
        let pv_su = self
            .value(
                &crate::metrics::bump_scalar_price(
                    &curves_vol_up,
                    &self.spot_id,
                    crate::metrics::bump_sizes::SPOT,
                )?,
                as_of,
            )?
            .amount();
        let pv_sd = self
            .value(
                &crate::metrics::bump_scalar_price(
                    &curves_vol_up,
                    &self.spot_id,
                    -crate::metrics::bump_sizes::SPOT,
                )?,
                as_of,
            )?
            .amount();
        let delta_up = (pv_su - pv_sd) / (2.0 * spot_bump_abs);

        // Delta at sigma-:
        let pv_su = self
            .value(
                &crate::metrics::bump_scalar_price(
                    &curves_vol_dn,
                    &self.spot_id,
                    crate::metrics::bump_sizes::SPOT,
                )?,
                as_of,
            )?
            .amount();
        let pv_sd = self
            .value(
                &crate::metrics::bump_scalar_price(
                    &curves_vol_dn,
                    &self.spot_id,
                    -crate::metrics::bump_sizes::SPOT,
                )?,
                as_of,
            )?
            .amount();
        let delta_dn = (pv_su - pv_sd) / (2.0 * spot_bump_abs);

        Ok((delta_up - delta_dn) / (2.0 * vol_bump_abs))
    }
}

impl crate::instruments::common::traits::OptionVolgaProvider for EquityOption {
    fn option_volga(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        base_pv: f64,
    ) -> finstack_core::Result<f64> {
        use crate::instruments::common::traits::Instrument;

        let vol_bump_abs = crate::metrics::bump_sizes::VOLATILITY;
        let curves_vol_up = crate::metrics::bump_surface_vol_absolute(
            market,
            self.vol_surface_id.as_str(),
            vol_bump_abs,
        )?;
        let curves_vol_dn = crate::metrics::bump_surface_vol_absolute(
            market,
            self.vol_surface_id.as_str(),
            -vol_bump_abs,
        )?;

        let pv_up = self.value(&curves_vol_up, as_of)?.amount();
        let pv_dn = self.value(&curves_vol_dn, as_of)?.amount();

        Ok((pv_up - 2.0 * base_pv + pv_dn) / (vol_bump_abs * vol_bump_abs))
    }
}

impl crate::instruments::common::traits::Instrument for EquityOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::EquityOption
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn spot_id(&self) -> Option<&str> {
        Some(&self.spot_id)
    }

    fn vol_surface_id(&self) -> Option<finstack_core::types::CurveId> {
        Some(self.vol_surface_id.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::equity_option::pricer;
        pricer::compute_pv(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use crate::instruments::equity_option::pricer;
    use crate::instruments::{
        common::traits::Attributes, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
    };
    use crate::test_utils::{date, flat_discount_with_tenor, flat_vol_surface};
    use finstack_core::{
        currency::Currency,
        dates::{Date, DayCount},
        market_data::{
            context::MarketContext, scalars::MarketScalar, term_structures::DiscountCurve,
        },
        money::Money,
        types::{CurveId, InstrumentId},
    };

    const DISC_ID: &str = "USD-OIS";
    const SPOT_ID: &str = "SPX-SPOT";
    const VOL_ID: &str = "SPX-VOL";
    const DIV_ID: &str = "SPX-DIV";

    fn build_market_context(
        as_of: Date,
        spot: f64,
        vol: f64,
        rate: f64,
        div_yield: f64,
    ) -> MarketContext {
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        MarketContext::new()
            .insert_discount(flat_discount_with_tenor(DISC_ID, as_of, rate, 5.0))
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, vol))
            .insert_price(SPOT_ID, MarketScalar::Unitless(spot))
            .insert_price(DIV_ID, MarketScalar::Unitless(div_yield))
    }

    fn base_option(expiry: Date) -> EquityOption {
        EquityOption::builder()
            .id(InstrumentId::new("EQ-OPT"))
            .underlying_ticker("SPX".to_string())
            .strike(Money::new(100.0, Currency::USD))
            .option_type(OptionType::Call)
            .exercise_style(ExerciseStyle::European)
            .expiry(expiry)
            .contract_size(100.0)
            .day_count(DayCount::Act365F)
            .settlement(SettlementType::Cash)
            .discount_curve_id(CurveId::new(DISC_ID))
            .spot_id(SPOT_ID.to_string())
            .vol_surface_id(CurveId::new(VOL_ID))
            .div_yield_id_opt(Some(DIV_ID.to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("should succeed")
    }

    fn approx_eq(actual: f64, expected: f64, tol: f64) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "expected {expected}, got {actual} (diff {diff} > {tol})"
        );
    }

    #[test]
    fn convenience_constructors_apply_standard_conventions() {
        let expiry = date(2025, 12, 31);
        let notional = Money::new(1_000_000.0, Currency::USD);
        let call = crate::test_utils::equity_option_european_call(
            "SPX-CALL", "SPX", 100.0, expiry, notional, 100.0,
        )
        .unwrap();
        assert_eq!(call.exercise_style, ExerciseStyle::European);
        assert_eq!(call.option_type, OptionType::Call);
        assert_eq!(call.discount_curve_id, CurveId::new(DISC_ID));
        assert_eq!(call.spot_id, "EQUITY-SPOT");
        assert_eq!(call.vol_surface_id, CurveId::new("EQUITY-VOL"));

        let put = crate::test_utils::equity_option_european_put(
            "SPX-PUT", "SPX", 90.0, expiry, notional, 50.0,
        )
        .unwrap();
        assert_eq!(put.option_type, OptionType::Put);
        assert_eq!(put.contract_size, 50.0);

        let american = crate::test_utils::equity_option_american_call(
            "SPX-AMER", "SPX", 105.0, expiry, notional, 75.0,
        )
        .unwrap();
        assert_eq!(american.exercise_style, ExerciseStyle::American);
        assert_eq!(american.contract_size, 75.0);
    }

    #[test]
    fn npv_and_greeks_match_pricer_outputs() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry);
        let curves = build_market_context(as_of, 105.0, 0.22, 0.03, 0.01);

        let price = option
            .value(&curves, as_of)
            .expect("NPV calculation should succeed in test");
        let (spot, r, q, sigma, t) = pricer::collect_inputs(&option, &curves, as_of)
            .expect("Input collection should succeed in test");
        let expected_unit = pricer::price_bs_unit(
            spot,
            option.strike.amount(),
            r,
            q,
            sigma,
            t,
            option.option_type,
        );
        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        approx_eq(price.amount(), expected_unit * option.contract_size, 5e-3);

        let greeks = option
            .greeks(&curves, as_of)
            .expect("Greeks calculation should succeed in test");
        let expected = pricer::compute_greeks(&option, &curves, as_of)
            .expect("Greeks computation should succeed in test");
        approx_eq(greeks.delta, expected.delta, 1e-6);
        approx_eq(greeks.gamma, expected.gamma, 1e-10);
        approx_eq(greeks.vega, expected.vega, 1e-6);
        approx_eq(greeks.theta, expected.theta, 1e-8);
        approx_eq(greeks.rho, expected.rho, 1e-6);

        approx_eq(
            option
                .delta(&curves, as_of)
                .expect("Delta calculation should succeed in test"),
            greeks.delta,
            1e-12,
        );
        approx_eq(
            option
                .gamma(&curves, as_of)
                .expect("Gamma calculation should succeed in test"),
            greeks.gamma,
            1e-12,
        );
        approx_eq(
            option
                .vega(&curves, as_of)
                .expect("Vega calculation should succeed in test"),
            greeks.vega,
            1e-12,
        );
        approx_eq(
            option
                .theta(&curves, as_of)
                .expect("Theta calculation should succeed in test"),
            greeks.theta,
            1e-12,
        );
        approx_eq(
            option
                .rho(&curves, as_of)
                .expect("Rho calculation should succeed in test"),
            greeks.rho,
            1e-12,
        );
    }

    #[test]
    fn implied_volatility_recovers_surface_value_and_respects_override() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry);
        let curves = build_market_context(as_of, 100.0, 0.30, 0.02, 0.01);

        let npv = option.value(&curves, as_of).expect("should succeed");
        let implied = option
            .implied_vol(&curves, as_of, npv.amount())
            .expect("should succeed");
        approx_eq(implied, 0.30, 1e-5);

        let mut override_option = base_option(expiry);
        let overrides = PricingOverrides {
            implied_volatility: Some(0.45),
            ..Default::default()
        };
        override_option.pricing_overrides = overrides;
        let override_price = override_option
            .value(&curves, as_of)
            .expect("should succeed");
        let (spot, r, q, _, t) =
            pricer::collect_inputs(&override_option, &curves, as_of).expect("should succeed");
        let expected = pricer::price_bs_unit(
            spot,
            override_option.strike.amount(),
            r,
            q,
            0.45,
            t,
            override_option.option_type,
        ) * override_option.contract_size;
        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        approx_eq(override_price.amount(), expected, 5e-3);
    }

    #[test]
    fn expired_options_return_intrinsic_value_and_static_greeks() {
        let expiry = date(2025, 1, 3);
        let as_of = expiry;
        let mut option = base_option(expiry);
        option.contract_size = 50.0;
        let curves = build_market_context(as_of, 120.0, 0.25, 0.01, 0.0);

        let pv = option.value(&curves, as_of).expect("should succeed");
        assert_eq!(pv.amount(), (120.0 - 100.0) * 50.0);

        let greeks = option.greeks(&curves, as_of).expect("should succeed");
        assert_eq!(greeks.delta, 50.0);
        assert_eq!(greeks.gamma, 0.0);
        assert_eq!(greeks.vega, 0.0);
        assert_eq!(greeks.theta, 0.0);
        assert_eq!(greeks.rho, 0.0);

        let implied = option
            .implied_vol(&curves, as_of, pv.amount())
            .expect("should succeed");
        assert_eq!(implied, 0.0);
    }

    /// Tests that separate day count handling works correctly when the discount curve
    /// uses ACT/360 (typical OIS convention) and volatility uses ACT/365F (equity standard).
    ///
    /// Market standard: Equity options use ACT/365F for vol time, but may discount on OIS
    /// curves with ACT/360. Mixing bases without proper separation rescales vols/rates
    /// and misstates greeks/theta.
    #[test]
    fn mixed_day_count_act365_vol_act360_discount() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2026, 1, 3); // 1 year expiry

        // Create an ACT/360 discount curve (typical OIS convention)
        let flat_rate: f64 = 0.05;
        let df_5y: f64 = (-flat_rate * 5.0).exp();
        let act360_curve = DiscountCurve::builder(DISC_ID)
            .base_date(as_of)
            .day_count(DayCount::Act360) // OIS convention
            .knots([(0.0, 1.0), (5.0, df_5y)])
            .build()
            .expect("DiscountCurve with ACT/360 should build successfully");

        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        let curves = MarketContext::new()
            .insert_discount(act360_curve)
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, 0.20))
            .insert_price(SPOT_ID, MarketScalar::Unitless(100.0))
            .insert_price(DIV_ID, MarketScalar::Unitless(0.02));

        let option = base_option(expiry);

        // Verify inputs are correctly separated
        let inputs = pricer::collect_inputs_extended(&option, &curves, as_of)
            .expect("collect_inputs_extended should succeed");

        // ACT/360: 365 days / 360 = 1.01389 years
        // ACT/365F: 365 days / 365 = 1.0 years
        let expected_t_rate = 365.0 / 360.0; // ACT/360 for rate
        let expected_t_vol = 365.0 / 365.0; // ACT/365F for vol

        approx_eq(inputs.t_rate, expected_t_rate, 1e-4);
        approx_eq(inputs.t_vol, expected_t_vol, 1e-4);

        // The difference between t_rate and t_vol should be non-trivial
        let time_diff = (inputs.t_rate - inputs.t_vol).abs();
        assert!(
            time_diff > 0.01,
            "t_rate and t_vol should differ significantly with ACT/360 vs ACT/365F: got {time_diff}"
        );

        // Verify pricing works and produces reasonable values
        let pv = option
            .value(&curves, as_of)
            .expect("NPV should succeed with mixed day counts");
        assert!(pv.amount() > 0.0, "Call option should have positive value");

        // Rate should be consistent with curve DF under t_vol
        let df_curve = curves
            .get_discount(DISC_ID)
            .expect("discount curve")
            .df(inputs.t_rate);
        let df_from_r = (-inputs.r * inputs.t_vol).exp();
        approx_eq(df_from_r, df_curve, 1e-10);

        // Verify greeks are computed correctly
        let greeks = option
            .greeks(&curves, as_of)
            .expect("Greeks should succeed with mixed day counts");
        assert!(greeks.delta > 0.0 && greeks.delta < option.contract_size);
        assert!(greeks.gamma > 0.0);
        assert!(greeks.vega > 0.0);

        // Verify price is within Black-Scholes tolerance
        // Using the inputs directly in the BS formula
        let bs_price = pricer::price_bs_unit(
            inputs.spot,
            option.strike.amount(),
            inputs.r,
            inputs.q,
            inputs.sigma,
            inputs.t_vol,
            option.option_type,
        ) * option.contract_size;

        // Slightly wider tolerance due to MonotoneConvex interpolation (vs Linear)
        // Same tolerance as other tests in this file
        approx_eq(pv.amount(), bs_price, 5e-3);
    }

    /// Tests that pricing fails with a clear error when div_yield_id is set but missing from
    /// the market context.
    ///
    /// This validates the fix for the silent fallback to 0.0 issue identified in the quant
    /// code review. Market data configuration errors should not be masked.
    #[test]
    fn pricing_fails_when_dividend_yield_missing() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);

        // Create option with div_yield_id that won't exist in market context
        let mut option = base_option(expiry);
        option.div_yield_id = Some("MISSING-DIV-YIELD".to_string());

        // Build market context WITHOUT the dividend yield
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        let curves = MarketContext::new()
            .insert_discount(flat_discount_with_tenor(DISC_ID, as_of, 0.03, 5.0))
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, 0.25))
            .insert_price(SPOT_ID, MarketScalar::Unitless(100.0));
        // Note: NOT inserting dividend yield

        // Pricing should fail with a validation error
        let result = option.value(&curves, as_of);
        assert!(
            result.is_err(),
            "Expected pricing to fail when div_yield_id is set but missing from market context"
        );

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("MISSING-DIV-YIELD") || err_msg.contains("dividend yield"),
            "Error message should mention the missing dividend yield ID, got: {}",
            err_msg
        );
    }

    /// Tests that pricing fails when div_yield_id returns a Price scalar instead of Unitless.
    ///
    /// Dividend yield should be a unitless decimal (e.g., 0.02 for 2%), not a price.
    /// This validates type safety in market data configuration.
    #[test]
    fn pricing_fails_when_dividend_yield_wrong_type() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let option = base_option(expiry);

        // Build market context with dividend yield as a Price instead of Unitless
        let expiries = [0.25, 0.5, 1.0, 2.0];
        let strikes = [80.0, 90.0, 100.0, 110.0, 120.0];
        let curves = MarketContext::new()
            .insert_discount(flat_discount_with_tenor(DISC_ID, as_of, 0.03, 5.0))
            .insert_surface(flat_vol_surface(VOL_ID, &expiries, &strikes, 0.25))
            .insert_price(SPOT_ID, MarketScalar::Unitless(100.0))
            // Wrong type: Price instead of Unitless
            .insert_price(DIV_ID, MarketScalar::Price(Money::new(0.02, Currency::USD)));

        // Pricing should fail with a validation error about wrong scalar type
        let result = option.value(&curves, as_of);
        assert!(
            result.is_err(),
            "Expected pricing to fail when div_yield_id returns Price instead of Unitless"
        );

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("unitless") || err_msg.contains("Price"),
            "Error message should mention the type mismatch, got: {}",
            err_msg
        );
    }

    #[test]
    fn bermudan_pricing_is_rejected_without_schedule() {
        let as_of = date(2025, 1, 3);
        let expiry = date(2025, 7, 3);
        let mut option = base_option(expiry);
        option.exercise_style = ExerciseStyle::Bermudan;
        let curves = build_market_context(as_of, 100.0, 0.25, 0.02, 0.01);

        let result = option.value(&curves, as_of);
        assert!(
            result.is_err(),
            "Expected Bermudan pricing to fail without exercise schedule"
        );

        let greeks = option.greeks(&curves, as_of);
        assert!(
            greeks.is_err(),
            "Expected Bermudan greeks to fail without exercise schedule"
        );
    }
}
