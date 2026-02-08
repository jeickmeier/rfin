//! Constants for structured credit instruments.
//!
//! This module contains all industry-standard constants, default values,
//! and fee structures used across structured credit modeling.

// ============================================================================
// TIME CONSTANTS
// ============================================================================

/// Average days per year for structured credit day count calculations.
///
/// Re-exported from `finstack_core::dates::AVERAGE_DAYS_PER_YEAR` (ACT/365.25).
/// Uses 365.25 to account for leap years over multi-year amortisation horizons.
pub use finstack_core::dates::AVERAGE_DAYS_PER_YEAR as DAYS_PER_YEAR;

/// Default months in a year
pub const MONTHS_PER_YEAR: i32 = 12;

/// Default periods per year for quarterly calculations
pub const QUARTERLY_PERIODS_PER_YEAR: f64 = 4.0;

// ============================================================================
// VALIDATION CONSTANTS
// ============================================================================

/// Default basis points divisor
pub const BASIS_POINTS_DIVISOR: f64 = 10_000.0;

/// Percentage conversion factor
pub const PERCENTAGE_MULTIPLIER: f64 = 100.0;

// ============================================================================
// MARKET-STANDARD NUMERICAL TOLERANCES
// ============================================================================
// These tolerances are calibrated to market conventions for structured credit.
// Reference: Bloomberg BVAL, Intex, and industry-standard pricing systems.

/// Solver tolerance for Z-spread calculations (decimal spread).
///
/// Market standard: 0.1 bps = 0.0001% = 1e-6 in decimal terms.
/// Z-spread is quoted in bps to 1 decimal place (e.g., 125.3 bps).
pub const Z_SPREAD_SOLVER_TOLERANCE: f64 = 1e-6;

/// Solver tolerance for YTM calculations (decimal yield).
///
/// Market standard: 0.1 bps = 1e-6 in decimal terms.
/// YTM is quoted in bps to 1 decimal place.
pub const YTM_SOLVER_TOLERANCE: f64 = 1e-6;

/// Initial bracket size for Z-spread solver (decimal spread).
///
/// ±500 bps is sufficient for most structured credit instruments.
/// Extreme distressed securities may require wider brackets.
pub const Z_SPREAD_INITIAL_BRACKET: f64 = 0.05; // ±500 bps

// ============================================================================
// SEASONALITY FACTORS
// ============================================================================

/// Mortgage prepayment seasonality adjustments by month (Jan=index 0)
pub const MORTGAGE_SEASONALITY: [f64; 12] = [
    0.94, 0.76, 0.74, 0.95, 0.98, 0.92, // Jan-Jun
    1.10, 1.18, 1.22, 1.23, 0.98, 1.00, // Jul-Dec
];

/// Credit card payment seasonality adjustments by month (Jan=index 0)
pub const CREDIT_CARD_SEASONALITY: [f64; 12] = [
    1.15, 1.10, 1.0, 0.95, 0.95, 0.95, // Jan-Jun (higher payments in Jan/Feb)
    0.95, 0.95, 1.0, 1.05, 1.05, 1.10, // Jul-Dec (higher in Dec)
];

// ============================================================================
// DEFAULT MODEL PARAMETERS
// ============================================================================

/// Baseline unemployment rate for default models
pub const BASELINE_UNEMPLOYMENT_RATE: f64 = 0.04;

/// Minimum prepayment rate (floor)
pub const MIN_PREPAYMENT_RATE: f64 = 0.0;

// ============================================================================
// SCENARIO ANALYSIS CONSTANTS
// ============================================================================

/// Standard PSA speeds for scenario analysis
pub const STANDARD_PSA_SPEEDS: &[f64] = &[0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0];

/// Standard CDR rates for scenario analysis
pub const STANDARD_CDR_RATES: &[f64] = &[0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.10, 0.15, 0.20];

/// Standard severity rates for scenario analysis
pub const STANDARD_SEVERITY_RATES: &[f64] = &[0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80];

// ============================================================================
// FEE DEFAULTS (in basis points per annum)
// ============================================================================

/// Standard CLO senior management fee (bps)
pub const CLO_SENIOR_MGMT_FEE_BPS: f64 = 40.0;

/// Standard CLO subordinated management fee (bps)
pub const CLO_SUBORDINATED_MGMT_FEE_BPS: f64 = 20.0;

/// Standard ABS servicing fee (bps)
pub const ABS_SERVICING_FEE_BPS: f64 = 50.0;

/// Standard CMBS master servicer fee (bps)
pub const CMBS_MASTER_SERVICER_FEE_BPS: f64 = 25.0;

/// Standard CMBS special servicer fee (bps)
pub const CMBS_SPECIAL_SERVICER_FEE_BPS: f64 = 25.0;

/// Standard RMBS servicing fee (bps)
pub const RMBS_SERVICING_FEE_BPS: f64 = 25.0;

