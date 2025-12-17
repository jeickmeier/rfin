//! Yield curve bootstrapping from market instruments.
//!
//! Implements market-standard multi-curve discount curve calibration using
//! deposits and OIS swaps. Forward curves are calibrated separately.
//!
//! # Module Structure
//!
//! - [`DiscountCurveCalibrator`]: Main calibrator struct and configuration
//! - [`bootstrap`]: Sequential bootstrapping algorithm
//! - [`global_solve`]: Global optimization algorithm
//!
//! # Features
//!
//! - **Adaptive scan grid**: Supports negative rate environments (DF > 1.0)
//! - **Settlement conventions**: Currency-specific T+0/T+2 handling
//! - **Day-count alignment**: Validates consistency between quotes and curve
//! - **Pre-validation**: Checks curve dependencies before bootstrap starts
//! - **Extrapolation policy**: Configurable flat-forward or flat-zero extrapolation
//!
//! # Market Conventions
//!
//! Default settlement by currency:
//! - **USD/EUR/JPY/CHF**: T+2
//! - **GBP**: T+0
//! - **AUD/CAD**: T+1

mod bootstrap;
mod global_solve;

use crate::calibration::config::CalibrationMethod;
use crate::calibration::pricing::CalibrationPricer;
use crate::calibration::quotes::RatesQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport, Calibrator, MultiCurveConfig};
use finstack_core::config::FinstackConfig;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::prelude::*;
use finstack_core::types::CurveId;

use serde::{Deserialize, Serialize};

/// Discount curve bootstrapper with market-standard conventions.
///
/// Implements sequential bootstrapping for OIS discount curves from deposits
/// and overnight-indexed swaps. Supports negative rate environments and
/// configurable settlement/extrapolation conventions.
///
/// # Example
///
/// ```ignore
/// use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
/// use finstack_core::currency::Currency;
///
/// let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
///     .with_extrapolation(ExtrapolationPolicy::FlatForward)
///     .with_include_spot_knot(true);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscountCurveCalibrator {
    /// Curve identifier
    pub curve_id: CurveId,
    /// Base date for the curve
    pub base_date: finstack_core::dates::Date,
    /// Interpolation used during solving and for the final curve
    pub solve_interp: InterpStyle,
    /// Extrapolation policy for the final curve
    #[serde(default = "default_extrapolation")]
    pub extrapolation: ExtrapolationPolicy,
    /// Calibration configuration (includes multi-curve settings and calibration_method)
    pub config: CalibrationConfig,
    /// Currency for the curve
    pub currency: Currency,
    /// Discount curve ID used during calibration instrument pricing.
    ///
    /// Defaults to the calibrator's `curve_id`. For multi-curve setups where
    /// this calibrator builds a forward curve, set this to your OIS/collateral
    /// discount curve ID.
    #[serde(default)]
    pub discount_curve_id: Option<CurveId>,
    /// Forward curve ID used for floating leg projections.
    ///
    /// Defaults to the calibrator's `curve_id` for OIS calibration where
    /// discount = forward. For tenor curve calibration, set this to the
    /// specific forward curve being built.
    #[serde(default)]
    pub forward_curve_id: Option<CurveId>,
    /// Use OIS-specific logic for swap pricing.
    ///
    /// When `true` (default), swaps use overnight-indexed compounding conventions
    /// and the discount curve as both discount and projection curve.
    /// When `false`, swaps use simple compounding with separate forward curve.
    #[serde(default = "default_use_ois_logic")]
    pub use_ois_logic: bool,
    /// Include an explicit spot knot at `t_spot` with DF=1.0.
    ///
    /// When `true` (default for OIS), the curve includes a knot at the settlement
    /// date with DF=1.0, making the spot-starting convention explicit on the curve
    /// timeline. This is market-standard for OIS curves where instruments settle
    /// on spot (T+2 for USD, T+0 for GBP, etc.).
    ///
    /// When `false`, the curve starts at base_date with only DF(0)=1.0.
    /// Use this for non-OIS curves or when base_date equals settlement_date.
    #[serde(default)]
    pub include_spot_knot: bool,
}

fn default_use_ois_logic() -> bool {
    true
}

fn default_extrapolation() -> ExtrapolationPolicy {
    ExtrapolationPolicy::FlatForward
}

pub(crate) fn default_curve_day_count(currency: Currency) -> DayCount {
    // Use ACT/365F to align with the core discount curve builder defaults and
    // scenario expectations (time expressed directly in year fractions).
    let _ = currency; // currency-based overrides can be added if needed
    DayCount::Act365F
}

