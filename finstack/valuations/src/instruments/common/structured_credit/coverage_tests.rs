//! Coverage tests and trigger monitoring for structured credit.

use finstack_core::dates::Date;
use finstack_core::money::Money;

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::pool::AssetPool;
use super::tranches::{AbsTranche, TrancheStructure};
use super::types::{CoverageTestType, CreditRating, TriggerConsequence};

#[cfg(test)]
use super::pool::PoolAsset;
#[cfg(test)]
use super::tranches::TrancheCoupon;
#[cfg(test)]
use super::types::{AssetType, DealType, LoanType, TrancheSeniority};

/// Definition of how to calculate a coverage test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TestDefinition {
    /// Type of test (OC, IC, Par Value)
    pub test_type: CoverageTestType,
    /// How to calculate numerator
    pub numerator: NumeratorDefinition,
    /// How to calculate denominator
    pub denominator: DenominatorDefinition,
    /// Required minimum level
    pub trigger_level: f64,
    /// Higher level required to cure breach
    pub cure_level: Option<f64>,
}

/// Definition of test numerator calculation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum NumeratorDefinition {
    /// Pool principal adjusted for defaults and haircuts
    AdjustedPoolPrincipal {
        /// Haircuts by rating for market value adjustments
        haircuts: HashMap<CreditRating, f64>,
        /// Whether to exclude defaulted assets
        exclude_defaulted: bool,
        /// Whether to apply recovery assumptions
        apply_recovery: bool,
    },
    /// Pool interest collections over test period
    PoolInterest {
        /// Include scheduled interest payments
        include_scheduled: bool,
        /// Include interest from recoveries
        include_recoveries: bool,
        /// Annualization factor
        annualization_factor: f64,
    },
    /// Par value of performing assets
    ParValue {
        /// Only count performing assets
        performing_only: bool,
    },
}

/// Definition of test denominator calculation
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DenominatorDefinition {
    /// Sum of senior tranches (for subordinate OC test)
    SeniorTranches {
        /// Include tranches up to this seniority level
        up_to_seniority: super::types::TrancheSeniority,
    },
    /// Specific tranche balance
    TrancheBalance {
        /// Tranche identifier
        tranche_id: String,
    },
    /// Sum of interest due to specified tranches
    InterestDue {
        /// Tranche identifiers
        tranches: Vec<String>,
        /// Annualization factor
        annualization_factor: f64,
    },
    /// Custom calculation
    Custom { description: String },
}

/// Results of coverage test calculations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TestResults {
    /// OC ratios by tranche
    pub oc_ratios: HashMap<String, f64>,
    /// IC ratios by tranche
    pub ic_ratios: HashMap<String, f64>,
    /// Par value test ratio
    pub par_value_ratio: Option<f64>,
    /// Custom test results
    pub custom_results: HashMap<String, f64>,
    /// List of breached tests
    pub breached_tests: Vec<BreachedTest>,
    /// Payment diversion details
    pub payment_diversion: PaymentDiversion,
}

/// Details of a breached coverage test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BreachedTest {
    pub test_name: String,
    pub tranche_id: String,
    pub current_level: f64,
    pub required_level: f64,
    pub breach_date: Date,
    pub consequences_applied: Vec<TriggerConsequence>,
}

/// Payment diversion due to trigger breaches
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentDiversion {
    /// Amount diverted from normal distribution
    pub amount_diverted: Money,
    /// Tranches that would have received diverted funds
    pub diverted_from: Vec<String>,
    /// Tranches that received the diverted funds
    pub diverted_to: Vec<String>,
    /// Reason for diversion
    pub reason: String,
}

impl Default for PaymentDiversion {
    fn default() -> Self {
        Self {
            amount_diverted: Money::new(0.0, finstack_core::currency::Currency::USD),
            diverted_from: Vec::new(),
            diverted_to: Vec::new(),
            reason: String::new(),
        }
    }
}

/// Coverage test framework
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTests {
    /// Test definitions by name
    pub test_definitions: HashMap<String, TestDefinition>,
    /// Current test results
    pub current_results: TestResults,
    /// Historical results for trending
    pub historical_results: Vec<(Date, TestResults)>,
}

impl CoverageTests {
    /// Create new coverage test framework
    pub fn new() -> Self {
        Self {
            test_definitions: HashMap::new(),
            current_results: TestResults {
                oc_ratios: HashMap::new(),
                ic_ratios: HashMap::new(),
                par_value_ratio: None,
                custom_results: HashMap::new(),
                breached_tests: Vec::new(),
                payment_diversion: PaymentDiversion::default(),
            },
            historical_results: Vec::new(),
        }
    }

