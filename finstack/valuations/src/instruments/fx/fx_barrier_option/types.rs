//! FX barrier option instrument definition.

use crate::instruments::barrier_option::types::BarrierType;
use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// FX barrier option instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FxBarrierOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike exchange rate
    pub strike: Money,
    /// Barrier level (exchange rate that triggers knock-in/out)
    pub barrier: Money,
    /// Option type (call or put on foreign currency)
    pub option_type: OptionType,
    /// Barrier type (up/down, in/out)
    pub barrier_type: BarrierType,
    /// Option expiry date
    pub expiry: Date,
    /// Notional amount in foreign currency
    pub notional: Money,
    /// Domestic currency (quote currency)
    pub domestic_currency: Currency,
    /// Foreign currency (base currency)
    pub foreign_currency: Currency,
    /// Day count convention
    pub day_count: finstack_core::dates::DayCount,
    /// Whether to use Gobet-Miri continuous barrier adjustment
    pub use_gobet_miri: bool,
    /// Domestic discount curve ID
    pub domestic_discount_curve_id: CurveId,
    /// Foreign discount curve ID
    pub foreign_discount_curve_id: CurveId,
    /// FX spot price identifier
    pub fx_spot_id: String,
    /// FX volatility surface ID
    pub fx_vol_id: CurveId,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::metrics::HasDiscountCurve for FxBarrierOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.domestic_discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for FxBarrierOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.domestic_discount_curve_id.clone())
            .build()
    }
}

impl FxBarrierOption {
    /// Create a canonical example FX barrier option (EURUSD up-and-out call).
    pub fn example() -> Self {
        use finstack_core::dates::DayCount;
        use time::Month;
        FxBarrierOptionBuilder::new()
            .id(InstrumentId::new("FXBAR-EURUSD-UO-CALL"))
            .strike(Money::new(1.10, Currency::USD))
            .barrier(Money::new(1.20, Currency::USD))
            .option_type(crate::instruments::OptionType::Call)
            .barrier_type(BarrierType::UpAndOut)
            .expiry(
                Date::from_calendar_date(2024, Month::December, 20).expect("Valid example date"),
            )
            .notional(Money::new(1_000_000.0, Currency::USD))
            .domestic_currency(Currency::USD)
            .foreign_currency(Currency::EUR)
            .day_count(DayCount::Act365F)
            .use_gobet_miri(false)
            .domestic_discount_curve_id(CurveId::new("USD-OIS"))
            .foreign_discount_curve_id(CurveId::new("EUR-OIS"))
            .fx_spot_id("EURUSD-SPOT".to_string())
            .fx_vol_id(CurveId::new("EURUSD-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example FxBarrierOption construction should not fail")
    }
    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::fx_barrier_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate the net present value using analytical method (default).
    /// Uses Reiner-Rubinstein continuous monitoring formulas with FX rate mapping.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::fx_barrier_option::pricer::FxBarrierOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = FxBarrierOptionAnalyticalPricer::new();
        let result = pricer
            .price_dyn(self, curves, as_of)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Ok(result.value)
    }
}

impl crate::instruments::common::traits::Instrument for FxBarrierOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::FxBarrierOption
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
