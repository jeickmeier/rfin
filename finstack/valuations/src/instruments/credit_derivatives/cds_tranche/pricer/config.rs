use crate::cashflow::primitives::CashFlow;
use crate::correlation::copula::{Copula, CopulaSpec};
use crate::correlation::recovery::RecoverySpec;
use finstack_core::dates::{Date, StubKind};
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::math::GaussHermiteQuadrature;
use finstack_core::types::Percentage;
use std::sync::OnceLock;

// ============================================================================
// Default Configuration Constants
// ============================================================================

/// Default quadrature order for Gauss-Hermite integration.
///
/// Industry standard (QuantLib, Bloomberg) uses 20-50 points.
/// 7 points is insufficient for accurate resolution of:
/// - Step-function-like integrands at extreme correlations
/// - Student-t heavy tails
pub(super) const DEFAULT_QUADRATURE_ORDER: u8 = 20;

/// Minimum correlation value for numerical stability (avoids division by near-zero)
const DEFAULT_MIN_CORRELATION: f64 = 0.01;

/// Maximum correlation value for numerical stability (avoids degenerate cases)
const DEFAULT_MAX_CORRELATION: f64 = 0.99;

/// Default bump size for CS01 calculation in basis points
const DEFAULT_CS01_BUMP_SIZE: f64 = 1.0;

/// Default correlation bump for Correlation01 calculation (absolute, e.g., 0.01 = 1%)
const DEFAULT_CORR_BUMP_ABS: f64 = 0.01;

/// Boundary width for smooth correlation clamping transitions
const DEFAULT_CORR_BOUNDARY_WIDTH: f64 = 0.005;

/// Fraction of incremental loss allocated to accrual-on-default (0.5 = mid-period)
const DEFAULT_AOD_ALLOCATION_FRACTION: f64 = 0.5;

/// Numerical tolerance for integration convergence and boundary checks
const DEFAULT_NUMERICAL_TOLERANCE: f64 = 1e-10;

/// Clip parameter for CDF arguments to prevent overflow (±10 sigma)
const DEFAULT_CDF_CLIP: f64 = 10.0;

/// Lower correlation threshold for adaptive integration (below this, use higher order)
/// Rationale: Near ρ=0, conditional probability is highly sensitive to market factor
const DEFAULT_ADAPTIVE_INTEGRATION_LOW: f64 = 0.05;

/// Upper correlation threshold for adaptive integration (above this, use higher order)
/// Rationale: Near ρ=1, integrand approaches step function requiring more points
const DEFAULT_ADAPTIVE_INTEGRATION_HIGH: f64 = 0.95;

/// Grid step for exact convolution method (fraction of portfolio notional)
const DEFAULT_GRID_STEP: f64 = 0.001;

/// Minimum variance threshold for SPA to avoid division by zero
const DEFAULT_SPA_VARIANCE_FLOOR: f64 = 1e-14;

/// Probability clamp epsilon to avoid 0/1 extremes in probits/CDFs
const DEFAULT_PROBABILITY_CLIP: f64 = 1e-12;

/// LGD floor to avoid zero exposure in corner cases
const DEFAULT_LGD_FLOOR: f64 = 1e-6;

/// Minimum grid step to avoid degenerate convolution buckets
const DEFAULT_GRID_STEP_MIN: f64 = 1e-6;

/// Hard cap on convolution PMF points before falling back to SPA
const DEFAULT_MAX_GRID_POINTS: usize = 200_000;

/// Default settlement lag for index CDS (T+1 since Big Bang 2009)
const DEFAULT_INDEX_SETTLEMENT_LAG: i32 = 1;

/// Default settlement lag for bespoke CDS tranches (T+3 per ISDA)
const DEFAULT_BESPOKE_SETTLEMENT_LAG: i32 = 3;

/// Maximum iterations for par spread solver
const DEFAULT_PAR_SPREAD_MAX_ITER: usize = 50;

/// Tolerance for par spread solver convergence
const DEFAULT_PAR_SPREAD_TOLERANCE: f64 = 1e-6;

// ============================================================================
// Helper Functions
// ============================================================================