    /// Add standard OC test for a tranche
    pub fn add_oc_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        let test_name = format!("{}_OC", tranche_id);
        let test_def = TestDefinition {
            test_type: CoverageTestType::OC,
            numerator: NumeratorDefinition::AdjustedPoolPrincipal {
                haircuts: Self::default_haircuts(),
                exclude_defaulted: true,
                apply_recovery: true,
            },
            denominator: DenominatorDefinition::TrancheBalance {
                tranche_id: tranche_id.clone(),
            },
            trigger_level,
            cure_level,
        };

        self.test_definitions.insert(test_name, test_def);
        self
    }

    /// Add standard IC test for a tranche
    pub fn add_ic_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        let test_name = format!("{}_IC", tranche_id);
        let test_def = TestDefinition {
            test_type: CoverageTestType::IC,
            numerator: NumeratorDefinition::PoolInterest {
                include_scheduled: true,
                include_recoveries: false,
                annualization_factor: 4.0, // Quarterly to annual
            },
            denominator: DenominatorDefinition::InterestDue {
                tranches: vec![tranche_id.clone()],
                annualization_factor: 4.0,
            },
            trigger_level,
            cure_level,
        };

        self.test_definitions.insert(test_name, test_def);
        self
    }

    /// Run all coverage tests
    pub fn run_tests(
        &mut self,
        pool: &AssetPool,
        tranches: &TrancheStructure,
        test_date: Date,
    ) -> finstack_core::Result<&TestResults> {
        let mut new_results = TestResults {
            oc_ratios: HashMap::new(),
            ic_ratios: HashMap::new(),
            par_value_ratio: None,
            custom_results: HashMap::new(),
            breached_tests: Vec::new(),
            payment_diversion: PaymentDiversion::default(),
        };

        // Run each defined test
        for (test_name, test_def) in &self.test_definitions {
            let ratio = self.calculate_test_ratio(test_def, pool, tranches)?;

            // Store result by type
            match test_def.test_type {
                CoverageTestType::OC => {
                    if let DenominatorDefinition::TrancheBalance { ref tranche_id } =
                        test_def.denominator
                    {
                        new_results.oc_ratios.insert(tranche_id.clone(), ratio);
                    }
                }
                CoverageTestType::IC => {
                    if let DenominatorDefinition::InterestDue { ref tranches, .. } =
                        test_def.denominator
                    {
                        if let Some(first_tranche) = tranches.first() {
                            new_results.ic_ratios.insert(first_tranche.clone(), ratio);
                        }
                    }
                }
                CoverageTestType::ParValue => {
                    new_results.par_value_ratio = Some(ratio);
                }
                CoverageTestType::Custom(_) => {
                    new_results.custom_results.insert(test_name.clone(), ratio);
                }
            }

            // Check for breach
            if ratio < test_def.trigger_level {
                let breach = BreachedTest {
                    test_name: test_name.clone(),
                    tranche_id: self.extract_tranche_id_from_test(test_def),
                    current_level: ratio,
                    required_level: test_def.trigger_level,
                    breach_date: test_date,
                    consequences_applied: vec![TriggerConsequence::DivertCashFlow], // Default consequence
                };
                new_results.breached_tests.push(breach);
            }
        }

        // Store historical results
        self.historical_results
            .push((test_date, self.current_results.clone()));
        self.current_results = new_results;

        Ok(&self.current_results)
    }

    /// Calculate ratio for a specific test definition
    fn calculate_test_ratio(
        &self,
        test_def: &TestDefinition,
        pool: &AssetPool,
        tranches: &TrancheStructure,
    ) -> finstack_core::Result<f64> {
        let numerator = self.calculate_numerator(&test_def.numerator, pool)?;
        let denominator = self.calculate_denominator(&test_def.denominator, pool, tranches)?;

        if denominator == 0.0 {
            Ok(f64::INFINITY) // Perfect coverage when denominator is zero
        } else {
            Ok(numerator / denominator)
        }
    }

    /// Calculate numerator value
    fn calculate_numerator(
        &self,
        def: &NumeratorDefinition,
        pool: &AssetPool,
    ) -> finstack_core::Result<f64> {
        match def {
            NumeratorDefinition::AdjustedPoolPrincipal {
                haircuts,
                exclude_defaulted,
                apply_recovery,
            } => {
                let mut total = 0.0;
                for asset in &pool.assets {
                    if *exclude_defaulted && asset.is_defaulted {
                        if *apply_recovery {
                            if let Some(recovery) = asset.recovery_amount {
                                total += recovery.amount();
                            }
                        }
                        continue;
                    }

                    let mut asset_value = asset.balance.amount();

                    // Apply haircuts based on rating
                    if let Some(rating) = asset.credit_quality {
                        if let Some(&haircut) = haircuts.get(&rating) {
                            asset_value *= 1.0 - haircut;
                        }
                    }

                    total += asset_value;
                }
                Ok(total)
            }
            NumeratorDefinition::PoolInterest {
                include_scheduled,
                include_recoveries,
                annualization_factor,
            } => {
                let mut total = 0.0;

                if *include_scheduled {
                    // Simplified: use current pool WAC
                    total += pool.weighted_avg_coupon() * pool.performing_balance().amount();
                }

                if *include_recoveries {
                    // Add interest from recoveries (simplified)
                    total += pool.cumulative_recoveries.amount() * 0.05; // Assume 5% on recoveries
                }

                Ok(total * annualization_factor)
            }
            NumeratorDefinition::ParValue { performing_only } => {
                if *performing_only {
                    Ok(pool.performing_balance().amount())
                } else {
                    Ok(pool.total_balance().amount())
                }
            }
        }
    }

    /// Calculate denominator value
    fn calculate_denominator(
        &self,
        def: &DenominatorDefinition,
        _pool: &AssetPool,
        tranches: &TrancheStructure,
    ) -> finstack_core::Result<f64> {
        match def {
            DenominatorDefinition::SeniorTranches { up_to_seniority } => {
                let total = tranches
                    .tranches
                    .iter()
                    .filter(|t| t.seniority <= *up_to_seniority)
                    .map(|t| t.current_balance.amount())
                    .sum();
                Ok(total)
            }
            DenominatorDefinition::TrancheBalance { tranche_id } => {
                let balance = tranches
                    .tranches
                    .iter()
                    .find(|t| t.id.as_str() == tranche_id)
                    .map(|t| t.current_balance.amount())
                    .unwrap_or(0.0);
                Ok(balance)
            }
            DenominatorDefinition::InterestDue {
                tranches: tranche_ids,
                annualization_factor,
            } => {
                let mut total = 0.0;
                for tranche_id in tranche_ids {
                    if let Some(tranche) = tranches
                        .tranches
                        .iter()
                        .find(|t| t.id.as_str() == tranche_id)
                    {
                        let current_rate = tranche.coupon.current_rate(
                            Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                        );
                        total += tranche.current_balance.amount() * current_rate;
                    }
                }
                Ok(total * annualization_factor)
            }
            DenominatorDefinition::Custom { .. } => {
                // Placeholder for custom calculations
                Ok(1.0)
            }
        }
    }

    /// Extract tranche ID from test definition (helper)
    fn extract_tranche_id_from_test(&self, test_def: &TestDefinition) -> String {
        match &test_def.denominator {
            DenominatorDefinition::TrancheBalance { tranche_id } => tranche_id.clone(),
            DenominatorDefinition::InterestDue { tranches, .. } => tranches
                .first()
                .cloned()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            _ => "UNKNOWN".to_string(),
        }
    }

    /// Default haircuts by credit rating
    fn default_haircuts() -> HashMap<CreditRating, f64> {
        let mut haircuts = HashMap::new();
        haircuts.insert(CreditRating::AAA, 0.0);
        haircuts.insert(CreditRating::AA, 0.0);
        haircuts.insert(CreditRating::A, 0.01);
        haircuts.insert(CreditRating::BBB, 0.02);
        haircuts.insert(CreditRating::BB, 0.05);
        haircuts.insert(CreditRating::B, 0.10);
        haircuts.insert(CreditRating::CCC, 0.20);
        haircuts.insert(CreditRating::CC, 0.30);
        haircuts.insert(CreditRating::C, 0.40);
        haircuts.insert(CreditRating::D, 0.50);
        haircuts.insert(CreditRating::NR, 0.15);
        haircuts
    }
}

