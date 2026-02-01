//! Range accrual instrument definition.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId, Rate};

/// Specifies how the range bounds are interpreted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BoundsType {
    /// Bounds are absolute price levels (e.g., 4500.0 for SPX).
    /// This is the market standard for rate-linked range accruals.
    #[default]
    Absolute,
    /// Bounds are relative to initial spot (e.g., 0.95 = 95% of initial).
    /// Common for equity-linked range accruals.
    RelativeToInitialSpot,
}

/// Range accrual instrument.
///
/// Range accrual notes pay coupons that accrue only when a reference rate or asset
/// stays within a specified range. The accrual is proportional to the number of
/// observation dates where the underlying is within [lower_bound, upper_bound].
///
/// # Bounds Interpretation
///
/// The `bounds_type` field controls how `lower_bound` and `upper_bound` are interpreted:
/// - `Absolute`: Bounds are absolute price levels (e.g., 4500.0 for SPX at 4700)
/// - `RelativeToInitialSpot`: Bounds are multipliers of the initial spot (e.g., 0.95 = 95%)
///
/// # Historical Fixings
///
/// For mid-life valuations, use `past_fixings_in_range` to specify how many past
/// observations were in range. The pricer will add this to expected future fixings.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RangeAccrual {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: String,
    /// Observation dates for range checking (must be sorted ascending)
    pub observation_dates: Vec<Date>,
    /// Lower bound of accrual range (interpretation depends on bounds_type)
    pub lower_bound: f64,
    /// Upper bound of accrual range (must be > lower_bound)
    pub upper_bound: f64,
    /// How to interpret the range bounds (default: Absolute)
    #[builder(default)]
    pub bounds_type: BoundsType,
    /// Coupon rate earned when in range (must be >= 0)
    pub coupon_rate: f64,
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
    /// Optional Quanto correlation (Asset vs FX)
    pub quanto_correlation: Option<f64>,
    /// Optional FX volatility surface ID (required for Quanto)
    pub fx_vol_surface_id: Option<CurveId>,
    /// Optional FX spot price identifier (required for proper Quanto vol lookup)
    pub fx_spot_id: Option<String>,
    /// Optional payment date (defaults to last observation date)
    pub payment_date: Option<Date>,
    /// Number of past observations that were in range (for mid-life valuations).
    /// If None, past observations are not included in the accrual calculation.
    pub past_fixings_in_range: Option<usize>,
    /// Total number of past observations (for mid-life valuations).
    /// Must be provided if `past_fixings_in_range` is set.
    pub total_past_observations: Option<usize>,
}

