//! Risk-specific metric calculators.

use super::traits::{MetricCalculator, MetricContext};
use super::ids::MetricId;
use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::Discount;
use hashbrown::HashMap;

/// Specification for DV01 tenor buckets.
#[derive(Clone, Debug)]
pub struct BucketSpec {
    /// Tenor points in years from curve base date.
    pub tenors: Vec<F>,
}

impl Default for BucketSpec {
    fn default() -> Self {
        // Standard bucket points: 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
        Self {
            tenors: vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0],
        }
    }
}

/// Calculates bucketed DV01 (sensitivity per tenor bucket).
#[derive(Default)]
pub struct BucketedDv01Calculator {
    /// Bucket specification to use.
    pub buckets: BucketSpec,
}

impl BucketedDv01Calculator {
    /// Create with custom bucket specification.
    pub fn with_buckets(buckets: BucketSpec) -> Self {
        Self { buckets }
    }

    /// Format a bucket label.
    fn bucket_label(&self, tenor_years: F) -> String {
        if tenor_years < 1.0 {
            format!("{}M", (tenor_years * 12.0).round() as i32)
        } else {
            format!("{:.0}Y", tenor_years)
        }
    }

    /// Compute bucketed DV01 for given flows.
    fn compute_bucketed(
        &self,
        flows: &[(Date, Money)],
        disc: &dyn Discount,
        dc: DayCount,
        base: Date,
    ) -> HashMap<String, F> {
        let mut result = HashMap::new();

        // Early return if no flows
        if flows.is_empty() {
            result.insert("bucketed_dv01_total".to_string(), 0.0);
            return result;
        }

        // Precompute each flow's time and assign to nearest bucket
        let mut idx_to_label: HashMap<usize, String> = HashMap::new();
        let mut bucket_flows: HashMap<usize, Vec<(Date, Money)>> = HashMap::new();
        let mut flow_data: Vec<(Date, Money, F, usize)> = Vec::with_capacity(flows.len());
        
        for &(date, amount) in flows {
            let t = DiscountCurve::year_fraction(base, date, dc).max(0.0);
            
            // Find nearest bucket
            let (idx, _) = self.buckets.tenors
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    (*a - t).abs().partial_cmp(&(*b - t).abs()).unwrap()
                })
                .unwrap_or((0, &self.buckets.tenors[0]));
            
            idx_to_label.entry(idx)
                .or_insert_with(|| self.bucket_label(self.buckets.tenors[idx]));
            bucket_flows.entry(idx).or_default().push((date, amount));
            flow_data.push((date, amount, t, idx));
        }

        // Compute baseline PV and cache discount factors
        let mut base_pv = 0.0;
        let mut df_cache: Vec<F> = Vec::with_capacity(flow_data.len());
        
        for (date, amount, _, _) in &flow_data {
            let df = DiscountCurve::df_on(disc, base, *date, dc);
            base_pv += amount.amount() * df;
            df_cache.push(df);
        }

        // Compute per-bucket DV01 by bumping each bucket
        let bp = 1e-4; // 1 basis point
        let mut total_dv01 = 0.0;
        
        for (bucket_idx, _) in bucket_flows.iter() {
            let mut bumped_pv = 0.0;
            
            for ((_, amount, t, idx), df) in flow_data.iter().zip(df_cache.iter()) {
                // Apply bump only to flows in this bucket
                let df_bumped = if idx == bucket_idx {
                    *df * (-bp * *t).exp()
                } else {
                    *df
                };
                bumped_pv += amount.amount() * df_bumped;
            }
            
            let dv01 = (base_pv - bumped_pv) / bp;
            let label = idx_to_label.get(bucket_idx)
                .cloned()
                .unwrap_or_else(|| self.bucket_label(self.buckets.tenors[*bucket_idx]));
            
            result.insert(format!("bucketed_dv01_{}", label.to_lowercase()), dv01);
            total_dv01 += dv01;
        }
        
        result.insert("bucketed_dv01_total".to_string(), total_dv01);
        result
    }
}

impl MetricCalculator for BucketedDv01Calculator {
    fn id(&self) -> MetricId {
        MetricId::BucketedDv01
    }
    
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Get or compute cashflows
        let flows = context.get_cached::<Vec<(Date, Money)>>("cashflows")
            .ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))?;
        
        // Get discount curve - try to infer from instrument or use default
        let disc_id = context.get_cached::<&'static str>("discount_curve_id")
            .map(|arc| *arc)
            .unwrap_or("USD-OIS");
        
        let disc = context.market_data.curves.discount(disc_id)?;
        
        // Get day count - try to infer or use default
        let dc = context.get_cached::<DayCount>("day_count")
            .map(|arc| *arc)
            .unwrap_or(DayCount::Act365F);
        
        let base = disc.base_date();
        
        // Compute all bucketed DV01s
        let bucketed = self.compute_bucketed(&flows, &*disc, dc, base);
        
        // Store individual bucket results in context
        // TODO: Handle dynamic bucket keys with MetricId
        // for (key, value) in bucketed.iter() {
        //     context.computed.insert(key.clone(), *value);
        // }
        
        // Return total as primary result
        Ok(bucketed.get("bucketed_dv01_total").copied().unwrap_or(0.0))
    }
    
    fn dependencies(&self) -> &[MetricId] {
        // No hard dependencies, but works better if cashflows are cached
        &[]
    }
}

/// Calculates theta (time decay) for options and time-sensitive instruments.
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn id(&self) -> MetricId {
        MetricId::Theta
    }
    
    fn calculate(&self, _context: &mut MetricContext) -> finstack_core::Result<F> {
        Err(finstack_core::Error::from(
            finstack_core::error::InputError::Invalid
        ))
    }
}

/// Helper trait for instruments to cache their cashflows for risk calculations.
pub trait CashflowCaching {
    /// Cache cashflows in the metric context for risk calculations.
    fn cache_cashflows(&self, context: &mut MetricContext, flows: Vec<(Date, Money)>) {
        context.cache_value("cashflows", flows);
    }
    
    /// Cache the discount curve ID to use.
    fn cache_discount_curve(&self, context: &mut MetricContext, curve_id: &'static str) {
        context.cache_value("discount_curve_id", curve_id);
    }
    
    /// Cache the day count convention.
    fn cache_day_count(&self, context: &mut MetricContext, dc: DayCount) {
        context.cache_value("day_count", dc);
    }
}

/// Register all risk metrics to a registry.
pub fn register_risk_metrics(registry: &mut super::MetricRegistry) {
    use std::sync::Arc;
    
    // BucketedDv01 applies to instruments with cashflows (Bond, IRS, etc.)
    let risk_types = &["Bond", "IRS", "Deposit"];
    
    registry
        .register_for_types(Arc::new(BucketedDv01Calculator::default()), risk_types)
        .register(Arc::new(ThetaCalculator)); // ThetaCalculator has is_applicable returning false, so it won't run anyway
}
