//! Interest Rate Swap (IRS) types and instrument trait implementations.
//!
//! Defines the `InterestRateSwap` instrument following the modern instrument
//! standards used across valuations: types live here; pricing is delegated to
//! `pricing::engine`; and metrics are split under `metrics/`.
//!
//! Public fields use strong newtype identifiers for safety: `InstrumentId` and
//! `CurveId`. Calendar identifiers remain `Option<&'static str>` for stable
//! serde and lookups.
use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{BusinessDayConvention, Frequency, StubKind};
use finstack_core::market_data::traits::Forward;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::{dates::Date, dates::DayCount, F};

use crate::cashflow::builder::{
    cf, CouponType, FixedCouponSpec, ScheduleParams,
};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
// discountable helpers not used after switching to curve-based df_on_date_curve
use crate::instruments::traits::{Attributes, Attributable, Instrument};
// Risk types used in risk.rs
use std::any::Any;

/// Direction of the swap from the perspective of the fixed rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayReceive {
    /// Pay fixed rate, receive floating rate.
    PayFixed,
    /// Receive fixed rate, pay floating rate.
    ReceiveFixed,
}

/// Par rate calculation method for IRS quotes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParRateMethod {
    /// Use forward-curve based float PV over the schedule (market standard when forward curve is available).
    ForwardBased,
    /// Use discount-curve ratio: (P(0,T0) - P(0,Tn)) / Sum_i alpha_i P(0,Ti) (bootstrapping alternative).
    DiscountRatio,
}

/// Specification for the fixed leg of an interest rate swap.
#[derive(Clone, Debug)]
pub struct FixedLegSpec {
    /// Discount curve identifier for pricing.
    pub disc_id: &'static str,
    /// Fixed rate (e.g., 0.05 for 5%).
    pub rate: F,
    /// Payment frequency.
    pub freq: Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for payment dates.
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments.
    pub calendar_id: Option<&'static str>,
    /// Stub period handling rule.
    pub stub: StubKind,
    /// Start date of the fixed leg.
    pub start: Date,
    /// End date of the fixed leg.
    pub end: Date,
    /// Optional par-rate calculation method override.
    pub par_method: Option<ParRateMethod>,
    /// If true, use simple interest on accrual fraction; if false, use per-period compound accumulation when deriving annuity for par.
    pub compounding_simple: bool,
}

/// Specification for the floating leg of an interest rate swap.
#[derive(Clone, Debug)]
pub struct FloatLegSpec {
    /// Discount curve identifier for pricing.
    pub disc_id: &'static str,
    /// Forward curve identifier for rate projections.
    pub fwd_id: &'static str,
    /// Spread in basis points added to the forward rate.
    pub spread_bp: F,
    /// Payment frequency.
    pub freq: Frequency,
    /// Day count convention for accrual.
    pub dc: DayCount,
    /// Business day convention for payment dates.
    pub bdc: BusinessDayConvention,
    /// Optional calendar for business day adjustments.
    pub calendar_id: Option<&'static str>,
    /// Stub period handling rule.
    pub stub: StubKind,
    /// Reset lag in business days for floating rate (fixing to value date).
    pub reset_lag_days: i32,
    /// Start date of the floating leg.
    pub start: Date,
    /// End date of the floating leg.
    pub end: Date,
}

/// Interest rate swap with fixed and floating legs.
///
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: InstrumentId,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Direction of the swap (PayFixed or ReceiveFixed).
    pub side: PayReceive,
    /// Fixed leg specification.
    pub fixed: FixedLegSpec,
    /// Floating leg specification.
    pub float: FloatLegSpec,
    /// Attributes for scenario selection and tagging.
    pub attributes: Attributes,
}