/// Parameters for the CDS Tranche pricing model.
///
/// This configuration controls all aspects of tranche pricing including:
/// - Copula model selection (Gaussian, Student-t, RFL, Multi-factor)
/// - Recovery model (constant or stochastic)
/// - Numerical integration parameters
/// - Risk metric bump sizes and methods
/// - ISDA convention settings
/// - Settlement and schedule generation
///
/// # ISDA Compliance
///
/// Default settings follow ISDA standard model conventions:
/// - Mid-period protection timing (`mid_period_protection = true`)
/// - Act/360 day count (set on instrument)
/// - Quarterly payment frequency on IMM dates
/// - T+1 settlement for index CDS
///
/// # Extended Models
///
/// The pricer supports multiple copula and recovery models:
///
/// ## Copula Models
/// - **Gaussian** (default): Standard one-factor, no tail dependence
/// - **Student-t**: Fat tails, captures tail dependence
/// - **RFL**: Random factor loading, stochastic correlation
/// - **Multi-factor**: Sector-specific correlation structure
///
/// ## Recovery Models
/// - **Constant** (default): Fixed recovery rate
/// - **Stochastic**: Recovery correlated with market factor
#[derive(Debug, Clone)]
pub struct CDSTranchePricerConfig {
    // ========================================================================
    // Model Selection
    // ========================================================================
    /// Copula model specification (default: Gaussian)
    pub copula_spec: CopulaSpec,
    /// Recovery model specification (default: use index recovery rate)
    pub recovery_spec: Option<RecoverySpec>,
    /// Whether to validate base correlation for arbitrage-free conditions
    pub validate_arbitrage_free: bool,
    /// Whether to enforce expected loss monotonicity in the EL curve.
    ///
    /// When `true` (default), if a computed EL value is less than the previous
    /// date's EL (which can occur due to base correlation model inconsistencies),
    /// it will be clamped to the previous value to ensure monotonicity.
    /// This prevents small arbitrage in leg PV calculations.
    pub enforce_el_monotonicity: bool,

    // ========================================================================
    // Numerical Integration
    // ========================================================================
    /// Number of quadrature points for numerical integration (5, 7, or 10)
    pub quadrature_order: u8,
    /// Whether to use issuer-specific curves if available
    pub use_issuer_curves: bool,
    /// Minimum correlation value for numerical stability
    pub min_correlation: f64,
    /// Maximum correlation value for numerical stability
    pub max_correlation: f64,

    // ========================================================================
    // Risk Metric Parameters
    // ========================================================================
    /// CS01 bump size (interpreted according to `cs01_bump_units`)
    pub cs01_bump_size: f64,
    /// Units for CS01 bump: hazard-rate bp or spread bp (additive)
    pub cs01_bump_units: Cs01BumpUnits,
    /// Correlation bump for correlation delta calculation (absolute)
    pub corr_bump_abs: f64,

    // ========================================================================
    // ISDA Convention Settings
    // ========================================================================
    /// Whether to use mid-period discounting for protection leg (ISDA standard: true)
    pub mid_period_protection: bool,
    /// Whether to include accrual-on-default in the premium leg
    pub accrual_on_default_enabled: bool,
    /// Fraction of incremental loss allocated to accrual-on-default (AoD)
    pub aod_allocation_fraction: f64,
    /// Stub convention for schedule generation
    pub schedule_stub: StubKind,
    /// If true, generate ISDA coupon dates (IMM-20 schedule)
    pub use_isda_coupon_dates: bool,
    /// Settlement lag in business days for index CDS (default: 1 for Big Bang)
    pub index_settlement_lag: i32,
    /// Settlement lag in business days for bespoke tranches (default: 3 per ISDA)
    pub bespoke_settlement_lag: i32,

    // ========================================================================
    // Numerical Stability
    // ========================================================================
    /// Smooth boundary width for correlation clamping transitions
    pub corr_boundary_width: f64,
    /// Numerical tolerance used by integration and boundary checks
    pub numerical_tolerance: f64,
    /// Clip parameter for CDF arguments to avoid overflow
    pub cdf_clip: f64,
    /// Correlation band within which to use standard quadrature
    pub adaptive_integration_low: f64,
    /// Correlation band within which to use standard quadrature
    pub adaptive_integration_high: f64,

