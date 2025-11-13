//! Asian option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Averaging method for Asian options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AveragingMethod {
    /// Arithmetic average: (1/n) Σ S_i
    Arithmetic,
    /// Geometric average: (Π S_i)^(1/n)
    Geometric,
}

/// Asian option instrument.
///
/// Asian options depend on the average price over a period rather than
/// just the terminal price. Supports both call and put options with
/// arithmetic or geometric averaging.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AsianOption {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub strike: Money,
    pub option_type: OptionType,
    pub averaging_method: AveragingMethod,
    pub expiry: Date,
    pub fixing_dates: Vec<Date>,
    pub notional: Money,
    pub day_count: finstack_core::dates::DayCount,
    pub discount_curve_id: CurveId,
    pub spot_id: String,
    pub vol_surface_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::metrics::HasDiscountCurve for AsianOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl AsianOption {
    /// Create a canonical example Asian option (arithmetic average).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let fixing_dates = vec![
            Date::from_calendar_date(2024, Month::January, 31).unwrap(),
            Date::from_calendar_date(2024, Month::February, 29).unwrap(),
            Date::from_calendar_date(2024, Month::March, 31).unwrap(),
            Date::from_calendar_date(2024, Month::April, 30).unwrap(),
            Date::from_calendar_date(2024, Month::May, 31).unwrap(),
            Date::from_calendar_date(2024, Month::June, 30).unwrap(),
        ];
        AsianOptionBuilder::new()
            .id(InstrumentId::new("ASIAN-SPX-ARITH-6M"))
            .underlying_ticker("SPX".to_string())
            .strike(Money::new(4500.0, Currency::USD))
            .option_type(crate::instruments::OptionType::Call)
            .averaging_method(AveragingMethod::Arithmetic)
            .expiry(Date::from_calendar_date(2024, Month::June, 30).unwrap())
            .fixing_dates(fixing_dates)
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some("SPX-DIV".to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example AsianOption construction should not fail")
    }

    /// Calculate the net present value of this Asian option using Monte Carlo.
    #[cfg(feature = "mc")]
    pub fn npv_mc(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::asian_option::pricer;
        pricer::npv(self, curves, as_of)
    }

    /// Calculate the net present value using analytical method (default).
    /// Uses geometric closed-form for geometric averaging, Turnbull-Wakeman for arithmetic.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::asian_option::pricer::{
            AsianOptionAnalyticalGeometricPricer, AsianOptionSemiAnalyticalTwPricer,
        };
        use crate::pricer::Pricer;

        match self.averaging_method {
            AveragingMethod::Geometric => {
                let pricer = AsianOptionAnalyticalGeometricPricer::new();
                let result = pricer
                    .price_dyn(self, curves, as_of)
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Ok(result.value)
            }
            AveragingMethod::Arithmetic => {
                let pricer = AsianOptionSemiAnalyticalTwPricer::new();
                let result = pricer
                    .price_dyn(self, curves, as_of)
                    .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;
                Ok(result.value)
            }
        }
    }
}

impl crate::instruments::common::traits::Instrument for AsianOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::AsianOption
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
