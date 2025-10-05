//! Unified coverage tests for structured credit instruments.
//!
//! This module consolidates the original and enhanced coverage test implementations,
//! providing backward compatibility while using the improved structure internally.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::pool::AssetPool;
use super::tranches::{AbsTranche, TrancheStructure};
use super::enums::{CreditRating, TriggerConsequence};

/// Overcollateralization test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OCTest {
    /// Required OC ratio (e.g., 1.25 = 125%)
    pub required_ratio: f64,
    /// Cure level if higher than trigger
    pub cure_level: Option<f64>,
    /// Current calculated ratio
    pub current_ratio: Option<f64>,
    /// Whether test is currently passing
    pub is_passing: bool,
    /// Cure amount if failing
    pub cure_amount: Option<Money>,
    /// Include cash in numerator
    pub include_cash: bool,
    /// Include only performing assets
    pub performing_only: bool,
}

impl OCTest {
    pub fn new(required_ratio: f64, cure_level: Option<f64>) -> Self {
        Self {
            required_ratio,
            cure_level,
            current_ratio: None,
            is_passing: false,
            cure_amount: None,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Calculate OC ratio for a tranche
    pub fn calculate(
        &mut self,
        pool: &AssetPool,
        tranche_balance: Money,
        senior_balance: Money,
        cash_balance: Money,
    ) -> f64 {
        let mut numerator = if self.performing_only {
            pool.performing_balance()
        } else {
            pool.total_balance()
        };

        if self.include_cash {
            numerator = numerator.checked_add(cash_balance).unwrap_or(numerator);
        }

        let denominator = tranche_balance
            .checked_add(senior_balance)
            .unwrap_or(tranche_balance);

        let ratio = if denominator.amount() > 0.0 {
            numerator.amount() / denominator.amount()
        } else {
            f64::INFINITY
        };

        self.current_ratio = Some(ratio);
        self.is_passing = ratio >= self.required_ratio;

        if !self.is_passing {
            let required_collateral = denominator.amount() * self.required_ratio;
            let shortfall = required_collateral - numerator.amount();
            self.cure_amount = Some(Money::new(shortfall.max(0.0), denominator.currency()));
        } else {
            self.cure_amount = None;
        }

        ratio
    }
}

/// Interest coverage test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ICTest {
    /// Required IC ratio (e.g., 1.20 = 120%)
    pub required_ratio: f64,
    /// Cure level if higher than trigger
    pub cure_level: Option<f64>,
    /// Current calculated ratio
    pub current_ratio: Option<f64>,
    /// Whether test is currently passing
    pub is_passing: bool,
}

impl ICTest {
    pub fn new(required_ratio: f64, cure_level: Option<f64>) -> Self {
        Self {
            required_ratio,
            cure_level,
            current_ratio: None,
            is_passing: false,
        }
    }