/// Standard CLO trustee annual fee (USD)
pub const CLO_TRUSTEE_FEE_ANNUAL: f64 = 50_000.0;

/// Standard ABS trustee annual fee (USD)
pub const ABS_TRUSTEE_FEE_ANNUAL: f64 = 25_000.0;

/// Standard CMBS trustee annual fee (USD)
pub const CMBS_TRUSTEE_FEE_ANNUAL: f64 = 75_000.0;

/// Standard RMBS trustee annual fee (USD)
pub const RMBS_TRUSTEE_FEE_ANNUAL: f64 = 30_000.0;

// ============================================================================
// SIMULATION CONSTANTS
// ============================================================================

/// Pool balance threshold (in base currency units) below which cashflow generation stops.
///
/// For example, for a USD-denominated pool, this means stop when balance < $100.
/// This prevents unnecessary computation for immaterial remaining balances.
pub const POOL_BALANCE_CLEANUP_THRESHOLD: f64 = 100.0;

/// Default resolution lag in months for cashflow generation
pub const DEFAULT_RESOLUTION_LAG_MONTHS: u32 = 6;

// ============================================================================
// PREPAYMENT MODEL DEFAULTS
// ============================================================================

/// Standard PSA ramp-up period (months)
pub const PSA_RAMP_MONTHS: u32 = 30;

/// Standard PSA terminal CPR
pub const PSA_TERMINAL_CPR: f64 = 0.06;

/// Default auto loan ABS speed (monthly)
pub const DEFAULT_AUTO_ABS_SPEED: f64 = 0.015;

/// Default auto loan ramp period (months)
pub const DEFAULT_AUTO_RAMP_MONTHS: u32 = 12;

// ============================================================================
// DEFAULT MODEL DEFAULTS
// ============================================================================

/// Standard SDA peak month for mortgages
pub const SDA_PEAK_MONTH: u32 = 30;

/// Standard SDA peak CDR
pub const SDA_PEAK_CDR: f64 = 0.006;

/// Standard SDA terminal CDR
pub const SDA_TERMINAL_CDR: f64 = 0.0003;

/// Default burnout threshold (months)
pub const DEFAULT_BURNOUT_THRESHOLD_MONTHS: u32 = 60;

// ============================================================================
// CONCENTRATION LIMITS
// ============================================================================

/// Default maximum single obligor concentration
pub const DEFAULT_MAX_OBLIGOR_CONCENTRATION: f64 = 0.02; // 2%

/// Default maximum top 5 obligor concentration
pub const DEFAULT_MAX_TOP5_CONCENTRATION: f64 = 0.075; // 7.5%

/// Default maximum top 10 obligor concentration
pub const DEFAULT_MAX_TOP10_CONCENTRATION: f64 = 0.125; // 12.5%

/// Default maximum second lien concentration
pub const DEFAULT_MAX_SECOND_LIEN: f64 = 0.10; // 10%

/// Default maximum covenant-lite concentration
pub const DEFAULT_MAX_COV_LITE: f64 = 0.65; // 65%

/// Default maximum DIP concentration
pub const DEFAULT_MAX_DIP: f64 = 0.05; // 5%

// ============================================================================
// STANDARD DEAL ASSUMPTIONS
// ============================================================================

/// Standard CLO CDR (annual)
pub const CLO_STANDARD_CDR: f64 = 0.02;
/// Standard CLO recovery rate
pub const CLO_STANDARD_RECOVERY: f64 = 0.40;
/// Standard CLO CPR (annual)
pub const CLO_STANDARD_CPR: f64 = 0.15;

/// Standard RMBS CDR (annual)
pub const RMBS_STANDARD_CDR: f64 = 0.006;
/// Standard RMBS recovery rate
pub const RMBS_STANDARD_RECOVERY: f64 = 0.60;
/// Standard RMBS CPR (annual)
pub const RMBS_STANDARD_CPR: f64 = 0.06;
/// Standard RMBS PSA speed
pub const RMBS_STANDARD_PSA: f64 = 1.0;
/// Standard RMBS SDA speed
pub const RMBS_STANDARD_SDA: f64 = 1.0;

/// Standard Auto ABS CDR (annual)
pub const ABS_AUTO_STANDARD_CDR: f64 = 0.02;
/// Standard Auto ABS recovery rate
pub const ABS_AUTO_STANDARD_RECOVERY: f64 = 0.45;
/// Standard Auto ABS speed (monthly)
pub const ABS_AUTO_STANDARD_SPEED: f64 = 0.015;

/// Standard CMBS CDR (annual)
pub const CMBS_STANDARD_CDR: f64 = 0.005;
/// Standard CMBS recovery rate
pub const CMBS_STANDARD_RECOVERY: f64 = 0.65;
/// Standard CMBS CPR (annual)
pub const CMBS_STANDARD_CPR: f64 = 0.10;
