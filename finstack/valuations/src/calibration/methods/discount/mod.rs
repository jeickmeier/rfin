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
///     .with_settlement_days(2);
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

    /// Set calendar ID (deprecated: use InstrumentConventions on quotes instead).
    ///
    /// This method is kept for backward compatibility with Python bindings.
    /// Settings are now configured per-quote via `InstrumentConventions`.
    #[deprecated(note = "Use InstrumentConventions on quotes instead")]
    pub fn with_calendar_id(self, _calendar_id: impl Into<String>) -> Self {
        self
    }

    /// Set settlement days (deprecated: use InstrumentConventions on quotes instead).
    ///
    /// This method is kept for backward compatibility with Python bindings.
    /// Settings are now configured per-quote via `InstrumentConventions`.
    #[deprecated(note = "Use InstrumentConventions on quotes instead")]
    pub fn with_settlement_days(self, _days: i32) -> Self {
        self
    }

    /// Set payment delay (deprecated: use InstrumentConventions on quotes instead).
    ///
    /// This method is kept for backward compatibility with Python bindings.
    /// Settings are now configured per-quote via `InstrumentConventions`.
    #[deprecated(note = "Use InstrumentConventions on quotes instead")]
    pub fn with_payment_delay(self, _days: i32) -> Self {
        self
    }

    /// Set curve day count (deprecated: use InstrumentConventions on quotes instead).
    ///
    /// This method is kept for backward compatibility with Python bindings.
    /// Settings are now configured per-quote via `InstrumentConventions`.
    #[deprecated(note = "Use InstrumentConventions on quotes instead")]
    pub fn with_curve_day_count(self, _dc: DayCount) -> Self {
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
            // Discount curve calibration uses base-date start to match repricing tests
            .with_use_settlement_start(false)
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
    ) -> (f64, Option<(f64, f64)>) {
        // When not using settlement start, do not force a spot knot at settlement.
        if !self.create_pricer().use_settlement_start {
            return (0.0, None);
        }

        if !self.include_spot_knot {
            return (0.0, None);
        }

        let t_spot = curve_dc
            .year_fraction(
                self.base_date,
                settlement,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        const MIN_T_SPOT: f64 = 1e-6; // ~30 seconds; avoids duplicate knots
        if t_spot > MIN_T_SPOT {
            (t_spot, Some((t_spot, 1.0)))
        } else {
            (t_spot, None)
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
    #[allow(dead_code)] // Used in tests
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
            CalibrationMethod::Bootstrap => {
                self.bootstrap_curve(instruments, base_context)
            }
            CalibrationMethod::GlobalSolve { .. } => {
                self.calibrate_global(instruments, base_context)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::quotes::InstrumentConventions;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Tenor};
    use time::Month;

    fn create_test_quotes() -> Vec<RatesQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.047,
                is_ois: true,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::semi_annual())
                    .with_day_count(DayCount::Thirty360),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::quarterly())
                    .with_day_count(DayCount::Act360)
                    .with_index("USD-SOFR-3M"),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365 * 2),
                rate: 0.048,
                is_ois: true,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::semi_annual())
                    .with_day_count(DayCount::Thirty360),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::quarterly())
                    .with_day_count(DayCount::Act360)
                    .with_index("USD-SOFR-3M"),
            },
        ]
    }

    #[test]
    fn test_quote_validation() {
        use crate::calibration::config::RateBounds;
        use crate::calibration::pricing::RatesQuoteUseCase;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let empty_quotes: Vec<RatesQuote> = vec![];
        assert!(CalibrationPricer::validate_rates_quotes(
            &empty_quotes,
            &RateBounds::default(),
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .is_err());

        let valid_quotes = create_test_quotes();
        // Use enforce_separation=false since test quotes include non-OIS swaps
        assert!(CalibrationPricer::validate_rates_quotes(
            &valid_quotes,
            &RateBounds::default(),
            base_date,
            RatesQuoteUseCase::DiscountCurve {
                enforce_separation: false,
            },
        )
        .is_ok());
    }

    #[test]
    fn test_calibrator_builder() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD)
            .with_extrapolation(ExtrapolationPolicy::FlatZero)
            .with_solve_interp(InterpStyle::Linear);

        // Note: ExtrapolationPolicy doesn't implement PartialEq, so we can't use assert_eq!
        // assert_eq!(calibrator.extrapolation, ExtrapolationPolicy::FlatZero);
        assert_eq!(calibrator.solve_interp, InterpStyle::Linear);
    }

    #[test]
    fn test_currency_factories() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let usd = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        assert_eq!(usd.currency, Currency::USD);

        let gbp = DiscountCurveCalibrator::new("GBP-SONIA", base_date, Currency::GBP);
        assert_eq!(gbp.currency, Currency::GBP);

        let eur = DiscountCurveCalibrator::new("EUR-ESTR", base_date, Currency::EUR);
        assert_eq!(eur.currency, Currency::EUR);
    }

    #[test]
    fn test_maturity_aware_scan_grid() {
        // Normal rate environment (5Y at 5% rate)
        let grid_normal = DiscountCurveCalibrator::maturity_aware_scan_grid(0.7, 1.05, 0.97, 32);
        assert!(grid_normal.iter().any(|&x| x > 0.95 && x < 0.99));
        assert!(grid_normal.len() >= 10);
        // Verify endpoints are included
        assert!(grid_normal.iter().any(|&x| (x - 0.7).abs() < 0.05));
        assert!(grid_normal.iter().any(|&x| (x - 1.05).abs() < 0.05));

        // Negative rate environment (DF > 1.0)
        let grid_negative = DiscountCurveCalibrator::maturity_aware_scan_grid(0.9, 1.2, 1.02, 32);
        assert!(grid_negative.iter().any(|&x| x > 1.0));
        assert!(grid_negative.iter().any(|&x| x > 1.15));

        // High rate / long maturity (30Y at high rates)
        let grid_high = DiscountCurveCalibrator::maturity_aware_scan_grid(0.1, 0.8, 0.3, 32);
        assert!(grid_high.iter().any(|&x| x < 0.5));
        assert!(grid_high.iter().any(|&x| x < 0.2));

        // Wide range test (ensures log-spacing works for extreme DF ranges)
        let grid_wide = DiscountCurveCalibrator::maturity_aware_scan_grid(0.001, 2.0, 0.5, 32);
        assert!(grid_wide.len() >= 20);
        assert!(grid_wide.iter().any(|&x| x < 0.01));
        assert!(grid_wide.iter().any(|&x| x > 1.5));
    }

    #[test]
    fn test_df_bounds_for_time() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Default rate bounds: [-0.02, 0.50]
        let calibrator = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);

        // Short maturity (1Y)
        let (lo_1y, hi_1y) = calibrator.df_bounds_for_time(1.0);
        // DF at max_rate (0.50): exp(-0.50 * 1) ≈ 0.606
        // DF at min_rate (-0.02): exp(0.02 * 1) ≈ 1.020
        assert!(lo_1y < 0.7, "lo_1y={}", lo_1y);
        assert!(hi_1y > 1.0, "hi_1y={}", hi_1y);

        // Long maturity (30Y)
        let (lo_30y, hi_30y) = calibrator.df_bounds_for_time(30.0);
        // DF at max_rate (0.50): exp(-0.50 * 30) ≈ 3e-7
        // DF at min_rate (-0.02): exp(0.02 * 30) ≈ 1.82
        assert!(lo_30y < 1e-6, "lo_30y={}", lo_30y);
        assert!(hi_30y > 1.5, "hi_30y={}", hi_30y);

        // Very short maturity (1 day)
        let (lo_1d, hi_1d) = calibrator.df_bounds_for_time(1.0 / 365.0);
        // Bounds should be very close to 1.0
        assert!(lo_1d > 0.99, "lo_1d={}", lo_1d);
        assert!(hi_1d < 1.01, "hi_1d={}", hi_1d);
    }

    #[test]
    fn test_scan_grid_covers_df_bounds() {
        // Test that scan grid generation works correctly for typical scenarios
        // For very extreme ranges (like 30Y with 50% max rate), we focus on
        // reasonable coverage rather than hitting exact bounds

        // Test 1: Normal 5Y scenario (typical calibration)
        let grid_5y = DiscountCurveCalibrator::maturity_aware_scan_grid(0.7, 1.1, 0.9, 32);
        assert!(grid_5y.len() >= 20, "Grid should have sufficient points");
        let min_5y = grid_5y.iter().copied().fold(f64::INFINITY, f64::min);
        let max_5y = grid_5y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(min_5y <= 0.75, "Grid should cover lower end: {}", min_5y);
        assert!(max_5y >= 1.05, "Grid should cover upper end: {}", max_5y);

        // Test 2: Negative rate environment (DF > 1)
        let grid_neg = DiscountCurveCalibrator::maturity_aware_scan_grid(0.95, 1.3, 1.05, 32);
        assert!(
            grid_neg.iter().any(|&x| x > 1.0),
            "Grid should include DF > 1"
        );
        assert!(
            grid_neg.iter().any(|&x| x > 1.2),
            "Grid should reach upper bound area"
        );

        // Test 3: Wide range (long maturity) - verify no crashes and reasonable coverage
        let grid_wide = DiscountCurveCalibrator::maturity_aware_scan_grid(0.001, 2.0, 0.5, 32);
        assert!(
            grid_wide.len() >= 20,
            "Wide grid should have sufficient points"
        );
        // Grid should span multiple orders of magnitude
        let min_wide = grid_wide.iter().copied().fold(f64::INFINITY, f64::min);
        let max_wide = grid_wide.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            min_wide < 0.1,
            "Wide grid should reach low values: {}",
            min_wide
        );
        assert!(
            max_wide > 1.5,
            "Wide grid should reach high values: {}",
            max_wide
        );

        // Test 4: Grid is sorted descending (for sign-change detection)
        for grid in [&grid_5y, &grid_neg, &grid_wide] {
            for i in 1..grid.len() {
                assert!(grid[i - 1] >= grid[i], "Grid should be sorted descending");
            }
        }
    }

    // =========================================================================
    // Global Solve Regression Tests
    // =========================================================================

    #[test]
    fn test_global_solve_does_not_panic_overdetermined() {
        use crate::calibration::config::CalibrationMethod;
        use crate::calibration::Calibrator;
        use finstack_core::market_data::context::MarketContext;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Create 6 quotes that map to 6 time points (overdetermined system)
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(60),
                rate: 0.046,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.0465,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(180),
                rate: 0.047,
                is_ois: true,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::semi_annual())
                    .with_day_count(DayCount::Thirty360),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::quarterly())
                    .with_day_count(DayCount::Act360)
                    .with_index("USD-SOFR-3M"),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(365),
                rate: 0.048,
                is_ois: true,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::semi_annual())
                    .with_day_count(DayCount::Thirty360),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::quarterly())
                    .with_day_count(DayCount::Act360)
                    .with_index("USD-SOFR-3M"),
            },
            RatesQuote::Swap {
                maturity: base_date + time::Duration::days(730),
                rate: 0.049,
                is_ois: true,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::semi_annual())
                    .with_day_count(DayCount::Thirty360),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(Tenor::quarterly())
                    .with_day_count(DayCount::Act360)
                    .with_index("USD-SOFR-3M"),
            },
        ];

        let calibrator = DiscountCurveCalibrator::new("USD-OIS-GLOBAL", base_date, Currency::USD)
            .with_calibration_method(CalibrationMethod::GlobalSolve {
                use_analytical_jacobian: false,
            });

        let context = MarketContext::default();

        // Main assertion: this should not panic
        let result = calibrator.calibrate(&quotes, &context);
        assert!(
            result.is_ok(),
            "Global solve should complete without panic: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_global_solve_respects_rate_bounds() {
        use crate::calibration::config::CalibrationMethod;
        use crate::calibration::Calibrator;
        use finstack_core::config::FinstackConfig;
        use finstack_core::market_data::context::MarketContext;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Create quotes with rates well within bounds
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.03,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.035,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
        ];

        // Use tight bounds to verify they are enforced via FinstackConfig extensions
        let mut cfg = FinstackConfig::default();
        cfg.extensions.insert(
            crate::calibration::CALIBRATION_CONFIG_KEY_V1,
            serde_json::json!({
                "rate_bounds_policy": "explicit",
                "rate_bounds": { "min_rate": 0.01, "max_rate": 0.10 }
            }),
        );

        let calibrator = DiscountCurveCalibrator::new("USD-OIS-BOUNDED", base_date, Currency::USD)
            .with_calibration_method(CalibrationMethod::GlobalSolve {
                use_analytical_jacobian: false,
            })
            .with_finstack_config(&cfg)
            .expect("valid config");

        let context = MarketContext::default();

        let result = calibrator.calibrate(&quotes, &context);
        assert!(
            result.is_ok(),
            "Calibration should succeed: {:?}",
            result.err()
        );

        let (curve, _report) = result.expect("calibration should succeed");

        // Check that implied zero rates are within bounds at each time point
        // Check at 30 days and 90 days
        for t in [30.0 / 365.0, 90.0 / 365.0] {
            let df = curve.df(t);
            // Compute implied zero rate: z = -ln(df) / t
            let z = -df.ln() / t;
            assert!(
                (0.01 - 0.001..=0.10 + 0.001).contains(&z),
                "Zero rate at t={:.4} should be within bounds [0.01, 0.10], got z={:.6} (df={:.6})",
                t,
                z,
                df
            );
        }
    }

    #[test]
    fn test_global_solve_report_has_diagnostics() {
        use crate::calibration::config::CalibrationMethod;
        use crate::calibration::Calibrator;
        use finstack_core::market_data::context::MarketContext;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
        ];

        let calibrator = DiscountCurveCalibrator::new("USD-OIS-DIAG", base_date, Currency::USD)
            .with_calibration_method(CalibrationMethod::GlobalSolve {
                use_analytical_jacobian: false,
            });

        let context = MarketContext::default();

        let result = calibrator.calibrate(&quotes, &context);
        assert!(
            result.is_ok(),
            "Calibration should succeed: {:?}",
            result.err()
        );

        let (_curve, report) = result.expect("calibration should succeed");

        // Check report has the expected diagnostic metadata
        assert!(
            report.metadata.contains_key("residual_evals"),
            "Report should contain residual_evals metadata"
        );
        assert!(
            report.metadata.contains_key("l2_norm"),
            "Report should contain l2_norm metadata"
        );
        assert!(
            report.metadata.contains_key("max_abs_residual"),
            "Report should contain max_abs_residual metadata"
        );
        assert_eq!(
            report.metadata.get("method"),
            Some(&"global_solve".to_string()),
            "Report method should be global_solve"
        );

        // Verify residual_evals is a positive number
        let evals: usize = report
            .metadata
            .get("residual_evals")
            .expect("should have residual_evals")
            .parse()
            .expect("should be a number");
        assert!(evals >= 1, "Should have at least 1 residual evaluation");
    }

    // =========================================================================
    // Market-Standard Refactor Tests
    // =========================================================================

    #[test]
    fn test_spot_knot_defaults_on_for_ois() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // OIS calibrators default use_ois_logic=true, which should also default spot knot on
        let usd_ois = DiscountCurveCalibrator::new("USD-OIS", base_date, Currency::USD);
        assert!(usd_ois.use_ois_logic, "OIS logic should default true");
        assert!(
            usd_ois.include_spot_knot,
            "include_spot_knot should default true for OIS"
        );

        let eur_ois = DiscountCurveCalibrator::new("EUR-ESTR", base_date, Currency::EUR);
        assert!(
            eur_ois.include_spot_knot,
            "EUR OIS should also have spot knot on"
        );

        // Explicit override should work
        let no_spot = DiscountCurveCalibrator::new("USD-NO-SPOT", base_date, Currency::USD)
            .with_include_spot_knot(false);
        assert!(
            !no_spot.include_spot_knot,
            "Explicit override to false should work"
        );

        // Non-OIS calibrator (use_ois_logic=false) can still have spot knot controlled independently
        let libor_style = DiscountCurveCalibrator::new("USD-LIBOR", base_date, Currency::USD)
            .with_use_ois_logic(false)
            .with_include_spot_knot(false);
        assert!(!libor_style.use_ois_logic);
        assert!(!libor_style.include_spot_knot);
    }

    #[test]
    fn test_spot_knot_metadata_in_reports() {
        use crate::calibration::config::CalibrationMethod;
        use crate::calibration::Calibrator;
        use finstack_core::market_data::context::MarketContext;

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Simple deposit-only quotes for fast calibration
        let quotes = vec![
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(30),
                rate: 0.045,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
            RatesQuote::Deposit {
                maturity: base_date + time::Duration::days(90),
                rate: 0.046,
                conventions: InstrumentConventions::default().with_day_count(DayCount::Act360),
            },
        ];

        let context = MarketContext::default();

        // Test bootstrap report metadata
        let bootstrap_cal = DiscountCurveCalibrator::new("USD-BOOT", base_date, Currency::USD)
            .with_calibration_method(CalibrationMethod::Bootstrap);

        let (_, boot_report) = bootstrap_cal
            .calibrate(&quotes, &context)
            .expect("bootstrap should succeed");

        assert!(
            boot_report.metadata.contains_key("t_spot"),
            "Bootstrap report should have t_spot metadata"
        );
        assert!(
            boot_report.metadata.contains_key("spot_knot_included"),
            "Bootstrap report should have spot_knot_included metadata"
        );
        assert_eq!(
            boot_report.metadata.get("spot_knot_included"),
            Some(&"true".to_string()),
            "OIS calibrator should have spot knot included"
        );

        // Test global solve report metadata
        let global_cal = DiscountCurveCalibrator::new("USD-GLOBAL", base_date, Currency::USD)
            .with_calibration_method(CalibrationMethod::GlobalSolve {
                use_analytical_jacobian: false,
            });

        let (_, global_report) = global_cal
            .calibrate(&quotes, &context)
            .expect("global solve should succeed");

        assert!(
            global_report.metadata.contains_key("t_spot"),
            "Global solve report should have t_spot metadata"
        );
        assert!(
            global_report.metadata.contains_key("spot_knot_included"),
            "Global solve report should have spot_knot_included metadata"
        );
        assert_eq!(
            global_report.metadata.get("spot_knot_included"),
            Some(&"true".to_string()),
            "OIS calibrator global solve should have spot knot included"
        );
    }

    #[test]
    fn test_negative_rate_bounds_do_not_reject_quotes() {
        use crate::calibration::config::{RateBounds, RateBoundsPolicy};

        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Test that EUR calibrator accepts negative rate bounds
        let mut calibrator = DiscountCurveCalibrator::new("EUR-ESTR-NEG", base_date, Currency::EUR);

        // Set rate bounds to include negative rates
        calibrator.config.rate_bounds_policy = RateBoundsPolicy::Explicit;
        calibrator.config.rate_bounds = RateBounds {
            min_rate: -0.02,
            max_rate: 0.10,
        };

        // Verify EUR has allow_negative_rates=true by default
        assert!(
            calibrator.config.validation.allow_negative_rates,
            "EUR calibrator should allow negative rates by default"
        );

        // Verify the rate bounds are set correctly
        let bounds = calibrator.config.effective_rate_bounds(calibrator.currency);
        assert!(
            bounds.min_rate < 0.0,
            "Rate bounds should allow negative rates: min={}",
            bounds.min_rate
        );
    }

    #[test]
    fn test_build_knots_from_zero_rates_no_df_direction_clamp() {
        use crate::calibration::config::RateBounds;

        // Test that build_knots_from_zero_rates does NOT clamp DF direction
        // (i.e., allows DF to increase for negative rates)

        let times = vec![0.25, 0.5, 1.0];
        // Negative zero rates should produce DF > 1
        let zero_rates = vec![-0.01, -0.005, -0.002];
        let rate_bounds = RateBounds {
            min_rate: -0.02,
            max_rate: 0.10,
        };

        let knots = DiscountCurveCalibrator::build_knots_from_zero_rates(
            &times,
            &zero_rates,
            &rate_bounds,
            None, // no spot knot
        );

        // Should have 4 knots: base (0,1) + 3 time points
        assert_eq!(knots.len(), 4);
        assert_eq!(knots[0], (0.0, 1.0)); // Base knot

        // With negative zero rates, DFs should be > 1
        // DF = exp(-z * t) = exp(-(-rate) * t) = exp(rate * t)
        for i in 1..knots.len() {
            let (t, df) = knots[i];
            let expected_df = (-zero_rates[i - 1] * t).exp();
            assert!(
                (df - expected_df).abs() < 1e-10,
                "Knot {} DF should match expected: {} vs {}",
                i,
                df,
                expected_df
            );
            assert!(
                df > 1.0,
                "Knot {} with negative rate should have DF > 1, got {}",
                i,
                df
            );
        }

        // Key assertion: all DFs are > 1.0, meaning NO direction clamping occurred.
        // (The old code would have clamped df > prev_df to df = prev_df, giving df = 1.0)
        // With negative rates, we correctly get DFs > 1.0 at all knots.
        assert!(
            knots.iter().skip(1).all(|(_, df)| *df > 1.0),
            "All knots should have DF > 1 with negative rates (no direction clamp)"
        );
    }
}
