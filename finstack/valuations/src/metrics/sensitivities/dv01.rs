//! Unified DV01 calculator supporting parallel and key-rate sensitivities.
//!
//! This module provides a single, flexible DV01 calculator with two mathematically
//! correct key-rate methods:
//!
//! 1. **Triangular Zero-Rate**: Fast, uses triangular weights on bucket grid
//! 2. **Par-Rate Bumping**: Gold standard, re-bootstraps curve from bumped quotes
//!
//! Both methods ensure: **sum of bucketed DV01 ≈ parallel DV01**
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
//! ## Example 1: Key-Rate DV01 for a Bond (Triangular Method)
//!
//! ```ignore
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::metrics::{
//!     UnifiedDv01Calculator, Dv01CalculatorConfig, MetricContext
//! };
//!
//! let calculator = UnifiedDv01Calculator::<Bond>::new(
//!     Dv01CalculatorConfig::triangular_key_rate()
//! );
//!
//! let total_dv01 = calculator.calculate(&mut context)?;
//! // Sum of bucketed DV01 will equal parallel DV01 within ~0.1%
//! ```
//!
//! ## Example 2: Par-Rate DV01 (Exact Sum)
//!
//! ```ignore
//! use finstack_valuations::metrics::{Dv01CalculatorConfig, ParRateContext};
//!
//! let par_context = ParRateContext::new(quotes, calibrator, base_context);
//! let calculator = UnifiedDv01Calculator::<Bond>::new(
//!     Dv01CalculatorConfig::par_rate_key_rate(par_context)
//! );
//!
//! let total_dv01 = calculator.calculate(&mut context)?;
//! // Sum of bucketed DV01 will equal parallel DV01 within numerical precision
//! ```

use crate::calibration::bumps::rates::{bump_discount_curve, find_closest_quote};
use crate::calibration::bumps::BumpRequest;
use crate::calibration::api::schema::DiscountCurveParams;
use crate::calibration::domain::quotes::RatesQuote;
use crate::calibration::CalibrationConfig;
use crate::instruments::common::pricing::HasDiscountCurve;
use crate::instruments::common::traits::{CurveDependencies, Instrument, RatesCurveKind};
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::MetricCalculator;
use crate::metrics::{MetricContext, MetricId};

use finstack_core::market_data::bumps::BumpSpec;
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use hashbrown::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

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
/// ```rust
/// use finstack_valuations::metrics::standard_ir_dv01_buckets;
///
/// let buckets = standard_ir_dv01_buckets();
/// assert_eq!(buckets.len(), 11);
/// assert_eq!(buckets[0], 0.25); // 3 months
/// assert_eq!(buckets[10], 30.0); // 30 years
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
    /// Key-rate buckets per curve using par-rate bumping with re-bootstrap.
    KeyRateParRate,
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

/// Context for par-rate DV01 calculation.
///
/// Par-rate DV01 is the gold standard for interest rate risk. It bumps the
/// par rate of calibration instruments and re-bootstraps the curve, ensuring
/// exact sum-to-parallel behavior (within numerical precision).
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::metrics::ParRateContext;
/// use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
///
/// let context = ParRateContext::new(quotes, calibrator, base_market);
/// ```
#[derive(Clone)]
pub struct ParRateContext {
    /// Calibration quotes sorted by maturity
    pub quotes: Vec<RatesQuote>,
    /// Discount-curve step parameters describing how to rebuild the curve.
    pub params: DiscountCurveParams,
    /// Global calibration settings (tolerances, bounds).
    pub settings: CalibrationConfig,
    /// Base market context (without the curve being calibrated)
    pub base_context: MarketContext,
}

impl ParRateContext {
    /// Create a new par-rate context.
    ///
    /// Quotes are automatically sorted by maturity date.
    pub fn new(
        quotes: Vec<RatesQuote>,
        params: DiscountCurveParams,
        base_context: MarketContext,
    ) -> Self {
        let mut sorted_quotes = quotes;
        sorted_quotes.sort_by(|a, b| {
            a.maturity_date()
                .partial_cmp(&b.maturity_date())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self {
            quotes: sorted_quotes,
            params,
            settings: CalibrationConfig::default(),
            base_context,
        }
    }

    /// Override plan-level calibration settings (tolerances, bounds).
    #[must_use]
    pub fn with_settings(mut self, settings: CalibrationConfig) -> Self {
        self.settings = settings;
        self
    }
}

/// Configuration for DV01 calculations.
#[derive(Clone)]
pub struct Dv01CalculatorConfig {
    /// Computation mode (parallel vs bucketed, triangular vs par-rate).
    pub mode: Dv01ComputationMode,
    /// Which curves to bump.
    pub curve_selection: CurveSelection,
    /// Bucket times for key-rate DV01 (in years).
    pub buckets: Vec<f64>,
    /// Calibration context for par-rate method (required for KeyRateParRate mode).
    pub par_rate_context: Option<Arc<ParRateContext>>,
}

impl std::fmt::Debug for Dv01CalculatorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dv01CalculatorConfig")
            .field("mode", &self.mode)
            .field("curve_selection", &self.curve_selection)
            .field("buckets", &self.buckets)
            .field("par_rate_context", &self.par_rate_context.is_some())
            .finish()
    }
}

impl Default for Dv01CalculatorConfig {
    fn default() -> Self {
        Self {
            mode: Dv01ComputationMode::KeyRateTriangular,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: standard_ir_dv01_buckets(),
            par_rate_context: None,
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
            par_rate_context: None,
        }
    }

    /// Create config for parallel DV01 per curve.
    pub fn parallel_per_curve() -> Self {
        Self {
            mode: Dv01ComputationMode::ParallelPerCurve,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: vec![],
            par_rate_context: None,
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

    /// Create config for key-rate DV01 (alias for triangular_key_rate).
    pub fn key_rate() -> Self {
        Self::triangular_key_rate()
    }

    /// Create config for par-rate key-rate DV01.
    ///
    /// This is the gold standard for interest rate risk. Requires a `ParRateContext`
    /// containing the calibration quotes used to build the curve.
    ///
    /// **Pros**: Exact sum = parallel (within numerical precision)
    /// **Cons**: Slower, requires calibration context
    pub fn par_rate_key_rate(context: ParRateContext) -> Self {
        Self {
            mode: Dv01ComputationMode::KeyRateParRate,
            curve_selection: CurveSelection::AllRateCurves,
            buckets: standard_ir_dv01_buckets(),
            par_rate_context: Some(Arc::new(context)),
        }
    }

    /// Set custom buckets.
    pub fn with_buckets(mut self, buckets: Vec<f64>) -> Self {
        self.buckets = buckets;
        self
    }

    /// Set curve selection.
    pub fn with_curve_selection(mut self, selection: CurveSelection) -> Self {
        self.curve_selection = selection;
        self
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
    I: Instrument + HasDiscountCurve + CurveDependencies + 'static,
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
            Dv01ComputationMode::KeyRateParRate => {
                self.compute_key_rate_par_rate(context, &curves, bump_bp)
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
                let primary = instrument.discount_curve_id();
                if market.get_discount_ref(primary.as_str()).is_ok() {
                    curves.push((primary.clone(), RatesCurveKind::Discount));
                }
            }
            CurveSelection::DiscountAndForward => {
                for curve_id in &deps.discount_curves {
                    if market.get_discount_ref(curve_id.as_str()).is_ok() {
                        curves.push((curve_id.clone(), RatesCurveKind::Discount));
                    }
                }
                for curve_id in &deps.forward_curves {
                    if market.get_forward_ref(curve_id.as_str()).is_ok() {
                        curves.push((curve_id.clone(), RatesCurveKind::Forward));
                    }
                }
            }
            CurveSelection::AllRateCurves => {
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
        // Use value_raw for high-precision sensitivity calculations
        let base_pv = context.instrument.value_raw(base_ctx, as_of)?;

        let mut bumps = HashMap::new();
        for (curve_id, _kind) in curves {
            bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));
        }

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
            let mut bumps = HashMap::new();
            bumps.insert(curve_id.clone(), BumpSpec::parallel_bp(bump_bp));

            let bumped_ctx = base_ctx.bump(bumps)?;
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
            let mut bumps = HashMap::new();
            bumps.insert(
                curve_id.clone(),
                BumpSpec::triangular_key_rate_bp(prev_bucket, target_time, next_bucket, bump_bp),
            );

            let bumped_ctx = base_ctx.bump(bumps)?;
            let bumped_pv = context.instrument.value_raw(&bumped_ctx, as_of)?;
            let dv01 = calculate_dv01_raw(base_pv, bumped_pv, bump_bp);

            series.push((label, dv01));
        }

        context.store_bucketed_series(metric_id, series.clone());
        let total: f64 = series.iter().map(|(_, v)| *v).sum();
        Ok(total)
    }

    /// Compute key-rate DV01 using par-rate bumping with re-bootstrap.
    ///
    /// This is the gold standard for interest rate risk. Bumps the par rate of
    /// calibration instruments and re-bootstraps the curve.
    fn compute_key_rate_par_rate(
        &self,
        context: &mut MetricContext,
        curves: &[(CurveId, RatesCurveKind)],
        bump_bp: f64,
    ) -> finstack_core::Result<f64> {
        let par_context = self.config.par_rate_context.as_ref().ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::error::InputError::NotFound {
                id: "ParRateContext required for par-rate DV01. Use Dv01CalculatorConfig::par_rate_key_rate()".to_string(),
            })
        })?;

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
                let curve_total = self.compute_par_rate_for_curve(
                    context,
                    curve_id,
                    metric_id.clone(),
                    bump_bp,
                    par_context,
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

    /// Compute par-rate key-rate DV01 for a single curve.
    fn compute_par_rate_for_curve(
        &self,
        context: &mut MetricContext,
        _curve_id: &CurveId,
        metric_id: MetricId,
        bump_bp: f64,
        par_context: &ParRateContext,
    ) -> finstack_core::Result<f64> {
        let base_ctx = context.curves.as_ref();
        let as_of = context.as_of;
        // Use value_raw for high-precision sensitivity calculations
        let base_pv = context.instrument.value_raw(base_ctx, as_of)?;

        let buckets = &self.config.buckets;
        let mut series: Vec<(String, f64)> = Vec::new();

        for &target_time in buckets {
            let label = format_bucket_label(target_time);

            // Find the quote closest to this bucket maturity
            let quote_index = find_closest_quote(&par_context.quotes, target_time, as_of);

            let dv01 = if quote_index.is_some() {
                // Use shared bumping logic
                let bump_request = BumpRequest::Tenors(vec![(target_time, bump_bp)]);

                match bump_discount_curve(
                    &par_context.quotes,
                    &par_context.params,
                    &par_context.base_context,
                    &bump_request,
                ) {
                    Ok(bumped_curve) => {
                        // Replace curve in context and reprice
                        let bumped_ctx = base_ctx.clone().insert_discount(bumped_curve);
                        let bumped_pv = context.instrument.value_raw(&bumped_ctx, as_of)?;
                        calculate_dv01_raw(base_pv, bumped_pv, bump_bp)
                    }
                    Err(e) => {
                        tracing::warn!(
                            bucket = label,
                            error = %e,
                            "Par-rate calibration failed for bucket, using 0.0"
                        );
                        0.0
                    }
                }
            } else {
                0.0
            };

            series.push((label, dv01));
        }

        context.store_bucketed_series(metric_id, series.clone());
        let total: f64 = series.iter().map(|(_, v)| *v).sum();
        Ok(total)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Standard IR bucket labels matching standard_ir_dv01_buckets() order.
const IR_BUCKET_LABELS: [&str; 11] = [
    "3m", "6m", "1y", "2y", "3y", "5y", "7y", "10y", "15y", "20y", "30y",
];

/// Generate bucket label from years.
#[inline]
pub fn format_bucket_label(years: f64) -> String {
    let standard_buckets = standard_ir_dv01_buckets();
    for (i, &bucket_time) in standard_buckets.iter().enumerate() {
        if (years - bucket_time).abs() < 0.01 {
            return IR_BUCKET_LABELS[i].to_string();
        }
    }

    if years < 1.0 {
        format!("{:.0}m", (years * 12.0).round())
    } else {
        format!("{:.0}y", years)
    }
}

/// Calculate DV01 from PV changes (high-precision f64 version).
///
/// Uses raw f64 values to avoid Money rounding precision loss in sensitivity calculations.
#[inline]
fn calculate_dv01_raw(base_pv: f64, bumped_pv: f64, bump_bp: f64) -> f64 {
    (bumped_pv - base_pv) / bump_bp
}

// Helpers moved to crate::calibration::bumps::rates
