//! Basis swap implementation for multi-curve calibration.
//!
//! A basis swap exchanges two floating rate payments with different tenors,
//! capturing the basis spread between them (e.g., 3M vs 6M).
//!
//! # Shared Infrastructure
//!
//! This module delegates to the shared swap leg pricing infrastructure in
//! [`crate::instruments::common::pricing::swap_legs`] for robust discounting
//! and numerical stability.

#[allow(unused_imports)] // Used in doc examples and tests
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::{
    dates::{
        CalendarRegistry, Date, DateExt, DayCountCtx, HolidayCalendar, Schedule, ScheduleBuilder,
        StubKind,
    },
    market_data::context::MarketContext,
    money::Money,
    types::{CurveId, InstrumentId},
    Result,
};

// Import shared swap leg pricing utilities
use crate::instruments::common::pricing::swap_legs::{FloatingLegParams, LegPeriod};

// Re-export from common parameters
pub use crate::instruments::common::parameters::legs::BasisSwapLeg;

/// Basis swap instrument that exchanges two floating rate payments with different tenors.
///
/// A basis swap allows parties to exchange floating rate payments based on different
/// reference rates (e.g., 3M SOFR vs 6M SOFR) plus an optional spread on one leg.
/// The primary leg typically receives the spread, while the reference leg pays flat.
///
/// # Examples
/// ```rust
/// use finstack_core::{dates::*, money::Money, currency::Currency, types::CurveId};
/// use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
/// use time::Month;
///
/// let primary_leg = BasisSwapLeg {
///     forward_curve_id: CurveId::new("3M-SOFR"),
///     frequency: Tenor::quarterly(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     spread: 0.0005,
///     payment_lag_days: 0,
///     reset_lag_days: 0,
/// };
///
/// let reference_leg = BasisSwapLeg {
///     forward_curve_id: CurveId::new("6M-SOFR"),
///     frequency: Tenor::semi_annual(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     spread: 0.0,
///     payment_lag_days: 0,
///     reset_lag_days: 0,
/// };
///
/// let swap = BasisSwap::new(
///     "BASIS_SWAP_001",
///     Money::new(1_000_000.0, Currency::USD),
///     Date::from_calendar_date(2024, Month::January, 3).expect("valid date"),
///     Date::from_calendar_date(2025, Month::January, 3).expect("valid date"),
///     primary_leg,
///     reference_leg,
///     CurveId::new("OIS"),
/// );
/// ```
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct BasisSwap {
    /// Unique identifier for this instrument.
    pub id: InstrumentId,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Start date of the swap.
    pub start_date: Date,
    /// Maturity date of the swap.
    pub maturity_date: Date,
    /// Primary leg that typically receives the spread.
    pub primary_leg: BasisSwapLeg,
    /// Reference leg that typically pays flat.
    pub reference_leg: BasisSwapLeg,
    /// Discount curve identifier for present value calculations.
    pub discount_curve_id: CurveId,
    /// Optional calendar identifier for business day adjustments.
    pub calendar_id: Option<String>,
    /// Allow calendar-day fallback when the calendar cannot be resolved.
    ///
    /// When `false` (default), missing calendars are treated as an input error to
    /// avoid silently misaligning schedule and payment-lag conventions.
    #[serde(default)]
    pub allow_calendar_fallback: bool,
    /// Stub handling convention for irregular periods.
    pub stub_kind: StubKind,
    /// Attributes for instrument selection and tagging.
    pub attributes: crate::instruments::common::traits::Attributes,
}

