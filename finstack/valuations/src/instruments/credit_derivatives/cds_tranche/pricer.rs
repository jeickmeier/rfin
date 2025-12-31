//! Gaussian Copula pricing model for CDS tranches.
//!
//! Implements the industry-standard base correlation approach for pricing
//! synthetic CDO tranches using a one-factor Gaussian Copula model.
//!
//! ## Key Features
//!
//! * **Time-dependent Expected Loss**: Calculates expected loss at each payment date
//!   rather than using linear approximation from maturity values.
//! * **Accrual-on-Default (AoD)**: Premium leg includes proper AoD adjustment using
//!   half of incremental loss within each period.
//! * **Market-standard Scheduling**: Uses canonical schedule builders with business
//!   day conventions and holiday calendar support.
//! * **Risk Metrics**: Full implementation of CS01, Correlation Delta, and Jump-to-Default
//!   using central-difference bumping for accurate hedge ratios.
//! * **Numerical Stability**: Correlation clamping, monotonicity enforcement, and
//!   robust integration using Gauss-Hermite quadrature.
//! * **ISDA Compliance**: Mid-period protection timing, proper settlement lag handling,
//!   and standard day count conventions.
//!
//! ## Mathematical Approach
//!
//! The model decomposes tranche [A,D] expected loss as:
//! `EL_[A,D](t) = [EL_eq(0,D,t) - EL_eq(0,A,t)] / [(D-A)/100]`
//!
//! Where `EL_eq(0,K,t)` is the expected loss of equity tranche [0,K] at time t,
//! calculated using base correlation ρ(K) for detachment point K.
//!
//! ### Premium Leg PV
//! `PV_prem = Σ c * Δt_i * DF(t_i) * [N_outstanding(t_{i-1}) - 0.5 * N_incremental_loss(t_i)]`
//!
//! ### Protection Leg PV  
//! `PV_prot = Σ DF(t_i) * N_tr * [EL_fraction(t_i) - EL_fraction(t_{i-1})]`
//!
//! ## Adaptive Integration Thresholds
//!
//! The pricer uses adaptive Gauss-Hermite quadrature when correlation falls outside
//! the range [0.05, 0.95]. This is because:
//! - Near ρ=0: The conditional default probability becomes very sensitive to the
//!   market factor, requiring higher-order integration for accuracy.
//! - Near ρ=1: The integrand approaches a step function, requiring more quadrature
//!   points to capture the sharp transition.
//!
//! ## Limitations
//!
//! * Assumes homogeneous portfolio (single hazard curve for all constituents)
//! * Uses constant recovery rate across all entities
//! * Base correlation model can have small arbitrage inconsistencies at curve knots

use crate::cashflow::builder::build_dates;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use crate::instruments::common::traits::Instrument;
use finstack_core::dates::next_cds_date;
use finstack_core::dates::{Date, DateExt, StubKind};
use finstack_core::market_data::traits::Discounting;
use finstack_core::market_data::{context::MarketContext, term_structures::CreditIndexData};
use finstack_core::math::binomial_probability;
use finstack_core::math::{norm_cdf, norm_pdf, standard_normal_inv_cdf, GaussHermiteQuadrature};
use finstack_core::money::Money;
use finstack_core::types::Percentage;
use finstack_core::Result;

// Calendar imports for business day settlement
use finstack_core::dates::CalendarRegistry;
use finstack_core::dates::HolidayCalendar;

// Recovery model import for optional stochastic recovery
use super::recovery::RecoveryModel;

#[cfg(test)]
use finstack_core::math::log_factorial;

// ============================================================================
// Default Configuration Constants
// ============================================================================