impl Default for CoverageTests {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage test calculation engine
pub struct CoverageTestEngine;

impl CoverageTestEngine {
    /// Calculate OC ratio for a tranche
    pub fn calculate_oc_ratio(
        pool: &AssetPool,
        tranche: &AbsTranche,
        tranches: &TrancheStructure,
        haircuts: &HashMap<CreditRating, f64>,
    ) -> f64 {
        // Numerator: Adjusted pool value
        let mut pool_value = 0.0;
        for asset in &pool.assets {
            if asset.is_defaulted {
                // Use recovery value for defaulted assets
                if let Some(recovery) = asset.recovery_amount {
                    pool_value += recovery.amount();
                }
            } else {
                let mut asset_value = asset.balance.amount();

                // Apply haircut based on rating
                if let Some(rating) = asset.credit_quality {
                    if let Some(&haircut) = haircuts.get(&rating) {
                        asset_value *= 1.0 - haircut;
                    }
                }
                pool_value += asset_value;
            }
        }

        // Denominator: Tranche balance + senior tranches
        let senior_balance = tranches.senior_balance(tranche.id.as_str()).amount();
        let denominator = senior_balance + tranche.current_balance.amount();

        if denominator == 0.0 {
            f64::INFINITY
        } else {
            pool_value / denominator
        }
    }

