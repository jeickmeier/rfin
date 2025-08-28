#![allow(missing_docs)]

pub mod metrics;

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::multicurve::CurveSet;

use crate::pricing::discountable::Discountable;
use crate::pricing::result::ValuationResult;
use crate::traits::{Priceable, CashflowProvider, DatedFlows};
use crate::cashflow::primitives::CFKind;
use crate::cashflow::builder::{cf, FixedCouponSpec, CouponType};
use finstack_core::dates::{BusinessDayConvention, StubKind};

// Re-export for compatibility in tests and external users referencing bond::AmortizationSpec
pub use crate::cashflow::amortization::AmortizationSpec;

#[derive(Clone, Debug)]
pub struct Bond {
    pub id: String,
    pub notional: Money,
    pub coupon: F,
    pub freq: finstack_core::dates::Frequency,
    pub dc: DayCount,
    pub issue: Date,
    pub maturity: Date,
    pub disc_id: &'static str,
    /// Optional quoted clean price (per notional unit). If provided, we compute YTM measures.
    pub quoted_clean: Option<F>,
    /// Optional call/put schedule (dates and redemption prices as % of par amount).
    pub call_put: Option<CallPutSchedule>,
    /// Optional amortization specification (principal paid during life).
    pub amortization: Option<AmortizationSpec>,
}

#[derive(Clone, Debug)]
pub struct CallPut { pub date: Date, pub price_pct_of_par: F }

#[derive(Clone, Debug, Default)]
pub struct CallPutSchedule { pub calls: Vec<CallPut>, pub puts: Vec<CallPut> }

impl Bond {
    /// Get the standard metrics for a bond based on its configuration.
    fn get_standard_metrics(&self) -> Vec<crate::metrics::MetricId> {
        use crate::metrics::MetricId;
        let mut metrics = vec![MetricId::Accrued, MetricId::CleanPrice];
        
        // Add dirty price and YTM-related metrics only if we have a quoted price
        if self.quoted_clean.is_some() {
            metrics.extend_from_slice(&[MetricId::DirtyPrice, MetricId::Ytm, MetricId::DurationMac, MetricId::DurationMod, MetricId::Convexity, MetricId::Cs01]);
        }
        
        // YTW only if we have call/put schedule and quoted price
        if self.call_put.is_some() && self.quoted_clean.is_some() {
            metrics.push(MetricId::Ytw);
        }
        
        metrics
    }
}

impl Priceable for Bond {
    /// Compute only the base present value (fast, no metrics).
    fn value(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<Money> {
        let disc = curves.discount(self.disc_id)?;
        let flows = self.build_schedule(curves, as_of)?;
        flows.npv(&*disc, disc.base_date(), self.dc)
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
            Arc::new(Instrument::Bond(self.clone())),
            "Bond".to_string(),
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
    
    /// Compute full valuation with all applicable standard metrics (backward compatible).
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<ValuationResult> {
        // Use the metrics framework to compute all standard bond metrics
        let standard_metrics = self.get_standard_metrics();
        self.price_with_metrics(curves, as_of, &standard_metrics)
    }
}

impl CashflowProvider for Bond {
    fn build_schedule(&self, _curves: &CurveSet, _as_of: Date) -> finstack_core::Result<DatedFlows> {
        // Build via unified cashflow builder
        let mut b = cf();
        b.principal(self.notional, self.issue, self.maturity);
        if let Some(am) = &self.amortization { b.amortization(am.clone()); }
        b.fixed_cf(FixedCouponSpec {
            coupon_type: CouponType::Cash,
            rate: self.coupon,
            freq: self.freq,
            dc: self.dc,
            bdc: BusinessDayConvention::Following,
            calendar_id: None,
            stub: StubKind::None,
        });
        let sched = b.build()?;

        // Map to holder flows: coupons positive, amortization as positive, include only positive notional (redemption)
        let flows: Vec<(Date, Money)> = sched
            .flows
            .iter()
            .filter_map(|cf| match cf.kind {
                CFKind::Fixed | CFKind::Stub => Some((cf.date, cf.amount)),
                CFKind::Amortization => Some((cf.date, Money::new(-cf.amount.amount(), cf.amount.currency()))),
                CFKind::Notional if cf.amount.amount() > 0.0 => Some((cf.date, cf.amount)),
                _ => None,
            })
            .collect();

        Ok(flows)
    }
}