    // ========================================================================
    // Heterogeneous Portfolio Settings
    // ========================================================================
    /// Heterogeneous issuer method when issuer curves are available
    pub hetero_method: HeteroMethod,
    /// Grid step for exact convolution method (fraction of portfolio notional)
    pub grid_step: f64,
    /// Minimum variance threshold for SPA to avoid division by zero
    pub spa_variance_floor: f64,
    /// Probability clamp epsilon to avoid 0/1 extremes in probits/CDFs
    pub probability_clip: f64,
    /// LGD floor to avoid zero exposure in corner cases
    pub lgd_floor: f64,
    /// Minimum grid step to avoid degenerate convolution buckets
    pub grid_step_min: f64,
    /// Hard cap on convolution PMF points before falling back to SPA
    pub max_grid_points: usize,

    // ========================================================================
    // Solver Settings
    // ========================================================================
    /// Maximum iterations for par spread solver
    pub par_spread_max_iter: usize,
    /// Tolerance for par spread solver convergence
    pub par_spread_tolerance: f64,
}

impl Default for CDSTranchePricerConfig {
    fn default() -> Self {
        Self {
            // Model selection
            copula_spec: CopulaSpec::default(),
            recovery_spec: None, // Use index recovery rate by default
            validate_arbitrage_free: true,
            enforce_el_monotonicity: true, // Prevent EL from decreasing over time

            // Numerical integration
            quadrature_order: DEFAULT_QUADRATURE_ORDER,
            use_issuer_curves: true,
            min_correlation: DEFAULT_MIN_CORRELATION,
            max_correlation: DEFAULT_MAX_CORRELATION,

            // Risk metrics
            cs01_bump_size: DEFAULT_CS01_BUMP_SIZE,
            cs01_bump_units: Cs01BumpUnits::HazardRateBp,
            corr_bump_abs: DEFAULT_CORR_BUMP_ABS,

            // ISDA conventions
            mid_period_protection: true, // ISDA standard
            accrual_on_default_enabled: true,
            aod_allocation_fraction: DEFAULT_AOD_ALLOCATION_FRACTION,
            schedule_stub: StubKind::ShortFront,
            use_isda_coupon_dates: false,
            index_settlement_lag: DEFAULT_INDEX_SETTLEMENT_LAG,
            bespoke_settlement_lag: DEFAULT_BESPOKE_SETTLEMENT_LAG,

            // Numerical stability
            corr_boundary_width: DEFAULT_CORR_BOUNDARY_WIDTH,
            numerical_tolerance: DEFAULT_NUMERICAL_TOLERANCE,
            cdf_clip: DEFAULT_CDF_CLIP,
            adaptive_integration_low: DEFAULT_ADAPTIVE_INTEGRATION_LOW,
            adaptive_integration_high: DEFAULT_ADAPTIVE_INTEGRATION_HIGH,

            // Heterogeneous portfolio
            hetero_method: HeteroMethod::Spa,
            grid_step: DEFAULT_GRID_STEP,
            spa_variance_floor: DEFAULT_SPA_VARIANCE_FLOOR,
            probability_clip: DEFAULT_PROBABILITY_CLIP,
            lgd_floor: DEFAULT_LGD_FLOOR,
            grid_step_min: DEFAULT_GRID_STEP_MIN,
            max_grid_points: DEFAULT_MAX_GRID_POINTS,

            // Solver
            par_spread_max_iter: DEFAULT_PAR_SPREAD_MAX_ITER,
            par_spread_tolerance: DEFAULT_PAR_SPREAD_TOLERANCE,
        }
    }
}

impl CDSTranchePricerConfig {
    /// Create configuration with Student-t copula.
    ///
    /// # Arguments
    /// * `df` - Degrees of freedom (typical: 4-10 for CDX)
    pub fn with_student_t_copula(mut self, df: f64) -> Self {
        self.copula_spec = CopulaSpec::student_t(df);
        self
    }

    /// Create configuration with Random Factor Loading copula.
    ///
    /// # Arguments
    /// * `loading_vol` - Loading volatility (typical: 0.05-0.20)
    pub fn with_rfl_copula(mut self, loading_vol: f64) -> Self {
        self.copula_spec = CopulaSpec::random_factor_loading(loading_vol);
        self
    }

    /// Create configuration with Random Factor Loading copula using typed volatility.
    pub fn with_rfl_copula_pct(mut self, loading_vol: Percentage) -> Self {
        self.copula_spec = CopulaSpec::random_factor_loading(loading_vol.as_decimal());
        self
    }

