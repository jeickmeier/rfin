//! CMS option instrument definition.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};

/// CMS option instrument (cap/floor on CMS rates).
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    pub day_count: DayCount,

    // --- Underlying Swap Conventions ---
    /// Fixed leg frequency of the underlying swap
    pub swap_fixed_freq: Tenor,
    /// Floating leg frequency of the underlying swap
    pub swap_float_freq: Tenor,
    /// Day count convention of the underlying swap fixed leg
    pub swap_day_count: DayCount,
    /// Optional day count convention of the underlying swap floating leg
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swap_float_day_count: Option<DayCount>,

    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Forward/projection curve ID for CMS rate projection
    pub forward_curve_id: CurveId,
    /// Volatility surface ID for CMS rates
    pub vol_surface_id: CurveId,
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

        CmsOption::builder()
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
            .vol_surface_id(CurveId::new("USD-CMS10Y-VOL"))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .expect("Example CmsOption construction should not fail")
    }
}

impl CmsOptionBuilder {
    /// Set the strike rate using a typed rate.
    pub fn strike_rate_rate(mut self, rate: Rate) -> Self {
        self.strike_rate = Some(rate.as_decimal());
        self
    }
}

impl crate::instruments::common_impl::traits::Instrument for CmsOption {
    impl_instrument_base!(crate::pricer::InstrumentType::CmsOption);

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::rates::cms_option::pricer::compute_pv(self, market, as_of)
    }

    fn effective_start_date(&self) -> Option<Date> {
        self.fixing_dates.first().copied()
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for CmsOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        let mut builder = crate::instruments::common_impl::traits::InstrumentCurves::builder();
        builder = builder.discount(self.discount_curve_id.clone());
        builder = builder.forward(self.forward_curve_id.clone());
        builder.build()
    }
}