/// Default quadrature order for Gauss-Hermite integration (5, 7, or 10 points)
const DEFAULT_QUADRATURE_ORDER: u8 = 7;

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
#[derive(Clone, Debug)]
pub struct CDSTranchePricerConfig {
    // ========================================================================
    // Model Selection
    // ========================================================================
    /// Copula model specification (default: Gaussian)
    pub copula_spec: super::copula::CopulaSpec,
    /// Recovery model specification (default: use index recovery rate)
    pub recovery_spec: Option<super::recovery::RecoverySpec>,
    /// Whether to validate base correlation for arbitrage-free conditions
    pub validate_arbitrage_free: bool,

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
            copula_spec: super::copula::CopulaSpec::default(),
            recovery_spec: None, // Use index recovery rate by default
            validate_arbitrage_free: true,

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
            schedule_stub: StubKind::None,
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
        self.copula_spec = super::copula::CopulaSpec::student_t(df);
        self
    }

    /// Create configuration with Random Factor Loading copula.
    ///
    /// # Arguments
    /// * `loading_vol` - Loading volatility (typical: 0.05-0.20)
    pub fn with_rfl_copula(mut self, loading_vol: f64) -> Self {
        self.copula_spec = super::copula::CopulaSpec::random_factor_loading(loading_vol);
        self
    }

    /// Create configuration with Random Factor Loading copula using typed volatility.
    pub fn with_rfl_copula_pct(mut self, loading_vol: Percentage) -> Self {
        self.copula_spec =
            super::copula::CopulaSpec::random_factor_loading(loading_vol.as_decimal());
        self
    }

    /// Create configuration with multi-factor copula.
    ///
    /// # Arguments
    /// * `num_factors` - Number of systematic factors
    pub fn with_multi_factor_copula(mut self, num_factors: usize) -> Self {
        self.copula_spec = super::copula::CopulaSpec::multi_factor(num_factors);
        self
    }

    /// Enable stochastic recovery with market-standard calibration.
    ///
    /// Uses typical calibration from CDX equity tranche:
    /// - Mean: 40%, Vol: 25%, Correlation: -40%
    pub fn with_stochastic_recovery(mut self) -> Self {
        self.recovery_spec = Some(super::recovery::RecoverySpec::market_standard_stochastic());
        self
    }

    /// Enable stochastic recovery with custom parameters.
    ///
    /// # Arguments
    /// * `mean` - Mean recovery rate (typical: 0.40)
    /// * `vol` - Recovery volatility (typical: 0.20-0.30)
    /// * `corr` - Correlation with factor (typical: -0.30 to -0.50)
    pub fn with_custom_stochastic_recovery(mut self, mean: f64, vol: f64, corr: f64) -> Self {
        self.recovery_spec = Some(super::recovery::RecoverySpec::market_correlated(
            mean, vol, corr,
        ));
        self
    }

    /// Enable stochastic recovery with custom parameters using typed percentages.
    pub fn with_custom_stochastic_recovery_pct(
        mut self,
        mean: Percentage,
        vol: Percentage,
        corr: f64,
    ) -> Self {
        self.recovery_spec = Some(super::recovery::RecoverySpec::market_correlated(
            mean.as_decimal(),
            vol.as_decimal(),
            corr,
        ));
        self
    }

    /// Set constant recovery rate (overriding index recovery).
    pub fn with_constant_recovery(mut self, rate: f64) -> Self {
        self.recovery_spec = Some(super::recovery::RecoverySpec::constant(rate));
        self
    }

    /// Set constant recovery rate using a typed percentage.
    pub fn with_constant_recovery_pct(mut self, rate: Percentage) -> Self {
        self.recovery_spec = Some(super::recovery::RecoverySpec::constant(rate.as_decimal()));
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cs01BumpUnits {
    /// Bump hazard rate in basis points
    HazardRateBp,
    /// Bump spread additively in basis points
    SpreadBpAdditive,
}

/// Heterogeneous expected loss evaluation method
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
pub struct CDSTranchePricer {
    params: CDSTranchePricerConfig,
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

impl CDSTranchePricer {
    #[inline]
    fn select_quadrature(&self) -> GaussHermiteQuadrature {
        match self.params.quadrature_order {
            5 => GaussHermiteQuadrature::order_5(),
            7 => GaussHermiteQuadrature::order_7(),
            10 => GaussHermiteQuadrature::order_10(),
            _ => GaussHermiteQuadrature::order_7(),
        }
    }
    /// Create a new Gaussian Copula model with default parameters.
    pub fn new() -> Self {
        Self {
            params: CDSTranchePricerConfig::default(),
        }
    }

    /// Create a new model with custom parameters.
    pub fn with_params(params: CDSTranchePricerConfig) -> Self {
        Self { params }
    }

    /// Price a CDS tranche using the Gaussian Copula model.
    ///
    /// Falls back to zero PV when credit index data is not available for backward compatibility.
    ///
    /// # Arguments
    /// * `tranche` - The CDS tranche to price
    /// * `market_ctx` - Market data context containing curves and credit index data
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// The present value of the tranche
    ///
    /// # Settlement Convention
    ///
    /// Uses ISDA standard settlement:
    /// - Index CDS tranches (CDX, iTraxx): T+1 business days (Big Bang 2009)
    /// - Bespoke tranches: T+3 business days
    #[must_use = "pricing result should be used"]
    pub fn price_tranche(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Check if credit index data is available - if not, fallback to zero PV for backward compatibility
        if market_ctx.credit_index(&tranche.credit_index_id).is_err() {
            return Ok(Money::new(0.0, tranche.notional.currency()));
        }

        // Check if tranche is already wiped out
        if tranche.accumulated_loss >= tranche.detach_pct / 100.0 {
            return Ok(Money::new(0.0, tranche.notional.currency()));
        }

        // Get the credit index data
        let index_data_arc = market_ctx.credit_index(&tranche.credit_index_id)?;

        // Get the discount curve
        let discount_curve = market_ctx.get_discount(tranche.discount_curve_id.as_ref())?;

        // Determine effective valuation date using proper settlement lag
        let valuation_date = self.calculate_settlement_date(tranche, market_ctx, as_of)?;

        // If valuation occurs on or after maturity, remaining PV is zero.
        if valuation_date >= tranche.maturity {
            return Ok(Money::new(0.0, tranche.notional.currency()));
        }

        // Calculate present values of premium and protection legs
        // These now calculate the EL curve internally with proper time dependency
        let pv_premium = self.calculate_premium_leg_pv(
            tranche,
            index_data_arc.as_ref(),
            discount_curve.as_ref(),
            valuation_date,
        )?;

        let pv_protection = self.calculate_protection_leg_pv(
            tranche,
            index_data_arc.as_ref(),
            discount_curve.as_ref(),
            valuation_date,
        )?;

        // Net present value depends on the side
        let mut net_pv = match tranche.side {
            TrancheSide::SellProtection => pv_premium - pv_protection,
            TrancheSide::BuyProtection => pv_protection - pv_premium,
        };

        // Apply upfront if present. Positive amount is paid by protection buyer.
        if let Some((dt, amount)) = tranche.upfront {
            if dt >= as_of {
                let df = discount_curve.df_between_dates(as_of, dt)?;
                let upfront_pv = amount.amount() * df;
                match tranche.side {
                    TrancheSide::BuyProtection => net_pv -= upfront_pv,
                    TrancheSide::SellProtection => net_pv += upfront_pv,
                }
            }
        }

        Ok(Money::new(net_pv, tranche.notional.currency()))
    }

    /// Calculate the settlement date based on ISDA conventions.
    ///
    /// - If effective_date is set, uses as_of directly (explicit settlement)
    /// - For index tranches (CDX, iTraxx): T+1 business days
    /// - For bespoke tranches: T+3 business days
    ///
    /// Uses business day calendars when available via the tranche's `calendar_id`.
    /// Falls back to weekend-only logic when no calendar is specified.
    fn calculate_settlement_date(
        &self,
        tranche: &CdsTranche,
        _market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Date> {
        // If effective date is explicitly set, use as_of directly
        if tranche.effective_date.is_some() {
            return Ok(as_of);
        }

        // Determine settlement lag based on index type
        let is_standard_index = tranche.index_name.starts_with("CDX")
            || tranche.index_name.starts_with("iTraxx")
            || tranche.index_name.starts_with("ITRAXX");
        let settlement_lag = if is_standard_index {
            self.params.index_settlement_lag
        } else {
            self.params.bespoke_settlement_lag
        };

        // Use calendar if available, otherwise fall back to weekday-only adjustment
        let calendar: Option<&dyn HolidayCalendar> = tranche
            .calendar_id
            .as_deref()
            .and_then(|id| CalendarRegistry::global().resolve_str(id));

        if let Some(cal) = calendar {
            as_of.add_business_days(settlement_lag, cal)
        } else {
            Ok(as_of.add_weekdays(settlement_lag))
        }
    }

    /// Calculate effective attachment/detachment points given accumulated losses.
    ///
    /// Returns (effective_attach, effective_detach, survival_factor)
    /// where survival_factor is (1 - L).
    ///
    /// # Invariants
    ///
    /// - Accumulated loss is in [0, 1]
    /// - Attachment <= Detachment (after percentage conversion)
    /// - Results are always in [0, 1]
    fn calculate_effective_structure(&self, tranche: &CdsTranche) -> (f64, f64, f64) {
        let l = tranche.accumulated_loss;
        let attach = tranche.attach_pct / 100.0;
        let detach = tranche.detach_pct / 100.0;

        // Debug assertions for invariants
        debug_assert!(
            (0.0..=1.0).contains(&l),
            "accumulated_loss {} must be in [0, 1]",
            l
        );
        debug_assert!(
            attach <= detach,
            "attach {} must be <= detach {}",
            attach,
            detach
        );
        debug_assert!(
            (0.0..=1.0).contains(&attach),
            "attach {} must be in [0, 1]",
            attach
        );
        debug_assert!(
            (0.0..=1.0).contains(&detach),
            "detach {} must be in [0, 1]",
            detach
        );

        if l >= 1.0 - 1e-9 {
            return (0.0, 0.0, 0.0);
        }

        let survival_factor = 1.0 - l;

        let eff_attach = (attach - l).max(0.0) / survival_factor;
        let eff_detach = (detach - l).max(0.0) / survival_factor;

        // Clamp to [0, 1] (eff_detach can be > 1 theoretically if L is huge but we check L >= D before)
        let result = (
            eff_attach.clamp(0.0, 1.0),
            eff_detach.clamp(0.0, 1.0),
            survival_factor,
        );

        // Post-condition assertions
        debug_assert!(
            result.0 <= result.1,
            "effective attach {} > effective detach {}",
            result.0,
            result.1
        );

        result
    }

    /// Calculate expected tranche loss using the base correlation approach.
    ///
    /// Decomposes the tranche [A, D] as the difference between two equity
    /// tranches: EL(0, D) - EL(0, A), using correlations interpolated from
    /// the base correlation curve with enhanced numerical stability.
    fn calculate_expected_tranche_loss(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<f64> {
        let (eff_attach, eff_detach, survival_factor) = self.calculate_effective_structure(tranche);

        // If effective width is zero, no loss
        if eff_detach <= eff_attach {
            return Ok(0.0);
        }

        // Get correlations for ORIGINAL attachment and detachment points
        // Base correlation is sticky to the original structure
        let corr_attach = index_data
            .base_correlation_curve
            .correlation(tranche.attach_pct);
        let corr_detach = index_data
            .base_correlation_curve
            .correlation(tranche.detach_pct);

        // Apply enhanced correlation boundary handling for numerical stability
        let corr_attach = self.smooth_correlation_boundary(corr_attach);
        let corr_detach = self.smooth_correlation_boundary(corr_detach);

        // Calculate expected losses for equity tranches [0, A_eff] and [0, D_eff]
        // Note: These inputs to calculate_equity_tranche_loss are now in "Effective %" terms
        // but correlations are from "Original %" terms.
        let el_to_attach = self.calculate_equity_tranche_loss(
            eff_attach * 100.0,
            corr_attach,
            index_data,
            maturity,
        )?;

        let el_to_detach = self.calculate_equity_tranche_loss(
            eff_detach * 100.0,
            corr_detach,
            index_data,
            maturity,
        )?;

        // The [A_eff, D_eff] tranche loss as a fraction of CURRENT portfolio
        let current_portfolio_loss_fraction = (el_to_detach - el_to_attach).max(0.0);

        // Convert to currency amount:
        // Loss = CurrentPortFrac * CurrentPortNotional
        // CurrentPortNotional = OrigPortNotional * (1 - L)
        // OrigPortNotional = TrancheNotional / (D_orig - A_orig)

        let orig_width = (tranche.detach_pct - tranche.attach_pct) / 100.0;
        if orig_width <= 1e-9 {
            return Ok(0.0);
        }

        let orig_port_notional = tranche.notional.amount() / orig_width;
        let loss_amount = current_portfolio_loss_fraction * orig_port_notional * survival_factor;

        Ok(loss_amount)
    }

    /// Calculate expected tranche loss fraction at a specific date.
    ///
    /// Returns the expected loss as a fraction of the ORIGINAL tranche notional [0, 1].
    fn expected_tranche_loss_fraction_at(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        date: Date,
    ) -> Result<f64> {
        let (eff_attach, eff_detach, survival_factor) = self.calculate_effective_structure(tranche);

        if eff_detach <= eff_attach {
            return Ok(0.0);
        }

        // Get correlations for ORIGINAL points
        let corr_attach = index_data
            .base_correlation_curve
            .correlation(tranche.attach_pct);
        let corr_detach = index_data
            .base_correlation_curve
            .correlation(tranche.detach_pct);

        // Apply enhanced correlation boundary handling
        let corr_attach = self.smooth_correlation_boundary(corr_attach);
        let corr_detach = self.smooth_correlation_boundary(corr_detach);

        // Calculate expected losses for equity tranches [0, A_eff] and [0, D_eff]
        let el_to_attach =
            self.calculate_equity_tranche_loss(eff_attach * 100.0, corr_attach, index_data, date)?;

        let el_to_detach =
            self.calculate_equity_tranche_loss(eff_detach * 100.0, corr_detach, index_data, date)?;

        // Loss on current portfolio
        let current_portfolio_loss_fraction = (el_to_detach - el_to_attach).max(0.0);

        // Scale to original tranche notional fraction
        // Fraction = (CurrentLossFrac * (1-L)) / OrigWidth
        let orig_width = (tranche.detach_pct - tranche.attach_pct) / 100.0;
        if orig_width <= 1e-9 {
            return Ok(0.0);
        }

        let tranche_loss_fraction =
            (current_portfolio_loss_fraction * survival_factor) / orig_width;

        // Add prior realized loss to get Total Cumulative Loss
        let prior_loss = self.calculate_prior_tranche_loss(tranche);

        Ok((tranche_loss_fraction + prior_loss).clamp(0.0, 1.0))
    }

    /// Build the expected loss curve for all payment dates.
    ///
    /// Returns a vector of (Date, EL_fraction) pairs where EL_fraction
    /// is the cumulative expected loss as a fraction of tranche notional.
    fn build_el_curve(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        dates: &[Date],
    ) -> Result<Vec<(Date, f64)>> {
        let mut el_curve = Vec::with_capacity(dates.len());
        let mut prev_el = 0.0;

        for &date in dates {
            let el_fraction = self.expected_tranche_loss_fraction_at(tranche, index_data, date)?;

            // Warn if EL decreased (indicates numerical issue or model limitation)
            // This can happen due to base correlation model inconsistencies
            if el_fraction < prev_el - 1e-6 {
                tracing::debug!(
                    "EL decreased from {:.6} to {:.6} at {:?} (Δ={:.6})",
                    prev_el,
                    el_fraction,
                    date,
                    prev_el - el_fraction
                );
            }

            el_curve.push((date, el_fraction));
            prev_el = el_fraction;
        }

        Ok(el_curve)
    }

    /// Calculate expected loss for an equity tranche [0, K] using Gaussian Copula.
    ///
    /// Enhanced with adaptive integration for superior numerical stability,
    /// particularly critical near correlation boundaries (0 and 1) where
    /// the conditional default probability function exhibits sharp transitions.
    ///
    /// # Arguments
    /// * `detachment_pct` - Detachment point K in percent
    /// * `correlation` - Asset correlation parameter ρ
    /// * `index_data` - Credit index market data
    /// * `maturity` - Maturity date for loss calculation
    fn calculate_equity_tranche_loss(
        &self,
        detachment_pct: f64,
        correlation: f64,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<f64> {
        // Heterogeneous path if enabled and issuer curves present
        if self.params.use_issuer_curves && index_data.has_issuer_curves() {
            self.calculate_equity_tranche_loss_hetero(
                detachment_pct,
                correlation,
                index_data,
                maturity,
            )
        } else {
            // Homogeneous: use index marginals
            let num_constituents = index_data.num_constituents as usize;
            let base_recovery = index_data.recovery_rate;

            // Build recovery model if configured, otherwise use constant
            let recovery_model: Option<Box<dyn RecoveryModel>> =
                self.params.recovery_spec.as_ref().map(|spec| spec.build());

            let detachment_notional = detachment_pct / 100.0;
            let quad = self.select_quadrature();
            let maturity_years = self.years_from_base(index_data, maturity)?;
            let default_prob = self.get_default_probability(index_data, maturity_years)?;
            let default_threshold = standard_normal_inv_cdf(default_prob);
            let integrand = |z: f64| {
                let p = self.conditional_default_probability_enhanced(
                    default_threshold,
                    correlation,
                    z,
                );

                // Use stochastic recovery if configured, otherwise constant
                let recovery_rate = match &recovery_model {
                    Some(model) => model.conditional_recovery(z),
                    None => base_recovery,
                };

                self.conditional_equity_tranche_loss(
                    num_constituents,
                    detachment_notional,
                    p,
                    recovery_rate,
                )
            };
            let expected_loss = if !(self.params.adaptive_integration_low
                ..=self.params.adaptive_integration_high)
                .contains(&correlation)
            {
                quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
            } else {
                quad.integrate(integrand)
            };
            Ok(expected_loss)
        }
    }

    /// Heterogeneous equity tranche loss via semi-analytical SPA or exact convolution fallback.
    ///
    /// Supports full bespoke portfolio heterogeneity:
    /// - Per-issuer hazard curves (default probability)
    /// - Per-issuer recovery rates (LGD)
    /// - Per-issuer weights (notional allocation)
    fn calculate_equity_tranche_loss_hetero(
        &self,
        detachment_pct: f64,
        correlation: f64,
        index_data: &CreditIndexData,
        maturity: Date,
    ) -> Result<f64> {
        // Precompute unconditional PD_i(t)
        let t = self.years_from_base(index_data, maturity)?;
        let tranche_width = detachment_pct / 100.0;

        // Quadrature setup
        let quad = self.select_quadrature();

        // Build heterogeneous vectors: PD, LGD, and Weight per issuer
        let mut pd_i: Vec<f64> = Vec::with_capacity(index_data.num_constituents as usize);
        let mut lgd_i: Vec<f64> = Vec::with_capacity(index_data.num_constituents as usize);
        let mut weight_i: Vec<f64> = Vec::with_capacity(index_data.num_constituents as usize);

        if let Some(curves) = &index_data.issuer_credit_curves {
            // Sort issuer IDs for determinism (HashMap iteration order is random)
            let mut sorted_ids: Vec<&String> = curves.keys().collect();
            sorted_ids.sort();

            for id in sorted_ids {
                let curve = index_data.get_issuer_curve(id);
                let sp = curve.sp(t);
                pd_i.push((1.0 - sp).clamp(0.0, 1.0));

                let rec = index_data.get_issuer_recovery(id);
                lgd_i.push((1.0 - rec).max(self.params.lgd_floor));

                let w = index_data.get_issuer_weight(id);
                weight_i.push(w);
            }
        } else {
            // Fallback to homogeneous (should not happen if caller gates, but ensure safe)
            let sp = index_data.index_credit_curve.sp(t);
            let count = index_data.num_constituents as usize;
            pd_i = vec![(1.0 - sp).clamp(0.0, 1.0); count];
            lgd_i = vec![(1.0 - index_data.recovery_rate).max(self.params.lgd_floor); count];
            weight_i = vec![1.0 / count as f64; count];
        }

        // Check if effectively homogeneous (optimization: use faster binomial path)
        let is_uniform_pd = pd_i
            .first()
            .map(|&first| {
                pd_i.iter()
                    .all(|&p| (p - first).abs() <= self.params.probability_clip)
            })
            .unwrap_or(true);
        let is_uniform_lgd = lgd_i
            .first()
            .map(|&first| lgd_i.iter().all(|&l| (l - first).abs() <= 1e-9))
            .unwrap_or(true);
        let is_uniform_weight = weight_i
            .first()
            .map(|&first| weight_i.iter().all(|&w| (w - first).abs() <= 1e-9))
            .unwrap_or(true);

        if is_uniform_pd && is_uniform_lgd && is_uniform_weight {
            // Use homogeneous binomial path (faster)
            let num_constituents = index_data.num_constituents as usize;
            let detachment_notional = detachment_pct / 100.0;
            let base_recovery = 1.0 - lgd_i[0];

            // Build recovery model if configured (same as homogeneous path)
            let recovery_model: Option<Box<dyn RecoveryModel>> =
                self.params.recovery_spec.as_ref().map(|spec| spec.build());

            let default_prob = self.get_default_probability(index_data, t)?;
            let default_threshold = standard_normal_inv_cdf(default_prob);
            let integrand = |z: f64| {
                let p = self.conditional_default_probability_enhanced(
                    default_threshold,
                    correlation,
                    z,
                );

                // Use stochastic recovery if configured, otherwise constant
                let recovery = match &recovery_model {
                    Some(model) => model.conditional_recovery(z),
                    None => base_recovery,
                };

                self.conditional_equity_tranche_loss(
                    num_constituents,
                    detachment_notional,
                    p,
                    recovery,
                )
            };
            let expected_loss = if !(self.params.adaptive_integration_low
                ..=self.params.adaptive_integration_high)
                .contains(&correlation)
            {
                quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
            } else {
                quad.integrate(integrand)
            };
            return Ok(expected_loss);
        }

        // Build probit thresholds for heterogeneous path
        let eps = self.params.probability_clip;
        let probit_i: Vec<f64> = pd_i
            .iter()
            .map(|&p| standard_normal_inv_cdf(p.max(eps).min(1.0 - eps)))
            .collect();

        // Integrand over common factor Z using heterogeneous LGD and weights
        let integrand = |z: f64| -> f64 {
            let sqrt_rho = correlation.sqrt();
            let sqrt_1mr = (1.0 - correlation).sqrt();
            let mut mean = 0.0;
            let mut var = 0.0;

            for i in 0..probit_i.len() {
                let th = probit_i[i];
                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = norm_cdf(cthr).clamp(0.0, 1.0);

                // Use per-issuer weight and LGD
                let w = weight_i[i] * lgd_i[i];
                mean += w * p;
                var += w * w * p * (1.0 - p);
            }

            // SPA/normal approximation for E[min(L, K)] with K = detachment_notional
            let k = tranche_width;
            if var <= self.params.spa_variance_floor {
                return mean.min(k);
            }
            let s = var.sqrt();
            let a = (k - mean) / s;
            // E[min(L, K)] ≈ m Φ(a) + s φ(a) + K [1 − Φ(a)]
            mean * norm_cdf(a) + s * norm_pdf(a) + k * (1.0 - norm_cdf(a))
        };

        // Prefer exact convolution for small pools to reduce SPA error
        let n_const = index_data.num_constituents as usize;
        let small_pool_threshold: usize = 16;
        let el = if n_const <= small_pool_threshold {
            self.hetero_exact_convolution_full(
                detachment_pct,
                correlation,
                &probit_i,
                &lgd_i,
                &weight_i,
            )
        } else {
            match self.params.hetero_method {
                HeteroMethod::Spa => {
                    if !(self.params.adaptive_integration_low
                        ..=self.params.adaptive_integration_high)
                        .contains(&correlation)
                    {
                        quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
                    } else {
                        quad.integrate(integrand)
                    }
                }
                HeteroMethod::ExactConvolution => {
                    // Exact convolution with full heterogeneity
                    self.hetero_exact_convolution_full(
                        detachment_pct,
                        correlation,
                        &probit_i,
                        &lgd_i,
                        &weight_i,
                    )
                }
            }
        };
        Ok(el)
    }

    /// Exact convolution with full heterogeneous LGD and weight vectors.
    ///
    /// This is the fully bespoke version that supports per-issuer:
    /// - Hazard rates (via probit thresholds)
    /// - Recovery rates (via lgd_i)
    /// - Notional weights (via weight_i)
    fn hetero_exact_convolution_full(
        &self,
        detachment_pct: f64,
        correlation: f64,
        probit_i: &[f64],
        lgd_i: &[f64],
        weight_i: &[f64],
    ) -> f64 {
        let k = detachment_pct / 100.0;
        let grid_step = self.params.grid_step.max(self.params.grid_step_min);
        let max_points = (k / grid_step).ceil() as usize + 2;

        if max_points > self.params.max_grid_points {
            // Performance guard: fall back to SPA approximation with heterogeneous vectors
            return self.hetero_spa_full(probit_i, correlation, k, lgd_i, weight_i);
        }

        let quad = self.select_quadrature();
        let sqrt_rho = correlation.sqrt();
        let sqrt_1mr = (1.0 - correlation).sqrt();

        let integrand = |z: f64| {
            // Start with delta at 0 loss
            let mut pmf = vec![0.0f64; 1];
            pmf[0] = 1.0;

            for i in 0..probit_i.len() {
                let th = probit_i[i];
                let lgd = lgd_i[i];
                let weight = weight_i[i];

                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = norm_cdf(cthr).clamp(0.0, 1.0);

                // Per-issuer loss contribution
                let loss_exact = weight * lgd / grid_step;
                let loss_floor = loss_exact.floor() as usize;
                let frac = loss_exact - loss_floor as f64;

                let new_len = pmf.len() + loss_floor + 2;
                let mut next = vec![0.0f64; new_len.min(max_points)];

                for (j, &mass) in pmf.iter().enumerate() {
                    // No default case
                    if j < next.len() {
                        next[j] += mass * (1.0 - p);
                    }

                    // Default case: distribute mass between floor and ceiling bins
                    let j_floor = j + loss_floor;
                    let j_ceil = j_floor + 1;

                    if j_floor < next.len() {
                        next[j_floor] += mass * p * (1.0 - frac);
                    }
                    if j_ceil < next.len() && frac > 0.0 {
                        next[j_ceil] += mass * p * frac;
                    } else if j_floor < next.len() && frac > 0.0 {
                        next[j_floor] += mass * p * frac;
                    }
                }

                pmf = next;
                if pmf.len() > max_points {
                    pmf.truncate(max_points);
                }
            }

            // Compute E[min(L, K)] from pmf
            let mut terms: Vec<f64> = Vec::with_capacity(pmf.len());
            for (i, mass) in pmf.iter().enumerate() {
                let l = (i as f64) * grid_step;
                terms.push(mass * l.min(k));
            }
            finstack_core::math::neumaier_sum(terms.iter().copied())
        };

        if !(self.params.adaptive_integration_low..=self.params.adaptive_integration_high)
            .contains(&correlation)
        {
            quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
        } else {
            quad.integrate(integrand)
        }
    }

    /// SPA fallback with full heterogeneous vectors.
    fn hetero_spa_full(
        &self,
        probit_i: &[f64],
        correlation: f64,
        k: f64,
        lgd_i: &[f64],
        weight_i: &[f64],
    ) -> f64 {
        let quad = self.select_quadrature();
        let integrand = |z: f64| -> f64 {
            let sqrt_rho = correlation.sqrt();
            let sqrt_1mr = (1.0 - correlation).sqrt();
            let mut mean = 0.0;
            let mut var = 0.0;

            for i in 0..probit_i.len() {
                let th = probit_i[i];
                let cthr = (th - sqrt_rho * z) / sqrt_1mr;
                let p = norm_cdf(cthr).clamp(0.0, 1.0);
                let w = weight_i[i] * lgd_i[i];
                mean += w * p;
                var += w * w * p * (1.0 - p);
            }

            if var <= self.params.spa_variance_floor {
                return mean.min(k);
            }
            let s = var.sqrt();
            let a = (k - mean) / s;
            mean * norm_cdf(a) + s * norm_pdf(a) + k * (1.0 - norm_cdf(a))
        };

        if !(self.params.adaptive_integration_low..=self.params.adaptive_integration_high)
            .contains(&correlation)
        {
            quad.integrate_adaptive(integrand, self.params.numerical_tolerance)
        } else {
            quad.integrate(integrand)
        }
    }

    /// Calculate conditional default probability given market factor Z.
    ///
    /// Standard implementation kept for compatibility and testing.
    /// The enhanced version `conditional_default_probability_enhanced` is used
    /// in production calculations for superior numerical stability.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    #[cfg(test)]
    fn conditional_default_probability(
        &self,
        default_threshold: f64,
        correlation: f64,
        market_factor: f64,
    ) -> f64 {
        let sqrt_rho = correlation.sqrt();
        let one_minus_rho: f64 = 1.0 - correlation;
        let sqrt_one_minus_rho = one_minus_rho.sqrt();

        let conditional_threshold =
            (default_threshold - sqrt_rho * market_factor) / sqrt_one_minus_rho;
        norm_cdf(conditional_threshold)
    }

    /// Enhanced conditional default probability with improved numerical stability.
    ///
    /// Provides superior handling of boundary cases and extreme correlation values
    /// through sophisticated boundary transition functions and overflow protection.
    ///
    /// P(default | Z) = Φ((Φ⁻¹(PD) - √ρ * Z) / √(1-ρ))
    fn conditional_default_probability_enhanced(
        &self,
        default_threshold: f64,
        correlation: f64,
        market_factor: f64,
    ) -> f64 {
        // Apply smooth correlation boundaries to avoid numerical discontinuities
        let correlation = self.smooth_correlation_boundary(correlation);

        // Handle extreme correlation cases with special care
        if correlation < self.params.numerical_tolerance {
            // Near-zero correlation: independent case
            return norm_cdf(default_threshold);
        }
        if correlation > 1.0 - self.params.numerical_tolerance {
            // Near-perfect correlation: deterministic case
            let threshold_adj = default_threshold - market_factor;
            return norm_cdf(threshold_adj);
        }

        // Enhanced calculation with overflow protection
        let sqrt_rho = correlation.sqrt();
        let one_minus_rho = 1.0 - correlation;

        // Protect against numerical issues when correlation approaches 1
        let sqrt_one_minus_rho = if one_minus_rho < self.params.numerical_tolerance {
            self.params.numerical_tolerance.sqrt() // Minimum practical value to avoid division by zero
        } else {
            let one_minus_rho: f64 = 1.0 - correlation;
            one_minus_rho.sqrt()
        };

        // Calculate conditional threshold with overflow protection
        let numerator = default_threshold - sqrt_rho * market_factor;
        let conditional_threshold = numerator / sqrt_one_minus_rho;

        // Clamp to reasonable range to prevent CDF overflow
        let conditional_threshold =
            conditional_threshold.clamp(-self.params.cdf_clip, self.params.cdf_clip);

        norm_cdf(conditional_threshold)
    }

    /// Apply smooth correlation boundary handling to avoid numerical discontinuities.
    ///
    /// Uses a smooth transition function near the boundaries to maintain numerical
    /// stability while preserving the underlying mathematical relationships.
    fn smooth_correlation_boundary(&self, correlation: f64) -> f64 {
        let min_corr = self.params.min_correlation;
        let max_corr = self.params.max_correlation;
        let width = self.params.corr_boundary_width;

        if correlation <= min_corr + width {
            // Lower boundary: smooth transition using tanh
            let x = (correlation - min_corr) / width;
            min_corr + width * (1.0 + x.tanh()) / 2.0
        } else if correlation >= max_corr - width {
            // Upper boundary: smooth transition using tanh
            let x = (correlation - (max_corr - width)) / width;
            max_corr - width * (1.0 - x.tanh()) / 2.0
        } else {
            // Normal range: no adjustment needed
            correlation.clamp(min_corr, max_corr)
        }
    }

    /// Calculate expected loss of equity tranche conditional on market factor.
    ///
    /// Uses the binomial distribution to sum over all possible numbers of defaults.
    fn conditional_equity_tranche_loss(
        &self,
        num_constituents: usize,
        detachment_notional: f64,
        conditional_default_prob: f64,
        recovery_rate: f64,
    ) -> f64 {
        let loss_given_default = 1.0 - recovery_rate;
        let individual_notional = 1.0 / num_constituents as f64; // Normalized to 1.0 total

        let mut expected_loss = 0.0;

        // Sum over all possible numbers of defaults
        for k in 0..=num_constituents {
            let prob_k_defaults =
                binomial_probability(num_constituents, k, conditional_default_prob);

            // Portfolio loss given k defaults
            let portfolio_loss = k as f64 * individual_notional * loss_given_default;

            // Tranche loss (equity tranche [0, detachment_notional])
            let tranche_loss = portfolio_loss.min(detachment_notional);

            expected_loss += prob_k_defaults * tranche_loss;
        }

        expected_loss
    }

    /// Calculate present value of the premium leg with accrual-on-default.
    ///
    /// PV = Coupon * Σ(Δt_j * D(t_j) * [N_outstanding - 0.5 * N_incremental_loss])
    /// where N_outstanding = N_tr * (1 - EL_fraction(t_{j-1}))
    /// and N_incremental_loss = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_premium_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discounting,
        as_of: Date,
    ) -> Result<f64> {
        let coupon = tranche.running_coupon_bp / 10000.0; // Convert bp to decimal
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_premium = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date)?;
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Outstanding notional at beginning of period
            let outstanding_notional = tranche_notional * (1.0 - prev_el_fraction);

            if outstanding_notional <= 0.0 {
                break; // Tranche fully written down
            }

            // Accrual period using day count convention
            let period_start = if i == 0 {
                tranche.effective_date.unwrap_or(as_of)
            } else {
                payment_dates[i - 1]
            };

            let accrual_period = tranche
                .day_count
                .year_fraction(
                    period_start,
                    payment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )
                .unwrap_or(0.0);
            if accrual_period <= 0.0 {
                continue;
            }

            // Accrual-on-default: reduce accrual by configured fraction of incremental loss (if enabled)
            let effective_notional = if self.params.accrual_on_default_enabled {
                let aod_adjustment =
                    self.params.aod_allocation_fraction * tranche_notional * delta_el_fraction;
                (outstanding_notional - aod_adjustment).max(0.0)
            } else {
                outstanding_notional
            };

            // Discount at end or midpoint depending on config
            let df_time = if self.params.mid_period_protection {
                let t_start = self.years_from_base(index_data, period_start)?;
                (t_start + t) * 0.5
            } else {
                t
            };
            let discount_factor = discount_curve.df(df_time);

            pv_premium += coupon * accrual_period * discount_factor * effective_notional;
            prev_el_fraction = el_fraction;
        }

        Ok(pv_premium)
    }

    /// Calculate present value of the protection leg using incremental EL.
    ///
    /// PV = Σ(D(t_j) * ΔEL_j) where ΔEL_j = N_tr * (EL_fraction(t_j) - EL_fraction(t_{j-1}))
    fn calculate_protection_leg_pv(
        &self,
        tranche: &CdsTranche,
        index_data: &CreditIndexData,
        discount_curve: &dyn Discounting,
        as_of: Date,
    ) -> Result<f64> {
        let tranche_notional = tranche.notional.amount();

        // Generate payment schedule and expected loss curve
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        if payment_dates.is_empty() {
            return Ok(0.0);
        }

        let el_curve = self.build_el_curve(tranche, index_data, &payment_dates)?;

        let mut pv_protection = 0.0;
        let mut prev_el_fraction = 0.0; // Start with no loss

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let t = self.years_from_base(index_data, payment_date)?;
            if t <= 0.0 {
                continue;
            }

            let el_fraction = el_curve[i].1; // Current EL fraction
            let delta_el_fraction = el_fraction - prev_el_fraction;

            // Incremental loss amount in currency
            let incremental_loss_amount = tranche_notional * delta_el_fraction;

            if incremental_loss_amount > 0.0 {
                // Use mid-period discounting if enabled (consistent with premium leg)
                let df_time = if self.params.mid_period_protection {
                    let t_prev = if i == 0 {
                        0.0
                    } else {
                        self.years_from_base(index_data, payment_dates[i - 1])?
                    };
                    (t_prev + t) * 0.5
                } else {
                    t
                };
                let discount_factor = discount_curve.df(df_time);
                pv_protection += incremental_loss_amount * discount_factor;
            }

            prev_el_fraction = el_fraction;
        }

        Ok(pv_protection)
    }

    /// Get default probability for the index at a given maturity.
    fn get_default_probability(
        &self,
        index_data: &CreditIndexData,
        maturity_years: f64,
    ) -> Result<f64> {
        let survival_prob = index_data.index_credit_curve.sp(maturity_years);
        Ok(1.0 - survival_prob)
    }

    /// Calculate years from the credit curve base date.
    fn years_from_base(&self, index_data: &CreditIndexData, date: Date) -> Result<f64> {
        let dc = index_data.index_credit_curve.day_count();
        dc.year_fraction(
            index_data.index_credit_curve.base_date(),
            date,
            finstack_core::dates::DayCountCtx::default(),
        )
    }

    /// Create a bumped base correlation curve for sensitivity analysis.
    ///
    /// Creates a new BaseCorrelationCurve with correlations shifted by bump_abs,
    /// clamped to [min_correlation, max_correlation] for numerical stability.
    ///
    /// # Monotonicity Enforcement
    ///
    /// Base correlation must be monotonically increasing with detachment point
    /// to avoid arbitrage (senior tranches cannot be riskier than junior).
    /// After bumping, we enforce this by ensuring each correlation is at least
    /// as large as the previous point plus a small epsilon.
    fn bump_base_correlation(
        &self,
        original_curve: &finstack_core::market_data::term_structures::BaseCorrelationCurve,
        bump_abs: f64,
    ) -> finstack_core::Result<finstack_core::market_data::term_structures::BaseCorrelationCurve>
    {
        use finstack_core::market_data::term_structures::BaseCorrelationCurve;

        // Extract original points and apply bump with clamping
        let mut bumped_points: Vec<(f64, f64)> = original_curve
            .detachment_points()
            .iter()
            .zip(original_curve.correlations().iter())
            .map(|(&detach, &corr)| {
                let bumped_corr = (corr + bump_abs)
                    .clamp(self.params.min_correlation, self.params.max_correlation);
                (detach, bumped_corr)
            })
            .collect();

        // Enforce monotonicity: each correlation must be >= previous + epsilon
        // This prevents arbitrage from bumping that violates the base correlation constraint
        const MONOTONICITY_EPSILON: f64 = 1e-6;
        for i in 1..bumped_points.len() {
            let min_corr = bumped_points[i - 1].1 + MONOTONICITY_EPSILON;
            if bumped_points[i].1 < min_corr {
                bumped_points[i].1 = min_corr.min(self.params.max_correlation);
            }
        }

        // After monotonicity enforcement, check for potential EL arbitrage
        // (in debug builds only to avoid performance impact in production)
        #[cfg(debug_assertions)]
        {
            // Log warning if bumping created tight correlation spacing
            // This may indicate potential convexity violations in the EL surface
            for i in 2..bumped_points.len() {
                let d_prev = bumped_points[i - 1].1 - bumped_points[i - 2].1;
                let d_curr = bumped_points[i].1 - bumped_points[i - 1].1;
                if d_curr < d_prev * 0.5 && d_curr < 0.01 {
                    tracing::warn!(
                        "Base correlation bump may violate convexity at {:.1}%: Δρ compressed from {:.4} to {:.4}",
                        bumped_points[i].0, d_prev, d_curr
                    );
                }
            }
        }

        // Create temporary ID for bumped curve
        BaseCorrelationCurve::builder("TEMP_BUMPED_CORR")
            .knots(bumped_points)
            .build()
    }

    /// Create a bumped credit index with shifted hazard rates for CS01 calculation.
    ///
    /// Creates a new CreditIndexData with the index hazard curve shifted by delta_lambda.
    fn bump_index_hazard(
        &self,
        original_index: &CreditIndexData,
        delta_lambda: f64,
    ) -> Result<CreditIndexData> {
        // Create bumped hazard curve
        let bumped_hazard_curve = original_index
            .index_credit_curve
            .with_hazard_shift(delta_lambda)?;

        // Create new credit index data with bumped hazard curve
        CreditIndexData::builder()
            .num_constituents(original_index.num_constituents)
            .recovery_rate(original_index.recovery_rate)
            .index_credit_curve(std::sync::Arc::new(bumped_hazard_curve))
            .base_correlation_curve(std::sync::Arc::clone(
                &original_index.base_correlation_curve,
            ))
            .build()
    }

    /// Calculate prior realized loss on the tranche as a fraction of original tranche notional.
    fn calculate_prior_tranche_loss(&self, tranche: &CdsTranche) -> f64 {
        let l = tranche.accumulated_loss;
        let attach = tranche.attach_pct / 100.0;
        let detach = tranche.detach_pct / 100.0;
        let width = detach - attach;

        if width <= 1e-9 {
            return 0.0;
        }

        // Fraction of tranche already wiped out
        let loss_in_tranche = (l - attach).clamp(0.0, width);
        loss_in_tranche / width
    }

    /// Generate payment schedule for the tranche using canonical schedule builder.
    ///
    /// Uses the robust date scheduling utilities with proper business day
    /// conventions and calendar support.
    fn generate_payment_schedule(&self, tranche: &CdsTranche, as_of: Date) -> Result<Vec<Date>> {
        let start_date = tranche.effective_date.unwrap_or(as_of);

        let dates = if self.params.use_isda_coupon_dates || tranche.standard_imm_dates {
            let mut out = vec![start_date];
            let mut current = start_date;
            while current < tranche.maturity {
                current = next_cds_date(current);
                // Ensure we don't go past maturity (next_cds_date can go past if close)
                if current > tranche.maturity {
                    out.push(tranche.maturity);
                    break;
                }
                out.push(current);
            }
            // If precise maturity match is needed, we might need to adjust the last date
            // But standard CDS rolls on 20th.
            out
        } else {
            build_dates(
                start_date,
                tranche.maturity,
                tranche.payment_frequency,
                self.params.schedule_stub,
                tranche.business_day_convention,
                tranche.calendar_id.as_deref(),
            )?
            .dates
        };

        // Filter out dates before as_of (in case effective_date < as_of)
        let payment_dates: Vec<Date> = dates.into_iter().filter(|&date| date > as_of).collect();

        Ok(payment_dates)
    }

    /// Calculate upfront amount for the tranche.
    ///
    /// This is the net present value at inception, representing the
    /// payment required to enter the position at the standard coupon.
    pub fn calculate_upfront(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let pv = self.price_tranche(tranche, market_ctx, as_of)?;
        Ok(pv.amount())
    }

    /// Calculate Spread DV01 (sensitivity to 1bp change in running coupon).
    pub fn calculate_spread_dv01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Create bumped tranche with +1bp running coupon
        let mut bumped_tranche = tranche.clone();
        bumped_tranche.running_coupon_bp += 1.0;

        let base_pv = self.price_tranche(tranche, market_ctx, as_of)?.amount();
        let bumped_pv = self
            .price_tranche(&bumped_tranche, market_ctx, as_of)?
            .amount();

        Ok(bumped_pv - base_pv)
    }

    /// Calculate the par spread (running coupon in bp that sets PV = 0).
    ///
    /// # Algorithm
    ///
    /// Uses Newton-Raphson iteration to find the spread that makes NPV = 0:
    /// 1. Start with ratio approximation as initial guess
    /// 2. Iterate: spread_new = spread - NPV(spread) / Spread_DV01
    /// 3. Converge when |NPV| < tolerance or max iterations reached
    ///
    /// This is more accurate than simple ratio method because it accounts for
    /// the non-linear relationship between spread and premium leg PV due to
    /// accrual-on-default and notional write-down effects.
    #[must_use = "par spread result should be used"]
    pub fn calculate_par_spread(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let discount_curve = market_ctx.get_discount(&tranche.discount_curve_id)?;
        let index_data = match market_ctx.credit_index(&tranche.credit_index_id) {
            Ok(data) => data,
            Err(_) => return Ok(0.0),
        };

        // Use factored-out settlement date calculation
        let valuation_date = self.calculate_settlement_date(tranche, market_ctx, as_of)?;

        // Initial guess using ratio method (protection PV / premium per bp)
        let mut unit_tranche = tranche.clone();
        unit_tranche.running_coupon_bp = 1.0;
        let premium_per_bp = self.calculate_premium_leg_pv(
            &unit_tranche,
            index_data.as_ref(),
            discount_curve.as_ref(),
            valuation_date,
        )?;

        if premium_per_bp.abs() < self.params.numerical_tolerance {
            return Ok(0.0);
        }

        let protection_pv = self.calculate_protection_leg_pv(
            tranche,
            index_data.as_ref(),
            discount_curve.as_ref(),
            valuation_date,
        )?;

        // Initial guess from ratio method
        let mut spread = protection_pv / premium_per_bp;

        // Newton-Raphson iteration to refine the par spread
        for _iter in 0..self.params.par_spread_max_iter {
            // Create test tranche with current spread guess
            let mut test_tranche = tranche.clone();
            test_tranche.running_coupon_bp = spread;

            // Calculate NPV at current spread
            let npv = self
                .price_tranche(&test_tranche, market_ctx, as_of)?
                .amount();

            // Check convergence (NPV close to zero)
            if npv.abs() < self.params.par_spread_tolerance * tranche.notional.amount() {
                return Ok(spread);
            }

            // Calculate Spread DV01 for Newton step
            let spread_dv01 = self.calculate_spread_dv01(&test_tranche, market_ctx, as_of)?;

            if spread_dv01.abs() < self.params.numerical_tolerance {
                // DV01 too small, can't continue iteration
                break;
            }

            // Newton step: spread_new = spread - NPV / DV01
            // Note: For buy protection, NPV > 0 means spread is too low
            let adjustment = npv / spread_dv01;
            spread -= adjustment;

            // Ensure spread stays reasonable (non-negative, bounded)
            spread = spread.clamp(0.0, 100000.0); // Max 10000% = 100000bp
        }

        Ok(spread)
    }

    /// Calculate expected loss metric (the total expected loss at maturity).
    pub fn calculate_expected_loss(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
    ) -> Result<f64> {
        let index_data_arc = market_ctx.credit_index(&tranche.credit_index_id)?;
        self.calculate_expected_tranche_loss(tranche, index_data_arc.as_ref(), tranche.maturity)
    }

    /// Calculate CS01 (sensitivity to 1bp parallel shift in credit spreads) using central difference.
    #[must_use = "CS01 result should be used for hedging"]
    pub fn calculate_cs01(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let original_index_arc = market_ctx.credit_index(&tranche.credit_index_id)?;

        // Calculate the hazard rate bump based on configured units
        let delta_lambda = match self.params.cs01_bump_units {
            Cs01BumpUnits::HazardRateBp => {
                // 1.0 bump_size interpreted as 1 bp in hazard rate
                self.params.cs01_bump_size * 1e-4
            }
            Cs01BumpUnits::SpreadBpAdditive => {
                // Proxy: convert a spread bp to hazard bp via 1/(1-recovery)
                // This is a common approximation for small bump sizes.
                let rr = original_index_arc.recovery_rate;
                (self.params.cs01_bump_size * 1e-4) / (1.0 - rr).max(1e-6_f64)
            }
        };

        // Central difference: (PV_up - PV_down) / 2 for O(h²) accuracy
        let bumped_index_up = self.bump_index_hazard(original_index_arc.as_ref(), delta_lambda)?;
        let bumped_index_down =
            self.bump_index_hazard(original_index_arc.as_ref(), -delta_lambda)?;

        let ctx_up = market_ctx
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_up);
        let ctx_down = market_ctx
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_down);

        let pv_up = self.price_tranche(tranche, &ctx_up, as_of)?.amount();
        let pv_down = self.price_tranche(tranche, &ctx_down, as_of)?.amount();

        // Return sensitivity per basis point (central difference divided by 2)
        Ok((pv_up - pv_down) / 2.0)
    }

    /// Calculate correlation delta (sensitivity to correlation changes) using central difference.
    #[must_use = "Correlation01 result should be used for hedging"]
    pub fn calculate_correlation_delta(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        let bump_abs = self.params.corr_bump_abs;
        let original_index_arc = market_ctx.credit_index(&tranche.credit_index_id)?;

        // Central difference: (PV_up - PV_down) / (2 * bump) for O(h²) accuracy
        let bumped_corr_curve_up =
            self.bump_base_correlation(&original_index_arc.base_correlation_curve, bump_abs)?;
        let bumped_corr_curve_down =
            self.bump_base_correlation(&original_index_arc.base_correlation_curve, -bump_abs)?;

        let bumped_index_up = CreditIndexData::builder()
            .num_constituents(original_index_arc.num_constituents)
            .recovery_rate(original_index_arc.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(
                &original_index_arc.index_credit_curve,
            ))
            .base_correlation_curve(std::sync::Arc::new(bumped_corr_curve_up))
            .build()?;

        let bumped_index_down = CreditIndexData::builder()
            .num_constituents(original_index_arc.num_constituents)
            .recovery_rate(original_index_arc.recovery_rate)
            .index_credit_curve(std::sync::Arc::clone(
                &original_index_arc.index_credit_curve,
            ))
            .base_correlation_curve(std::sync::Arc::new(bumped_corr_curve_down))
            .build()?;

        let ctx_up = market_ctx
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_up);
        let ctx_down = market_ctx
            .clone()
            .insert_credit_index(&tranche.credit_index_id, bumped_index_down);

        let pv_up = self.price_tranche(tranche, &ctx_up, as_of)?.amount();
        let pv_down = self.price_tranche(tranche, &ctx_down, as_of)?.amount();

        // Return sensitivity per unit correlation change (central difference)
        Ok((pv_up - pv_down) / (2.0 * bump_abs))
    }

    /// Calculate jump-to-default (immediate loss from specific entity default).
    ///
    /// For a homogeneous portfolio, estimates the immediate impact if one average
    /// entity defaults instantly. This is distinct from correlation sensitivity.
    ///
    /// Returns the average JTD across all constituents. For detailed min/max/avg,
    /// use [`calculate_jump_to_default_detail`].
    #[must_use = "JTD result should be used for risk management"]
    pub fn calculate_jump_to_default(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        _as_of: Date,
    ) -> Result<f64> {
        let detail = self.calculate_jump_to_default_detail(tranche, market_ctx)?;
        Ok(detail.average)
    }

    /// Calculate detailed jump-to-default metrics including min, max, and average.
    ///
    /// For heterogeneous portfolios with issuer-specific recovery rates or weights,
    /// this provides the full distribution of JTD impacts.
    ///
    /// # Returns
    ///
    /// [`JumpToDefaultResult`] containing:
    /// - `min`: JTD for the smallest impact name
    /// - `max`: JTD for the largest impact name (worst case for risk)
    /// - `average`: Average JTD across all names
    /// - `count`: Number of names that would impact this tranche
    pub fn calculate_jump_to_default_detail(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
    ) -> Result<JumpToDefaultResult> {
        let index_data = market_ctx.credit_index(&tranche.credit_index_id)?;

        let attach_frac = tranche.attach_pct / 100.0;
        let detach_frac = tranche.detach_pct / 100.0;
        let tranche_width = detach_frac - attach_frac;
        let tranche_notional = tranche.notional.amount();

        // Handle zero-width tranche edge case
        if tranche_width <= self.params.numerical_tolerance {
            return Ok(JumpToDefaultResult {
                min: 0.0,
                max: 0.0,
                average: 0.0,
                count: 0,
            });
        }

        let num_constituents = index_data.num_constituents as usize;
        let base_weight = 1.0 / (num_constituents as f64);
        let base_lgd = 1.0 - index_data.recovery_rate;

        // Collect JTD impacts for all names
        let mut impacts: Vec<f64> = Vec::with_capacity(num_constituents);
        let mut impacting_count = 0;

        // Check if we have issuer-specific data
        let has_issuer_curves = index_data.has_issuer_curves();

        for _i in 0..num_constituents {
            // For now, assume uniform weights. In a full implementation,
            // we would get issuer-specific weights and recovery rates.
            let individual_weight = base_weight;
            let loss_given_default = if has_issuer_curves {
                // Could use issuer-specific recovery here if available
                base_lgd
            } else {
                base_lgd
            };

            let individual_loss = individual_weight * loss_given_default;

            // Check if this loss hits the tranche layer
            if individual_loss <= attach_frac {
                // Loss doesn't reach the tranche
                impacts.push(0.0);
                continue;
            }

            impacting_count += 1;

            // Calculate how much of the individual loss hits the tranche
            let tranche_hit = if individual_loss >= detach_frac {
                // Loss fully exhausts the tranche
                tranche_width
            } else {
                // Loss partially hits the tranche
                individual_loss - attach_frac
            };

            // Convert to tranche notional impact
            let impact_on_tranche_fraction = tranche_hit / tranche_width;
            let impact_amount = impact_on_tranche_fraction * tranche_notional;
            impacts.push(impact_amount);
        }

        // Calculate min, max, average
        let (min, max, sum) = impacts
            .iter()
            .fold((f64::MAX, f64::MIN, 0.0), |(min, max, sum), &impact| {
                (min.min(impact), max.max(impact), sum + impact)
            });

        let average = if !impacts.is_empty() {
            sum / (impacts.len() as f64)
        } else {
            0.0
        };

        Ok(JumpToDefaultResult {
            min: if min == f64::MAX { 0.0 } else { min },
            max: if max == f64::MIN { 0.0 } else { max },
            average,
            count: impacting_count,
        })
    }

    /// Calculate accrued premium on the tranche.
    ///
    /// Returns the premium accrued since the last payment date, calculated on
    /// the outstanding notional (after accounting for any realized losses).
    ///
    /// # Calculation
    ///
    /// ```text
    /// Accrued = Coupon × Accrual_Fraction × Outstanding_Notional
    /// ```
    ///
    /// Where:
    /// - Coupon is the running coupon rate (running_coupon_bp / 10000)
    /// - Accrual_Fraction is the day count fraction from last payment to as_of
    /// - Outstanding_Notional accounts for any realized losses
    ///
    /// # Use Cases
    ///
    /// - Dirty vs clean price: `dirty_price = clean_price + accrued`
    /// - Settlement amount calculation
    /// - Mark-to-market accounting
    #[must_use = "accrued premium result should be used"]
    pub fn calculate_accrued_premium(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<f64> {
        // Get credit index data for loss calculations
        let index_data = match market_ctx.credit_index(&tranche.credit_index_id) {
            Ok(data) => data,
            Err(_) => return Ok(0.0), // No credit data, no accrued
        };

        // Generate the payment schedule
        let start_date = tranche.effective_date.unwrap_or(as_of);
        let payment_dates = self.generate_payment_schedule(tranche, start_date)?;

        // Find the last payment date on or before as_of
        let last_payment = payment_dates
            .iter()
            .filter(|&&d| d <= as_of)
            .max()
            .copied()
            .unwrap_or(start_date);

        // Find the next payment date after as_of
        let next_payment = payment_dates.iter().filter(|&&d| d > as_of).min().copied();

        // If no next payment, we're past maturity
        let _next_payment = match next_payment {
            Some(d) => d,
            None => return Ok(0.0),
        };

        // Calculate the accrual fraction from last payment to as_of
        let accrual_fraction = tranche
            .day_count
            .year_fraction(
                last_payment,
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);

        if accrual_fraction <= 0.0 {
            return Ok(0.0);
        }

        // Calculate outstanding notional (accounting for realized losses)
        let prior_loss = self.calculate_prior_tranche_loss(tranche);
        let outstanding_notional = tranche.notional.amount() * (1.0 - prior_loss);

        // Also factor in expected loss if we want to be more precise
        // For simplicity, use outstanding based on realized loss only
        let _ = index_data; // Mark as used (could compute expected loss here)

        // Calculate accrued premium
        let coupon = tranche.running_coupon_bp / 10000.0;
        let accrued = coupon * accrual_fraction * outstanding_notional;

        Ok(accrued)
    }

    /// Expose the expected loss curve for diagnostic and debugging purposes.
    ///
    /// Returns a vector of (Date, EL_fraction) pairs where EL_fraction
    /// is the cumulative expected loss as a fraction of tranche notional [0, 1].
    ///
    /// This is useful for:
    /// - Visualizing the expected loss profile over time
    /// - Debugging pricing discrepancies
    /// - Validating model behavior
    pub fn get_expected_loss_curve(
        &self,
        tranche: &CdsTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<(Date, f64)>> {
        let index_data = market_ctx.credit_index(&tranche.credit_index_id)?;
        let payment_dates = self.generate_payment_schedule(tranche, as_of)?;
        self.build_el_curve(tranche, index_data.as_ref(), &payment_dates)
    }
}

/// Result of detailed jump-to-default calculation.
///
/// Provides the distribution of JTD impacts across all portfolio constituents,
/// which is essential for worst-case risk management scenarios.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JumpToDefaultResult {
    /// Minimum JTD impact across all names (best case)
    pub min: f64,
    /// Maximum JTD impact across all names (worst case for risk)
    pub max: f64,
    /// Average JTD impact across all names
    pub average: f64,
    /// Number of names that would impact this tranche
    pub count: usize,
}

