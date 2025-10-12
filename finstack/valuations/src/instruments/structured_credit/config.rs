//! Configuration and constants for structured credit instruments.
//!
//! This module combines constants and deal-level configuration to provide
//! a unified interface for structured credit parameters.

use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use std::collections::HashMap;

use crate::instruments::irs::InterestRateSwap;
use super::components::enums::CreditRating;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// CONSTANTS
// ============================================================================

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

/// Standard CLO trustee annual fee (USD)
pub const CLO_TRUSTEE_FEE_ANNUAL: f64 = 50_000.0;

// ============================================================================
// Simulation Constants
// ============================================================================

/// Pool balance threshold (in base currency units) below which cashflow generation stops.
///
/// For example, for a USD-denominated pool, this means stop when balance < $100.
/// This prevents unnecessary computation for immaterial remaining balances.
pub const POOL_BALANCE_CLEANUP_THRESHOLD: f64 = 100.0;

/// Default resolution lag in months for cashflow generation
pub const DEFAULT_RESOLUTION_LAG_MONTHS: u32 = 6;

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

// ============================================================================
// DEAL CONFIGURATION STRUCTURES
// ============================================================================

/// Complete deal configuration for structured credit instruments
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DealConfig {
    /// Key deal dates
    pub dates: DealDates,
    /// Fee structure
    pub fees: DealFees,
    /// Coverage test parameters
    pub coverage_tests: CoverageTestConfig,
    /// Default prepayment and default assumptions
    pub default_assumptions: DefaultAssumptions,
    /// Hedge swaps (leveraging existing IRS infrastructure)
    #[cfg_attr(feature = "serde", serde(default))]
    pub hedge_swaps: Vec<InterestRateSwap>,
}

/// Key dates for a structured credit deal
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DealDates {
    /// Deal closing date
    pub closing_date: Date,
    /// First payment date
    pub first_payment_date: Date,
    /// End of reinvestment period (if applicable)
    pub reinvestment_end_date: Option<Date>,
    /// Legal final maturity date
    pub legal_maturity: Date,
    /// Payment frequency
    pub payment_frequency: Frequency,
}

impl DealDates {
    /// Create new deal dates with required fields
    pub fn new(
        closing_date: Date,
        first_payment_date: Date,
        legal_maturity: Date,
        payment_frequency: Frequency,
    ) -> Self {
        Self {
            closing_date,
            first_payment_date,
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency,
        }
    }

    /// Add reinvestment period
    pub fn with_reinvestment_end(mut self, end_date: Date) -> Self {
        self.reinvestment_end_date = Some(end_date);
        self
    }
}

/// Fee structure for structured credit deals
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DealFees {
    /// Annual trustee fee (fixed amount)
    pub trustee_fee_annual: Money,
    /// Senior management fee (basis points per annum on collateral)
    pub senior_mgmt_fee_bps: f64,
    /// Subordinated management fee (basis points per annum)
    pub subordinated_mgmt_fee_bps: f64,
    /// Servicing fee (basis points per annum)
    pub servicing_fee_bps: f64,
    /// Master servicer fee (for CMBS/RMBS, basis points)
    pub master_servicer_fee_bps: Option<f64>,
    /// Special servicer fee (for CMBS, basis points)
    pub special_servicer_fee_bps: Option<f64>,
}

impl DealFees {
    /// Create CLO-style fee structure
    pub fn clo_standard(base_currency: finstack_core::currency::Currency) -> Self {
        Self {
            trustee_fee_annual: Money::new(50_000.0, base_currency),
            senior_mgmt_fee_bps: 40.0,       // 40 bps
            subordinated_mgmt_fee_bps: 20.0, // 20 bps
            servicing_fee_bps: 0.0,
            master_servicer_fee_bps: None,
            special_servicer_fee_bps: None,
        }
    }

    /// Create ABS-style fee structure
    pub fn abs_standard(base_currency: finstack_core::currency::Currency) -> Self {
        Self {
            trustee_fee_annual: Money::new(25_000.0, base_currency),
            senior_mgmt_fee_bps: 0.0,
            subordinated_mgmt_fee_bps: 0.0,
            servicing_fee_bps: 50.0, // 50 bps servicing
            master_servicer_fee_bps: None,
            special_servicer_fee_bps: None,
        }
    }

