//! Unified DV01 calculator supporting parallel and key-rate sensitivities.
//!
//! This module provides a single, flexible DV01 calculator with two mathematically
//! correct key-rate methods:
//!
//! 1. **Triangular Zero-Rate**: Fast, uses triangular weights on bucket grid
//!
//! Both methods ensure: **sum of bucketed DV01 ≈ parallel DV01**
//!
//! # Units and Sign Convention
//!
//! - **DV01 is expressed in currency units per basis point (1bp = 0.0001)**
//! - A DV01 of -100 means the instrument loses $100 when rates rise by 1bp
//! - Positive DV01: instrument gains value when rates rise (rare, e.g., short positions)
//! - Negative DV01: instrument loses value when rates rise (typical for long bonds)
//!
//! # Key Features
//!
//! - **Type-safe curve discovery**: Uses [`CurveDependencies`] trait to discover curves at compile time
//! - **Mathematically correct**: Triangular weights partition unity across bucket grid
//! - **Multiple curve types**: Handles discount, forward, and credit curves
//! - **Par-rate option**: Re-bootstrap curve for exact sum-to-parallel behavior
//!
//! # Quick Start
//!
//! For DV01 calculations, use the [`MetricId::Dv01`] or [`MetricId::BucketedDv01`]
//! metrics via the [`Instrument::price_with_metrics`] method:
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::{Bond, Instrument};
//! use finstack_valuations::metrics::MetricId;
//!
//! let bond = Bond::example();
//! let result = bond.price_with_metrics(&market, as_of, &[MetricId::Dv01])?;
//! // DV01 is in currency units per 1bp rate move
//! ```

use crate::instruments::common::traits::{CurveDependencies, Instrument, RatesCurveKind};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::MetricCalculator;
use crate::metrics::{MetricContext, MetricId};

use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use std::marker::PhantomData;

// =============================================================================
// Standard Buckets
// =============================================================================

/// Standard IR key-rate buckets in years.
///
/// Returns the industry-standard interest rate sensitivity buckets used for
/// key-rate DV01 calculations. These buckets cover the full maturity spectrum
/// from 3 months to 30 years, matching standard market conventions.
///
/// # Returns
///
/// Vector of bucket maturities in years: [0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30]
///
/// # Examples
///
/// ```rust,ignore
/// // This function is internal - use MetricId::BucketedDv01 for public API
/// use finstack_valuations::metrics::sensitivities::dv01::standard_ir_dv01_buckets;
///
/// let buckets = standard_ir_dv01_buckets();
/// assert_eq!(buckets.len(), 11);
/// assert_eq!(buckets[0], 0.25); // 3 months
/// ```
pub fn standard_ir_dv01_buckets() -> Vec<f64> {
    sens_config::STANDARD_BUCKETS_YEARS.to_vec()
}

// =============================================================================
// Configuration Types
// =============================================================================

/// DV01 calculation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dv01ComputationMode {
    /// Single scalar from parallel bump of all curves together.
    ParallelCombined,
    /// Per-curve parallel bumps (stored as series).
    ParallelPerCurve,
    /// Key-rate buckets per curve using triangular zero-rate bumps.
    KeyRateTriangular,
}

/// Configuration for DV01 calculations.
#[derive(Clone)]
pub struct Dv01CalculatorConfig {
    /// Computation mode (parallel vs bucketed, triangular vs par-rate).
    pub mode: Dv01ComputationMode,
    /// Bucket times for key-rate DV01 (in years).
    pub buckets: Vec<f64>,
}

impl std::fmt::Debug for Dv01CalculatorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dv01CalculatorConfig")
            .field("mode", &self.mode)
            .field("buckets", &self.buckets)
            .finish()
    }
}

impl Default for Dv01CalculatorConfig {
    fn default() -> Self {
        Self {
            mode: Dv01ComputationMode::KeyRateTriangular,
            buckets: standard_ir_dv01_buckets(),
        }
    }
}

impl Dv01CalculatorConfig {
    /// Create config for parallel DV01 (all curves together).
    pub fn parallel_combined() -> Self {
        Self {
            mode: Dv01ComputationMode::ParallelCombined,
            buckets: vec![],
        }
    }

