//! Simplified coverage tests for structured credit instruments.
//!
//! This module provides OC and IC test calculations for waterfall diversion.
//! Removed: ParValue tests, historical tracking, aggregate results.

use finstack_core::money::Money;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::AssetPool;

/// Simplified coverage test type (OC/IC only)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTest {
    /// Overcollateralization test
    OC {
        /// Required OC ratio (e.g., 1.25 = 125%)
        required_ratio: f64,
        /// Include cash in numerator
        include_cash: bool,
        /// Include only performing assets
        performing_only: bool,
    },
    /// Interest coverage test
    IC {
        /// Required IC ratio (e.g., 1.20 = 120%)
        required_ratio: f64,
    },
}

impl CoverageTest {
    /// Create new OC test with standard settings
    pub fn new_oc(required_ratio: f64) -> Self {
        Self::OC {
            required_ratio,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Create new IC test
    pub fn new_ic(required_ratio: f64) -> Self {
        Self::IC { required_ratio }
    }

    /// Get the required ratio for this test
    pub fn required_level(&self) -> f64 {
        match self {
            Self::OC { required_ratio, .. } => *required_ratio,
            Self::IC { required_ratio, .. } => *required_ratio,
        }
    }

    /// Calculate the test result
    pub fn calculate(&self, context: &TestContext) -> TestResult {
        match self {
            Self::OC {
                required_ratio,
                include_cash,
                performing_only,
            } => self.calculate_oc(context, *required_ratio, *include_cash, *performing_only),
            Self::IC { required_ratio } => self.calculate_ic(context, *required_ratio),
        }
    }

    fn calculate_oc(
        &self,
        context: &TestContext,
        required_ratio: f64,
        include_cash: bool,
        performing_only: bool,
    ) -> TestResult {
        // Find the target tranche
        let tranche = context
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == context.tranche_id)
            .expect("Tranche not found in context");

        // Compute tranche balance
        let tranche_balance = tranche.current_balance;

        // Compute senior balance (all tranches with higher priority)
        let senior_balance = context.tranches.senior_balance(context.tranche_id);

        // Calculate numerator
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

        // Calculate denominator
        let denominator = tranche_balance
            .checked_add(senior_balance)
            .unwrap_or(tranche_balance);

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
        }
    }

    fn calculate_ic(&self, context: &TestContext, required_ratio: f64) -> TestResult {
        // Find the target tranche
        let tranche = context
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == context.tranche_id)
            .expect("Tranche not found in context");

        // Compute interest due for this tranche
        // Simplified: assume quarterly payment at current coupon rate
        let interest_due = Money::new(
            tranche.current_balance.amount() * tranche.coupon.current_rate(context.as_of) / 4.0,
            tranche.current_balance.currency(),
        );

        // Compute senior interest due
        let senior_tranches = context.tranches.senior_to(context.tranche_id);
        let senior_interest_due = senior_tranches
            .iter()
            .try_fold(Money::new(0.0, interest_due.currency()), |acc, t| {
                let interest = Money::new(
                    t.current_balance.amount() * t.coupon.current_rate(context.as_of) / 4.0,
                    t.current_balance.currency(),
                );
                acc.checked_add(interest)
            })
            .unwrap_or_else(|_| Money::new(0.0, interest_due.currency()));

        let total_interest_due = interest_due
            .checked_add(senior_interest_due)
            .unwrap_or(interest_due);

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
        }
    }
}

/// Context needed to calculate coverage tests.
///
/// Simplified context that provides pool and tranche structure references,
/// allowing the test to compute derived values (like senior balance, interest due) internally.
#[derive(Debug)]
pub struct TestContext<'a> {
    pub pool: &'a AssetPool,
    pub tranches: &'a super::TrancheStructure,
    pub tranche_id: &'a str,
    pub as_of: finstack_core::dates::Date,
    pub cash_balance: Money,
    pub interest_collections: Money,
}

/// Shared result structure for coverage tests
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TestResult {
    /// Current calculated ratio
    pub current_ratio: f64,
    /// Whether test is currently passing
    pub is_passing: bool,
    /// Cure amount if failing (OC tests only)
    pub cure_amount: Option<Money>,
}

// CoverageTests collection removed - use individual CoverageTest::calculate() for ad-hoc checks

#[cfg(test)]
mod coverage_test_tests {
    use super::super::DealType;
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_oc_test_creation() {
        let test = CoverageTest::new_oc(1.15);
        assert_eq!(test.required_level(), 1.15);
    }

    #[test]
    fn test_oc_test_calculation() {
        use super::super::{Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure};
        use finstack_core::dates::Date;
        use time::Month;

        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_oc(1.25);

        // Create a simple tranche structure
        let tranche = Tranche::new(
            "TEST_TRANCHE",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(100_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![tranche]).unwrap();

        let context = TestContext {
            pool: &pool,
            tranches: &tranches,
            tranche_id: "TEST_TRANCHE",
            as_of: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(0.0, Currency::USD),
        };

        let result = test.calculate(&context);

        // Empty pool should give 0 ratio
        assert_eq!(result.current_ratio, 0.0);
        assert!(!result.is_passing);
    }

    #[test]
    fn test_ic_test_calculation() {
        use super::super::{Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure};
        use finstack_core::dates::Date;
        use time::Month;

        let pool = AssetPool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_ic(1.20);

        // Create a tranche structure
        // Interest due = 100k * 5% / 4 = 1,250
        // To get ratio of 1.2, we need collections = 1.2 * 1,250 = 1,500
        let tranche = Tranche::new(
            "TEST_TRANCHE",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(100_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![tranche]).unwrap();

        let context = TestContext {
            pool: &pool,
            tranches: &tranches,
            tranche_id: "TEST_TRANCHE",
            as_of: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(1_500.0, Currency::USD),
        };

        let result = test.calculate(&context);

        // Should pass at 1.2 ratio
        assert!((result.current_ratio - 1.2).abs() < 0.01);
        assert!(result.is_passing);
    }
}
