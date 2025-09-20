//! Basis swap implementation for multi-curve calibration.
//!
//! A basis swap exchanges two floating rate payments with different tenors,
//! capturing the basis spread between them (e.g., 3M vs 6M).

use crate::cashflow::builder::schedule_utils::{build_dates, PeriodSchedule};
use crate::instruments::traits::{Attributable, Instrument};
use finstack_core::{
    dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind},
    money::Money,
    types::{CurveId, InstrumentId},
    F,
};
use std::any::Any;

/// Specification for one leg of a basis swap.
///
/// Each leg defines the floating rate characteristics including the forward curve,
/// payment frequency, day count convention, and optional spread.
///
/// # Examples
/// ```rust
/// use finstack_core::dates::{DayCount, Frequency, BusinessDayConvention};
/// use finstack_core::types::CurveId;
/// use finstack_valuations::instruments::basis_swap::BasisSwapLeg;
///
/// let leg = BasisSwapLeg {
///     forward_curve_id: CurveId::new("3M-SOFR"),
///     frequency: Frequency::quarterly(),
///     day_count: DayCount::Act360,
///     bdc: BusinessDayConvention::ModifiedFollowing,
///     spread: 0.0005, // 5 basis points
/// };
/// ```
#[derive(Clone, Debug)]
pub struct BasisSwapLeg {
    /// Forward curve identifier for this leg.
    pub forward_curve_id: CurveId,
    /// Payment frequency for the leg.
    pub frequency: Frequency,
    /// Day count convention for accrual calculations.
    pub day_count: DayCount,
    /// Business day convention for date adjustments.
    pub bdc: BusinessDayConvention,
    /// Optional spread in decimal form (e.g., 0.0005 for 5 basis points).
    pub spread: F,
}

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
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    pub attributes: crate::instruments::traits::Attributes,
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
            attributes: crate::instruments::traits::Attributes::default(),
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
}

impl Attributable for BasisSwap {
    fn attributes(&self) -> &crate::instruments::traits::Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
        &mut self.attributes
    }
}

impl Instrument for BasisSwap {
    fn id(&self) -> &str {
        self.id.as_str()
    }
    fn instrument_type(&self) -> &'static str {
        "BasisSwap"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &crate::instruments::traits::Attributes {
        <Self as Attributable>::attributes(self)
    }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes {
        <Self as Attributable>::attributes_mut(self)
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::traits::Priceable;
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