    /// Create config for parallel DV01 per curve.
    pub fn parallel_per_curve() -> Self {
        Self {
            mode: Dv01ComputationMode::ParallelPerCurve,
            buckets: vec![],
        }
    }

    /// Create config for triangular key-rate DV01.
    ///
    /// This is the default and recommended method for most use cases.
    /// Uses triangular weights on the bucket grid, ensuring sum ≈ parallel within ~0.1%.
    pub fn triangular_key_rate() -> Self {
        Self {
            mode: Dv01ComputationMode::KeyRateTriangular,
            ..Self::default()
        }
    }
}

// =============================================================================
// Unified DV01 Calculator
// =============================================================================

/// Unified DV01 calculator supporting all computation modes.
///
/// This calculator provides two mathematically correct key-rate methods:
///
/// 1. **Triangular Zero-Rate** (`KeyRateTriangular`): Uses triangular weights
///    defined by the bucket grid, ensuring sum of bucketed DV01 ≈ parallel DV01.
///
/// 2. **Par-Rate Bumping** (`KeyRateParRate`): Bumps par rates of calibration
///    instruments and re-bootstraps, ensuring exact sum = parallel.
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
    I: Instrument + CurveDependencies + 'static,
{
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let instrument: &I = context.instrument_as()?;

        // Resolve bump size from `FinstackConfig` (user-facing, reproducible).
        let bump_bp = sens_config::from_finstack_config_or_default(context.config())?.rate_bump_bp;

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
            Dv01ComputationMode::KeyRateTriangular => {
                self.compute_key_rate_triangular(context, &curves, bump_bp)
            }
        }
    }
}