impl RangeAccrual {
    /// Create a canonical example range accrual (monthly observations).
    ///
    /// This example uses relative bounds (95%-105% of initial spot) which is
    /// typical for equity-linked range accruals.
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let observation_dates = vec![
            Date::from_calendar_date(2024, Month::January, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::February, 29).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::March, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::April, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::May, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::June, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::July, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::August, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::September, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::October, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::November, 30).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::December, 31).expect("Valid example date"),
        ];
        RangeAccrualBuilder::new()
            .id(InstrumentId::new("RANGE-SPX-1Y"))
            .underlying_ticker("SPX".to_string())
            .observation_dates(observation_dates)
            .lower_bound(0.95) // 95% of initial spot
            .upper_bound(1.05) // 105% of initial spot
            .bounds_type(BoundsType::RelativeToInitialSpot)
            .coupon_rate(0.08) // 8% annual if inside range
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some("SPX-DIV".to_string()))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .quanto_correlation_opt(None)
            .fx_vol_surface_id_opt(None)
            .fx_spot_id_opt(None)
            .payment_date_opt(None)
            .past_fixings_in_range_opt(None)
            .total_past_observations_opt(None)
            .build()
            .expect("Example RangeAccrual construction should not fail")
    }

    /// Create an example with absolute bounds (typical for rate-linked range accruals).
    #[allow(clippy::expect_used)] // Example uses hardcoded valid values
    pub fn example_absolute_bounds() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        use time::Month;
        let observation_dates = vec![
            Date::from_calendar_date(2024, Month::January, 31).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::February, 29).expect("Valid example date"),
            Date::from_calendar_date(2024, Month::March, 31).expect("Valid example date"),
        ];
        RangeAccrualBuilder::new()
            .id(InstrumentId::new("RANGE-SOFR-3M"))
            .underlying_ticker("SOFR".to_string())
            .observation_dates(observation_dates)
            .lower_bound(0.04) // 4% lower bound
            .upper_bound(0.06) // 6% upper bound
            .bounds_type(BoundsType::Absolute)
            .coupon_rate(0.05) // 5% annual if inside range
            .notional(Money::new(1_000_000.0, Currency::USD))
            .day_count(DayCount::Act360)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SOFR-RATE".to_string())
            .vol_surface_id(CurveId::new("SOFR-VOL"))
            .div_yield_id_opt(None)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .quanto_correlation_opt(None)
            .fx_vol_surface_id_opt(None)
            .fx_spot_id_opt(None)
            .payment_date_opt(None)
            .past_fixings_in_range_opt(None)
            .total_past_observations_opt(None)
            .build()
            .expect("Example RangeAccrual construction should not fail")
    }

    /// Validate the range accrual parameters.
    ///
    /// Checks:
    /// - At least one observation date exists
    /// - Observation dates are sorted in ascending order
    /// - Lower bound is strictly less than upper bound
    /// - Coupon rate is non-negative
    /// - Quanto fields are consistent (if correlation is set, fx_vol_surface must be set)
    /// - Past fixing fields are consistent
    pub fn validate(&self) -> finstack_core::Result<()> {
        // Check observation dates
        if self.observation_dates.is_empty() {
            return Err(finstack_core::Error::Validation(
                "RangeAccrual requires at least one observation date".to_string(),
            ));
        }

        // Check observation dates are sorted
        for i in 1..self.observation_dates.len() {
            if self.observation_dates[i - 1] >= self.observation_dates[i] {
                return Err(finstack_core::Error::Validation(
                    "RangeAccrual observation_dates must be sorted in ascending order".to_string(),
                ));
            }
        }

        // Check bound ordering
        if self.lower_bound >= self.upper_bound {
            return Err(finstack_core::Error::Validation(format!(
                "RangeAccrual lower_bound ({}) must be strictly less than upper_bound ({})",
                self.lower_bound, self.upper_bound
            )));
        }

        // Check coupon rate
        if self.coupon_rate < 0.0 {
            return Err(finstack_core::Error::Validation(format!(
                "RangeAccrual coupon_rate ({}) must be non-negative",
                self.coupon_rate
            )));
        }

        // Check quanto field consistency
        if self.quanto_correlation.is_some() && self.fx_vol_surface_id.is_none() {
            return Err(finstack_core::Error::Validation(
                "RangeAccrual quanto_correlation requires fx_vol_surface_id to be set".to_string(),
            ));
        }

        // Check past fixing field consistency
        match (self.past_fixings_in_range, self.total_past_observations) {
            (Some(in_range), Some(total)) => {
                if in_range > total {
                    return Err(finstack_core::Error::Validation(format!(
                        "RangeAccrual past_fixings_in_range ({}) cannot exceed total_past_observations ({})",
                        in_range, total
                    )));
                }
            }
            (Some(_), None) => {
                return Err(finstack_core::Error::Validation(
                    "RangeAccrual past_fixings_in_range requires total_past_observations to be set"
                        .to_string(),
                ));
            }
            (None, Some(_)) => {
                return Err(finstack_core::Error::Validation(
                    "RangeAccrual total_past_observations requires past_fixings_in_range to be set"
                        .to_string(),
                ));
            }
            (None, None) => {} // Both unset is valid
        }

        Ok(())
    }

    /// Get the effective lower bound for a given initial spot.
    ///
    /// For `Absolute` bounds, returns the bound as-is.
    /// For `RelativeToInitialSpot`, returns `initial_spot * lower_bound`.
    pub fn effective_lower_bound(&self, initial_spot: f64) -> f64 {
        match self.bounds_type {
            BoundsType::Absolute => self.lower_bound,
            BoundsType::RelativeToInitialSpot => initial_spot * self.lower_bound,
        }
    }

    /// Get the effective upper bound for a given initial spot.
    ///
    /// For `Absolute` bounds, returns the bound as-is.
    /// For `RelativeToInitialSpot`, returns `initial_spot * upper_bound`.
    pub fn effective_upper_bound(&self, initial_spot: f64) -> f64 {
        match self.bounds_type {
            BoundsType::Absolute => self.upper_bound,
            BoundsType::RelativeToInitialSpot => initial_spot * self.upper_bound,
        }
    }
}

impl RangeAccrualBuilder {
    /// Set the coupon rate using a typed rate.
    pub fn coupon_rate_rate(mut self, rate: Rate) -> Self {
        self.coupon_rate = Some(rate.as_decimal());
        self
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

    fn market_dependencies(&self) -> crate::instruments::common::dependencies::MarketDependencies {
        crate::instruments::common::dependencies::MarketDependencies::from_curves_and_equity(self)
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.validate()?;
        #[cfg(feature = "mc")]
        {
            crate::instruments::range_accrual::pricer::compute_pv(self, market, as_of)
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        self.validate()?;
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

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for RangeAccrual {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
