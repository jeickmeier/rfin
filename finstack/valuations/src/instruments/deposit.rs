#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;

use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{CashflowProvider, Priceable, DatedFlows};
use crate::cashflow::builder::{cf, FixedCouponSpec, CouponType};
use finstack_core::dates::{BusinessDayConvention, StubKind, Frequency};

#[derive(Clone, Debug)]
pub struct Deposit {
    pub id: String,
    pub notional: Money,
    pub start: Date,
    pub end: Date,
    pub day_count: DayCount,
    /// Optional quoted simple rate r (annualised) for the deposit.
    pub quote_rate: Option<F>,
    /// Discount curve id used for valuation and par extraction.
    pub disc_id: &'static str,
}

impl Priceable for Deposit {
    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(&*disc, disc.base_date(), self.day_count)
    }
    
    /// Compute value with specific metrics using the metrics framework.
    fn price_with_metrics(
        &self, 
        curves: &CurveSet, 
        as_of: Date, 
        metrics: &[&str]
    ) -> finstack_core::Result<ValuationResult> {
        use crate::metrics::{MetricContext, standard_registry};
        use std::sync::Arc;
        
        // Compute base value
        let base_value = self.value(curves, as_of)?;
        
        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(self.clone()) as Arc<dyn std::any::Any + Send + Sync>,
            "Deposit".to_string(),
            Arc::new(curves.clone()),
            as_of,
            base_value,
        );
        
        // Get registry and compute requested metrics
        let registry = standard_registry();
        let measures = registry.compute(metrics, &mut context)?;
        
        // Create result
        let mut result = ValuationResult::stamped(self.id.clone(), as_of, base_value);
        result.measures = measures;
        
        Ok(result)
    }
    
    /// Compute full valuation with all standard deposit metrics (backward compatible).
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Standard deposit metrics
        let mut standard_metrics = vec!["yf", "df_start", "df_end", "deposit_par_rate"];
        
        // Add quote-related metrics if we have a quoted rate
        if self.quote_rate.is_some() {
            standard_metrics.push("df_end_from_quote");
            standard_metrics.push("quote_rate");
        }
        
        let mut result = self.price_with_metrics(curves, as_of, &standard_metrics)?;
        
        // For backward compatibility, rename deposit_par_rate back to par_rate
        if let Some(value) = result.measures.remove("deposit_par_rate") {
            result.measures.insert("par_rate".to_string(), value);
        }
        
        Ok(result)
    }
}

impl CashflowProvider for Deposit {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Build a single-period schedule using a custom day-step equal to the total span
        let days = (self.end - self.start).whole_days();
        let rate = self.quote_rate.unwrap_or(0.0);

        let mut b = cf();
        b.principal(self.notional, self.start, self.end)
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate,
                freq: if days <= 1 { Frequency::daily() } 
                      else if days == 7 { Frequency::weekly() } 
                      else if days == 14 { Frequency::biweekly() } 
                      else { Frequency::monthly() },
                dc: self.day_count,
                bdc: BusinessDayConvention::Unadjusted,
                calendar_id: None,
                stub: StubKind::None,
            });
        let sched = b.build()?;

        // Map to two-flow holder schedule: principal out at start; redemption at end including interest
        // Sum all amounts on end date except the initial notional outflow
        let mut redemption = Money::new(0.0, self.notional.currency());
        for cf in &sched.flows {
            if cf.date == self.end {
                // Include both coupon and final notional
                redemption = (redemption + cf.amount)?;
            }
        }
        Ok(vec![(self.start, self.notional * -1.0), (self.end, redemption)])
    }
}