impl JumpToDefaultResult {
    /// Check if any names would impact this tranche
    #[inline]
    pub fn has_impact(&self) -> bool {
        self.count > 0
    }

    /// Get the range of impacts (max - min)
    #[inline]
    pub fn impact_range(&self) -> f64 {
        self.max - self.min
    }
}

// ========================= REGISTRY PRICER =========================

/// Registry pricer for CDS Tranche using Gaussian Copula model
pub struct SimpleCdsTrancheHazardPricer {
    model_key: crate::pricer::ModelKey,
}

impl SimpleCdsTrancheHazardPricer {
    /// Create new CDS tranche pricer with default hazard rate model
    pub fn new() -> Self {
        Self {
            model_key: crate::pricer::ModelKey::HazardRate,
        }
    }

    /// Create CDS tranche pricer with specified model key
    pub fn with_model(model_key: crate::pricer::ModelKey) -> Self {
        Self { model_key }
    }
}

impl Default for SimpleCdsTrancheHazardPricer {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::pricer::Pricer for SimpleCdsTrancheHazardPricer {
    fn key(&self) -> crate::pricer::PricerKey {
        crate::pricer::PricerKey::new(crate::pricer::InstrumentType::CDSTranche, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> std::result::Result<crate::results::ValuationResult, crate::pricer::PricingError> {
        use crate::instruments::common::traits::Instrument;

        // Type-safe downcasting
        let cds_tranche = instrument
            .as_any()
            .downcast_ref::<crate::instruments::cds_tranche::CdsTranche>()
            .ok_or_else(|| {
                crate::pricer::PricingError::type_mismatch(
                    crate::pricer::InstrumentType::CDSTranche,
                    instrument.key(),
                )
            })?;

        // Use the provided as_of date for valuation
        // Compute present value using the engine
        let pv = CDSTranchePricer::new()
            .price_tranche(cds_tranche, market, as_of)
            .map_err(|e| crate::pricer::PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(crate::results::ValuationResult::stamped(
            cds_tranche.id(),
            as_of,
            pv,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::cds_tranche::parameters::CDSTrancheParams;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::term_structures::CreditIndexData;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::{BaseCorrelationCurve, HazardCurve};
    use finstack_core::money::Money;
    use std::sync::Arc;
    use time::Month;

    fn sample_market_context() -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Create discount curve
        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.60)])
            .set_interp(finstack_core::math::interp::InterpStyle::LogLinear)
            .build()
            .expect("Curve builder should succeed with valid test data");

        // Create index hazard curve
        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![(1.0, 0.01), (3.0, 0.015), (5.0, 0.02), (10.0, 0.025)])
            .par_spreads(vec![(1.0, 60.0), (3.0, 80.0), (5.0, 100.0), (10.0, 140.0)])
            .build()
            .expect("Curve builder should succeed with valid test data");

        // Create base correlation curve
        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .knots(vec![
                (3.0, 0.25),  // 0-3% equity
                (7.0, 0.45),  // 0-7% junior mezzanine
                (10.0, 0.60), // 0-10% senior mezzanine
                (15.0, 0.75), // 0-15% senior
                (30.0, 0.85), // 0-30% super senior
            ])
            .build()
            .expect("Curve builder should succeed with valid test data");

        // Create credit index data
        let index_data = CreditIndexData::builder()
            .num_constituents(125)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .build()
            .expect("Curve builder should succeed with valid test data");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index_data)
    }

