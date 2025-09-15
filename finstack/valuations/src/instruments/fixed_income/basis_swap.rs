//! Basis swap implementation for multi-curve calibration.
//!
//! A basis swap exchanges two floating rate payments with different tenors,
//! capturing the basis spread between them (e.g., 3M vs 6M).

use crate::instruments::traits::Priceable;
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::{
    dates::{BusinessDayConvention, Date, DayCount, DayCountCtx, Frequency, StubKind},
    market_data::context::MarketContext,
    money::Money,
    prelude::*,
    types::{CurveId, InstrumentId},
    F,
};

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
#[derive(Clone, Debug)]
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

    /// Calculate the present value of a floating leg.
    fn price_float_leg(
        &self,
        leg: &BasisSwapLeg,
        context: &MarketContext,
        valuation_date: Date,
    ) -> Result<Money> {
        // Get curves
        let discount_curve = context
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                self.discount_curve_id.as_str(),
            )?;
        let forward_curve = context
            .get::<finstack_core::market_data::term_structures::forward_curve::ForwardCurve>(
                leg.forward_curve_id.as_str(),
            )?;

        // Generate payment schedule using simple date logic
        let mut schedule = Vec::new();
        let mut current = self.start_date;

        schedule.push(current);

        // Add regular periods based on frequency
        while current < self.maturity_date {
            current = match leg.frequency.months() {
                Some(months) => finstack_core::dates::add_months(current, months as i32),
                None => match leg.frequency.days() {
                    Some(days) => current + time::Duration::days(days as i64),
                    None => break,
                },
            };

            if current <= self.maturity_date {
                schedule.push(current);
            }
        }

        // Ensure maturity is included
        if schedule.last() != Some(&self.maturity_date) {
            schedule.push(self.maturity_date);
        }

        let mut pv = 0.0;
        let dc_ctx = DayCountCtx::default();

        for i in 0..schedule.len() - 1 {
            let period_start = schedule[i];
            let period_end = schedule[i + 1];

            // Skip past periods
            if period_end <= valuation_date {
                continue;
            }

            // Calculate forward rate for the period
            // Need to convert dates to time fractions from base date
            let dc_ctx_for_time = DayCountCtx::default();
            let t1 =
                DayCount::Act360.year_fraction(self.start_date, period_start, dc_ctx_for_time)?;
            let t2 =
                DayCount::Act360.year_fraction(self.start_date, period_end, dc_ctx_for_time)?;
            let forward_rate = forward_curve.rate_period(t1, t2);

            // Add spread if applicable
            let total_rate = forward_rate + leg.spread;

            // Calculate year fraction
            let year_frac = leg
                .day_count
                .year_fraction(period_start, period_end, dc_ctx)?;

            // Calculate payment amount
            let payment = self.notional.amount() * total_rate * year_frac;

            // Discount to present value
            let dc_ctx_for_df = DayCountCtx::default();
            let t_end =
                DayCount::Act360.year_fraction(self.start_date, period_end, dc_ctx_for_df)?;
            let df = discount_curve.df(t_end);
            pv += payment * df;
        }

        Ok(Money::new(pv, self.notional.currency()))
    }
}

impl Priceable for BasisSwap {
    fn value(&self, context: &MarketContext, valuation_date: Date) -> Result<Money> {
        // Price both legs
        let primary_pv = self.price_float_leg(&self.primary_leg, context, valuation_date)?;
        let reference_pv = self.price_float_leg(&self.reference_leg, context, valuation_date)?;

        // Net present value (primary receives, reference pays)
        Ok(Money::new(
            primary_pv.amount() - reference_pv.amount(),
            primary_pv.currency(),
        ))
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        _metrics: &[MetricId],
    ) -> Result<ValuationResult> {
        // For now, just compute PV
        let pv = self.value(context, as_of)?;

        // Create result with basic metric
        let mut measures = indexmap::IndexMap::new();
        measures.insert("pv".to_string(), pv.amount());

        let result = ValuationResult::stamped(self.id.as_str(), as_of, pv).with_measures(measures);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::term_structures::{
        discount_curve::DiscountCurve, forward_curve::ForwardCurve,
    };
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
