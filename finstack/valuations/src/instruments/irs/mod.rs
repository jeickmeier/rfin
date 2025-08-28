//! Interest rate swap instrument implementation.

pub mod metrics;

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::dates::{Frequency, BusinessDayConvention, StubKind};
use finstack_core::dates::holiday::calendars::calendar_by_id;
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::traits::{Discount, Forward};

use crate::cashflow::builder::{cf, FixedCouponSpec, FloatingCouponSpec as BuilderFloat, CouponType};
use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Priceable, CashflowProvider, DatedFlows};

/// Direction of the swap from the perspective of the fixed rate.
#[derive(Clone, Copy, Debug)]
pub enum PayReceive { 
    /// Pay fixed rate, receive floating rate.
    PayFixed, 
    /// Receive fixed rate, pay floating rate.
    ReceiveFixed 
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
    /// Start date of the floating leg.
    pub start: Date,
    /// End date of the floating leg.
    pub end: Date,
}

/// Interest rate swap with fixed and floating legs.
/// 
/// Represents a standard interest rate swap where one party pays
/// a fixed rate and the other pays a floating rate plus spread.
#[derive(Clone, Debug)]
pub struct InterestRateSwap {
    /// Unique identifier for the swap.
    pub id: String,
    /// Notional amount for both legs.
    pub notional: Money,
    /// Direction of the swap (PayFixed or ReceiveFixed).
    pub side: PayReceive,
    /// Fixed leg specification.
    pub fixed: FixedLegSpec,
    /// Floating leg specification.
    pub float: FloatLegSpec,
}

impl InterestRateSwap {
    /// Compute PV of fixed leg (helper for value calculation).
    fn pv_fixed_leg(&self, disc: &dyn Discount) -> finstack_core::Result<Money> {
        let base = disc.base_date();
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
        
        // Discount coupon flows only
        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter(|cf| cf.kind == crate::cashflow::primitives::CFKind::Fixed || 
                        cf.kind == crate::cashflow::primitives::CFKind::Stub)
            .map(|cf| (cf.date, cf.amount))
            .collect();
        flows.npv(disc, base, sched.day_count)
    }

    /// Compute PV of floating leg (helper for value calculation).
    fn pv_float_leg(&self, disc: &dyn Discount, fwd: &dyn Forward) -> finstack_core::Result<Money> {
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        
        let base = disc.base_date();
        let builder = finstack_core::dates::ScheduleBuilder::new(self.float.start, self.float.end)
            .frequency(self.float.freq)
            .stub_rule(self.float.stub);
            
        let sched_dates: Vec<Date> = if let Some(id) = self.float.calendar_id {
            if let Some(cal) = calendar_by_id(id) {
                builder.adjust_with(self.float.bdc, cal).build().collect()
            } else {
                builder.build_raw().collect()
            }
        } else {
            builder.build_raw().collect()
        };

        if sched_dates.len() < 2 {
            return Ok(Money::new(0.0, self.notional.currency()));
        }
        
        let mut prev = sched_dates[0];
        let mut flows: Vec<(Date, Money)> = Vec::with_capacity(sched_dates.len().saturating_sub(1));
        for &d in &sched_dates[1..] {
            let t1 = DiscountCurve::year_fraction(base, prev, self.float.dc);
            let t2 = DiscountCurve::year_fraction(base, d, self.float.dc);
            let yf = DiscountCurve::year_fraction(prev, d, self.float.dc);
            let f = fwd.rate_period(t1, t2);
            let rate = f + (self.float.spread_bp * 1e-4);
            let coupon = self.notional * (rate * yf);
            flows.push((d, coupon));
            prev = d;
        }
        flows.npv(disc, base, self.float.dc)
    }
}

impl Priceable for InterestRateSwap {
    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, _as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.fixed.disc_id)?;
        let fwd = curves.forecast(self.float.fwd_id)?;
        
        let pv_fixed = self.pv_fixed_leg(&*disc)?;
        let pv_float = self.pv_float_leg(&*disc, &*fwd)?;
        
        match self.side {
            PayReceive::PayFixed => pv_float - pv_fixed,
            PayReceive::ReceiveFixed => pv_fixed - pv_float,
        }
    }
    
    /// Compute value with specific metrics using the metrics framework.
    fn price_with_metrics(
        &self, 
        curves: &CurveSet, 
        as_of: Date, 
        metrics: &[crate::metrics::MetricId]
    ) -> finstack_core::Result<ValuationResult> {
        use crate::instruments::Instrument;
        use crate::metrics::{MetricContext, standard_registry};
        use std::sync::Arc;
        
        // Compute base value
        let base_value = self.value(curves, as_of)?;
        
        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(Instrument::IRS(self.clone())),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );
        
        // Get registry and compute requested metrics
        let registry = standard_registry();
        let metric_measures = registry.compute(metrics, &mut context)?;
        
        // Convert MetricId keys to String keys for ValuationResult
        let measures: hashbrown::HashMap<String, finstack_core::F> = metric_measures
            .into_iter()
            .map(|(k, v)| (k.as_str().to_string(), v))
            .collect();
        
        // Create result
        let mut result = ValuationResult::stamped(self.id.clone(), as_of, base_value);
        result.measures = measures;
        
        Ok(result)
    }
    
    /// Compute full valuation with all standard IRS metrics (backward compatible).
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Standard IRS metrics
        use crate::metrics::MetricId;
        let standard_metrics = [MetricId::Annuity, MetricId::ParRate, MetricId::Dv01, MetricId::PvFixed, MetricId::PvFloat];
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

impl CashflowProvider for InterestRateSwap {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Use builder to generate both legs; then map signs by side
        let mut fixed_b = cf();
        fixed_b.principal(self.notional, self.fixed.start, self.fixed.end)
            .fixed_cf(FixedCouponSpec { 
                coupon_type: CouponType::Cash, 
                rate: self.fixed.rate, 
                freq: self.fixed.freq, 
                dc: self.fixed.dc, 
                bdc: self.fixed.bdc, 
                calendar_id: self.fixed.calendar_id, 
                stub: self.fixed.stub 
            });
        let fixed_sched = fixed_b.build()?;

        let mut float_b = cf();
        float_b.principal(self.notional, self.float.start, self.float.end)
            .floating_cf(BuilderFloat { 
                index_id: self.float.fwd_id, 
                margin_bp: self.float.spread_bp, 
                gearing: 1.0, 
                coupon_type: CouponType::Cash, 
                freq: self.float.freq, 
                dc: self.float.dc, 
                bdc: self.float.bdc, 
                calendar_id: self.float.calendar_id, 
                stub: self.float.stub, 
                reset_lag_days: 2 
            });
        let float_sched = float_b.build()?;

        let mut flows: Vec<(Date, Money)> = Vec::new();
        for cf in fixed_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::Fixed || 
               cf.kind == crate::cashflow::primitives::CFKind::Stub {
                let amt = match self.side { 
                    PayReceive::ReceiveFixed => cf.amount, 
                    PayReceive::PayFixed => cf.amount * -1.0 
                };
                flows.push((cf.date, amt));
            }
        }
        for cf in float_sched.flows {
            if cf.kind == crate::cashflow::primitives::CFKind::FloatReset {
                let amt = match self.side { 
                    PayReceive::ReceiveFixed => cf.amount * -1.0, 
                    PayReceive::PayFixed => cf.amount 
                };
                flows.push((cf.date, amt));
            }
        }
        Ok(flows)
    }
}