    /// Create CMBS-style fee structure
    pub fn cmbs_standard(base_currency: finstack_core::currency::Currency) -> Self {
        Self {
            trustee_fee_annual: Money::new(75_000.0, base_currency),
            senior_mgmt_fee_bps: 0.0,
            subordinated_mgmt_fee_bps: 0.0,
            servicing_fee_bps: 0.0,
            master_servicer_fee_bps: Some(25.0),  // 25 bps
            special_servicer_fee_bps: Some(25.0), // 25 bps
        }
    }

    /// Create RMBS-style fee structure
    pub fn rmbs_standard(base_currency: finstack_core::currency::Currency) -> Self {
        Self {
            trustee_fee_annual: Money::new(30_000.0, base_currency),
            senior_mgmt_fee_bps: 0.0,
            subordinated_mgmt_fee_bps: 0.0,
            servicing_fee_bps: 25.0, // 25 bps
            master_servicer_fee_bps: None,
            special_servicer_fee_bps: None,
        }
    }
}

/// Coverage test configuration
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTestConfig {
    /// OC trigger levels by tranche ID
    pub oc_triggers: HashMap<String, f64>,
    /// IC trigger levels by tranche ID
    pub ic_triggers: HashMap<String, f64>,
    /// Haircuts by rating for OC calculations
    pub haircuts: HashMap<CreditRating, f64>,
    /// Par value test threshold (if applicable)
    pub par_value_threshold: Option<f64>,
}

impl CoverageTestConfig {
    /// Create new empty configuration
    pub fn new() -> Self {
        Self {
            oc_triggers: HashMap::new(),
            ic_triggers: HashMap::new(),
            haircuts: Self::default_haircuts(),
            par_value_threshold: None,
        }
    }

    /// Standard CLO haircuts (conservative)
    pub fn default_haircuts() -> HashMap<CreditRating, f64> {
        let mut haircuts = HashMap::new();
        haircuts.insert(CreditRating::AAA, 0.0);
        haircuts.insert(CreditRating::AA, 0.0);
        haircuts.insert(CreditRating::A, 0.01); // 1%
        haircuts.insert(CreditRating::BBB, 0.02); // 2%
        haircuts.insert(CreditRating::BB, 0.05); // 5%
        haircuts.insert(CreditRating::B, 0.10); // 10%
        haircuts.insert(CreditRating::CCC, 0.20); // 20%
        haircuts.insert(CreditRating::CC, 0.30); // 30%
        haircuts.insert(CreditRating::C, 0.40); // 40%
        haircuts.insert(CreditRating::D, 0.50); // 50%
        haircuts.insert(CreditRating::NR, 0.15); // 15%
        haircuts
    }

    /// Add OC test for a tranche
    pub fn add_oc_test(&mut self, tranche_id: impl Into<String>, trigger_level: f64) -> &mut Self {
        self.oc_triggers.insert(tranche_id.into(), trigger_level);
        self
    }

    /// Add IC test for a tranche
    pub fn add_ic_test(&mut self, tranche_id: impl Into<String>, trigger_level: f64) -> &mut Self {
        self.ic_triggers.insert(tranche_id.into(), trigger_level);
        self
    }

    /// Set custom haircuts
    pub fn with_haircuts(mut self, haircuts: HashMap<CreditRating, f64>) -> Self {
        self.haircuts = haircuts;
        self
    }
}

impl Default for CoverageTestConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Default assumptions for structured credit modeling
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DefaultAssumptions {
    /// Base annual CDR (Constant Default Rate)
    pub base_cdr_annual: f64,
    /// Base recovery rate on defaults
    pub base_recovery_rate: f64,
    /// Base annual CPR (Constant Prepayment Rate)
    pub base_cpr_annual: f64,
    /// PSA speed multiplier (for RMBS)
    pub psa_speed: Option<f64>,
    /// SDA speed multiplier (for RMBS)
    pub sda_speed: Option<f64>,
    /// ABS monthly prepayment speed (for auto ABS)
    pub abs_speed_monthly: Option<f64>,
    /// Asset-type specific annual CPRs
    #[cfg_attr(feature = "serde", serde(default))]
    pub cpr_by_asset_type: HashMap<String, f64>,
    /// Asset-type specific annual CDRs
    #[cfg_attr(feature = "serde", serde(default))]
    pub cdr_by_asset_type: HashMap<String, f64>,
    /// Asset-type specific recovery rates
    #[cfg_attr(feature = "serde", serde(default))]
    pub recovery_by_asset_type: HashMap<String, f64>,
}

