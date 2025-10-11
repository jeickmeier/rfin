//! Unified coverage tests for structured credit instruments.
//!
//! This module consolidates coverage test implementations into a single enum
//! with shared result types for improved type safety and reduced code duplication.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::enums::{CreditRating, TriggerConsequence};
use super::pool::AssetPool;
use super::tranches::TrancheStructure;

/// Unified coverage test type
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTest {
    /// Overcollateralization test
    OC {
        /// Required OC ratio (e.g., 1.25 = 125%)
        required_ratio: f64,
        /// Cure level if higher than trigger
        cure_level: Option<f64>,
        /// Include cash in numerator
        include_cash: bool,
        /// Include only performing assets
        performing_only: bool,
    },
    /// Interest coverage test
    IC {
        /// Required IC ratio (e.g., 1.20 = 120%)
        required_ratio: f64,
        /// Cure level if higher than trigger
        cure_level: Option<f64>,
    },
    /// Par value test
    ParValue {
        /// Required par value threshold (e.g., 0.90 = 90%)
        threshold: f64,
        /// Cure level if higher than trigger
        cure_level: Option<f64>,
    },
}

impl CoverageTest {
    /// Create new OC test with standard settings
    pub fn new_oc(required_ratio: f64, cure_level: Option<f64>) -> Self {
        Self::OC {
            required_ratio,
            cure_level,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Create new IC test
    pub fn new_ic(required_ratio: f64, cure_level: Option<f64>) -> Self {
        Self::IC {
            required_ratio,
            cure_level,
        }
    }

    /// Create new par value test
    pub fn new_par_value(threshold: f64, cure_level: Option<f64>) -> Self {
        Self::ParValue {
            threshold,
            cure_level,
        }
    }

    /// Get the required ratio/threshold for this test
    pub fn required_level(&self) -> f64 {
        match self {
            Self::OC { required_ratio, .. } => *required_ratio,
            Self::IC { required_ratio, .. } => *required_ratio,
            Self::ParValue { threshold, .. } => *threshold,
        }
    }

    /// Get the cure level for this test
    pub fn cure_level(&self) -> Option<f64> {
        match self {
            Self::OC { cure_level, .. } => *cure_level,
            Self::IC { cure_level, .. } => *cure_level,
            Self::ParValue { cure_level, .. } => *cure_level,
        }
    }

    /// Calculate the test result
    pub fn calculate(&self, context: &TestContext) -> TestResult {
        match self {
            Self::OC {
                required_ratio,
                cure_level,
                include_cash,
                performing_only,
            } => self.calculate_oc(
                context,
                *required_ratio,
                *cure_level,
                *include_cash,
                *performing_only,
            ),
            Self::IC {
                required_ratio,
                cure_level,
            } => self.calculate_ic(context, *required_ratio, *cure_level),
            Self::ParValue {
                threshold,
                cure_level,
            } => self.calculate_par_value(context, *threshold, *cure_level),
        }
    }

    fn calculate_oc(
        &self,
        context: &TestContext,
        required_ratio: f64,
        cure_level: Option<f64>,
        include_cash: bool,
        performing_only: bool,
    ) -> TestResult {
        let numerator = if performing_only {
            context.pool.performing_balance()
        } else {
            context.pool.total_balance()
        };

        let numerator = if include_cash {
            numerator
                .checked_add(context.cash_balance)
                .unwrap_or(numerator)
        } else {
            numerator
        };

        let denominator = context
            .tranche_balance
            .checked_add(context.senior_balance)
            .unwrap_or(context.tranche_balance);

        let ratio = if denominator.amount() > 0.0 {
            numerator.amount() / denominator.amount()
        } else {
            f64::INFINITY
        };

        let is_passing = ratio >= required_ratio;

        let cure_amount = if !is_passing {
            let required_collateral = denominator.amount() * required_ratio;
            let shortfall = required_collateral - numerator.amount();
            Some(Money::new(shortfall.max(0.0), denominator.currency()))
        } else {
            None
        };

        TestResult {
            current_ratio: ratio,
            is_passing,
            cure_amount,
            cure_level,
        }
    }

    fn calculate_ic(
        &self,
        context: &TestContext,
        required_ratio: f64,
        cure_level: Option<f64>,
    ) -> TestResult {
        let total_interest_due = context
            .interest_due
            .checked_add(context.senior_interest_due)
            .unwrap_or(context.interest_due);

        let ratio = if total_interest_due.amount() > 0.0 {
            context.interest_collections.amount() / total_interest_due.amount()
        } else {
            f64::INFINITY
        };

        let is_passing = ratio >= required_ratio;

        TestResult {
            current_ratio: ratio,
            is_passing,
            cure_amount: None, // IC tests don't have a cure amount
            cure_level,
        }
    }

    fn calculate_par_value(
        &self,
        context: &TestContext,
        threshold: f64,
        cure_level: Option<f64>,
    ) -> TestResult {
        let total_par = context.pool.total_balance().amount();
        let current_par = context.pool.performing_balance().amount();

        let ratio = if total_par > 0.0 {
            current_par / total_par
        } else {
            f64::INFINITY
        };

        let is_passing = ratio >= threshold;

        TestResult {
            current_ratio: ratio,
            is_passing,
            cure_amount: None, // Par value tests don't have a cure amount
            cure_level,
        }
    }
}

/// Context needed to calculate coverage tests
#[derive(Debug)]
pub struct TestContext<'a> {
    pub pool: &'a AssetPool,
    pub tranche_balance: Money,
    pub senior_balance: Money,
    pub cash_balance: Money,
    pub interest_collections: Money,
    pub interest_due: Money,
    pub senior_interest_due: Money,
}

/// Shared result structure for all coverage tests
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TestResult {
    /// Current calculated ratio
    pub current_ratio: f64,
    /// Whether test is currently passing
    pub is_passing: bool,
    /// Cure amount if failing (OC tests only)
    pub cure_amount: Option<Money>,
    /// Cure level if higher than trigger
    pub cure_level: Option<f64>,
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
    pub amount_diverted: Money,
    pub diverted_from: Vec<String>,
    pub diverted_to: Vec<String>,
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

/// Unified coverage test framework
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTests {
    /// All coverage tests by tranche and test type
    pub tests: HashMap<String, CoverageTest>,
    /// Current test results by tranche
    pub current_results: HashMap<String, TestResult>,
    /// Aggregate test results for reporting
    pub aggregate_results: TestResults,
    /// Historical results for trending
    pub historical_results: Vec<(Date, TestResults)>,
}

impl CoverageTests {
    /// Create new coverage test framework
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
            current_results: HashMap::new(),
            aggregate_results: TestResults {
                oc_ratios: HashMap::new(),
                ic_ratios: HashMap::new(),
                par_value_ratio: None,
                custom_results: HashMap::new(),
                breached_tests: Vec::new(),
                payment_diversion: PaymentDiversion::default(),
            },
            historical_results: Vec::with_capacity(super::constants::HISTORICAL_COVERAGE_CAPACITY),
        }
    }