    /// Calculate IC ratio for a tranche
    pub fn calculate_ic_ratio(
        pool: &AssetPool,
        tranche: &AbsTranche,
        tranches: &TrancheStructure,
        annualization_factor: f64,
    ) -> f64 {
        // Numerator: Pool interest (annualized)
        let pool_interest =
            pool.weighted_avg_coupon() * pool.performing_balance().amount() * annualization_factor;

        // Denominator: Interest due to this tranche and senior tranches (annualized)
        let mut interest_due = 0.0;

        // Add senior tranche interest
        for senior_tranche in tranches.senior_to(tranche.id.as_str()) {
            let rate = senior_tranche
                .coupon
                .current_rate(Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
            interest_due += senior_tranche.current_balance.amount() * rate * annualization_factor;
        }

        // Add this tranche's interest
        let rate = tranche
            .coupon
            .current_rate(Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
        interest_due += tranche.current_balance.amount() * rate * annualization_factor;

        if interest_due == 0.0 {
            f64::INFINITY
        } else {
            pool_interest / interest_due
        }
    }

    /// Calculate par value test ratio
    pub fn calculate_par_value_ratio(pool: &AssetPool, tranches: &TrancheStructure) -> f64 {
        let par_value = pool.performing_balance().amount();
        let aggregate_tranche_balance = tranches.total_size.amount();

        if aggregate_tranche_balance == 0.0 {
            f64::INFINITY
        } else {
            par_value / aggregate_tranche_balance
        }
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
    fn test_coverage_test_creation() {
        let mut tests = CoverageTests::new();
        tests.add_oc_test("SENIOR_A".to_string(), 1.15, Some(1.20));

        assert!(tests.test_definitions.contains_key("SENIOR_A_OC"));

        let test_def = &tests.test_definitions["SENIOR_A_OC"];
        assert_eq!(test_def.trigger_level, 1.15);
        assert_eq!(test_def.cure_level, Some(1.20));
    }

    #[test]
    fn test_oc_ratio_calculation() {
        // Create simple pool with one asset
        let mut pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        pool.assets.push(PoolAsset {
            id: "ASSET1".into(),
            asset_type: AssetType::Loan {
                loan_type: LoanType::FirstLien,
                industry: None,
            },
            balance: Money::new(1_000_000.0, Currency::USD),
            rate: 0.08,
            maturity: test_date(),
            credit_quality: Some(CreditRating::B),
            industry: None,
            obligor_id: Some("OBLIGOR1".to_string()),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: None,
        });

        // Create complete tranche structure (equity + senior)
        let equity_tranche = AbsTranche::new(
            "EQUITY",
            0.0,
            10.0,
            TrancheSeniority::Equity,
            Money::new(100_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.12 },
            test_date(),
        )
        .unwrap();

        let senior_tranche = AbsTranche::new(
            "SENIOR",
            10.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(900_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            test_date(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![equity_tranche, senior_tranche.clone()]).unwrap();

        let haircuts = CoverageTests::default_haircuts();
        let oc_ratio =
            CoverageTestEngine::calculate_oc_ratio(&pool, &senior_tranche, &tranches, &haircuts);

        // Should be pool value / tranche balance = (1M * 0.9) / 900K = 1.0
        assert!((oc_ratio - 1.0).abs() < 0.01);
    }
}