impl DefaultAssumptions {
    /// CLO default assumptions
    pub fn clo_standard() -> Self {
        Self {
            base_cdr_annual: 0.02,    // 2% CDR
            base_recovery_rate: 0.40, // 40% recovery (60% severity)
            base_cpr_annual: 0.15,    // 15% CPR
            psa_speed: None,
            sda_speed: None,
            abs_speed_monthly: None,
            cpr_by_asset_type: HashMap::new(),
            cdr_by_asset_type: HashMap::new(),
            recovery_by_asset_type: HashMap::new(),
        }
    }

    /// RMBS default assumptions
    pub fn rmbs_standard() -> Self {
        Self {
            base_cdr_annual: 0.006,   // 0.6% CDR (100% SDA peak)
            base_recovery_rate: 0.60, // 60% recovery (40% severity)
            base_cpr_annual: 0.06,    // 6% CPR (100% PSA)
            psa_speed: Some(1.0),     // 100% PSA
            sda_speed: Some(1.0),     // 100% SDA
            abs_speed_monthly: None,
            cpr_by_asset_type: HashMap::new(),
            cdr_by_asset_type: HashMap::new(),
            recovery_by_asset_type: HashMap::new(),
        }
    }

    /// Auto ABS default assumptions
    pub fn abs_auto_standard() -> Self {
        Self {
            base_cdr_annual: 0.02,    // 2% CDR
            base_recovery_rate: 0.45, // 45% recovery
            base_cpr_annual: 0.0,     // Not used for auto
            psa_speed: None,
            sda_speed: None,
            abs_speed_monthly: Some(0.015), // 1.5% ABS
            cpr_by_asset_type: HashMap::new(),
            cdr_by_asset_type: HashMap::new(),
            recovery_by_asset_type: HashMap::new(),
        }
    }

    /// CMBS default assumptions
    pub fn cmbs_standard() -> Self {
        Self {
            base_cdr_annual: 0.005,   // 0.5% CDR
            base_recovery_rate: 0.65, // 65% recovery (collateral-backed)
            base_cpr_annual: 0.10,    // 10% CPR (open period)
            psa_speed: None,
            sda_speed: None,
            abs_speed_monthly: None,
            cpr_by_asset_type: HashMap::new(),
            cdr_by_asset_type: HashMap::new(),
            recovery_by_asset_type: HashMap::new(),
        }
    }
}

impl Default for DefaultAssumptions {
    fn default() -> Self {
        let mut cpr_by_asset_type = HashMap::new();
        cpr_by_asset_type.insert("mortgage".to_string(), 0.06); // 100% PSA
        cpr_by_asset_type.insert("rmbs".to_string(), 0.06);
        cpr_by_asset_type.insert("auto".to_string(), 0.18);
        cpr_by_asset_type.insert("abs_auto".to_string(), 0.18);
        cpr_by_asset_type.insert("card".to_string(), 0.15);
        cpr_by_asset_type.insert("credit_card".to_string(), 0.15);
        cpr_by_asset_type.insert("cc".to_string(), 0.15);
        cpr_by_asset_type.insert("commercial".to_string(), 0.10);
        cpr_by_asset_type.insert("cmbs".to_string(), 0.10);
        cpr_by_asset_type.insert("cre".to_string(), 0.10);
        cpr_by_asset_type.insert("student".to_string(), 0.03);
        cpr_by_asset_type.insert("student_loan".to_string(), 0.03);

        let mut cdr_by_asset_type = HashMap::new();
        cdr_by_asset_type.insert("mortgage".to_string(), 0.002);
        cdr_by_asset_type.insert("rmbs".to_string(), 0.002);
        cdr_by_asset_type.insert("auto".to_string(), 0.02);
        cdr_by_asset_type.insert("abs_auto".to_string(), 0.02);
        cdr_by_asset_type.insert("consumer".to_string(), 0.02);
        cdr_by_asset_type.insert("card".to_string(), 0.048); // 0.4% monthly MDR to annual CDR
        cdr_by_asset_type.insert("credit_card".to_string(), 0.048);
        cdr_by_asset_type.insert("corporate".to_string(), 0.02);
        cdr_by_asset_type.insert("clo".to_string(), 0.02);
        cdr_by_asset_type.insert("commercial".to_string(), 0.02);

        let mut recovery_by_asset_type = HashMap::new();
        recovery_by_asset_type.insert("mortgage".to_string(), 0.60);
        recovery_by_asset_type.insert("rmbs".to_string(), 0.60);
        recovery_by_asset_type.insert("collateral".to_string(), 0.60);
        recovery_by_asset_type.insert("auto".to_string(), 0.45);
        recovery_by_asset_type.insert("abs_auto".to_string(), 0.45);
        recovery_by_asset_type.insert("consumer".to_string(), 0.45);
        recovery_by_asset_type.insert("card".to_string(), 0.05);
        recovery_by_asset_type.insert("credit_card".to_string(), 0.05);
        recovery_by_asset_type.insert("unsecured".to_string(), 0.05);
        recovery_by_asset_type.insert("corporate".to_string(), 0.40);
        recovery_by_asset_type.insert("clo".to_string(), 0.40);
        recovery_by_asset_type.insert("commercial".to_string(), 0.40);

        Self {
            base_cdr_annual: 0.02,
            base_recovery_rate: 0.40,
            base_cpr_annual: 0.05,
            psa_speed: None,
            sda_speed: None,
            abs_speed_monthly: None,
            cpr_by_asset_type,
            cdr_by_asset_type,
            recovery_by_asset_type,
        }
    }
}

