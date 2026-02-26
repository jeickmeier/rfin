//! CMS option instrument definition.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// CMS option instrument (cap/floor on CMS rates).
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct CmsOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike (fixed rate for CMS option)
    #[serde(alias = "strike_rate")]
    pub strike: Decimal,
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
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    #[serde(default)]
    #[builder(default)]
    pub attributes: Attributes,
}

impl CmsOption {
    pub(crate) fn strike_f64(&self) -> finstack_core::Result<f64> {
        self.strike
            .to_f64()
            .ok_or(finstack_core::InputError::ConversionOverflow.into())
    }

    /// Create a CMS option from a schedule specification.
    ///
    /// Generates fixing and payment dates from `start_date`, `maturity`, and `frequency`
    /// using standard market conventions (Modified Following BDC, weekends-only calendar).
    /// This is the preferred way to construct standard CMS cap/floor instruments.
    ///
    /// # Parameters
    ///
    /// - `id` – instrument identifier
    /// - `start_date` – start of the first accrual period
    /// - `maturity` – end of the last accrual period
    /// - `frequency` – coupon/observation frequency (e.g. `Tenor::quarterly()`)
    /// - `cms_tenor` – tenor of the underlying swap in years (e.g. `10.0` for 10Y)
    /// - `strike` – option strike rate
    /// - `option_type` – cap (`Call`) or floor (`Put`)
    /// - `notional` – notional amount
    /// - `day_count` – day count for accrual fractions and vol interpolation
    /// - `swap_fixed_freq` – fixed-leg coupon frequency of the underlying CMS swap
    /// - `swap_float_freq` – floating-leg coupon frequency of the underlying CMS swap
    /// - `swap_day_count` – day count for the fixed leg of the underlying CMS swap
    /// - `discount_curve_id` – discount curve identifier
    /// - `forward_curve_id` – forward/projection curve identifier
    /// - `vol_surface_id` – volatility surface identifier
    ///
    /// # Errors
    ///
    /// Returns an error if the generated schedule is empty (e.g. `maturity <= start_date`).
    #[allow(clippy::too_many_arguments)]
    pub fn from_schedule(
        id: impl Into<InstrumentId>,
        start_date: Date,
        maturity: Date,
        frequency: Tenor,
        cms_tenor: f64,
        strike: Decimal,
        option_type: OptionType,
        notional: finstack_core::money::Money,
        day_count: finstack_core::dates::DayCount,
        swap_fixed_freq: Tenor,
        swap_float_freq: Tenor,
        swap_day_count: finstack_core::dates::DayCount,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> finstack_core::Result<Self> {
        use crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID;
        use crate::cashflow::builder::periods::{build_periods, BuildPeriodsParams};
        use finstack_core::dates::{BusinessDayConvention, StubKind};

        let periods = build_periods(BuildPeriodsParams {
            start: start_date,
            end: maturity,
            frequency,
            stub: StubKind::ShortFront,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: WEEKENDS_ONLY_ID,
            end_of_month: false,
            day_count,
            payment_lag_days: 0,
            reset_lag_days: None,
        })?;

        if periods.is_empty() {
            return Err(finstack_core::Error::Input(
                finstack_core::InputError::Invalid,
            ));
        }

        let fixing_dates: Vec<Date> = periods.iter().map(|p| p.accrual_start).collect();
        let payment_dates: Vec<Date> = periods.iter().map(|p| p.payment_date).collect();
        let accrual_fractions: Vec<f64> = periods.iter().map(|p| p.accrual_year_fraction).collect();

        CmsOption::builder()
            .id(id.into())
            .strike(strike)
            .cms_tenor(cms_tenor)
            .fixing_dates(fixing_dates)
            .payment_dates(payment_dates)
            .accrual_fractions(accrual_fractions)
            .option_type(option_type)
            .notional(notional)
            .day_count(day_count)
            .swap_fixed_freq(swap_fixed_freq)
            .swap_float_freq(swap_float_freq)
            .swap_day_count(swap_day_count)
            .discount_curve_id(discount_curve_id.into())
            .forward_curve_id(forward_curve_id.into())
            .vol_surface_id(vol_surface_id.into())
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))
    }

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
            .strike(Decimal::try_from(0.025).expect("valid decimal"))
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
    /// Set the strike using a typed rate.
    pub fn strike_rate(mut self, rate: Rate) -> Self {
        self.strike = Decimal::try_from(rate.as_decimal()).ok();
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

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
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
