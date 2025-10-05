//! Basis swap implementation for multi-curve calibration.
//!
//! A basis swap exchanges two floating rate payments with different tenors,
//! capturing the basis spread between them (e.g., 3M vs 6M).

use crate::cashflow::builder::schedule_utils::{build_dates, PeriodSchedule};
#[allow(unused_imports)] // Used in doc examples and tests
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency};
use finstack_core::{
    dates::{Date, DayCountCtx, StubKind},
    market_data::MarketContext,
    money::Money,
    types::{CurveId, InstrumentId},
    Result,
};

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
///     frequency: Frequency::quarterly(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     spread: 0.0005,
/// };
///
/// let reference_leg = BasisSwapLeg {
///     forward_curve_id: CurveId::new("6M-SOFR"),
///     frequency: Frequency::semi_annual(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     spread: 0.0,
/// };
///
/// let swap = BasisSwap::new(
///     "BASIS_SWAP_001",
///     Money::new(1_000_000.0, Currency::USD),
///     Date::from_calendar_date(2024, Month::January, 3).unwrap(),
///     Date::from_calendar_date(2025, Month::January, 3).unwrap(),
///     primary_leg,
///     reference_leg,
///     CurveId::new("OIS"),
/// );
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
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
    pub calendar_id: Option<&'static str>,
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
    pub fn with_calendar(mut self, calendar_id: &'static str) -> Self {
        self.calendar_id = Some(calendar_id);
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

    /// Builds a period schedule for the specified leg using shared schedule utilities.
    ///
    /// # Arguments
    /// * `leg` — The leg to build a schedule for
    ///
    /// # Returns
    /// A `PeriodSchedule` containing the payment dates for the leg.
    pub fn leg_schedule(&self, leg: &BasisSwapLeg) -> PeriodSchedule {
        build_dates(
            self.start_date,
            self.maturity_date,
            leg.frequency,
            self.stub_kind,
            leg.bdc,
            self.calendar_id,
        )
    }

    /// Calculates the present value of a floating rate leg.
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
        schedule: &PeriodSchedule,
        context: &MarketContext,
        valuation_date: Date,
    ) -> Result<Money> {
        // Get curves
        let disc = context.get_discount_ref(self.discount_curve_id.clone())?;
        let fwd = context.get_forward_ref(leg.forward_curve_id.clone())?;

        let mut pv = 0.0;
        let currency = self.notional.currency();
        let dc_ctx = DayCountCtx::default();

        // Iterate periods
        for i in 1..schedule.dates.len() {
            let period_start = schedule.dates[i - 1];
            let period_end = schedule.dates[i];

            // Skip past periods
            if period_end <= valuation_date {
                continue;
            }

            // Forward rate for the accrual period using the forward curve's own time basis
            let fwd_dc = fwd.day_count();
            let fwd_base = fwd.base_date();
            let t_start = fwd_dc.year_fraction(fwd_base, period_start, dc_ctx)?;
            let t_end = fwd_dc.year_fraction(fwd_base, period_end, dc_ctx)?;
            let forward_rate = fwd.rate_period(t_start, t_end);

            // Total rate (add spread)
            let total_rate = forward_rate + leg.spread;

            // Accrual year fraction
            let year_frac = leg
                .day_count
                .year_fraction(period_start, period_end, dc_ctx)?;

            // Payment
            let payment = self.notional.amount() * total_rate * year_frac;

            // Discount factor to payment date using the discount curve's own day-count basis
            let df = disc.df_on_date_curve(period_end);
            pv += payment * df;
        }

        Ok(Money::new(pv, currency))
    }

    /// Calculates the discounted accrual sum (annuity) for a leg.
    ///
    /// This method computes the sum of discounted year fractions for a leg,
    /// which is useful for DV01 calculations and par spread computations.
    ///
    /// # Arguments
    /// * `leg` — The leg specification
    /// * `schedule` — Period schedule for the leg
    /// * `curves` — Market context containing the discount curve
    ///
    /// # Returns
    /// The discounted accrual sum as a floating point value.
    pub fn annuity_for_leg(
        &self,
        leg: &BasisSwapLeg,
        schedule: &PeriodSchedule,
        curves: &MarketContext,
    ) -> Result<f64> {
        let disc = curves.get_discount_ref(self.discount_curve_id.clone())?;
        let mut annuity = 0.0;
        let mut prev = schedule.dates[0];
        for &d in &schedule.dates[1..] {
            let yf = leg
                .day_count
                .year_fraction(prev, d, DayCountCtx::default())?;
            // Discount using the curve's own day-count basis
            let df = disc.df_on_date_curve(d);
            annuity += yf * df;
            prev = d;
        }
        Ok(annuity)
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
        let primary_schedule = self.leg_schedule(&self.primary_leg);
        let reference_schedule = self.leg_schedule(&self.reference_leg);

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
crate::impl_instrument!(
    BasisSwap,
    crate::pricer::InstrumentType::BasisSwap,
    "BasisSwap",
    pv = |s, curves, as_of| {
        // Use the instrument's own npv method
        s.npv(curves, as_of)
    }
);

impl crate::instruments::common::HasDiscountCurve for BasisSwap {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, forward_curve::ForwardCurve,
    };
    use finstack_core::market_data::MarketContext;
    use time::Month;

    // Helper function for tests
    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
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
            .unwrap();

        let forward_3m = ForwardCurve::builder("3M-SOFR", 0.25)
            .base_date(base_date)
            .knots(vec![(0.0, 0.03), (1.0, 0.03), (2.0, 0.03)])
            .build()
            .unwrap();

        let forward_6m = ForwardCurve::builder("6M-SOFR", 0.5)
            .base_date(base_date)
            .knots(vec![(0.0, 0.0305), (1.0, 0.0305), (2.0, 0.0305)])
            .build()
            .unwrap();

        // Create context
        let context = MarketContext::new()
            .insert_discount(discount_curve)
            .insert_forward(forward_3m)
            .insert_forward(forward_6m);

        // Create basis swap: 3M receives 6M + 5bp
        let primary_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("3M-SOFR"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0005, // 5bp
        };

        let reference_leg = BasisSwapLeg {
            forward_curve_id: CurveId::new("6M-SOFR"),
            frequency: Frequency::semi_annual(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        };

        let swap = BasisSwap::new(
            "TEST_BASIS",
            Money::new(1_000_000.0, Currency::USD),
            start_date,
            maturity,
            primary_leg,
            reference_leg,
            CurveId::new("OIS"),
        );

        // Price the swap
        let pv = swap.value(&context, base_date).unwrap();

        // The PV should be close to zero if the spread correctly prices the basis
        assert!(
            pv.amount().abs() < 1000.0,
            "PV should be small: {}",
            pv.amount()
        );
    }
}
