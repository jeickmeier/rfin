//! Barrier option instrument definition.

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Barrier type for barrier options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BarrierType {
    /// Up-and-out: option knocked out if S >= B
    UpAndOut,
    /// Up-and-in: option activated if S >= B
    UpAndIn,
    /// Down-and-out: option knocked out if S <= B
    DownAndOut,
    /// Down-and-in: option activated if S <= B
    DownAndIn,
}

/// Default for use_gobet_miri field.
///
/// Returns `true` to enable discrete barrier monitoring correction by default.
/// This matches the recommended production setting.
fn default_gobet_miri() -> bool {
    true
}

/// Barrier option instrument.
///
/// Barrier options are options with a barrier level that can knock in or out.
///
/// # Barrier Monitoring
///
/// Real-world barriers are typically monitored discretely (e.g., daily closes), not continuously.
/// Continuous barrier formulas underestimate discrete barrier option values. The `use_gobet_miri`
/// flag enables the Gobet-Miri discrete monitoring correction (β ≈ 0.5826), which adjusts the
/// effective barrier level: `H_adj = H × exp(±0.5826 × σ × √Δt)`.
///
/// **Recommendation**: Set `use_gobet_miri = true` (the default) for real-world pricing.
/// Only disable for continuous monitoring benchmarks or academic comparisons.
///
/// # References
///
/// - Broadie, Glasserman & Kou (1997), "A Continuity Correction for Discrete Barrier Options"
/// - Gobet (2000), "Weak Approximation of Killed Diffusion Using Euler Schemes"
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BarrierOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: String,
    /// Strike price
    pub strike: Money,
    /// Barrier level (price that triggers knock-in/out)
    pub barrier: Money,
    /// Optional rebate amount (paid at expiry if barrier condition met)
    pub rebate: Option<Money>,
    /// Option type (call or put)
    pub option_type: OptionType,
    /// Barrier type (up/down, in/out)
    pub barrier_type: BarrierType,
    /// Option expiry date
    pub expiry: Date,
    /// Notional amount
    pub notional: Money,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Whether to use Gobet-Miri discrete barrier adjustment for Monte Carlo pricing.
    ///
    /// When `true` (recommended), applies the Broadie-Glasserman-Kou / Gobet-Miri correction
    /// to account for discrete barrier monitoring. This adjusts the effective barrier by
    /// `exp(±0.5826 × σ × √Δt)` where Δt is the time step.
    ///
    /// # Default Value
    ///
    /// **Defaults to `true`** for both builder and serde deserialization, as this
    /// reflects real-world discrete monitoring (daily closes). Set to `false` only
    /// for continuous monitoring benchmarks or academic comparisons.
    ///
    /// # Production Recommendation
    ///
    /// Always use `true` for production pricing of barrier options. Continuous
    /// barrier formulas systematically underestimate discrete barrier option values.
    #[builder(default = default_gobet_miri())]
    #[cfg_attr(feature = "serde", serde(default = "default_gobet_miri"))]
    pub use_gobet_miri: bool,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Spot price identifier
    pub spot_id: String,
    /// Volatility surface ID
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<CurveId>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl BarrierOption {
    /// Create a canonical example barrier option (up-and-out call).
    ///
    /// Note: Uses `use_gobet_miri = true` by default for realistic discrete monitoring.
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::macros::date;
        BarrierOption::builder()
            .id(InstrumentId::new("BAR-SPX-UO-CALL"))
            .underlying_ticker("SPX".to_string())
            .strike(Money::new(4500.0, Currency::USD))
            .barrier(Money::new(5000.0, Currency::USD))
            .rebate(Money::new(50.0, Currency::USD))
            .option_type(crate::instruments::OptionType::Call)
            .barrier_type(BarrierType::UpAndOut)
            .expiry(date!(2024 - 12 - 20))
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .use_gobet_miri(true) // Enable discrete monitoring correction (recommended)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example BarrierOption with valid constants should never fail")
            })
    }

    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::exotics::barrier_option::pricer;
        pricer::compute_pv(self, curves, as_of)
    }
}

impl crate::instruments::common_impl::traits::Instrument for BarrierOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BarrierOption
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
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

    fn market_dependencies(
        &self,
    ) -> crate::instruments::common_impl::dependencies::MarketDependencies {
        crate::instruments::common_impl::dependencies::MarketDependencies::from_curves_and_equity(
            self,
        )
    }

    /// Compute the present value using the analytical (continuous monitoring) pricer.
    ///
    /// **Note**: This uses continuous monitoring Reiner-Rubinstein formulas regardless
    /// of the `use_gobet_miri` setting. For discrete-monitoring-corrected prices,
    /// use [`npv_mc()`](BarrierOption::npv_mc) instead.
    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::exotics::barrier_option::pricer::BarrierOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = BarrierOptionAnalyticalPricer::new();
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
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        None
    }
}