impl BasisSwap {
    /// Creates a new basis swap with the specified parameters.
    ///
    /// # Arguments
    /// * `id` — Unique identifier for the swap
    /// * `notional` — Notional amount for both legs
    /// * `start_date` — Start date of the swap
    /// * `maturity_date` — Maturity date of the swap
    /// * `primary_leg` — Primary leg specification (typically receives spread)
    /// * `reference_leg` — Reference leg specification (typically pays flat)
    /// * `discount_curve_id` — Discount curve identifier for present value calculations
    ///
    /// # Returns
    /// A new `BasisSwap` instance with default calendar and stub settings.
    pub fn new(
        id: impl Into<String>,
        notional: Money,
        start_date: Date,
        maturity_date: Date,
        primary_leg: BasisSwapLeg,
        reference_leg: BasisSwapLeg,
        discount_curve_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: InstrumentId::new(id.into()),
            notional,
            start_date,
            maturity_date,
            primary_leg,
            reference_leg,
            discount_curve_id: discount_curve_id.into(),
            calendar_id: None,
            allow_calendar_fallback: false,
            stub_kind: StubKind::None,
            attributes: crate::instruments::common::traits::Attributes::default(),
        }
    }

    /// Sets the calendar for business day adjustments.
    ///
    /// # Arguments
    /// * `calendar_id` — Calendar identifier for date adjustments
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_calendar(mut self, calendar_id: impl Into<String>) -> Self {
        self.calendar_id = Some(calendar_id.into());
        self
    }

    /// Allow (or disallow) calendar-day fallback when the calendar cannot be resolved.
    ///
    /// When enabled, schedule generation skips business-day adjustment and payment lags
    /// are applied as calendar days. Use this only for exploratory/testing workflows.
    pub fn with_allow_calendar_fallback(mut self, allow: bool) -> Self {
        self.allow_calendar_fallback = allow;
        self
    }

    /// Sets the stub handling convention for irregular periods.
    ///
    /// # Arguments
    /// * `stub_kind` — Stub handling convention
    ///
    /// # Returns
    /// Self for method chaining.
    pub fn with_stub(mut self, stub_kind: StubKind) -> Self {
        self.stub_kind = stub_kind;
        self
    }

    fn resolve_calendar(&self) -> Result<Option<&'static dyn HolidayCalendar>> {
        match self.calendar_id.as_deref() {
            Some(id) => {
                if let Some(cal) = CalendarRegistry::global().resolve_str(id) {
                    Ok(Some(cal))
                } else if self.allow_calendar_fallback {
                    tracing::warn!(
                        instrument_id = %self.id.as_str(),
                        calendar_id = id,
                        "Calendar not found; falling back to unadjusted schedule and calendar-day lags"
                    );
                    Ok(None)
                } else {
                    Err(finstack_core::Error::Input(
                        finstack_core::InputError::NotFound {
                            id: format!("calendar '{}'", id),
                        },
                    ))
                }
            }
            None => {
                if self.allow_calendar_fallback {
                    tracing::warn!(
                        instrument_id = %self.id.as_str(),
                        "No calendar_id set; falling back to unadjusted schedule and calendar-day lags"
                    );
                    Ok(None)
                } else {
                    Err(finstack_core::Error::Input(
                        finstack_core::InputError::NotFound {
                            id: "BasisSwap calendar_id".to_string(),
                        },
                    ))
                }
            }
        }
    }

    /// Builds a period schedule for the specified leg using shared schedule utilities.
    ///
    /// # Arguments
    /// * `leg` — The leg to build a schedule for
    ///
    /// # Returns
    /// A `Schedule` containing the accrual boundary dates for the leg.
    pub fn leg_schedule(&self, leg: &BasisSwapLeg) -> Result<Schedule> {
        let cal = self.resolve_calendar()?;

        let mut builder = ScheduleBuilder::try_new(self.start_date, self.maturity_date)?
            .frequency(leg.frequency)
            .stub_rule(self.stub_kind);

        if let Some(cal) = cal {
            builder = builder.adjust_with(leg.bdc, cal);
        }

        builder.build()
    }

    /// Calculates the present value of a floating rate leg.
    ///
    /// This method uses the shared swap leg pricing infrastructure for
    /// robust discounting and numerical stability (Kahan summation).
    ///
    /// # Arguments
    /// * `leg` — The leg specification
    /// * `schedule` — Period schedule for the leg
    /// * `context` — Market context containing curves and rates
    /// * `valuation_date` — Date for present value calculation
    ///
    /// # Returns
    /// The present value of the floating leg as a `Money` amount.
    pub fn pv_float_leg(
        &self,
        leg: &BasisSwapLeg,
        schedule: &Schedule,
        context: &MarketContext,
        valuation_date: Date,
    ) -> Result<Money> {
        if schedule.dates.len() < 2 {
            return Err(finstack_core::Error::Validation(
                "BasisSwap leg schedule must contain at least 2 dates".to_string(),
            ));
        }

        if leg.payment_lag_days < 0 || leg.reset_lag_days < 0 {
            return Err(finstack_core::Error::Validation(
                "BasisSwap leg lags must be non-negative".to_string(),
            ));
        }

        // Get curves
        let disc = context.get_discount(&self.discount_curve_id)?;
        let fwd = context.get_forward(&leg.forward_curve_id)?;
        let cal = self.resolve_calendar()?;

        let currency = self.notional.currency();
        let dc_ctx = DayCountCtx::default();

        // Build periods from schedule
        let mut periods = Vec::with_capacity(schedule.dates.len() - 1);
        for i in 1..schedule.dates.len() {
            let period_start = schedule.dates[i - 1];
            let period_end = schedule.dates[i];

            let payment_date = if leg.payment_lag_days == 0 {
                period_end
            } else if let Some(cal) = cal {
                period_end.add_business_days(leg.payment_lag_days, cal)?
            } else {
                period_end + time::Duration::days(leg.payment_lag_days as i64)
            };

            // Skip past periods
            if payment_date <= valuation_date {
                continue;
            }

            // Calculate reset date
            let reset_date = if leg.reset_lag_days == 0 {
                period_start
            } else if let Some(cal) = cal {
                period_start.add_business_days(-leg.reset_lag_days, cal)?
            } else {
                period_start - time::Duration::days(leg.reset_lag_days as i64)
            };

            // Year fraction for accrual
            let year_frac = leg
                .day_count
                .year_fraction(period_start, period_end, dc_ctx)?;

            periods.push(LegPeriod {
                accrual_start: period_start,
                accrual_end: period_end,
                reset_date: Some(reset_date),
                year_fraction: year_frac,
            });
        }

        // Build floating leg params - spread is in decimal form for BasisSwap
        let params = FloatingLegParams::full(
            leg.spread * 10000.0, // spread_bp - convert decimal to bp for shared function
            1.0,                  // gearing
            true,                 // gearing_includes_spread
            None,                 // index_floor_bp
            None,                 // index_cap_bp
            None,                 // all_in_floor_bp
            None,                 // all_in_cap_bp
            leg.payment_lag_days, // payment_delay_days
            self.calendar_id.clone(),
        );

        // Use shared pricing function
        let pv = crate::instruments::common::pricing::swap_legs::pv_floating_leg(
            periods.into_iter(),
            self.notional.amount(),
            &params,
            disc.as_ref(),
            fwd.as_ref(),
            valuation_date,
        )?;

        Ok(Money::new(pv, currency))
    }

    /// Calculates the discounted accrual sum (annuity) for a leg.
    ///
    /// This method computes the sum of discounted year fractions for a leg,
    /// which is useful for DV01 calculations and par spread computations.
    /// Uses the shared swap leg pricing infrastructure for robust discounting.
    ///
    /// # Arguments
    /// * `leg` — The leg specification
    /// * `schedule` — Period schedule for the leg
    /// * `curves` — Market context containing the discount curve
    /// * `as_of` — Valuation date for discounting
    ///
    /// # Returns
    /// The discounted accrual sum as a floating point value.
    pub fn annuity_for_leg(
        &self,
        leg: &BasisSwapLeg,
        schedule: &Schedule,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        if schedule.dates.len() < 2 {
            return Err(finstack_core::Error::Validation(
                "BasisSwap leg schedule must contain at least 2 dates".to_string(),
            ));
        }

        if leg.payment_lag_days < 0 {
            return Err(finstack_core::Error::Validation(
                "BasisSwap payment lag must be non-negative".to_string(),
            ));
        }

        let disc = curves.get_discount(&self.discount_curve_id)?;

        // Build periods from schedule
        let mut periods = Vec::with_capacity(schedule.dates.len() - 1);
        let mut prev = schedule.dates[0];

        for &d in &schedule.dates[1..] {
            // Calculate year fraction for the period
            let yf = leg
                .day_count
                .year_fraction(prev, d, DayCountCtx::default())?;

            periods.push(LegPeriod {
                accrual_start: prev,
                accrual_end: d,
                reset_date: None,
                year_fraction: yf,
            });

            prev = d;
        }

        // Use shared annuity function with robust discounting
        crate::instruments::common::pricing::swap_legs::leg_annuity(
            periods.into_iter(),
            disc.as_ref(),
            as_of,
            leg.payment_lag_days,
            self.calendar_id.as_deref(),
        )
    }

    /// Compute the net present value (NPV) of the basis swap.
    ///
    /// # Arguments
    /// * `curves` — Market context containing all necessary curves
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// The NPV as the difference between primary and reference leg PVs.
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> Result<Money> {
        // Build schedules
        let primary_schedule = self.leg_schedule(&self.primary_leg)?;
        let reference_schedule = self.leg_schedule(&self.reference_leg)?;

        // Calculate PV for each leg
        let primary_pv = self.pv_float_leg(&self.primary_leg, &primary_schedule, curves, as_of)?;
        let reference_pv =
            self.pv_float_leg(&self.reference_leg, &reference_schedule, curves, as_of)?;

        // Return net PV
        primary_pv - reference_pv
    }
}

