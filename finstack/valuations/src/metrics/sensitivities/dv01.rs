//! Unified DV01 calculator supporting parallel and key-rate sensitivities.
//!
//! This module provides a single, flexible DV01 calculator that replaces the
//! various specialized calculators with a unified approach.
//!
//! # Key Features
//!
//! - **Type-safe curve discovery**: Uses [`CurveDependencies`] trait to discover curves at compile time
//! - **Flexible computation modes**: Parallel or key-rate, combined or per-curve
//! - **Multiple curve types**: Handles discount, forward, and credit curves
//! - **Leverages core infrastructure**: Uses [`finstack_core::market_data::bumps::BumpSpec`] 
//!   and [`finstack_core::market_data::MarketContext::bump`] for consistent bumping
//!
//! # Advantages Over Legacy Implementation
//!
//! 1. **No runtime downcasting**: Uses trait bounds instead of runtime type checks
//! 2. **Unified codebase**: Single implementation for all modes (parallel/bucketed/per-curve)
//! 3. **Extensible**: Easy to add new curve types or computation modes
//! 4. **Better error messages**: Compile-time trait bounds catch issues early
//!
//! # Quick Start
//!
//! ## Example 1: Key-Rate DV01 for a Bond
//!
//! ```ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{
//!     UnifiedDv01Calculator, Dv01CalculatorConfig, MetricContext
//! };
//! use finstack_valuations::instruments::common::traits::Instrument;
//! use std::sync::Arc;
//!
//! // Create bond and market context (details omitted)
//! let bond = Bond::fixed(...);
//! let market = MarketContext::new()...;
//! let as_of = ...;
//!
//! // Calculate bucketed DV01
//! let calculator = UnifiedDv01Calculator::<Bond>::new(
//!     Dv01CalculatorConfig::key_rate()
//! );
//!
//! let base_value = bond.value(&market, as_of)?;
//! let mut context = MetricContext::new(
//!     Arc::new(bond),
//!     Arc::new(market),
//!     as_of,
//!     base_value,
//! );
//!
//! let total_dv01 = calculator.calculate(&mut context)?;
//! let series = context.get_series(&MetricId::BucketedDv01)?;
//! ```
//!
//! ## Example 2: Parallel DV01 for an IRS
//!
//! ```ignore
//! use finstack_valuations::instruments::InterestRateSwap;
//! use finstack_valuations::metrics::{
//!     UnifiedDv01Calculator, Dv01CalculatorConfig
//! };
//!
//! // For parallel DV01 (single scalar value)
//! let calculator = UnifiedDv01Calculator::<InterestRateSwap>::new(
//!     Dv01CalculatorConfig::parallel_combined()
//! );
//!
//! let total_dv01 = calculator.calculate(&mut context)?;
//! ```
//!
//! ## Example 3: Custom Configuration
//!
//! ```ignore
//! use finstack_valuations::metrics::{
//!     Dv01CalculatorConfig, Dv01ComputationMode, CurveSelection
//! };
//!
//! // Custom buckets, per-curve, discount only
//! let config = Dv01CalculatorConfig {
//!     mode: Dv01ComputationMode::KeyRatePerCurve,
//!     curve_selection: CurveSelection::DiscountOnly,
//!     buckets: vec![1.0, 5.0, 10.0, 30.0],  // Custom bucket times
//! };
//!
//! let calculator = UnifiedDv01Calculator::<Bond>::new(config);
//! ```
//!
//! # Architecture
//!
//! ## Curve Discovery
//!
//! Instruments implement [`CurveDependencies`] to declare their curves:
//!
//! ```ignore
//! impl CurveDependencies for Bond {
//!     fn curve_dependencies(&self) -> InstrumentCurves {
//!         let mut builder = InstrumentCurves::builder()
//!             .discount(self.discount_curve_id.clone());
//!         
//!         if let Some(ref credit_curve_id) = self.credit_curve_id {
//!             builder = builder.credit(credit_curve_id.clone());
//!         }
//!         
//!         builder.build()
//!     }
//! }
//! ```
//!
//! ## Computation Flow
//!
//! 1. **Collect curves**: Based on configuration and [`CurveDependencies`]
//! 2. **For each curve/bucket**: Create [`BumpSpec`] and bump [`MarketContext`]
//! 3. **Reprice instrument**: Call `instrument.value()` with bumped context
//! 4. **Calculate sensitivity**: `(bumped_pv - base_pv) / bump_bp`
//! 5. **Store results**: Total + optional series in [`MetricContext`]
//!