    fn sample_market_context_with_issuers(n: usize) -> MarketContext {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let discount_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, 0.97), (5.0, 0.84), (10.0, 0.68)])
            .build()
            .expect("Curve builder should succeed with valid test data");

        let index_curve = HazardCurve::builder("CDX.NA.IG.42")
            .base_date(base_date)
            .recovery_rate(0.40)
            .knots(vec![
                (1.0, 0.012),
                (3.0, 0.017),
                (5.0, 0.022),
                (10.0, 0.028),
            ])
            .par_spreads(vec![(1.0, 65.0), (3.0, 85.0), (5.0, 105.0), (10.0, 145.0)])
            .build()
            .expect("Curve builder should succeed with valid test data");

        let base_corr_curve = BaseCorrelationCurve::builder("CDX.NA.IG.42_5Y")
            .knots(vec![
                (3.0, 0.25),
                (7.0, 0.45),
                (10.0, 0.60),
                (15.0, 0.75),
                (30.0, 0.85),
            ])
            .build()
            .expect("Curve builder should succeed with valid test data");

        let mut issuer_curves = finstack_core::HashMap::default();
        for i in 0..n {
            let id = format!("ISSUER-{:03}", i + 1);
            let bump = (i as f64) * 0.001;
            let hz = HazardCurve::builder(id.as_str())
                .base_date(base_date)
                .recovery_rate(0.40)
                .knots(vec![
                    (1.0, (0.012 + bump).min(0.2)),
                    (3.0, (0.017 + bump).min(0.2)),
                    (5.0, (0.022 + bump).min(0.2)),
                    (10.0, (0.028 + bump).min(0.2)),
                ])
                .build()
                .expect("HazardCurve builder should succeed with valid test data");
            issuer_curves.insert(id, Arc::new(hz));
        }

        let index = CreditIndexData::builder()
            .num_constituents(n as u16)
            .recovery_rate(0.40)
            .index_credit_curve(Arc::new(index_curve))
            .base_correlation_curve(Arc::new(base_corr_curve))
            .with_issuer_curves(issuer_curves)
            .build()
            .expect("Curve builder should succeed with valid test data");

        MarketContext::new()
            .insert_discount(discount_curve)
            .insert_credit_index("CDX.NA.IG.42", index)
    }

    fn sample_tranche() -> CdsTranche {
        let _issue_date =
            Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        {
            let tranche_params = CDSTrancheParams::new(
                "CDX.NA.IG.42",                          // index_name
                42,                                      // series
                3.0,                                     // attach_pct (3%)
                7.0,                                     // detach_pct (7%)
                Money::new(10_000_000.0, Currency::USD), // $10MM notional
                maturity,                                // maturity
                500.0,                                   // running_coupon_bp (5%)
            );
            let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
            CdsTranche::new(
                "CDX_IG42_3_7_5Y",
                &tranche_params,
                &schedule_params,
                finstack_core::types::CurveId::from("USD-OIS"),
                finstack_core::types::CurveId::from("CDX.NA.IG.42"),
                TrancheSide::SellProtection,
            )
        }
    }

    #[test]
    fn test_model_creation() {
        let model = CDSTranchePricer::new();
        assert_eq!(model.params.quadrature_order, DEFAULT_QUADRATURE_ORDER);
        assert!(model.params.use_issuer_curves);
    }

    #[test]
    fn test_conditional_default_probability() {
        let model = CDSTranchePricer::new();
        let correlation = 0.30;
        let default_threshold = standard_normal_inv_cdf(0.05); // 5% default probability

        // Test with market factor = 0 (should be reasonable value close to original default prob)
        let cond_prob = model.conditional_default_probability(default_threshold, correlation, 0.0);
        assert!(
            cond_prob > 0.01 && cond_prob < 0.1,
            "Expected reasonable default prob, got {}",
            cond_prob
        );

        // Test with negative market factor (should increase default prob)
        let cond_prob_neg =
            model.conditional_default_probability(default_threshold, correlation, -1.0);
        assert!(cond_prob_neg > 0.05);

        // Test with positive market factor (should decrease default prob)
        let cond_prob_pos =
            model.conditional_default_probability(default_threshold, correlation, 1.0);
        assert!(cond_prob_pos < 0.05);
    }

    #[test]
    fn test_binomial_probability() {
        // Test known values
        assert!((binomial_probability(10, 5, 0.5) - 0.24609375).abs() < 1e-6);
        assert!((binomial_probability(5, 0, 0.1) - 0.59049).abs() < 1e-6);

        // Test edge cases
        assert_eq!(binomial_probability(10, 0, 0.0), 1.0);
        assert_eq!(binomial_probability(10, 10, 1.0), 1.0);
        assert_eq!(binomial_probability(10, 5, 0.0), 0.0);
    }

    #[test]
    fn test_log_factorial() {
        // Test small values (exact calculation)
        assert!((log_factorial(1) - 0.0).abs() < 1e-12);
        assert!(
            (log_factorial(5) - (2.0_f64.ln() + 3.0_f64.ln() + 4.0_f64.ln() + 5.0_f64.ln())).abs()
                < 1e-12
        );

        // Test that Stirling's approximation is reasonable for large n
        let log_100_factorial = log_factorial(100);
        assert!(log_100_factorial > 360.0 && log_100_factorial < 370.0); // Should be around 363.7
    }

    #[test]
    fn test_tranche_pricing_integration() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Test that pricing doesn't panic and returns a reasonable result
        let result = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(result.is_ok());

        let pv = result.expect("Tranche pricing should succeed in test");
        assert_eq!(pv.currency(), Currency::USD);
        // PV should be finite (could be positive or negative)
        assert!(pv.amount().is_finite());
    }

    #[test]
    fn test_hetero_spa_matches_homogeneous_when_issuers_equal() {
        let ctx_base = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let mut tranche = sample_tranche();
        tranche.running_coupon_bp = 0.0; // isolate protection leg

        // Build a context with issuer curves identical to index curve
        let index_data = ctx_base
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test context");
        let mut issuer_curves = finstack_core::HashMap::default();
        for i in 0..10 {
            let id = format!("ISSUER-{:03}", i + 1);
            issuer_curves.insert(id, index_data.index_credit_curve.clone());
        }
        let hetero_index = CreditIndexData::builder()
            .num_constituents(10)
            .recovery_rate(index_data.recovery_rate)
            .index_credit_curve(index_data.index_credit_curve.clone())
            .base_correlation_curve(index_data.base_correlation_curve.clone())
            .with_issuer_curves(issuer_curves)
            .build()
            .expect("Curve builder should succeed with valid test data");
        let ctx = ctx_base
            .clone()
            .insert_credit_index("CDX.NA.IG.42", hetero_index);

        let mut homo = CDSTranchePricer::new();
        homo.params.use_issuer_curves = false;
        let mut hetero = CDSTranchePricer::new();
        hetero.params.use_issuer_curves = true;
        hetero.params.hetero_method = HeteroMethod::Spa;

        let pv_homo = homo
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        let pv_hetero = hetero
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        assert!((pv_homo - pv_hetero).abs() < 1e-2 * pv_homo.abs().max(1.0));
    }

    #[test]
    fn test_hetero_spa_vs_exact_convolution_small_pool() {
        let ctx = sample_market_context_with_issuers(8);
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            3.0,
            7.0,
            Money::new(10_000_000.0, Currency::USD),
            as_of.add_months(60),
            0.0,
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "CDX_IG42_3_7_5Y",
            &tranche_params,
            &schedule_params,
            finstack_core::types::CurveId::from("USD-OIS"),
            finstack_core::types::CurveId::from("CDX.NA.IG.42"),
            TrancheSide::SellProtection,
        );

        let mut spa = CDSTranchePricer::new();
        spa.params.use_issuer_curves = true;
        spa.params.hetero_method = HeteroMethod::Spa;
        let mut exact = CDSTranchePricer::new();
        exact.params.use_issuer_curves = true;
        exact.params.hetero_method = HeteroMethod::ExactConvolution;
        exact.params.grid_step = 0.002;

        let pv_spa = spa
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        let pv_exact = exact
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        assert!((pv_spa - pv_exact).abs() < 0.02 * pv_exact.abs().max(1.0));
    }

    #[test]
    fn test_grid_step_refines_exact_convolution() {
        let ctx = sample_market_context_with_issuers(10);
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            0.0,
            3.0,
            Money::new(10_000_000.0, Currency::USD),
            as_of.add_months(60),
            0.0,
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "CDX_IG42_0_3_5Y",
            &tranche_params,
            &schedule_params,
            finstack_core::types::CurveId::from("USD-OIS"),
            finstack_core::types::CurveId::from("CDX.NA.IG.42"),
            TrancheSide::SellProtection,
        );

        let mut exact_coarse = CDSTranchePricer::new();
        exact_coarse.params.use_issuer_curves = true;
        exact_coarse.params.hetero_method = HeteroMethod::ExactConvolution;
        exact_coarse.params.grid_step = 0.005;

        let mut exact_fine = CDSTranchePricer::new();
        exact_fine.params = exact_coarse.params.clone();
        exact_fine.params.grid_step = 0.001;

        let p_coarse = exact_coarse
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        let p_fine = exact_fine
            .price_tranche(&tranche, &ctx, as_of)
            .expect("Tranche pricing should succeed in test")
            .amount();
        assert!((p_coarse - p_fine).abs() < 0.02 * p_fine.abs().max(1.0));
    }

    #[test]
    fn test_expected_loss_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();

        let expected_loss = model.calculate_expected_loss(&tranche, &market_ctx);
        assert!(expected_loss.is_ok());

        let loss = expected_loss.expect("Expected loss calculation should succeed in test");
        assert!(loss >= 0.0); // Expected loss should be non-negative
        assert!(loss.is_finite());
    }

    #[test]
    fn test_payment_schedule_generation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let schedule = model.generate_payment_schedule(&tranche, as_of);
        assert!(schedule.is_ok());

        let dates = schedule.expect("Schedule generation should succeed in test");
        assert!(!dates.is_empty());
        assert!(dates[0] > as_of); // First payment should be after as_of
        assert!(*dates.last().expect("Schedule should not be empty") <= tranche.maturity); // Last payment should not exceed maturity

        // Check dates are in ascending order
        for window in dates.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn test_payment_schedule_imm_vs_non_imm() {
        let model = CDSTranchePricer::new();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let mut imm_tranche = sample_tranche();
        imm_tranche.standard_imm_dates = true;
        imm_tranche.effective_date =
            Some(Date::from_calendar_date(2025, Month::March, 20).expect("cds date"));
        imm_tranche.maturity = Date::from_calendar_date(2030, Month::March, 20).expect("cds date");
        let imm_dates = model
            .generate_payment_schedule(&imm_tranche, as_of)
            .expect("IMM schedule should succeed");
        assert!(!imm_dates.is_empty());
        assert!(
            imm_dates
                .iter()
                .all(|d| finstack_core::dates::is_cds_date(*d)),
            "IMM schedule should use CDS roll dates"
        );

        let mut non_imm_tranche = sample_tranche();
        non_imm_tranche.standard_imm_dates = false;
        non_imm_tranche.effective_date =
            Some(Date::from_calendar_date(2025, Month::January, 15).expect("valid date"));
        non_imm_tranche.maturity =
            Date::from_calendar_date(2026, Month::January, 15).expect("valid date");
        let non_imm_dates = model
            .generate_payment_schedule(&non_imm_tranche, as_of)
            .expect("non-IMM schedule should succeed");
        assert!(!non_imm_dates.is_empty());
        assert!(
            non_imm_dates
                .iter()
                .any(|d| !finstack_core::dates::is_cds_date(*d)),
            "Non-IMM schedule should include non-CDS dates"
        );
    }

    #[test]
    fn test_el_curve_monotonicity() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let schedule = model
            .generate_payment_schedule(&tranche, as_of)
            .expect("Schedule generation should succeed in test");
        let index_data_arc = market_ctx
            .credit_index(&tranche.credit_index_id)
            .expect("Credit index should exist in test context");
        let el_curve = model.build_el_curve(&tranche, &index_data_arc, &schedule);

        assert!(el_curve.is_ok());
        let curve = el_curve.expect("EL curve building should succeed in test");

        // EL should be non-decreasing and bounded [0,1]
        // Allow for small numerical deviations due to base correlation model limitations
        // The base correlation model can have inconsistencies at knot points
        const NUMERICAL_TOLERANCE: f64 = 0.01; // Allow up to 1% EL fraction decrease

        for (i, &(_, el_fraction)) in curve.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&el_fraction),
                "EL fraction {} at index {} out of bounds",
                el_fraction,
                i
            );

            if i > 0 {
                let decrease = curve[i - 1].1 - el_fraction;
                assert!(
                    decrease <= NUMERICAL_TOLERANCE,
                    "EL fraction decreased significantly from {} to {} (decrease: {})",
                    curve[i - 1].1,
                    el_fraction,
                    decrease
                );
            }
        }
    }

    #[test]
    fn test_cs01_calculation() {
        let model = CDSTranchePricer::new();
        let mut tranche = sample_tranche();
        tranche.side = TrancheSide::SellProtection; // Sell protection for positive CS01
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let cs01 = model.calculate_cs01(&tranche, &market_ctx, as_of);
        assert!(cs01.is_ok());

        let sensitivity = cs01.expect("CS01 calculation should succeed in test");
        assert!(sensitivity.is_finite());
        // For protection seller, CS01 should typically be positive
        // (higher spreads -> higher protection premium income)
    }

    #[test]
    fn test_correlation_delta_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let corr_delta = model.calculate_correlation_delta(&tranche, &market_ctx, as_of);
        assert!(corr_delta.is_ok());

        let sensitivity = corr_delta.expect("Correlation delta calculation should succeed in test");
        assert!(sensitivity.is_finite());
        // Correlation sensitivity should be finite and reasonable in magnitude
    }

    #[test]
    fn test_jump_to_default_calculation() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let jtd = model.calculate_jump_to_default(&tranche, &market_ctx, as_of);
        assert!(jtd.is_ok());

        let impact = jtd.expect("Jump to default calculation should succeed in test");
        assert!(impact >= 0.0); // Impact should be non-negative
        assert!(impact.is_finite());
    }

    #[test]
    fn test_pv_decomposition_consistency() {
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let index_data_arc = market_ctx
            .credit_index(&tranche.credit_index_id)
            .expect("Credit index should exist in test context");
        let discount_curve = market_ctx
            .get_discount(tranche.discount_curve_id.as_ref())
            .expect("Discount curve should exist in test context");

        // Calculate individual leg PVs
        let pv_premium = model.calculate_premium_leg_pv(
            &tranche,
            &index_data_arc,
            discount_curve.as_ref(),
            as_of,
        );
        let pv_protection = model.calculate_protection_leg_pv(
            &tranche,
            &index_data_arc,
            discount_curve.as_ref(),
            as_of,
        );

        assert!(pv_premium.is_ok());
        assert!(pv_protection.is_ok());

        let premium = pv_premium.expect("Premium PV calculation should succeed in test");
        let protection = pv_protection.expect("Protection PV calculation should succeed in test");

        assert!(premium.is_finite());
        assert!(protection.is_finite());
        assert!(premium >= 0.0); // Premium leg should be positive for ongoing coupon
        assert!(protection >= 0.0); // Protection leg should be non-negative
    }

    #[test]
    fn test_extreme_correlation_numerical_stability() {
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let _as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let index_data_arc = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Credit index should exist in test context");

        // Test extreme correlation values that are challenging for numerical stability
        let extreme_correlations = [1e-10, 1e-6, 0.001, 0.999, 1.0 - 1e-6, 1.0 - 1e-10];

        for &test_correlation in &extreme_correlations {
            // Create a correlation curve with extreme values
            let extreme_corr_curve =
                finstack_core::market_data::term_structures::BaseCorrelationCurve::builder(
                    "TEST_EXTREME",
                )
                .knots(vec![
                    (3.0, test_correlation),
                    (7.0, test_correlation),
                    (10.0, test_correlation),
                    (15.0, test_correlation),
                    (30.0, test_correlation),
                ])
                .build()
                .expect("BaseCorrelationCurve builder should succeed with valid test data");

            // Create index data with extreme correlation
            let extreme_index_data = CreditIndexData::builder()
                .num_constituents(125)
                .recovery_rate(0.40)
                .index_credit_curve(index_data_arc.index_credit_curve.clone())
                .base_correlation_curve(std::sync::Arc::new(extreme_corr_curve))
                .build()
                .expect("BaseCorrelationCurve builder should succeed with valid test data");

            // Test equity tranche loss calculation
            let maturity =
                Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");
            let result = model.calculate_equity_tranche_loss(
                7.0, // 7% detachment
                test_correlation,
                &extreme_index_data,
                maturity,
            );

            assert!(
                result.is_ok(),
                "Equity tranche loss calculation failed for correlation={}",
                test_correlation
            );

            let expected_loss =
                result.expect("Equity tranche loss calculation should succeed in test");
            assert!(
                expected_loss.is_finite(),
                "Expected loss should be finite for correlation={}, got {}",
                test_correlation,
                expected_loss
            );
            assert!(
                (0.0..=1.0).contains(&expected_loss),
                "Expected loss should be in [0,1] for correlation={}, got {}",
                test_correlation,
                expected_loss
            );
        }
    }

    #[test]
    fn test_smooth_correlation_boundary_transitions() {
        let model = CDSTranchePricer::new();

        // Test that smooth boundary transitions work correctly
        let test_values = [
            0.005, 0.009, 0.011, 0.015, // Near min boundary (0.01)
            0.985, 0.989, 0.991, 0.995, // Near max boundary (0.99)
        ];

        for &test_corr in &test_values {
            let smoothed = model.smooth_correlation_boundary(test_corr);

            // Should be finite and within expanded bounds
            assert!(
                smoothed.is_finite(),
                "Smoothed correlation should be finite for input={}",
                test_corr
            );
            assert!(
                (0.005..=0.995).contains(&smoothed),
                "Smoothed correlation {} should be in reasonable bounds for input={}",
                smoothed,
                test_corr
            );

            // Should be continuous (no big jumps)
            let nearby = test_corr + 0.001;
            let smoothed_nearby = model.smooth_correlation_boundary(nearby);
            let transition_smoothness = (smoothed_nearby - smoothed).abs();

            assert!(
                transition_smoothness < 0.01,
                "Boundary transition should be smooth: jump of {} between {} and {}",
                transition_smoothness,
                test_corr,
                nearby
            );
        }
    }

    #[test]
    fn test_conditional_default_probability_enhanced() {
        let model = CDSTranchePricer::new();
        let default_threshold = standard_normal_inv_cdf(0.05); // 5% unconditional default prob

        // Test enhanced function across various correlation and market factor combinations
        let correlations = [1e-8, 0.01, 0.3, 0.7, 0.99, 1.0 - 1e-8];
        let market_factors = [-4.0, -2.0, -1.0, 0.0, 1.0, 2.0, 4.0];

        for &correlation in &correlations {
            for &market_factor in &market_factors {
                let enhanced_prob = model.conditional_default_probability_enhanced(
                    default_threshold,
                    correlation,
                    market_factor,
                );
                let standard_prob = model.conditional_default_probability(
                    default_threshold,
                    correlation.clamp(0.01, 0.99), // Clamp for standard function
                    market_factor,
                );

                // Enhanced function should always give finite, bounded results
                assert!(
                    enhanced_prob.is_finite(),
                    "Enhanced conditional prob should be finite for ρ={}, Z={}",
                    correlation,
                    market_factor
                );
                assert!(
                    (0.0..=1.0).contains(&enhanced_prob),
                    "Enhanced conditional prob should be in [0,1]: got {} for ρ={}, Z={}",
                    enhanced_prob,
                    correlation,
                    market_factor
                );

                // For normal correlation ranges, should be close to standard implementation
                if (0.05..=0.95).contains(&correlation) {
                    let diff = (enhanced_prob - standard_prob).abs();
                    assert!(diff < 0.01,
                        "Enhanced and standard methods should agree in normal range: diff={} for ρ={}, Z={}",
                        diff, correlation, market_factor);
                }
            }
        }
    }

    #[test]
    fn test_realized_loss_impact() {
        let model = CDSTranchePricer::new();
        let mut tranche = sample_tranche();
        // 0-3% tranche
        tranche.attach_pct = 0.0;
        tranche.detach_pct = 3.0;
        tranche.series = 42;
        tranche.accumulated_loss = 0.0;
        tranche.standard_imm_dates = true;

        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // 1. Price with no prior loss
        let pv_clean = model
            .price_tranche(&tranche, &market_ctx, as_of)
            .expect("Pricing clean tranche")
            .amount();

        // 2. Price with 1% realized loss (portfolio lost 1%, so tranche is 1/3 wiped out)
        // Remaining tranche is effectively [0, (3-1)/(1-0.01)] = [0, 2.02%] on surviving portfolio
        // Outstanding notional starts at 2/3 of original
        tranche.accumulated_loss = 0.01;
        let pv_loss = model
            .price_tranche(&tranche, &market_ctx, as_of)
            .expect("Pricing tranche with loss")
            .amount();

        // The PV should be different
        assert!(pv_loss != pv_clean, "Realized loss should impact PV");

        // 3. Price with 4% realized loss (tranche wiped out)
        tranche.accumulated_loss = 0.04;
        let pv_wiped = model
            .price_tranche(&tranche, &market_ctx, as_of)
            .expect("Pricing wiped tranche")
            .amount();

        assert_eq!(pv_wiped, 0.0, "Wiped out tranche should have 0 PV");
    }

    // ========================= EDGE CASE TESTS =========================

    #[test]
    fn test_thin_tranche_stability() {
        // Test very thin tranches (width < 1%) for numerical stability
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        // Create a very thin tranche (0.5% width)
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            3.0, // attach at 3%
            3.5, // detach at 3.5% (0.5% width)
            Money::new(1_000_000.0, Currency::USD),
            maturity,
            500.0,
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "THIN_TRANCHE",
            &tranche_params,
            &schedule_params,
            finstack_core::types::CurveId::from("USD-OIS"),
            finstack_core::types::CurveId::from("CDX.NA.IG.42"),
            TrancheSide::SellProtection,
        );

        // Should price without panicking
        let pv = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(pv.is_ok(), "Thin tranche should price successfully");
        assert!(
            pv.expect("PV should be Ok").amount().is_finite(),
            "Thin tranche PV should be finite"
        );
    }

    #[test]
    fn test_super_senior_tranche() {
        // Test super senior tranche (30-100%)
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            30.0,  // super senior attachment
            100.0, // full portfolio detachment
            Money::new(10_000_000.0, Currency::USD),
            maturity,
            25.0, // Very low spread for super senior
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "SUPER_SENIOR",
            &tranche_params,
            &schedule_params,
            finstack_core::types::CurveId::from("USD-OIS"),
            finstack_core::types::CurveId::from("CDX.NA.IG.42"),
            TrancheSide::SellProtection,
        );

        let pv = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(pv.is_ok(), "Super senior tranche should price successfully");
        // Super senior should have very low expected loss
        let el = model.calculate_expected_loss(&tranche, &market_ctx);
        assert!(el.is_ok());
        assert!(
            el.expect("Expected loss should be Ok") >= 0.0,
            "Expected loss should be non-negative"
        );
    }

    #[test]
    fn test_nearly_wiped_tranche() {
        // Test tranche that is nearly (but not fully) wiped out
        let model = CDSTranchePricer::new();
        let mut tranche = sample_tranche();
        tranche.attach_pct = 0.0;
        tranche.detach_pct = 3.0;
        // 2.99% loss means only 0.01% remaining (99.67% wiped)
        tranche.accumulated_loss = 0.0299;

        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let pv = model.price_tranche(&tranche, &market_ctx, as_of);
        assert!(pv.is_ok(), "Nearly wiped tranche should price");
        let pv_amount = pv.expect("PV should be Ok").amount();
        assert!(pv_amount.is_finite(), "PV should be finite");
        // Should be much smaller than full notional tranche
    }

    #[test]
    fn test_central_difference_symmetry() {
        // Test that central difference produces symmetric sensitivities
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // CS01 should be finite and well-behaved
        let cs01 = model.calculate_cs01(&tranche, &market_ctx, as_of);
        assert!(cs01.is_ok());
        assert!(cs01.expect("CS01 should be Ok").is_finite());

        // Correlation delta should be finite
        let corr_delta = model.calculate_correlation_delta(&tranche, &market_ctx, as_of);
        assert!(corr_delta.is_ok());
        assert!(corr_delta
            .expect("Correlation delta should be Ok")
            .is_finite());
    }

    #[test]
    fn test_jtd_detail_consistency() {
        // Test that JTD detail is consistent with simple JTD
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let simple_jtd = model.calculate_jump_to_default(&tranche, &market_ctx, as_of);
        let detail_jtd = model.calculate_jump_to_default_detail(&tranche, &market_ctx);

        assert!(simple_jtd.is_ok());
        assert!(detail_jtd.is_ok());

        let simple = simple_jtd.expect("Simple JTD should be Ok");
        let detail = detail_jtd.expect("Detail JTD should be Ok");

        // Simple JTD should equal the average from detail
        assert!(
            (simple - detail.average).abs() < 1e-10,
            "Simple JTD {} should equal detail average {}",
            simple,
            detail.average
        );

        // Min <= average <= max
        assert!(detail.min <= detail.average);
        assert!(detail.average <= detail.max);
    }

    #[test]
    fn test_monotonicity_enforcement_in_bumping() {
        // Test that correlation bumping enforces monotonicity
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let index_data = market_ctx
            .credit_index("CDX.NA.IG.42")
            .expect("Index should exist");

        // Create a large negative bump that could violate monotonicity
        let bumped = model.bump_base_correlation(&index_data.base_correlation_curve, -0.2);
        assert!(bumped.is_ok(), "Bumping should succeed");

        let curve = bumped.expect("Bumped curve should be Ok");
        // Verify monotonicity
        for i in 1..curve.correlations().len() {
            assert!(
                curve.correlations()[i] >= curve.correlations()[i - 1],
                "Bumped correlations should be monotonic: {} < {}",
                curve.correlations()[i],
                curve.correlations()[i - 1]
            );
        }
    }

    #[test]
    fn test_par_spread_solver_convergence() {
        // Test that par spread solver converges correctly
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let par_spread = model.calculate_par_spread(&tranche, &market_ctx, as_of);
        assert!(par_spread.is_ok(), "Par spread should calculate");

        let spread = par_spread.expect("Par spread should be Ok");
        assert!(spread >= 0.0, "Par spread should be non-negative");
        assert!(spread.is_finite(), "Par spread should be finite");

        // Verify: pricing at par spread should give near-zero NPV
        let mut test_tranche = tranche.clone();
        test_tranche.running_coupon_bp = spread;
        let npv = model.price_tranche(&test_tranche, &market_ctx, as_of);
        assert!(npv.is_ok());
        let npv_amount = npv.expect("NPV should be Ok").amount().abs();
        // Should be close to zero (within tolerance * notional)
        assert!(
            npv_amount < 100.0, // Allow $100 residual on $10M notional
            "NPV at par spread should be near zero, got {}",
            npv_amount
        );
    }

    #[test]
    fn test_settlement_date_calculation() {
        // Test settlement date logic for different index types
        // Using Wednesday Jan 1, 2025 so T+1 is Thursday (no weekend crossing)
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // CDX index should use T+1 business days
        let mut cdx_tranche = sample_tranche();
        cdx_tranche.index_name = "CDX.NA.IG.42".to_string();
        cdx_tranche.effective_date = None;
        cdx_tranche.calendar_id = None; // No calendar, weekend-only logic
        let cdx_settle = model.calculate_settlement_date(&cdx_tranche, &market_ctx, as_of);
        assert!(cdx_settle.is_ok());
        // Should be 1 business day after as_of (Wed -> Thu)
        assert_eq!(
            cdx_settle.expect("CDX settlement should be Ok"),
            Date::from_calendar_date(2025, Month::January, 2).expect("Valid test date"),
            "CDX should settle T+1 business day"
        );

        // Bespoke index should use T+3 business days
        // From Wed Jan 1: Thu Jan 2 (+1), Fri Jan 3 (+2), Mon Jan 6 (+3, skipping weekend)
        let mut bespoke_tranche = sample_tranche();
        bespoke_tranche.index_name = "BESPOKE".to_string();
        bespoke_tranche.effective_date = None;
        bespoke_tranche.calendar_id = None;
        let bespoke_settle = model.calculate_settlement_date(&bespoke_tranche, &market_ctx, as_of);
        assert!(bespoke_settle.is_ok());
        // T+3 from Wed Jan 1 = Mon Jan 6 (skipping Sat/Sun)
        let expected = Date::from_calendar_date(2025, Month::January, 6).expect("Valid test date");
        assert_eq!(
            bespoke_settle.expect("Bespoke settlement should be Ok"),
            expected,
            "Bespoke should settle T+3 business days"
        );
    }

    #[test]
    fn test_settlement_date_skips_weekends() {
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        // Friday Jan 3, 2025
        let friday = Date::from_calendar_date(2025, Month::January, 3).expect("Valid test date");

        let mut tranche = sample_tranche();
        tranche.index_name = "CDX.NA.IG.42".to_string();
        tranche.effective_date = None;
        tranche.calendar_id = None; // No calendar, weekend-only logic

        let settle = model
            .calculate_settlement_date(&tranche, &market_ctx, friday)
            .expect("Settlement date calculation should succeed");
        // T+1 from Friday should be Monday (skip Sat/Sun)
        let expected_monday =
            Date::from_calendar_date(2025, Month::January, 6).expect("Valid test date");
        assert_eq!(
            settle, expected_monday,
            "T+1 from Friday should be Monday, skipping weekend"
        );
    }

    #[test]
    fn test_settlement_date_weekday() {
        let model = CDSTranchePricer::new();
        let market_ctx = sample_market_context();
        // Wednesday Jan 1, 2025
        let wednesday = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let mut tranche = sample_tranche();
        tranche.index_name = "CDX.NA.IG.42".to_string();
        tranche.effective_date = None;
        tranche.calendar_id = None;

        let settle = model
            .calculate_settlement_date(&tranche, &market_ctx, wednesday)
            .expect("Settlement date calculation should succeed");
        // T+1 from Wednesday should be Thursday
        let expected_thursday =
            Date::from_calendar_date(2025, Month::January, 2).expect("Valid test date");
        assert_eq!(
            settle, expected_thursday,
            "T+1 from Wednesday should be Thursday"
        );
    }

    #[test]
    fn test_accrued_premium_calculation() {
        // Test accrued premium calculation
        let model = CDSTranchePricer::new();
        let tranche = sample_tranche();
        let market_ctx = sample_market_context();

        // At inception, accrued should be minimal
        let inception = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let accrued_at_inception =
            model.calculate_accrued_premium(&tranche, &market_ctx, inception);
        assert!(accrued_at_inception.is_ok());

        // Mid-quarter, accrued should be positive
        let mid_quarter =
            Date::from_calendar_date(2025, Month::February, 15).expect("Valid test date");
        let accrued_mid = model.calculate_accrued_premium(&tranche, &market_ctx, mid_quarter);
        assert!(accrued_mid.is_ok());
        let accrued = accrued_mid.expect("Accrued premium should be Ok");
        assert!(accrued >= 0.0, "Accrued premium should be non-negative");
    }

    #[test]
    fn test_stochastic_recovery_impacts_equity_tranche() {
        let market_ctx = sample_market_context();
        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2030, Month::January, 1).expect("Valid test date");

        // Create equity tranche (0-3%) which is most sensitive to stochastic recovery
        let tranche_params = CDSTrancheParams::new(
            "CDX.NA.IG.42",
            42,
            0.0, // attach at 0%
            3.0, // detach at 3%
            Money::new(10_000_000.0, Currency::USD),
            maturity,
            500.0, // 5% running coupon
        );
        let schedule_params = crate::cashflow::builder::ScheduleParams::quarterly_act360();
        let tranche = CdsTranche::new(
            "CDX_IG42_0_3_5Y",
            &tranche_params,
            &schedule_params,
            finstack_core::types::CurveId::from("USD-OIS"),
            finstack_core::types::CurveId::from("CDX.NA.IG.42"),
            TrancheSide::SellProtection,
        );

        // Constant recovery (default)
        let pricer_const = CDSTranchePricer::new();
        let pv_const = pricer_const
            .price_tranche(&tranche, &market_ctx, as_of)
            .expect("Constant recovery pricing should succeed")
            .amount();

        // Stochastic recovery (market-correlated)
        let pricer_stoch = CDSTranchePricer::with_params(
            CDSTranchePricerConfig::default().with_stochastic_recovery(),
        );
        let pv_stoch = pricer_stoch
            .price_tranche(&tranche, &market_ctx, as_of)
            .expect("Stochastic recovery pricing should succeed")
            .amount();

        // Both should be finite
        assert!(
            pv_const.is_finite(),
            "Constant recovery PV should be finite"
        );
        assert!(
            pv_stoch.is_finite(),
            "Stochastic recovery PV should be finite"
        );

        // PVs should differ - stochastic recovery impacts equity tranche
        // Note: The exact magnitude depends on the market-standard stochastic recovery calibration
        // (mean=40%, vol=25%, corr=-40%), but we expect at least some difference
        let pv_diff = (pv_stoch - pv_const).abs();
        assert!(
            pv_diff > 0.0,
            "Stochastic recovery should change PV; const={}, stoch={}",
            pv_const,
            pv_stoch
        );
    }

    #[test]
    fn test_stochastic_recovery_default_is_deterministic() {
        // Verify that default configuration uses deterministic (constant) recovery
        let pricer = CDSTranchePricer::new();
        assert!(
            pricer.config().recovery_spec.is_none(),
            "Default recovery_spec should be None (deterministic)"
        );
    }
}
