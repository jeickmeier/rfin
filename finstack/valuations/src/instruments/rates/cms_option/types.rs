//! CMS option instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};

/// CMS option instrument (cap/floor on CMS rates).
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CmsOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike rate (fixed rate for CMS option)
    pub strike_rate: f64,
    /// Tenor of the CMS swap in years (e.g., 10.0 for 10Y)
    pub cms_tenor: f64,
    /// Observation/fixing dates for CMS rate
    pub fixing_dates: Vec<Date>,
    /// Payment dates for each period (usually fixing date + lag or period end)
    pub payment_dates: Vec<Date>,
    /// Accrual fractions for each period
    pub accrual_fractions: Vec<f64>,
    /// Option type (call or put on CMS rate)
    pub option_type: OptionType,
    /// Notional amount
    pub notional: Money,
    /// Day count convention for the option accrual
    pub day_count: finstack_core::dates::DayCount,

    // --- Underlying Swap Conventions ---
    /// Fixed leg frequency of the underlying swap
    pub swap_fixed_freq: finstack_core::dates::Tenor,
    /// Floating leg frequency of the underlying swap
    pub swap_float_freq: finstack_core::dates::Tenor,
    /// Day count convention of the underlying swap fixed leg
    pub swap_day_count: finstack_core::dates::DayCount,
    /// Optional day count convention of the underlying swap floating leg
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub swap_float_day_count: Option<finstack_core::dates::DayCount>,

    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Optional forward/projection curve ID (defaults to discount curve if not provided)
    pub forward_curve_id: Option<CurveId>,
    /// Optional volatility surface ID for CMS rates
    pub vol_surface_id: Option<CurveId>, // Optional volatility surface for CMS rates
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl CmsOption {
    /// Create a canonical example CMS option (10Y CMS caplet style).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::{DayCount, Tenor};
        use time::Month;

        let fixing_dates = vec![
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid example date"),
            Date::from_calendar_date(2025, Month::June, 20).expect("Valid example date"),
            Date::from_calendar_date(2025, Month::September, 22).expect("Valid example date"),
            Date::from_calendar_date(2025, Month::December, 22).expect("Valid example date"),
        ];
        let payment_dates = vec![
            Date::from_calendar_date(2025, Month::June, 20).expect("Valid example date"),
            Date::from_calendar_date(2025, Month::September, 22).expect("Valid example date"),
            Date::from_calendar_date(2025, Month::December, 22).expect("Valid example date"),
            Date::from_calendar_date(2026, Month::March, 20).expect("Valid example date"),
        ];
        let accrual_fractions = vec![0.25, 0.25, 0.25, 0.25];

        CmsOptionBuilder::new()
            .id(InstrumentId::new("CMSOPT-10Y-USD"))
            .strike_rate(0.025)
            .cms_tenor(10.0)
            .fixing_dates(fixing_dates)
            .payment_dates(payment_dates)
            .accrual_fractions(accrual_fractions)
            .option_type(crate::instruments::OptionType::Call)
            .notional(Money::new(10_000_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .swap_fixed_freq(Tenor::semi_annual())
            .swap_float_freq(Tenor::quarterly())
            .swap_day_count(DayCount::Thirty360)
            .swap_float_day_count_opt(Some(DayCount::Act360))
            .discount_curve_id(CurveId::new("USD-OIS"))
            .forward_curve_id(CurveId::new("USD-LIBOR-3M"))
            .vol_surface_id_opt(Some(CurveId::new("USD-CMS10Y-VOL")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example CmsOption construction should not fail")
    }
    /// Calculate the net present value of this CMS option.
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::cms_option::pricer;
        pricer::npv(self, curves, as_of)
    }
}

impl CmsOptionBuilder {
    /// Set the strike rate using a typed rate.
    pub fn strike_rate_rate(mut self, rate: Rate) -> Self {
        self.strike_rate = Some(rate.as_decimal());
        self
    }
}

impl crate::instruments::common::traits::Instrument for CmsOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CmsOption
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
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CmsOption {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for CmsOption {
    fn forward_curve_ids(&self) -> Vec<CurveId> {
        self.forward_curve_id.clone().into_iter().collect()
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for CmsOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        let mut builder = crate::instruments::common::traits::InstrumentCurves::builder();
        builder = builder.discount(self.discount_curve_id.clone());
        if let Some(fwd) = &self.forward_curve_id {
            builder = builder.forward(fwd.clone());
        }
        builder.build()
    }
}