use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{CurveDependencies, Instrument, RatesCurveKind};
use crate::metrics::{MetricContext, MetricId};
use crate::metrics::traits::MetricCalculator;
use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use hashbrown::HashMap;
use std::marker::PhantomData;

/// Standard IR key-rate buckets in years.
pub fn standard_ir_dv01_buckets() -> Vec<f64> {
    vec![0.25, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0]
}

/// DV01 calculation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dv01ComputationMode {
    /// Single scalar from parallel bump of all curves together.
    ParallelCombined,
    /// Per-curve parallel bumps (stored as series).
    ParallelPerCurve,
    /// Key-rate buckets per curve.
    KeyRatePerCurve,
}

/// Which curves to include in DV01 calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveSelection {
    /// Only primary discount curve.
    DiscountOnly,
    /// Discount + all forward curves.
    DiscountAndForward,
    /// All rate-sensitive curves (discount + forward + extra discount).
    AllRateCurves,
}

/// Configuration for DV01 calculations.
#[derive(Clone, Debug)]
pub struct Dv01CalculatorConfig {
    /// Computation mode (parallel vs bucketed).
    pub mode: Dv01ComputationMode,
    /// Which curves to bump.
    pub curve_selection: CurveSelection,
    /// Bucket times for key-rate DV01 (in years).
    pub buckets: Vec<f64>,
}

impl Default for Dv01CalculatorConfig {
    fn default() -> Self {
        Self {
            mode: Dv01ComputationMode::KeyRatePerCurve,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: standard_ir_dv01_buckets(),
        }
    }
}

impl Dv01CalculatorConfig {
    /// Create config for parallel DV01 (all curves together).
    pub fn parallel_combined() -> Self {
        Self {
            mode: Dv01ComputationMode::ParallelCombined,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: vec![],
        }
    }
    
    /// Create config for parallel DV01 per curve.
    pub fn parallel_per_curve() -> Self {
        Self {
            mode: Dv01ComputationMode::ParallelPerCurve,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: vec![],
        }
    }
    
    /// Create config for key-rate DV01.
    pub fn key_rate() -> Self {
        Self::default()
    }
}

/// Unified DV01 calculator supporting all computation modes.
pub struct UnifiedDv01Calculator<I> {
    config: Dv01CalculatorConfig,
    _phantom: PhantomData<I>,
}

impl<I> UnifiedDv01Calculator<I> {
    /// Create a new calculator with the given configuration.
    pub fn new(config: Dv01CalculatorConfig) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }
}

impl<I> Default for UnifiedDv01Calculator<I> {
    fn default() -> Self {
        Self::new(Dv01CalculatorConfig::default())
    }
}

impl<I> MetricCalculator for UnifiedDv01Calculator<I>
where
    I: Instrument + HasDiscountCurve + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;
        
        // Get bump size from overrides or default
        let bump_bp = context
            .pricing_overrides
            .as_ref()
            .and_then(|po| po.rate_bump_bp)
            .unwrap_or(1.0);
        
        // Collect curves based on configuration
        let curves = self.collect_curves(instrument, context.curves.as_ref())?;
        
        // Compute DV01 based on mode
        match self.config.mode {
            Dv01ComputationMode::ParallelCombined => {
                self.compute_parallel_combined(context, &curves, bump_bp)
            }
            Dv01ComputationMode::ParallelPerCurve => {
                self.compute_parallel_per_curve(context, &curves, bump_bp)
            }
            Dv01ComputationMode::KeyRatePerCurve => {
                self.compute_key_rate_per_curve(context, &curves, bump_bp)
            }
        }
    }
}