    /// Add a coverage test for a tranche
    pub fn add_test(&mut self, key: String, test: CoverageTest) -> &mut Self {
        self.tests.insert(key, test);
        self
    }

    /// Add OC test for a tranche (convenience method)
    pub fn add_oc_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        let key = format!("{}_OC", tranche_id);
        self.tests
            .insert(key, CoverageTest::new_oc(trigger_level, cure_level));
        self
    }

    /// Add IC test for a tranche (convenience method)
    pub fn add_ic_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        let key = format!("{}_IC", tranche_id);
        self.tests
            .insert(key, CoverageTest::new_ic(trigger_level, cure_level));
        self
    }

    /// Add par value test for a tranche (convenience method)
    pub fn add_par_value_test(
        &mut self,
        tranche_id: String,
        threshold: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        let key = format!("{}_ParValue", tranche_id);
        self.tests
            .insert(key, CoverageTest::new_par_value(threshold, cure_level));
        self
    }

    /// Run all coverage tests
    pub fn run_tests(
        &mut self,
        pool: &AssetPool,
        tranches: &TrancheStructure,
        test_date: Date,
    ) -> finstack_core::Result<&TestResults> {
        let mut new_aggregate = TestResults {
            oc_ratios: HashMap::with_capacity(tranches.tranches.len()),
            ic_ratios: HashMap::with_capacity(tranches.tranches.len()),
            par_value_ratio: None,
            custom_results: HashMap::new(),
            breached_tests: Vec::with_capacity(tranches.tranches.len() * 2),
            payment_diversion: PaymentDiversion::default(),
        };

        let base_ccy = pool.base_currency();
        let cash_balance = Money::new(0.0, base_ccy);

        // Calculate tests for each tranche
        let mut cumulative_senior_balance = Money::new(0.0, base_ccy);

        for tranche in &tranches.tranches {
            let tranche_id_str = tranche.id.to_string();

            // Calculate interest for IC tests
            let pool_interest = Money::new(
                pool.performing_balance().amount() * pool.weighted_avg_coupon()
                    / super::constants::QUARTERLY_PERIODS_PER_YEAR,
                base_ccy,
            );
            let tranche_interest = Money::new(
                tranche.current_balance.amount() * tranche.coupon.current_rate(test_date)
                    / super::constants::QUARTERLY_PERIODS_PER_YEAR,
                base_ccy,
            );

            // Create test context
            let context = TestContext {
                pool,
                tranche_balance: tranche.current_balance,
                senior_balance: cumulative_senior_balance,
                cash_balance,
                interest_collections: pool_interest,
                interest_due: tranche_interest,
                senior_interest_due: Money::new(0.0, base_ccy),
            };

            // Run all tests for this tranche
            for (test_key, test) in &self.tests {
                // Check if this test belongs to this tranche
                if !test_key.starts_with(&tranche_id_str) {
                    continue;
                }

                let result = test.calculate(&context);
                self.current_results
                    .insert(test_key.clone(), result.clone());

                // Update aggregate results
                if test_key.ends_with("_OC") {
                    new_aggregate
                        .oc_ratios
                        .insert(tranche_id_str.clone(), result.current_ratio);
                } else if test_key.ends_with("_IC") {
                    new_aggregate
                        .ic_ratios
                        .insert(tranche_id_str.clone(), result.current_ratio);
                } else if test_key.ends_with("_ParValue") {
                    new_aggregate.par_value_ratio = Some(result.current_ratio);
                }

                // Record breached tests
                if !result.is_passing {
                    new_aggregate.breached_tests.push(BreachedTest {
                        test_name: test_key.clone(),
                        tranche_id: tranche_id_str.clone(),
                        current_level: result.current_ratio,
                        required_level: test.required_level(),
                        breach_date: test_date,
                        consequences_applied: vec![TriggerConsequence::DivertCashFlow],
                    });
                }
            }

            cumulative_senior_balance = cumulative_senior_balance
                .checked_add(tranche.current_balance)
                .unwrap_or(cumulative_senior_balance);
        }

        // Store historical results
        self.historical_results
            .push((test_date, self.aggregate_results.clone()));
        self.aggregate_results = new_aggregate;

        Ok(&self.aggregate_results)
    }

    /// Default haircuts by credit rating
    pub fn default_haircuts() -> HashMap<CreditRating, f64> {
        super::deal_config::CoverageTestConfig::default_haircuts()
    }
}

