//! Quanto option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Quanto option instrument.
///
/// Quanto options have payoffs that depend on an underlying asset in one currency
/// but are settled in another currency, creating FX exposure.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct QuantoOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub equity_strike: Money,
    pub option_type: OptionType,
    pub expiry: Date,
    pub notional: Money,
    pub domestic_currency: Currency,
    pub foreign_currency: Currency,
    pub correlation: f64, // Correlation between equity and FX
    pub day_count: finstack_core::dates::DayCount,
    pub discount_curve_id: CurveId,
    pub spot_id: String,
    pub vol_surface_id: CurveId,
    pub div_yield_id: Option<String>,
    pub fx_rate_id: Option<String>,
    pub fx_vol_id: Option<CurveId>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::metrics::HasDiscountCurve for QuantoOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl QuantoOption {
    /// Create a canonical example quanto equity option (Nikkei in USD).
    pub fn example() -> Self {
        use finstack_core::dates::DayCount;
        use time::Month;
        QuantoOptionBuilder::new()
            .id(InstrumentId::new("QUANTO-NKY-USD-CALL"))
            .underlying_ticker("NKY".to_string())
            .equity_strike(Money::new(35000.0, Currency::JPY))
            .option_type(crate::instruments::OptionType::Call)
            .expiry(Date::from_calendar_date(2024, Month::December, 20).unwrap())
            .notional(Money::new(1_000_000.0, Currency::USD))
            .domestic_currency(Currency::USD)
            .foreign_currency(Currency::JPY)
            .correlation(-0.2)
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("NKY-SPOT".to_string())
            .vol_surface_id(CurveId::new("NKY-VOL"))
            .div_yield_id_opt(Some("NKY-DIV".to_string()))
            .fx_rate_id_opt(Some("USDJPY-SPOT".to_string()))
            .fx_vol_id_opt(Some(CurveId::new("USDJPY-VOL")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example QuantoOption construction should not fail")
    }
    /// Calculate the net present value using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::quanto_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate the net present value using analytical method (default).
    /// Uses quanto-adjusted Black-Scholes with correlation and FX vol.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::quanto_option::pricer::QuantoOptionAnalyticalPricer;
        use crate::pricer::Pricer;

        let pricer = QuantoOptionAnalyticalPricer::new();
        let result = pricer
            .price_dyn(self, curves, as_of)
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
        Ok(result.value)
    }
}

impl crate::instruments::common::traits::Instrument for QuantoOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::QuantoOption
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
        market: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Default to analytical pricing
        self.npv(market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::MarketContext,
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
        )
    }
}
