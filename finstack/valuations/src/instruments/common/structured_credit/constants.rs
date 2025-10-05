//! Constants for structured credit calculations.
//!
//! This module centralizes common constants used throughout the structured credit
//! framework to improve maintainability and consistency.

/// Days per year for day count calculations
pub const DAYS_PER_YEAR: f64 = 365.25;

/// Floating point tolerance for validation checks
pub const VALIDATION_TOLERANCE: f64 = 1e-6;

/// Default periods per year for quarterly calculations
pub const QUARTERLY_PERIODS_PER_YEAR: f64 = 4.0;

/// Default capacity for historical coverage test storage (10 years quarterly)
pub const HISTORICAL_COVERAGE_CAPACITY: usize = 120;

/// Default basis points divisor
pub const BASIS_POINTS_DIVISOR: f64 = 10_000.0;

/// Percentage conversion factor
pub const PERCENTAGE_MULTIPLIER: f64 = 100.0;

/// Default months in a year
pub const MONTHS_PER_YEAR: i32 = 12;

// ============================================================================
// Seasonality Factors
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
// Default Model Parameters
// ============================================================================

/// Baseline unemployment rate for default models
pub const BASELINE_UNEMPLOYMENT_RATE: f64 = 0.04;

/// Minimum prepayment rate (floor)
pub const MIN_PREPAYMENT_RATE: f64 = 0.0;

// ============================================================================
// Structured Credit Default Rates
// ============================================================================

/// Standard PSA speeds for scenario analysis
pub const STANDARD_PSA_SPEEDS: &[f64] = &[0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0];

/// Standard CDR rates for scenario analysis
pub const STANDARD_CDR_RATES: &[f64] = &[0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.10, 0.15, 0.20];

/// Standard severity rates for scenario analysis
pub const STANDARD_SEVERITY_RATES: &[f64] = &[0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80];

// ============================================================================
// Fee Defaults (in basis points per annum)
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

// ============================================================================
// Prepayment Model Defaults
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
// Default Model Defaults
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
// Concentration Limits
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
