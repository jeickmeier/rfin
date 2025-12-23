//! Barrier option instrument definition.

use crate::instruments::common::traits::Attributes;
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

/// Barrier option instrument.
///
/// Barrier options are options with a barrier level that can knock in or out.
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
    /// Whether to use Gobet-Miri continuous barrier adjustment
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
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        BarrierOptionBuilder::new()
            .id(InstrumentId::new("BAR-SPX-UO-CALL"))
            .underlying_ticker("SPX".to_string())
            .strike(Money::new(4500.0, Currency::USD))
            .barrier(Money::new(5000.0, Currency::USD))
            .rebate(Money::new(50.0, Currency::USD))
            .option_type(crate::instruments::OptionType::Call)
            .barrier_type(BarrierType::UpAndOut)
            .expiry(
                Date::from_calendar_date(2024, Month::December, 20).expect("Valid example date"),
            )
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .use_gobet_miri(false)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example BarrierOption construction should not fail")
    }

    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::barrier_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate the net present value using analytical method (default).
    /// Uses Reiner-Rubinstein continuous monitoring formulas.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::barrier_option::pricer::BarrierOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = BarrierOptionAnalyticalPricer::new();
        let result = pricer
            .price_dyn(self, curves, as_of)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Ok(result.value)
    }
}

impl crate::instruments::common::traits::Instrument for BarrierOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BarrierOption
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
        // Default to analytical pricing
        self.npv(market, as_of)
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
        )
    }
}