    /// Create configuration with multi-factor copula.
    ///
    /// # Arguments
    /// * `num_factors` - Number of systematic factors
    pub fn with_multi_factor_copula(mut self, num_factors: usize) -> Self {
        self.copula_spec = CopulaSpec::multi_factor(num_factors);
        self
    }

    /// Enable stochastic recovery with market-standard calibration.
    ///
    /// Uses typical calibration from CDX equity tranche:
    /// - Mean: 40%, Vol: 25%, Correlation: -40%
    pub fn with_stochastic_recovery(mut self) -> Self {
        self.recovery_spec = Some(RecoverySpec::market_standard_stochastic());
        self
    }

    /// Enable stochastic recovery with custom parameters.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery rate (typical: 0.40)
    /// * `vol` - Recovery volatility (typical: 0.20-0.30)
    /// * `corr` - Correlation with factor (typical: -0.30 to -0.50)
    pub fn with_custom_stochastic_recovery(mut self, mean: f64, vol: f64, corr: f64) -> Self {
        self.recovery_spec = Some(RecoverySpec::market_correlated(mean, vol, corr));
        self
    }

    /// Enable stochastic recovery with custom parameters using typed percentages.
    pub fn with_custom_stochastic_recovery_pct(
        mut self,
        mean: Percentage,
        vol: Percentage,
        corr: f64,
    ) -> Self {
        self.recovery_spec = Some(RecoverySpec::market_correlated(
            mean.as_decimal(),
            vol.as_decimal(),
            corr,
        ));
        self
    }

    /// Set constant recovery rate (overriding index recovery).
    pub fn with_constant_recovery(mut self, rate: f64) -> Self {
        self.recovery_spec = Some(RecoverySpec::constant(rate));
        self
    }

    /// Set constant recovery rate using a typed percentage.
    pub fn with_constant_recovery_pct(mut self, rate: Percentage) -> Self {
        self.recovery_spec = Some(RecoverySpec::constant(rate.as_decimal()));
        self
    }

    /// Enable or disable arbitrage-free validation of base correlation.
    pub fn with_arbitrage_validation(mut self, enabled: bool) -> Self {
        self.validate_arbitrage_free = enabled;
        self
    }

    /// Set quadrature order for numerical integration.
    pub fn with_quadrature_order(mut self, order: u8) -> Self {
        self.quadrature_order = order;
        self
    }
}

/// Units for CS01 bumping in tranche pricer
/// Units for CS01 credit spread bumping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cs01BumpUnits {
    /// Bump hazard rate in basis points
    HazardRateBp,
}

/// Heterogeneous expected loss evaluation method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeteroMethod {
    /// Saddle-point approximation (SPA) method
    Spa,
    /// Exact convolution method (slower but more accurate)
    ExactConvolution,
}

/// Copula-based pricing engine for CDS tranches.
///
/// Supports multiple copula models (Gaussian, Student-t, RFL, Multi-factor)
/// and optional stochastic recovery for market-standard tranche pricing.
///
/// The copula instance and quadrature table are constructed lazily on first use
/// and cached for the pricer's lifetime. Heterogeneous EL evaluation calls into
/// copula dispatch and quadrature selection from hot integration loops.
///
/// **Cache invariant:** `params.copula_spec` and `params.quadrature_order` must
/// not be mutated after the first call to [`Self::copula`] or quadrature
/// selection. To change either, construct a new pricer via [`Self::with_params`].
/// Other config fields (`grid_step`, `hetero_method`, `use_issuer_curves`, etc.)
/// are *not* cached and may be mutated freely.
pub struct CDSTranchePricer {
    pub(super) params: CDSTranchePricerConfig,
    pub(super) copula_cache: OnceLock<Box<dyn Copula + Send + Sync>>,
    pub(super) quadrature_cache: OnceLock<GaussHermiteQuadrature>,
}

pub(super) type ProjectionInputs = (
    std::sync::Arc<CreditIndexData>,
    Date,
    Vec<Date>,
    Vec<(Date, f64)>,
);

#[derive(Debug, Clone)]
pub(super) struct ProjectedDiscountedRow {
    pub(super) cashflow: CashFlow,
    pub(super) discount_time: Option<f64>,
}

impl Default for CDSTranchePricer {
    fn default() -> Self {
        Self::new()
    }
}

impl CDSTranchePricer {
    /// Get the current configuration.
    pub fn config(&self) -> &CDSTranchePricerConfig {
        &self.params
    }
}