// Attributable implementation is provided by the impl_instrument! macro

// Use the macro to implement Instrument with pricing
impl crate::instruments::common::traits::Instrument for BasisSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BasisSwap
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
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for BasisSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

impl crate::instruments::common::pricing::HasForwardCurves for BasisSwap {
    fn forward_curve_ids(&self) -> Vec<finstack_core::types::CurveId> {
        vec![
            self.primary_leg.forward_curve_id.clone(),
            self.reference_leg.forward_curve_id.clone(),
        ]
    }
}

impl crate::instruments::common::traits::CurveDependencies for BasisSwap {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.primary_leg.forward_curve_id.clone())
            .forward(self.reference_leg.forward_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
    use time::Month;

    // Helper function for tests
    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid date"), day)
            .expect("should succeed")
    }

    #[test]
    fn test_basis_swap_pricing() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 1, 3);
        let maturity = date(2025, 1, 3);

        // Create test curves with flat rates
        let discount_curve = DiscountCurve::builder("OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
            .build()
            .expect("should succeed");

        let forward_3m = ForwardCurve::builder("3M-SOFR", 0.25)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03), (2.0, 0.03)])
            .build()
            .expect("should succeed");

        let forward_6m = ForwardCurve::builder("6M-SOFR", 0.5)
            .base_date(base_date)
            .knots(vec![(0.0, 0.0305), (1.0, 0.0305), (2.0, 0.0305)])
            .build()
            .expect("should succeed");

        // Create context
        let context = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_3m)
            .insert_forward(forward_6m);

        // Create basis swap: 3M receives 6M + 5bp
        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("3M-SOFR"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0005, // 5bp
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("6M-SOFR"),
            frequency: Tenor::semi_annual(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let swap = BasisSwap::new(
            "TEST_BASIS",
            Money::new(1_000_000.0, Currency::USD),
            start_date,
            maturity,
            primary_leg,
            reference_leg,
            CurveId::new("OIS"),
        )
        .with_calendar("usny");

        // Price the swap
        let pv = swap.value(&context, base_date).expect("should succeed");

        // The PV should be close to zero if the spread correctly prices the basis
        assert!(
            pv.amount().abs() < 1000.0,
            "PV should be small: {}",
            pv.amount()
        );
    }

    #[test]
    fn test_basis_swap_requires_calendar_by_default() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 1, 3);
        let maturity = date(2025, 1, 3);

        let discount_curve = DiscountCurve::builder("OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.99)])
            .build()
            .expect("should succeed");
        let forward_3m = ForwardCurve::builder("3M-SOFR", 0.25)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03)])
            .build()
            .expect("should succeed");
        let forward_6m = ForwardCurve::builder("6M-SOFR", 0.5)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03)])
            .build()
            .expect("should succeed");

        let context = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_3m)
            .insert_forward(forward_6m);

        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("3M-SOFR"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };
        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("6M-SOFR"),
            frequency: Tenor::semi_annual(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let swap = BasisSwap::new(
            "TEST_BASIS_NO_CAL",
            Money::new(1_000_000.0, Currency::USD),
            start_date,
            maturity,
            primary_leg,
            reference_leg,
            CurveId::new("OIS"),
        );

        let err = swap.value(&context, base_date).expect_err("should fail");
        assert!(
            format!("{err}").contains("calendar"),
            "Expected calendar error, got: {err}"
        );
    }

    #[test]
    fn test_basis_swap_payment_lag_affects_pv() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 1, 3);
        let maturity = date(2025, 1, 3);

        // Use a steep-ish discount curve so payment timing matters.
        let discount_curve = DiscountCurve::builder("OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.90), (2.0, 0.82)])
            .build()
            .expect("should succeed");

        let forward_3m = ForwardCurve::builder("3M-SOFR", 0.25)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03), (2.0, 0.03)])
            .build()
            .expect("should succeed");
        let forward_6m = ForwardCurve::builder("6M-SOFR", 0.5)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03), (2.0, 0.03)])
            .build()
            .expect("should succeed");

        let context = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_3m)
            .insert_forward(forward_6m);

        let primary_leg_no_lag = BasisSwapLeg {
            forward_curve_id: CurveId::new("3M-SOFR"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0010,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };
        let primary_leg_with_lag = BasisSwapLeg {
            payment_lag_days: 10,
            ..primary_leg_no_lag.clone()
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("6M-SOFR"),
            frequency: Tenor::semi_annual(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
            payment_lag_days: 0,
            reset_lag_days: 0,
        };

        let swap_no_lag = BasisSwap::new(
            "TEST_BASIS_NO_LAG",
            Money::new(10_000_000.0, Currency::USD),
            start_date,
            maturity,
            primary_leg_no_lag,
            reference_leg.clone(),
            CurveId::new("OIS"),
        )
        .with_calendar("usny");

        let swap_with_lag = BasisSwap::new(
            "TEST_BASIS_WITH_LAG",
            Money::new(10_000_000.0, Currency::USD),
            start_date,
            maturity,
            primary_leg_with_lag,
            reference_leg,
            CurveId::new("OIS"),
        )
        .with_calendar("usny");

        let pv_no_lag = swap_no_lag
            .value(&context, base_date)
            .expect("should succeed");
        let pv_with_lag = swap_with_lag
            .value(&context, base_date)
            .expect("should succeed");

        assert!(
            (pv_no_lag.amount() - pv_with_lag.amount()).abs() > 1e-6,
            "Expected payment lag to change PV: no_lag={}, with_lag={}",
            pv_no_lag.amount(),
            pv_with_lag.amount()
        );
    }
}
