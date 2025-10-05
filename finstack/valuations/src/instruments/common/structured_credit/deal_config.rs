//! Deal-level configuration for structured credit instruments.
//!
//! Provides configurable parameters for fees, dates, and other deal-specific
//! settings that should not be hardcoded.

use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use std::collections::HashMap;

use super::enums::CreditRating;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
        }
    }
}

impl DealConfig {
    /// Create standard deal configuration for a given deal type
    pub fn standard(
        deal_type: super::DealType,
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        let (fees, default_assumptions) = match deal_type {
            super::DealType::CLO => (
                DealFees::clo_standard(base_currency),
                DefaultAssumptions::clo_standard(),
            ),
            super::DealType::RMBS => (
                DealFees::rmbs_standard(base_currency),
                DefaultAssumptions::rmbs_standard(),
            ),
            super::DealType::ABS | super::DealType::Auto => (
                DealFees::abs_standard(base_currency),
                DefaultAssumptions::abs_auto_standard(),
            ),
            super::DealType::CMBS => (
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
        }
    }

    /// Create CLO deal configuration
    pub fn clo_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        Self::standard(super::DealType::CLO, dates, base_currency)
    }

    /// Create RMBS deal configuration
    pub fn rmbs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        Self::standard(super::DealType::RMBS, dates, base_currency)
    }

    /// Create ABS deal configuration
    pub fn abs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        Self::standard(super::DealType::ABS, dates, base_currency)
    }

    /// Create CMBS deal configuration
    pub fn cmbs_standard(
        dates: DealDates,
        base_currency: finstack_core::currency::Currency,
    ) -> Self {
        Self::standard(super::DealType::CMBS, dates, base_currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
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
