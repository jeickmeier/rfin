//! Lookback option instrument definition.
//!
//! # Monitoring Convention
//!
//! **Important**: This implementation uses **continuous monitoring** formulas
//! (Goldman-Sosin-Gatto closed-form solutions). Real-world lookback options are
//! typically monitored discretely (e.g., daily closes), which affects pricing.
//!
//! ## Continuous vs Discrete Monitoring
//!
//! | Aspect | Continuous | Discrete |
//! |--------|------------|----------|
//! | Monitoring | Every instant | At specific times (e.g., daily) |
//! | Pricing | Analytical formulas | Monte Carlo or numerical |
//! | Value | Higher (more observations) | Lower (fewer opportunities) |
//! | Greeks | Closed-form | Numerical |
//!
//! ## Discrete Monitoring Adjustment
//!
//! For daily-monitored lookbacks, the continuous price can be adjusted using
//! the Broadie-Glasserman-Kou correction factor:
//!
//! ```text
//! M_discrete ≈ M_continuous × exp(0.5826 × σ × √Δt)  [for max]
//! m_discrete ≈ m_continuous × exp(-0.5826 × σ × √Δt) [for min]
//! ```
//!
//! where:
//! - `σ` = volatility
//! - `Δt` = monitoring interval (e.g., 1/252 for daily)
//! - `0.5826 = -ζ(1/2)/√(2π)` (Riemann zeta constant)
//!
//! ## Production Recommendation
//!
//! For production pricing of discretely-monitored lookbacks, use Monte Carlo
//! simulation (`npv_mc`) with the actual monitoring dates rather than the
//! continuous analytical formulas.
//!
//! # References
//!
//! - Goldman, M. B., Sosin, H. B., & Gatto, M. A. (1979). "Path Dependent Options."
//! - Broadie, M., Glasserman, P., & Kou, S. G. (1997). "A Continuity Correction
//!   for Discrete Barrier Options."

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Lookback option type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LookbackType {
    /// Fixed strike lookback: payoff depends on max/min relative to fixed strike
    FixedStrike,
    /// Floating strike lookback: strike is determined by path extremum
    FloatingStrike,
}

/// Lookback option instrument.
///
/// # Monitoring Convention
///
/// This instrument uses **continuous monitoring** for analytical pricing. Real-world
/// lookback options are typically monitored discretely (daily closes). The continuous
/// formulas provide an upper bound; for accurate discrete pricing, use Monte Carlo.
///
/// See module-level documentation for details on discrete monitoring adjustments.
///
/// # Observed Extrema
///
/// For seasoned options (where some monitoring has already occurred), provide:
/// - `observed_min`: Minimum spot observed so far (for floating calls / fixed puts)
/// - `observed_max`: Maximum spot observed so far (for floating puts / fixed calls)
///
/// If not provided, the current spot is used as the starting extremum.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LookbackOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: String,
    /// Strike price (None for floating strike lookbacks)
    pub strike: Option<Money>, // None for floating strike
    /// Option type (call or put)
    pub option_type: OptionType,
    /// Lookback type (fixed or floating strike)
    pub lookback_type: LookbackType,
    /// Option expiry date
    pub expiry: Date,
    /// Notional amount
    pub notional: Money,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Spot price identifier
    pub spot_id: String,
    /// Volatility surface ID
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<String>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Observed minimum spot price since inception (required for Floating Call / Fixed Put)
    pub observed_min: Option<Money>,
    /// Observed maximum spot price since inception (required for Floating Put / Fixed Call)
    pub observed_max: Option<Money>,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::instruments::common::pricing::HasDiscountCurve for LookbackOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for LookbackOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl LookbackOption {
    /// Create a canonical example lookback option (fixed strike call).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::macros::date;
        LookbackOptionBuilder::new()
            .id(InstrumentId::new("LOOKBACK-SPX-FIXED-CALL"))
            .underlying_ticker("SPX".to_string())
            .strike_opt(Some(Money::new(4500.0, Currency::USD)))
            .option_type(crate::instruments::OptionType::Call)
            .lookback_type(LookbackType::FixedStrike)
            .expiry(date!(2024 - 12 - 20))
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some("SPX-DIV".to_string()))
            .pricing_overrides(PricingOverrides::default())
            .observed_min_opt(None)
            .observed_max_opt(None)
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example LookbackOption with valid constants should never fail")
            })
    }
    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::lookback_option::pricer;
        pricer::compute_pv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for LookbackOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::LookbackOption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
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

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::lookback_option::pricer::LookbackOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = LookbackOptionAnalyticalPricer::new();
        let result = pricer
            .price_dyn(self, market, as_of)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Ok(result.value)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }
}