impl DealConfig {
    /// Create standard deal configuration for a given deal type
    pub fn standard(
        deal_type: super::components::enums::DealType,
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        use super::components::enums::DealType;
        let (fees, default_assumptions) = match deal_type {
            DealType::CLO => (
                DealFees::clo_standard(base_currency),
                DefaultAssumptions::clo_standard(),
            ),
            DealType::RMBS => (
                DealFees::rmbs_standard(base_currency),
                DefaultAssumptions::rmbs_standard(),
            ),
            DealType::ABS | DealType::Auto => (
                DealFees::abs_standard(base_currency),
                DefaultAssumptions::abs_auto_standard(),
            ),
            DealType::CMBS => (
                DealFees::cmbs_standard(base_currency),
                DefaultAssumptions::cmbs_standard(),
            ),
            _ => (
                DealFees::abs_standard(base_currency),
                DefaultAssumptions::abs_auto_standard(),
            ),
        };

        Self {
            dates,
            fees,
            coverage_tests: CoverageTestConfig::new(),
            default_assumptions,
            hedge_swaps: Vec::new(),
        }
    }

    /// Create CLO deal configuration
    pub fn clo_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        use super::components::enums::DealType;
        Self::standard(DealType::CLO, dates, base_currency)
    }

    /// Create RMBS deal configuration
    pub fn rmbs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        use super::components::enums::DealType;
        Self::standard(DealType::RMBS, dates, base_currency)
    }

    /// Create ABS deal configuration
    pub fn abs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        use super::components::enums::DealType;
        Self::standard(DealType::ABS, dates, base_currency)
    }

    /// Create CMBS deal configuration
    pub fn cmbs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        use super::components::enums::DealType;
        Self::standard(DealType::CMBS, dates, base_currency)
    }

    /// Add hedge swap for interest rate or basis risk management
    /// Leverages existing IRS infrastructure from finstack
    pub fn with_hedge_swap(mut self, swap: InterestRateSwap) -> Self {
        self.hedge_swaps.push(swap);
        self
    }

    /// Add multiple hedge swaps
    pub fn with_hedge_swaps(mut self, swaps: Vec<InterestRateSwap>) -> Self {
        self.hedge_swaps.extend(swaps);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
    }

    #[test]
    fn test_deal_dates_creation() {
        let dates = DealDates::new(
            test_date(),
            test_date(),
            test_date(),
            Frequency::quarterly(),
        );

        assert_eq!(dates.closing_date, test_date());
        assert!(dates.reinvestment_end_date.is_none());
    }

    #[test]
    fn test_clo_fee_structure() {
        let fees = DealFees::clo_standard(Currency::USD);

        assert_eq!(fees.trustee_fee_annual.amount(), 50_000.0);
        assert_eq!(fees.senior_mgmt_fee_bps, 40.0);
        assert_eq!(fees.subordinated_mgmt_fee_bps, 20.0);
    }

    #[test]
    fn test_coverage_test_config() {
        let mut config = CoverageTestConfig::new();
        config.add_oc_test("CLASS_A", 1.25);
        config.add_ic_test("CLASS_A", 1.20);

        assert_eq!(config.oc_triggers.get("CLASS_A"), Some(&1.25));
        assert_eq!(config.ic_triggers.get("CLASS_A"), Some(&1.20));
    }

    #[test]
    fn test_auto_recovery_rate() {
        let assumptions = DefaultAssumptions::abs_auto_standard();

        assert_eq!(assumptions.base_recovery_rate, 0.45);
    }
}

