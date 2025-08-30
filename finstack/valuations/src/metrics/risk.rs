//! Risk-specific metric calculators.
//! 
//! Provides specialized calculators for risk metrics including bucketed DV01
//! and time decay (theta). These metrics help quantify interest rate risk
//! and time value of financial instruments.

use super::traits::{MetricCalculator, MetricContext};
use super::ids::MetricId;
use finstack_core::prelude::*;
use finstack_core::F;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::traits::Discount;
use hashbrown::HashMap;

/// Specification for DV01 tenor buckets.
/// 
/// Defines the tenor points used for bucketed DV01 calculations.
/// Standard buckets cover 3M to 30Y with configurable points for
/// detailed risk analysis and hedging decisions.
/// 
/// # Default Buckets
/// 
/// The default specification includes standard tenor points:
/// - 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
/// 
/// See unit tests and `examples/` for usage.
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
/// 
/// Breaks down interest rate sensitivity by maturity buckets for
/// better risk management and hedging decisions. Each bucket represents
/// the sensitivity to a parallel shift in that specific tenor region.
/// 
/// # How It Works
/// 
/// 1. **Flow Assignment**: Each cashflow is assigned to the nearest tenor bucket
/// 2. **Sensitivity Calculation**: DV01 is computed per bucket using small rate shifts
/// 3. **Risk Aggregation**: Total risk is the sum of all bucket sensitivities
/// 
/// See unit tests and `examples/` for usage.
#[derive(Default)]
pub struct BucketedDv01Calculator {
    /// Bucket specification to use.
    pub buckets: BucketSpec,
}

impl BucketedDv01Calculator {
    /// Creates a calculator with custom bucket specification.
    /// 
    /// # Arguments
    /// * `buckets` - Custom bucket specification for the analysis
    /// 
    /// See unit tests and `examples/` for usage.
    pub fn with_buckets(buckets: BucketSpec) -> Self {
        Self { buckets }
    }

    /// Formats a bucket label for display.
    /// 
    /// Converts tenor years to human-readable labels:
    /// - < 1 year: "XM" (e.g., "6M" for 0.5 years)
    /// - ≥ 1 year: "XY" (e.g., "5Y" for 5.0 years)
    /// 
    /// # Arguments
    /// * `tenor_years` - Tenor in years
    /// 
    /// # Returns
    /// Formatted string label for the bucket
    fn bucket_label(&self, tenor_years: F) -> String {
        if tenor_years < 1.0 {
            format!("{}M", (tenor_years * 12.0).round() as i32)
        } else {
            format!("{:.0}Y", tenor_years)
        }
    }

    /// Computes bucketed DV01 for given cashflows.
    /// 
    /// Assigns each cashflow to the nearest tenor bucket and calculates
    /// the sensitivity within each bucket. This provides detailed risk
    /// breakdown for hedging and risk management.
    /// 
    /// # Arguments
    /// * `flows` - Vector of (date, money) tuples representing cashflows
    /// * `disc` - Discount curve for present value calculations
    /// * `dc` - Day count convention for time calculations
    /// * `base` - Base date for year fraction calculations
    /// 
    /// # Returns
    /// HashMap mapping bucket labels to DV01 values
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
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Get or compute cashflows
        let flows = context.cashflows.as_ref()
            .ok_or_else(|| finstack_core::Error::from(
                finstack_core::error::InputError::NotFound
            ))?;
        
        // Get discount curve - try to infer from instrument or use default
        let disc_id = context.discount_curve_id
            .unwrap_or("USD-OIS");
        
        let disc = context.curves.discount(disc_id)?;
        
        // Get day count - try to infer or use default
        let dc = context.day_count
            .unwrap_or(DayCount::Act365F);
        
        let base = disc.base_date();
        
        // Compute all bucketed DV01s
        let bucketed = self.compute_bucketed(flows, &*disc, dc, base);
        
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
/// 
/// Theta measures the rate of change in an option's value with respect to time.
/// This is particularly important for options and other derivatives where
/// time value plays a significant role in pricing.
/// 
/// # Note
/// 
/// This calculator is currently a placeholder and returns an error.
/// Future implementations will compute actual theta values based on
/// option pricing models and time sensitivity analysis.
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, _context: &mut MetricContext) -> finstack_core::Result<F> {
        Err(finstack_core::Error::from(
            finstack_core::error::InputError::Invalid
        ))
    }
}

/// Helper trait for instruments to cache their cashflows for risk calculations.
/// 
/// This trait provides convenience methods for instruments to cache
/// commonly needed data in the metric context, improving performance
/// for risk calculations that need cashflows, discount curves, and
/// day count conventions.
/// 
/// See unit tests and `examples/` for usage.
pub trait CashflowCaching {
    /// Caches cashflows in the metric context for risk calculations.
    /// 
    /// This method stores the instrument's cashflow schedule in the context,
    /// allowing risk calculators to access it without recomputation.
    /// 
    /// # Arguments
    /// * `context` - Metric context to cache cashflows in
    /// * `flows` - Vector of (date, money) tuples representing cashflows
    fn cache_cashflows(&self, context: &mut MetricContext, flows: Vec<(Date, Money)>) {
        context.cashflows = Some(flows);
    }
    
    /// Caches the discount curve ID to use.
    /// 
    /// This method stores the identifier for the discount curve that should
    /// be used for risk calculations involving this instrument.
    /// 
    /// # Arguments
    /// * `context` - Metric context to cache the curve ID in
    /// * `curve_id` - Static string identifier for the discount curve
    fn cache_discount_curve(&self, context: &mut MetricContext, curve_id: &'static str) {
        context.discount_curve_id = Some(curve_id);
    }
    
    /// Caches the day count convention.
    /// 
    /// This method stores the day count convention that should be used
    /// for time calculations in risk metrics.
    /// 
    /// # Arguments
    /// * `context` - Metric context to cache the day count in
    /// * `dc` - Day count convention to use
    fn cache_day_count(&self, context: &mut MetricContext, dc: DayCount) {
        context.day_count = Some(dc);
    }
}

/// Registers all risk metrics to a registry.
/// 
/// This function adds the standard risk metrics (bucketed DV01 and theta)
/// to the provided metric registry. Bucketed DV01 is registered for all
/// instrument types, while theta is registered globally.
/// 
/// # Arguments
/// * `registry` - Metric registry to add risk metrics to
/// 
/// See unit tests and `examples/` for usage.
pub fn register_risk_metrics(registry: &mut super::MetricRegistry) {
    use std::sync::Arc;
    use super::MetricId;
    
    registry
        .register_metric(MetricId::BucketedDv01, Arc::new(BucketedDv01Calculator::default()), &["Bond", "IRS", "Deposit"])
        .register_metric(MetricId::Theta, Arc::new(ThetaCalculator), &[]);
}