impl Default for CoverageTests {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::enums::DealType;
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_coverage_test_creation() {
        let mut tests = CoverageTests::new();
        tests.add_oc_test("SENIOR_A".to_string(), 1.15, Some(1.20));

        assert!(tests.tests.contains_key("SENIOR_A_OC"));
        let test = &tests.tests["SENIOR_A_OC"];
        assert_eq!(test.required_level(), 1.15);
        assert_eq!(test.cure_level(), Some(1.20));
    }

    #[test]
    fn test_oc_test_calculation() {
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_oc(1.25, None);

        let context = TestContext {
            pool: &pool,
            tranche_balance: Money::new(100_000.0, Currency::USD),
            senior_balance: Money::new(0.0, Currency::USD),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(0.0, Currency::USD),
            interest_due: Money::new(0.0, Currency::USD),
            senior_interest_due: Money::new(0.0, Currency::USD),
        };

        let result = test.calculate(&context);

        // Empty pool should give 0 ratio
        assert_eq!(result.current_ratio, 0.0);
        assert!(!result.is_passing);
    }

    #[test]
    fn test_ic_test_calculation() {
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_ic(1.20, None);

        let context = TestContext {
            pool: &pool,
            tranche_balance: Money::new(100_000.0, Currency::USD),
            senior_balance: Money::new(0.0, Currency::USD),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(12_000.0, Currency::USD),
            interest_due: Money::new(10_000.0, Currency::USD),
            senior_interest_due: Money::new(0.0, Currency::USD),
        };

        let result = test.calculate(&context);

        // 12000 / 10000 = 1.2, exactly at the threshold
        assert_eq!(result.current_ratio, 1.2);
        assert!(result.is_passing);
    }

    #[test]
    fn test_par_value_test_calculation() {
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_par_value(0.90, None);

        // Note: In a real test, we would add assets to the pool
        // For now, we test with an empty pool
        let context = TestContext {
            pool: &pool,
            tranche_balance: Money::new(80_000.0, Currency::USD),
            senior_balance: Money::new(0.0, Currency::USD),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(0.0, Currency::USD),
            interest_due: Money::new(0.0, Currency::USD),
            senior_interest_due: Money::new(0.0, Currency::USD),
        };

        let result = test.calculate(&context);

        // Empty pool will have INFINITY ratio (0.0 / 0.0 case is handled as INFINITY)
        assert!(result.current_ratio == f64::INFINITY || result.current_ratio.is_nan());
        assert!(result.is_passing); // INFINITY >= 0.90
    }
}
