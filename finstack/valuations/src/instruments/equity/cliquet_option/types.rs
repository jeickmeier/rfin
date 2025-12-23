//! Cliquet option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Cliquet option instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CliquetOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: String,
    /// Reset dates for periodic return locking
    pub reset_dates: Vec<Date>,
    /// Local cap on individual period returns
    pub local_cap: f64,
    /// Local floor on individual period returns (default 0.0)
    pub local_floor: f64,
    /// Global cap on sum of all period returns
    pub global_cap: f64,
    /// Global floor on sum of all period returns (default 0.0)
    pub global_floor: f64,
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
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
    /// Payoff aggregation type (default: Additive)
    #[builder(default)]
    pub payoff_type: CliquetPayoffType,
}

/// Cliquet payoff aggregation type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CliquetPayoffType {
    /// Additive: Sum of period returns
    #[default]
    Additive,
    /// Multiplicative: Product of (1 + period returns) - 1
    Multiplicative,
}

// Implement HasDiscountCurve for GenericParallelDv01
impl crate::metrics::HasDiscountCurve for CliquetOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for CliquetOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl CliquetOption {
    /// Create a canonical example cliquet option (quarterly resets with local/global caps).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let reset_dates = vec![
            Date::from_calendar_date(2024, Month::March, 29).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::June, 28).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::September, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::December, 31).expect("Valid example date"),
        ];
        CliquetOptionBuilder::new()
            .id(InstrumentId::new("CLIQ-SPX-QTR"))
            .underlying_ticker("SPX".to_string())
            .reset_dates(reset_dates)
            .local_cap(0.05) // 5% per period
            .local_floor(0.0) // 0% per period (min)
            .global_cap(0.20) // 20% max cumulative
            .global_floor(0.0) // 0% min cumulative
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some("SPX-DIV".to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example CliquetOption construction should not fail")
    }
    /// Calculate the net present value of this cliquet option.
    #[cfg(feature = "mc")]
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::cliquet_option::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for CliquetOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CliquetOption
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
        #[cfg(feature = "mc")]
        {
            self.npv(market, as_of)
        }
        #[cfg(not(feature = "mc"))]
        {
            let _ = (market, as_of);
            Err(finstack_core::Error::Validation(
                "MC feature required for CliquetOption pricing".to_string(),
            ))
        }
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