impl<I> UnifiedDv01Calculator<I>
where
    I: Instrument + CurveDependencies + 'static,
{
    /// Collect curves based on configuration and what exists in the market.
    fn collect_curves(
        &self,
        instrument: &I,
        market: &MarketContext,
    ) -> finstack_core::Result<Vec<(CurveId, RatesCurveKind)>> {
        let deps = instrument.curve_dependencies();
        let mut curves = Vec::new();
        for (curve_id, kind) in deps.all_with_kind() {
            match kind {
                RatesCurveKind::Discount => {
                    if market.get_discount(curve_id.as_str()).is_ok() {
                        curves.push((curve_id, kind));
                    }
                }
                RatesCurveKind::Forward => {
                    if market.get_forward(curve_id.as_str()).is_ok() {
                        curves.push((curve_id, kind));
                    }
                }
                RatesCurveKind::Credit => {
                    // Skip credit curves for DV01
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
        // Use value_raw for high-precision sensitivity calculations
        let base_pv = context.instrument.value_raw(base_ctx, as_of)?;

        let bumps: Vec<MarketBump> = curves
            .iter()
            .map(|(curve_id, _kind)| MarketBump::Curve {
                id: curve_id.clone(),
                spec: BumpSpec::parallel_bp(bump_bp),
            })
            .collect();

        let bumped_ctx = base_ctx.bump(bumps)?;
        let bumped_pv = context.instrument.value_raw(&bumped_ctx, as_of)?;

        let dv01 = calculate_dv01_raw(base_pv, bumped_pv, bump_bp);
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
        // Use value_raw for high-precision sensitivity calculations
        let base_pv = context.instrument.value_raw(base_ctx, as_of)?;

        let mut series = Vec::new();
        let mut total_dv01 = 0.0;

        for (curve_id, _kind) in curves {
            let bumped_ctx = base_ctx.bump([MarketBump::Curve {
                id: curve_id.clone(),
                spec: BumpSpec::parallel_bp(bump_bp),
            }])?;
            let bumped_pv = context.instrument.value_raw(&bumped_ctx, as_of)?;
            let dv01 = calculate_dv01_raw(base_pv, bumped_pv, bump_bp);

            series.push((curve_id.as_str().to_string(), dv01));
            total_dv01 += dv01;
        }

        context.store_bucketed_series(MetricId::BucketedDv01, series);
        Ok(total_dv01)
    }

    /// Compute key-rate DV01 using triangular zero-rate bumps.
    ///
    /// This method uses triangular weights defined by the bucket grid (not curve knots),
    /// ensuring that the sum of bucketed DV01 equals parallel DV01.
    fn compute_key_rate_triangular(
        &self,
        context: &mut MetricContext,
        curves: &[(CurveId, RatesCurveKind)],
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        if curves.is_empty() {
            return Ok(0.0);
        }

        let mut total_dv01 = 0.0;

        for (i, (curve_id, _kind)) in curves.iter().enumerate() {
            let (metric_id, should_compute) = if curves.len() == 1 || i == 0 {
                (MetricId::BucketedDv01, true)
            } else {
                (
                    MetricId::custom(format!("bucketed_dv01::{}", curve_id.as_str())),
                    true,
                )
            };

            if should_compute {
                let curve_total = self.compute_triangular_for_curve(
                    context,
                    curve_id,
                    metric_id.clone(),
                    bump_bp,
                )?;

                total_dv01 += curve_total;

                if i == 0 && curves.len() > 1 {
                    let curve_specific_id =
                        MetricId::custom(format!("bucketed_dv01::{}", curve_id.as_str()));
                    if let Some(series) = context.get_series(&metric_id) {
                        context.store_bucketed_series(curve_specific_id, series.clone());
                    }
                }
            }
        }

        Ok(total_dv01)
    }

    /// Compute triangular key-rate DV01 for a single curve.
    ///
    /// Uses triangular weights based on the bucket grid, ensuring proper partitioning.
    fn compute_triangular_for_curve(
        &self,
        context: &mut MetricContext,
        curve_id: &CurveId,
        metric_id: MetricId,
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        let base_ctx = context.curves.as_ref();
        let as_of = context.as_of;
        // Use value_raw for high-precision sensitivity calculations
        let base_pv = context.instrument.value_raw(base_ctx, as_of)?;

        let buckets = &self.config.buckets;
        let mut series: Vec<(String, f64)> = Vec::new();

        for (i, &target_time) in buckets.iter().enumerate() {
            let label = format_bucket_label(target_time);

            // Determine bucket neighbors for proper triangular weight
            // This is the key to ensuring sum of bucketed DV01 = parallel DV01
            let prev_bucket = if i == 0 { 0.0 } else { buckets[i - 1] };
            let next_bucket = if i == buckets.len() - 1 {
                f64::INFINITY // Last bucket extends to infinity
            } else {
                buckets[i + 1]
            };

            // Create triangular key-rate bump with proper neighbors
            let bumped_ctx = base_ctx.bump([MarketBump::Curve {
                id: curve_id.clone(),
                spec: BumpSpec::triangular_key_rate_bp(
                    prev_bucket,
                    target_time,
                    next_bucket,
                    bump_bp,
                ),
            }])?;
            let bumped_pv = context.instrument.value_raw(&bumped_ctx, as_of)?;
            let dv01 = calculate_dv01_raw(base_pv, bumped_pv, bump_bp);

            series.push((label, dv01));
        }

        context.store_bucketed_series(metric_id, series.clone());
        let total: f64 = series.iter().map(|(_, v)| *v).sum();
        Ok(total)
    }
}

// Re-export for backward compatibility and convenience
pub use super::config::format_bucket_label;

/// Calculate DV01 from PV changes (high-precision f64 version).
///
/// Uses raw f64 values to avoid Money rounding precision loss in sensitivity calculations.
///
/// # Units
///
/// Returns DV01 in **currency units per basis point**. For example:
/// - If `base_pv = 1_000_000` and `bumped_pv = 999_500` with `bump_bp = 1.0`
/// - DV01 = (999_500 - 1_000_000) / 1.0 = -500
/// - This means the instrument loses $500 per 1bp rate increase
///
/// # Arguments
///
/// * `base_pv` - Present value before bump (in currency units)
/// * `bumped_pv` - Present value after bump (in currency units)
/// * `bump_bp` - Bump size in basis points (typically 1.0)
#[inline]
fn calculate_dv01_raw(base_pv: f64, bumped_pv: f64, bump_bp: f64) -> f64 {
    (bumped_pv - base_pv) / bump_bp
}