impl DiscountCurveCalibrator {
    /// Create a new discount curve calibrator with currency-appropriate defaults.
    ///
    /// Default settings:
    /// - Interpolation: MonotoneConvex (arbitrage-free forwards)
    /// - Extrapolation: FlatForward (standard for risk)
    /// - Settlement: Currency-specific (T+2 for USD/EUR, T+0 for GBP)
    /// - Day count: Currency-specific (ACT/360 for USD/EUR, ACT/365 for GBP)
    /// - Payment delay: 0 (set to 2 for Bloomberg OIS matching)
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        let mut config = CalibrationConfig::default();
        // Market standard: allow negative-rate discount factors in common negative-rate regimes.
        // Validation still auto-detects the regime; this flag only enables the allowance.
        if matches!(currency, Currency::EUR | Currency::JPY | Currency::CHF) {
            config.validation.allow_negative_rates = true;
        }

        // Default use_ois_logic to true, which also determines default include_spot_knot
        let use_ois_logic = true;

        Self {
            curve_id: curve_id.into(),
            base_date,
            solve_interp: InterpStyle::MonotoneConvex, // Default; arbitrage-free
            extrapolation: ExtrapolationPolicy::FlatForward,
            config, // Defaults include calibration_method
            currency,
            discount_curve_id: None,          // Will default to curve_id
            forward_curve_id: None,           // Will default to curve_id
            use_ois_logic,                    // Default to OIS conventions
            include_spot_knot: use_ois_logic, // Default on for OIS (market-standard spot anchoring)
        }
    }

    /// Set the interpolation used both during solving and for the final curve.
    pub fn with_solve_interp(mut self, interpolation: InterpStyle) -> Self {
        self.solve_interp = interpolation;
        self
    }

    /// Select calibration method (bootstrap vs global solve).
    ///
    /// This is a convenience method that forwards to `config.calibration_method`.
    pub fn with_calibration_method(mut self, method: CalibrationMethod) -> Self {
        self.config.calibration_method = method;
        self
    }

    /// Set the extrapolation policy for the final curve.
    ///
    /// - `FlatForward`: Constant forward rate beyond last knot (standard for risk)
    /// - `FlatZero`: Constant zero rate beyond last knot (some regulatory uses)
    pub fn with_extrapolation(mut self, policy: ExtrapolationPolicy) -> Self {
        self.extrapolation = policy;
        self
    }

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `CalibrationConfig` from `FinstackConfig.extensions["valuations.calibration.v1"]`.
    /// If not present, uses `CalibrationConfig::default()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the extension is malformed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use finstack_core::config::FinstackConfig;
    /// use finstack_valuations::calibration::methods::DiscountCurveCalibrator;
    ///
    /// let cfg = FinstackConfig::default();
    /// let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
    ///     .with_finstack_config(&cfg)?;
    /// ```
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = CalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
    }

    /// Set multi-curve framework configuration.
    pub fn with_multi_curve_config(mut self, multi_curve_config: MultiCurveConfig) -> Self {
        self.config.multi_curve = multi_curve_config;
        self
    }

    /// Set the discount curve ID used during instrument pricing.
    ///
    /// For single-curve OIS calibration, this defaults to `curve_id`.
    /// For multi-curve setups, set this to your collateral/OIS discount curve.
    pub fn with_discount_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.discount_curve_id = Some(curve_id.into());
        self
    }

    /// Set the forward curve ID used for floating leg projections.
    ///
    /// For single-curve OIS calibration, this defaults to `curve_id`.
    /// For multi-curve setups, set this to the forward curve being calibrated.
    pub fn with_forward_curve_id(mut self, curve_id: impl Into<CurveId>) -> Self {
        self.forward_curve_id = Some(curve_id.into());
        self
    }

    /// Enable or disable OIS-specific swap pricing logic.
    ///
    /// When `true` (default):
    /// - Swaps use overnight-indexed compounding conventions (SOFR, SONIA, etc.)
    /// - The discount curve is used as both discount and projection curve
    /// - Compounding method is inferred from the index (SOFR, SONIA, €STR, TONA)
    ///
    /// When `false`:
    /// - Swaps use simple compounding on the floating leg
    /// - Separate discount and forward curves are used
    /// - Suitable for term rate (LIBOR-style) swap calibration
    pub fn with_use_ois_logic(mut self, use_ois: bool) -> Self {
        self.use_ois_logic = use_ois;
        self
    }

    /// Enable or disable explicit spot knot inclusion.
    ///
    /// When `true` (default for OIS):
    /// - Includes a knot at `t_spot` (settlement date) with DF=1.0
    /// - Makes spot-starting convention explicit on the curve timeline
    /// - Market-standard for OIS curves (small approximation over 0-2 day spot period)
    ///
    /// When `false`:
    /// - Curve starts at base_date with only DF(0)=1.0
    /// - Use for non-OIS curves or when base_date equals settlement
    pub fn with_include_spot_knot(mut self, include: bool) -> Self {
        self.include_spot_knot = include;
        self
    }

    /// Get the effective discount curve ID (explicit or defaults to curve_id).
    pub(crate) fn effective_discount_curve_id(&self) -> CurveId {
        self.discount_curve_id
            .clone()
            .unwrap_or_else(|| self.curve_id.clone())
    }

    /// Get the effective forward curve ID (explicit or defaults to curve_id).
    pub(crate) fn effective_forward_curve_id(&self) -> CurveId {
        self.forward_curve_id
            .clone()
            .unwrap_or_else(|| self.curve_id.clone())
    }

    /// Get effective settlement days (explicit or currency default).
    ///
    /// Currency-specific defaults:
    /// - GBP: T+0 (same-day)
    /// - AUD, CAD: T+1
    /// - USD, EUR, JPY, CHF, and others: T+2
    ///
    /// Override via quote conventions.
    /// Calculate settlement date from base date using business-day calendar.
    ///
    /// Delegates to the internal `CalibrationPricer` for the actual computation.
    pub fn settlement_date(&self) -> finstack_core::Result<Date> {
        self.create_pricer().settlement_date(self.currency)
    }

    /// Create a CalibrationPricer configured from this calibrator's settings.
    ///
    /// The pricer encapsulates all the instrument pricing logic needed for
    /// calibration, using the calibrator's curve IDs and settlement conventions.
    /// Per-instrument conventions (payment delay, reset lag, calendar) come from
    /// the quote's `InstrumentConventions`.
    pub(crate) fn create_pricer(&self) -> CalibrationPricer {
        CalibrationPricer::new(self.base_date, self.effective_discount_curve_id())
            .with_forward_curve_id(self.effective_forward_curve_id())
            .with_settlement_days(0)
            .with_use_settlement_start(true)
    }

    /// Validate a calibrated discount curve using the configured validation policy.
    ///
    /// Use `config.validation.allow_negative_rates=true` to permit negative-rate regimes
    /// where short-end discount factors may exceed 1.0.
    pub(crate) fn validate_calibrated_curve(&self, curve: &DiscountCurve) -> Result<()> {
        use crate::calibration::validation::CurveValidator;

        if self.config.verbose {
            tracing::debug!("Validating calibrated discount curve {}", self.curve_id);
        }

        curve
            .validate(&self.config.validation)
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Calibrated discount curve {} failed validation: {}",
                    self.curve_id, e
                ),
                category: "yield_curve_validation".to_string(),
            })
    }

    /// Compute maturity-aware discount-factor bounds implied by configured rate bounds.
    ///
    /// Returns `(t_spot, optional_knot)` where `optional_knot` is `Some((t_spot, 1.0))`
    /// if spot knot inclusion is enabled and `t_spot` is meaningful.
    pub(crate) fn compute_spot_knot(
        &self,
        curve_dc: DayCount,
        settlement: Date,
    ) -> Result<(f64, Option<(f64, f64)>)> {
        // When not using settlement start, do not force a spot knot at settlement.
        if !self.create_pricer().use_settlement_start {
            return Ok((0.0, None));
        }

        if !self.include_spot_knot {
            return Ok((0.0, None));
        }

        let t_spot = curve_dc
            .year_fraction(
                self.base_date,
                settlement,
                finstack_core::dates::DayCountCtx::default(),
            )
            .map_err(|e| finstack_core::Error::Calibration {
                message: format!(
                    "Year fraction calculation failed for settlement spot knot: {}",
                    e
                ),
                category: "yield_curve".to_string(),
            })?;

        let min_t_spot = self.config.discount_curve.min_t_spot;
        if t_spot > min_t_spot {
            Ok((t_spot, Some((t_spot, 1.0))))
        } else {
            Ok((t_spot, None))
        }
    }

    /// Generate a maturity-aware scan grid for discount factor solving.
    ///
    /// Uses log-spaced points across the DF bounds derived from rate bounds,
    /// with finer resolution near the initial guess. This handles both positive
    /// and negative rate environments robustly.
    ///
    /// # Arguments
    ///
    /// * `df_lo` - Lower bound for discount factor (from `df_bounds_for_time`)
    /// * `df_hi` - Upper bound for discount factor (from `df_bounds_for_time`)
    /// * `initial_df` - Initial guess to center fine grid around
    /// * `num_points` - Total number of scan points (default ~32)
    pub(crate) fn maturity_aware_scan_grid(
        df_lo: f64,
        df_hi: f64,
        initial_df: f64,
        num_points: usize,
    ) -> Vec<f64> {
        let mut grid = Vec::with_capacity(num_points + 20);

        // Always include bounds to ensure we can detect sign changes at boundaries
        if df_lo.is_finite() && df_lo > 0.0 {
            grid.push(df_lo);
        }
        if df_hi.is_finite() && df_hi > 0.0 && (df_hi - df_lo).abs() > 1e-10 {
            grid.push(df_hi);
        }

        // Clamp initial guess to bounds
        let center = initial_df.clamp(df_lo, df_hi);

        // Use log-spacing for wide ranges, but handle edge cases
        // If df_lo is very small (< 1e-6), use hybrid approach
        let use_log_spacing = df_lo > 1e-6 && df_hi / df_lo > 10.0;

        if use_log_spacing {
            // Log-spaced points across the full range (stable for wide DF ranges)
            let log_lo = df_lo.ln();
            let log_hi = df_hi.ln();
            let coarse_points = num_points.saturating_sub(10).max(8);
            for i in 1..coarse_points {
                let t = i as f64 / coarse_points as f64;
                let log_df = log_lo + t * (log_hi - log_lo);
                let df = log_df.exp();
                if df.is_finite() && df > 0.0 && df > df_lo && df < df_hi {
                    grid.push(df);
                }
            }
        } else {
            // Linear spacing for narrow ranges or when df_lo is very small
            let coarse_points = num_points.saturating_sub(10).max(8);
            for i in 1..coarse_points {
                let t = i as f64 / coarse_points as f64;
                let df = df_lo + t * (df_hi - df_lo);
                if df.is_finite() && df > 0.0 && df > df_lo && df < df_hi {
                    grid.push(df);
                }
            }
        }

        // Finer grid near the initial guess (small fixed step for precision)
        // Use adaptive step size based on the range
        let range = df_hi - df_lo;
        let fine_step = (range * 0.01).clamp(0.001, 0.01);
        for i in -10..=10 {
            let df = center + i as f64 * fine_step;
            if df >= df_lo && df <= df_hi && df.is_finite() && df > 0.0 {
                grid.push(df);
            }
        }

        // Sort descending (from high DF downward for sign-change detection)
        grid.sort_by(|a, b| b.total_cmp(a));
        // Deduplicate close points
        grid.dedup_by(|a, b| (*a - *b).abs() < (df_hi - df_lo) * 0.001);
        grid
    }

    /// Compute maturity-aware discount factor bounds based on rate bounds.
    ///
    /// Returns (df_lo, df_hi) corresponding to (max_rate, min_rate).
    /// Uses continuous compounding approximation: df = exp(-r * t).
    pub(crate) fn df_bounds_for_time(&self, time: f64) -> (f64, f64) {
        let bounds = self.config.effective_rate_bounds(self.currency);
        let df_lo = (-bounds.max_rate * time).exp();
        let df_hi = (-bounds.min_rate * time).exp();
        (df_lo, df_hi)
    }

    /// Build a discount curve from knots using calibrator configuration.
    pub(crate) fn build_curve(
        &self,
        curve_id: CurveId,
        day_count: DayCount,
        knots: Vec<(f64, f64)>,
    ) -> Result<DiscountCurve> {
        DiscountCurve::builder(curve_id)
            .base_date(self.base_date)
            .day_count(day_count)
            .knots(knots)
            .set_interp(self.solve_interp)
            .extrapolation(self.extrapolation)
            .allow_non_monotonic()
            .build()
            .map_err(|e| finstack_core::Error::Calibration {
                message: e.to_string(),
                category: "curve_build".to_string(),
            })
    }
}

impl Calibrator<RatesQuote, DiscountCurve> for DiscountCurveCalibrator {
    fn calibrate(
        &self,
        instruments: &[RatesQuote],
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Note: All quote validation (including multi-curve integrity checks) is now
        // centralized in `CalibrationPricer::validate_rates_quotes` and called within
        // bootstrap_curve_with_solver / calibrate_global. This eliminates duplicate
        // validation and ensures consistent behavior across all calibration paths.

        match self.config.calibration_method {
            CalibrationMethod::Bootstrap => self.bootstrap_curve(instruments, base_context),
            CalibrationMethod::GlobalSolve { .. } => {
                self.calibrate_global(instruments, base_context)
            }
        }
    }
}