impl InterestRateSwap {
    /// Create a standard USD pay-fixed swap with common market conventions.
    ///
    /// This convenience constructor eliminates the need for a builder in the most common case.
    pub fn usd_pay_fixed(
        id: InstrumentId,
        notional: Money,
        fixed_rate: F,
        start: Date,
        end: Date,
    ) -> Self {
        let sched = ScheduleParams::usd_standard();
        let fixed = FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fixed_rate,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR-3M",
            spread_bp: 0.0,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            reset_lag_days: 2,
            start,
            end,
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::PayFixed)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("USD pay-fixed swap construction should not fail")
    }

    /// Create a standard USD receive-fixed swap with common market conventions.
    pub fn usd_receive_fixed(
        id: InstrumentId,
        notional: Money,
        fixed_rate: F,
        start: Date,
        end: Date,
    ) -> Self {
        let sched = ScheduleParams::usd_standard();
        let fixed = FixedLegSpec {
            disc_id: "USD-OIS",
            rate: fixed_rate,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR-3M",
            spread_bp: 0.0,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            reset_lag_days: 2,
            start,
            end,
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::ReceiveFixed)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("USD receive-fixed swap construction should not fail")
    }

    /// Create a basis swap (float vs float with different indices/spreads).
    pub fn usd_basis_swap(
        id: InstrumentId,
        notional: Money,
        start: Date,
        end: Date,
        primary_spread_bp: F,   // Spread on the "fixed" leg (really floating)
        reference_spread_bp: F, // Spread on the "float" leg
    ) -> Self {
        // Approximate basis swap by using fixed leg to carry the primary spread as a fixed coupon
        let sched = ScheduleParams::usd_standard();
        let fixed = FixedLegSpec {
            disc_id: "USD-OIS",
            rate: primary_spread_bp * 1e-4,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            start,
            end,
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            disc_id: "USD-OIS",
            fwd_id: "USD-SOFR-6M",
            spread_bp: reference_spread_bp,
            freq: sched.freq,
            dc: sched.dc,
            bdc: sched.bdc,
            calendar_id: sched.calendar_id,
            stub: sched.stub,
            reset_lag_days: 2,
            start,
            end,
        };
        Self::builder()
            .id(id)
            .notional(notional)
            .side(PayReceive::PayFixed)
            .fixed(fixed)
            .float(float)
            .build()
            .expect("USD basis swap construction should not fail")
    }

    /// Compute PV of fixed leg (helper for value calculation).
    pub(crate) fn pv_fixed_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
    ) -> finstack_core::Result<Money> {
        let mut b = cf();
        b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id,
                stub: self.fixed.stub,
            });
        let sched = b.build()?;

        // Sum discounted coupon flows using the curve's own day-count via df_on_date_curve
        let mut total = Money::new(0.0, self.notional.currency());
        for cf in &sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                // Use curve policy for discounting
                let df = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::df_on_date_curve(disc, cf.date);
                let disc_amt = cf.amount * df;
                total = (total + disc_amt)?;
            }
        }
        Ok(total)
    }

    /// Compute PV of floating leg (helper for value calculation).
    pub(crate) fn pv_float_leg(
        &self,
        disc: &finstack_core::market_data::term_structures::discount_curve::DiscountCurve,
        fwd: &dyn Forward,
    ) -> finstack_core::Result<Money> {
        let builder = finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
            .frequency(self.float.freq)
            .stub_rule(self.float.stub);

        let sched_dates: Vec<Date> = if let Some(id) = self.float.calendar_id {
            if let Some(cal) = calendar_by_id(id) {
                builder
                    .adjust_with(self.float.bdc, cal)
                    .build()
                    .unwrap()
                    .into_iter()
                    .collect()
            } else {
                builder.build().unwrap().into_iter().collect()
            }
        } else {
            builder.build().unwrap().into_iter().collect()
        };

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }

        let mut prev = sched_dates[0];
        let mut total = Money::new(0.0, self.notional.currency());
        for &d in &sched_dates[1..] {
            let base = disc.base_date();
            let t1 = self
                .float
                .dc
                .year_fraction(base, prev, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let t2 = self
                .float
                .dc
                .year_fraction(base, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let yf = self
                .float
                .dc
                .year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())
                .unwrap_or(0.0);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            // Discount using curve's own day-count
            let df = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::df_on_date_curve(disc, d);
            let disc_amt = coupon * df;
            total = (total + disc_amt)?;
            prev = d;
        }
        Ok(total)
    }
}

// Explicit trait implementations for modern instrument style
impl Attributable for InterestRateSwap {
    fn attributes(&self) -> &Attributes { &self.attributes }
    fn attributes_mut(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Instrument for InterestRateSwap {
    fn id(&self) -> &str { self.id.as_str() }
    fn instrument_type(&self) -> &'static str { "InterestRateSwap" }
    fn as_any(&self) -> &dyn Any { self }
    fn attributes(&self) -> &Attributes { <Self as Attributable>::attributes(self) }
    fn attributes_mut(&mut self) -> &mut Attributes { <Self as Attributable>::attributes_mut(self) }
    fn clone_box(&self) -> Box<dyn Instrument> { Box::new(self.clone()) }
}

// RiskMeasurable impl moved to `risk.rs`

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // Use builder to generate both legs; then map signs by side
        let mut fixed_b = cf();
        fixed_b
            .principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: self.fixed.rate,
                freq: self.fixed.freq,
                dc: self.fixed.dc,
                bdc: self.fixed.bdc,
                calendar_id: self.fixed.calendar_id,
                stub: self.fixed.stub,
            });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = cf();
        float_b
            .principal(self.notional, self.float.start, self.float.end)
            .floating_cf(crate::cashflow::builder::FloatingCouponSpec {
                index_id: self.float.fwd_id,
                margin_bp: self.float.spread_bp,
                gearing: 1.0,
                coupon_type: CouponType::Cash,
                freq: self.float.freq,
                dc: self.float.dc,
                bdc: self.float.bdc,
                calendar_id: self.float.calendar_id,
                stub: self.float.stub,
                reset_lag_days: self.float.reset_lag_days,
            });
        let float_sched = float_b.build()?;

        let mut flows: Vec<(Date, Money)> = Vec::new();
        for cf in fixed_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed
                || cf.kind == crate::cashflow::primitives::CFKind::Stub
            {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount,
                    PayReceive::PayFixed => cf.amount * -1.0,
                };
                flows.push((cf.date, amt));
            }
        }
        for cf in float_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
                let amt = match self.side {
                    PayReceive::ReceiveFixed => cf.amount * -1.0,
                    PayReceive::PayFixed => cf.amount,
                };
                flows.push((cf.date, amt));
            }
        }
        Ok(flows)
    }
}