    /// Calculate IC ratio
    pub fn calculate(
        &mut self,
        interest_collections: Money,
        interest_due: Money,
        senior_interest_due: Money,
    ) -> f64 {
        let total_interest_due = interest_due
            .checked_add(senior_interest_due)
            .unwrap_or(interest_due);

        let ratio = if total_interest_due.amount() > 0.0 {
            interest_collections.amount() / total_interest_due.amount()
        } else {
            f64::INFINITY
        };

        self.current_ratio = Some(ratio);
        self.is_passing = ratio >= self.required_ratio;

        ratio
    }
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
///
/// This struct provides backward compatibility with the old API while using
/// the enhanced implementation internally.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTests {
    /// OC tests by tranche
    pub oc_tests: HashMap<String, OCTest>,
    /// IC tests by tranche
    pub ic_tests: HashMap<String, ICTest>,
    /// Current test results
    pub current_results: TestResults,
    /// Historical results for trending
    pub historical_results: Vec<(Date, TestResults)>,
}

impl CoverageTests {
    /// Create new coverage test framework
    pub fn new() -> Self {
        Self {
            oc_tests: HashMap::new(),
            ic_tests: HashMap::new(),
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

    /// Add standard OC test for a tranche (backward compatibility)
    pub fn add_oc_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        self.oc_tests
            .insert(tranche_id, OCTest::new(trigger_level, cure_level));
        self
    }

    /// Add standard IC test for a tranche (backward compatibility)
    pub fn add_ic_test(
        &mut self,
        tranche_id: String,
        trigger_level: f64,
        cure_level: Option<f64>,
    ) -> &mut Self {
        self.ic_tests
            .insert(tranche_id, ICTest::new(trigger_level, cure_level));
        self
    }

    /// Run all coverage tests (backward compatibility)
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

        let base_ccy = pool.base_currency();
        let cash_balance = Money::new(0.0, base_ccy); // Would come from pool accounts

        // Calculate tests for each tranche
        let mut cumulative_senior_balance = Money::new(0.0, base_ccy);

        for tranche in &tranches.tranches {
            let tranche_id = tranche.id.to_string();

            // OC test
            if let Some(oc_test) = self.oc_tests.get_mut(&tranche_id) {
                let ratio = oc_test.calculate(
                    pool,
                    tranche.current_balance,
                    cumulative_senior_balance,
                    cash_balance,
                );
                new_results.oc_ratios.insert(tranche_id.clone(), ratio);

                if !oc_test.is_passing {
                    new_results.breached_tests.push(BreachedTest {
                        test_name: format!("{}_OC", tranche_id),
                        tranche_id: tranche_id.clone(),
                        current_level: ratio,
                        required_level: oc_test.required_ratio,
                        breach_date: test_date,
                        consequences_applied: vec![TriggerConsequence::DivertCashFlow],
                    });
                }
            }

            // IC test
            if let Some(ic_test) = self.ic_tests.get_mut(&tranche_id) {
                // Simplified interest calculation
                let pool_interest = Money::new(
                    pool.performing_balance().amount() * pool.weighted_avg_coupon() / 4.0,
                    base_ccy,
                );
                let tranche_interest = Money::new(
                    tranche.current_balance.amount() * tranche.coupon.current_rate(test_date) / 4.0,
                    base_ccy,
                );

                let ratio = ic_test.calculate(
                    pool_interest,
                    tranche_interest,
                    Money::new(0.0, base_ccy), // Would calculate senior interest
                );
                new_results.ic_ratios.insert(tranche_id.clone(), ratio);

                if !ic_test.is_passing {
                    new_results.breached_tests.push(BreachedTest {
                        test_name: format!("{}_IC", tranche_id),
                        tranche_id: tranche_id.clone(),
                        current_level: ratio,
                        required_level: ic_test.required_ratio,
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
            .push((test_date, self.current_results.clone()));
        self.current_results = new_results;

        Ok(&self.current_results)
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

/// Coverage test calculation engine (static methods for backward compatibility)
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
                if let Some(recovery) = asset.recovery_amount {
                    pool_value += recovery.amount();
                }
            } else {
                let mut asset_value = asset.balance.amount();
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
        let pool_interest =
            pool.weighted_avg_coupon() * pool.performing_balance().amount() * annualization_factor;

        let mut interest_due = 0.0;
        for senior_tranche in tranches.senior_to(tranche.id.as_str()) {
            let rate = senior_tranche
                .coupon
                .current_rate(Date::from_calendar_date(2025, time::Month::January, 1).unwrap());
            interest_due += senior_tranche.current_balance.amount() * rate * annualization_factor;
        }

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
    use super::super::enums::DealType;
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_coverage_test_creation() {
        let mut tests = CoverageTests::new();
        tests.add_oc_test("SENIOR_A".to_string(), 1.15, Some(1.20));

        assert!(tests.oc_tests.contains_key("SENIOR_A"));
        let test = &tests.oc_tests["SENIOR_A"];
        assert_eq!(test.required_ratio, 1.15);
        assert_eq!(test.cure_level, Some(1.20));
    }

    #[test]
    fn test_oc_test_calculation() {
        let mut oc_test = OCTest::new(1.25, None);
        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);

        let ratio = oc_test.calculate(
            &pool,
            Money::new(100_000.0, Currency::USD),
            Money::new(0.0, Currency::USD),
            Money::new(0.0, Currency::USD),
        );

        // Empty pool should give 0 ratio
        assert_eq!(ratio, 0.0);
        assert!(!oc_test.is_passing);
    }
}
