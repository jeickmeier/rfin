//! Range accrual instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// Range accrual instrument.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RangeAccrual {
    pub id: InstrumentId,
    pub underlying_ticker: String,
    pub observation_dates: Vec<Date>,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub coupon_rate: f64,
    pub notional: Money,
    pub day_count: finstack_core::dates::DayCount,
    pub discount_curve_id: CurveId,
    pub spot_id: String,
    pub vol_surface_id: CurveId,
    pub div_yield_id: Option<String>,
    pub pricing_overrides: PricingOverrides,
    pub attributes: Attributes,
}

impl RangeAccrual {
    /// Create a canonical example range accrual (monthly observations).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let observation_dates = vec![
            Date::from_calendar_date(2024, Month::January, 31).unwrap(),
            Date::from_calendar_date(2024, Month::February, 29).unwrap(),
            Date::from_calendar_date(2024, Month::March, 31).unwrap(),
            Date::from_calendar_date(2024, Month::April, 30).unwrap(),
            Date::from_calendar_date(2024, Month::May, 31).unwrap(),
            Date::from_calendar_date(2024, Month::June, 30).unwrap(),
            Date::from_calendar_date(2024, Month::July, 31).unwrap(),
            Date::from_calendar_date(2024, Month::August, 31).unwrap(),
            Date::from_calendar_date(2024, Month::September, 30).unwrap(),
            Date::from_calendar_date(2024, Month::October, 31).unwrap(),
            Date::from_calendar_date(2024, Month::November, 30).unwrap(),
            Date::from_calendar_date(2024, Month::December, 31).unwrap(),
        ];
        RangeAccrualBuilder::new()
            .id(InstrumentId::new("RANGE-SPX-1Y"))
            .underlying_ticker("SPX".to_string())
            .observation_dates(observation_dates)
            .lower_bound(0.95) // 95% of initial
            .upper_bound(1.05) // 105% of initial
            .coupon_rate(0.08) // 8% annual if inside range
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some("SPX-DIV".to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example RangeAccrual construction should not fail")
    }
    /// Calculate the net present value of this range accrual.
    #[cfg(feature = "mc")]
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::range_accrual::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for RangeAccrual {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::RangeAccrual
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
        #[cfg(feature = "mc")]
        {
            self.npv(market, as_of)
        }
        #[cfg(not(feature = "mc"))]
        {
            let _ = (market, as_of);
            Err(finstack_core::Error::Validation(
                "MC feature required for RangeAccrual pricing".to_string(),
            ))
        }
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

impl crate::instruments::common::pricing::HasDiscountCurve for RangeAccrual {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for RangeAccrual {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
