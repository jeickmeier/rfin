//! Basis swap implementation for multi-curve calibration.
//!
//! A basis swap exchanges two floating rate payments with different tenors,
//! capturing the basis spread between them (e.g., 3M vs 6M).

use crate::cashflow::builder::schedule_utils::{build_dates, PeriodSchedule};
use finstack_core::{
    dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind},
    money::Money,
    types::{CurveId, InstrumentId},
    F,
};
use crate::instruments::traits::{Attributable, Instrument};
use std::any::Any;

/// Basis swap specification for one leg
#[derive(Clone, Debug)]
pub struct BasisSwapLeg {
    /// Forward curve identifier for this leg
    pub forward_curve_id: CurveId,
    /// Payment frequency
    pub frequency: Frequency,
    /// Day count convention
    pub day_count: DayCount,
    /// Business day convention
    pub bdc: BusinessDayConvention,
    /// Optional spread (in decimal, not basis points)
    pub spread: F,
}

/// Basis swap instrument (float vs float).
///
/// Exchanges two floating rate payments with different tenors,
/// plus an optional spread on one leg.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct BasisSwap {
    /// Instrument identifier
    pub id: InstrumentId,
    /// Notional amount
    pub notional: Money,
    /// Start date
    pub start_date: Date,
    /// Maturity date
    pub maturity_date: Date,
    /// Primary leg (receives spread)
    pub primary_leg: BasisSwapLeg,
    /// Reference leg (pays flat)
    pub reference_leg: BasisSwapLeg,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
    /// Calendar identifier for date adjustments
    pub calendar_id: Option<&'static str>,
    /// Stub handling
    pub stub_kind: StubKind,
    /// Attributes for selection and tagging
    pub attributes: crate::instruments::traits::Attributes,
}

impl BasisSwap {
    /// Create a new basis swap.
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

    /// Set calendar for date adjustments.
    pub fn with_calendar(mut self, calendar_id: &'static str) -> Self {
        self.calendar_id = Some(calendar_id);
        self
    }

    /// Set stub handling.
    pub fn with_stub(mut self, stub_kind: StubKind) -> Self {
        self.stub_kind = stub_kind;
        self
    }

    /// Build a canonical period schedule for a given leg using shared schedule utils.
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
    fn id(&self) -> &str { self.id.as_str() }
    fn instrument_type(&self) -> &'static str { "BasisSwap" }
    fn as_any(&self) -> &dyn Any { self }
    fn attributes(&self) -> &crate::instruments::traits::Attributes { <Self as Attributable>::attributes(self) }
    fn attributes_mut(&mut self) -> &mut crate::instruments::traits::Attributes { <Self as Attributable>::attributes_mut(self) }
    fn clone_box(&self) -> Box<dyn Instrument> { Box::new(self.clone()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, forward_curve::ForwardCurve,
    };
    use finstack_core::market_data::MarketContext;
    use finstack_core::currency::Currency;
    use crate::instruments::traits::Priceable;
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