impl<I> UnifiedDv01Calculator<I>
where
    I: Instrument + HasDiscountCurve + CurveDependencies + 'static,
{
    /// Collect curves based on configuration and what exists in the market.
    fn collect_curves(
        &self,
        instrument: &I,
        market: &MarketContext,
    ) -> finstack_core::Result<Vec<(CurveId, RatesCurveKind)>> {
        let deps = instrument.curve_dependencies();
        let mut curves = Vec::new();
        
        match self.config.curve_selection {
            CurveSelection::DiscountOnly => {
                // Only primary discount curve
                let primary = instrument.discount_curve_id();
                if market.get_discount_ref(primary.as_str()).is_ok() {
                    curves.push((primary.clone(), RatesCurveKind::Discount));
                }
            }
            CurveSelection::DiscountAndForward => {
                // Discount curves
                for curve_id in &deps.discount_curves {
                    if market.get_discount_ref(curve_id.as_str()).is_ok() {
                        curves.push((curve_id.clone(), RatesCurveKind::Discount));
                    }
                }
                // Forward curves
                for curve_id in &deps.forward_curves {
                    if market.get_forward_ref(curve_id.as_str()).is_ok() {
                        curves.push((curve_id.clone(), RatesCurveKind::Forward));
                    }
                }
            }
            CurveSelection::AllRateCurves => {
                // All curves from dependencies
                for (curve_id, kind) in deps.all_with_kind() {
                    match kind {
                        RatesCurveKind::Discount => {
                            if market.get_discount_ref(curve_id.as_str()).is_ok() {
                                curves.push((curve_id, kind));
                            }
                        }
                        RatesCurveKind::Forward => {
                            if market.get_forward_ref(curve_id.as_str()).is_ok() {
                                curves.push((curve_id, kind));
                            }
                        }
                        RatesCurveKind::Credit => {
                            // Skip credit curves for DV01
                        }
                    }
                }
            }
        }
        
        if curves.is_empty() {
            tracing::warn!(
                instrument_type = std::any::type_name::<I>(),
                "UnifiedDv01Calculator: No rate curves found in market context, returning 0.0"
            );
        }
        
        Ok(curves)
    }
    
    /// Compute parallel DV01 with all curves bumped together.
    fn compute_parallel_combined(
        &self,
        context: &mut MetricContext,
        curves: &[(CurveId, RatesCurveKind)],
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        if curves.is_empty() {
            return Ok(0.0);
        }
        
        let base_ctx = context.curves.as_ref();
        let as_of = context.as_of;
        
        // Recalculate base PV (see dv01.rs module docs for rationale)
        let base_pv = context.instrument.value(base_ctx, as_of)?;
        
        // Create bump specs for all curves
        let mut bumps = HashMap::new();
        for (curve_id, _kind) in curves {
            bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
        }
        
        // Apply all bumps together
        let bumped_ctx = base_ctx.bump(bumps)?;
        let bumped_pv = context.instrument.value(&bumped_ctx, as_of)?;
        
        let dv01 = calculate_dv01(base_pv, bumped_pv, bump_bp);
        Ok(dv01)
    }
    
    /// Compute parallel DV01 per curve and store as series.
    fn compute_parallel_per_curve(
        &self,
        context: &mut MetricContext,
        curves: &[(CurveId, RatesCurveKind)],
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        if curves.is_empty() {
            return Ok(0.0);
        }
        
        let base_ctx = context.curves.as_ref();
        let as_of = context.as_of;
        
        // Recalculate base PV
        let base_pv = context.instrument.value(base_ctx, as_of)?;
        
        // Bump each curve individually
        let mut series = Vec::new();
        let mut total_dv01 = 0.0;
        
        for (curve_id, _kind) in curves {
            let mut bumps = HashMap::new();
            bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
            
            let bumped_ctx = base_ctx.bump(bumps)?;
            let bumped_pv = context.instrument.value(&bumped_ctx, as_of)?;
            let dv01 = calculate_dv01(base_pv, bumped_pv, bump_bp);
            
            series.push((curve_id.as_str().to_string(), dv01));
            total_dv01 += dv01;
        }
        
        // Store per-curve series
        context.store_bucketed_series(MetricId::BucketedDv01, series);
        
        Ok(total_dv01)
    }
    
    /// Compute key-rate DV01 per curve.
    fn compute_key_rate_per_curve(
        &self,
        context: &mut MetricContext,
        curves: &[(CurveId, RatesCurveKind)],
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        if curves.is_empty() {
            return Ok(0.0);
        }
        
        let mut total_dv01 = 0.0;
        
        // Process each curve
        for (i, (curve_id, _kind)) in curves.iter().enumerate() {
            let (metric_id, should_compute) = if curves.len() == 1 {
                // Single curve: use standard key
                (MetricId::BucketedDv01, true)
            } else if i == 0 {
                // Multiple curves: primary curve gets both standard and curve-specific keys
                // But we only compute once and store under both keys
                (MetricId::BucketedDv01, true)
            } else {
                // Non-primary curves: use curve-specific key
                (MetricId::custom(format!("bucketed_dv01::{}", curve_id.as_str())), true)
            };
            
            if should_compute {
                let curve_total = self.compute_key_rate_for_curve(
                    context,
                    curve_id,
                    metric_id.clone(),
                    bump_bp,
                )?;
                
                total_dv01 += curve_total;
                
                // For primary curve in multi-curve instrument, also copy to curve-specific key
                if i == 0 && curves.len() > 1 {
                    let curve_specific_id = MetricId::custom(format!("bucketed_dv01::{}", curve_id.as_str()));
                    if let Some(series) = context.get_series(&metric_id) {
                        context.store_bucketed_series(curve_specific_id, series.clone());
                    }
                }
            }
        }
        
        Ok(total_dv01)
    }
    
    /// Compute key-rate DV01 for a single curve.
    fn compute_key_rate_for_curve(
        &self,
        context: &mut MetricContext,
        curve_id: &CurveId,
        metric_id: MetricId,
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        let base_ctx = context.curves.as_ref();
        let as_of = context.as_of;
        
        // Recalculate base PV
        let base_pv = context.instrument.value(base_ctx, as_of)?;
        
        let mut series: Vec<(String, f64)> = Vec::new();
        
        for &time_years in &self.config.buckets {
            let label = format_bucket_label(time_years);
            
            // Create key-rate bump spec
            let mut bumps = HashMap::new();
            bumps.insert(curve_id.clone(), BumpSpec::key_rate_bp(time_years, bump_bp));
            
            let bumped_ctx = base_ctx.bump(bumps)?;
            let bumped_pv = context.instrument.value(&bumped_ctx, as_of)?;
            let dv01 = calculate_dv01(base_pv, bumped_pv, bump_bp);
            
            series.push((label, dv01));
        }
        
        context.store_bucketed_series(metric_id, series.clone());
        let total: f64 = series.iter().map(|(_, v)| *v).sum();
        Ok(total)
    }
}

// Helper functions

/// Generate bucket label from years.
#[inline]
fn format_bucket_label(years: f64) -> String {
    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
}

/// Calculate DV01 from PV changes.
#[inline]
fn calculate_dv01(base_pv: Money, bumped_pv: Money, bump_bp: f64) -> f64 {
    (bumped_pv.amount() - base_pv.amount()) / bump_bp
}
