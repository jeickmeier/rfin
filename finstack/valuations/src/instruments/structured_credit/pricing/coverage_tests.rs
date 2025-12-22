//! Coverage test calculations for structured credit instruments.
//!
//! This module provides OC and IC test calculations for waterfall diversion.

use crate::instruments::structured_credit::types::{Pool, TrancheStructure};
use crate::instruments::structured_credit::utils::frequency_periods_per_year;
use finstack_core::error::{Error as CoreError, InputError};
use finstack_core::money::Money;
use finstack_core::types::ratings::CreditRating;
use finstack_core::Result;
use finstack_core::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Coverage test type (OC/IC).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTest {
    /// Overcollateralization test.
    OC {
        /// Unique test identifier.
        id: String,
        /// Required OC ratio (e.g., 1.25 = 125%).
        required_ratio: f64,
        /// Include cash in numerator.
        include_cash: bool,
        /// Include only performing assets.
        performing_only: bool,
    },
    /// Interest coverage test.
    IC {
        /// Unique test identifier.
        id: String,
        /// Required IC ratio (e.g., 1.20 = 120%).
        required_ratio: f64,
    },
}

impl CoverageTest {
    /// Create new OC test with standard settings.
    pub fn new_oc(required_ratio: f64) -> Self {
        Self::OC {
            id: format!("oc_test_{}", (required_ratio * 100.0) as u32),
            required_ratio,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Create new OC test with explicit ID.
    pub fn new_oc_with_id(id: impl Into<String>, required_ratio: f64) -> Self {
        Self::OC {
            id: id.into(),
            required_ratio,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Create new IC test.
    pub fn new_ic(required_ratio: f64) -> Self {
        Self::IC {
            id: format!("ic_test_{}", (required_ratio * 100.0) as u32),
            required_ratio,
        }
    }

    /// Create new IC test with explicit ID.
    pub fn new_ic_with_id(id: impl Into<String>, required_ratio: f64) -> Self {
        Self::IC {
            id: id.into(),
            required_ratio,
        }
    }

    /// Get the test ID.
    pub fn id(&self) -> &str {
        match self {
            Self::OC { id, .. } => id.as_str(),
            Self::IC { id, .. } => id.as_str(),
        }
    }

    /// Get the required ratio for this test.
    pub fn required_level(&self) -> f64 {
        match self {
            Self::OC { required_ratio, .. } => *required_ratio,
            Self::IC { required_ratio, .. } => *required_ratio,
        }
    }

    /// Calculate the test result.
    pub fn calculate(&self, context: &TestContext) -> Result<TestResult> {
        match self {
            Self::OC {
                id,
                required_ratio,
                include_cash,
                performing_only,
            } => self.calculate_oc(
                context,
                id.clone(),
                *required_ratio,
                *include_cash,
                *performing_only,
            ),
            Self::IC { id, required_ratio } => {
                self.calculate_ic(context, id.clone(), *required_ratio)
            }
        }
    }

    fn calculate_oc(
        &self,
        context: &TestContext,
        test_id: String,
        required_ratio: f64,
        include_cash: bool,
        performing_only: bool,
    ) -> Result<TestResult> {
        let tranche = context
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == context.tranche_id)
            .ok_or_else(|| {
                CoreError::from(InputError::NotFound {
                    id: format!("tranche:{}", context.tranche_id),
                })
            })?;

        let tranche_balance = tranche.current_balance;
        let senior_balance = context.tranches.senior_balance(context.tranche_id);

        let mut numerator =
            collateral_balance_with_haircuts(context.pool, performing_only, context.haircuts)?;

        if include_cash {
            numerator = numerator.checked_add(context.cash_balance)?;
        }

        let denominator = tranche_balance
            .checked_add(senior_balance)
            .unwrap_or(tranche_balance);

        let ratio = if denominator.amount() > 0.0 {
            numerator.amount() / denominator.amount()
        } else {
            f64::INFINITY
        };

        let mut is_passing = ratio >= required_ratio;
        if let Some(threshold) = context.par_value_threshold {
            if ratio < threshold {
                is_passing = false;
            }
        }

        let cure_amount = if !is_passing {
            let required_collateral = denominator.amount() * required_ratio;
            let shortfall = required_collateral - numerator.amount();
            Some(Money::new(shortfall.max(0.0), denominator.currency()))
        } else {
            None
        };

        Ok(TestResult {
            test_id,
            current_ratio: ratio,
            is_passing,
            cure_amount,
        })
    }

    fn calculate_ic(
        &self,
        context: &TestContext,
        test_id: String,
        required_ratio: f64,
    ) -> Result<TestResult> {
        let tranche = context
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == context.tranche_id)
            .ok_or_else(|| {
                CoreError::from(InputError::NotFound {
                    id: format!("tranche:{}", context.tranche_id),
                })
            })?;

        let periods_per_year = frequency_periods_per_year(tranche.payment_frequency);
        let interest_due = Money::new(
            tranche.current_balance.amount() * tranche.coupon.current_rate(context.as_of)
                / periods_per_year,
            tranche.current_balance.currency(),
        );

        let senior_tranches = context.tranches.senior_to(context.tranche_id);
        let senior_interest_due = senior_tranches
            .iter()
            .try_fold(Money::new(0.0, interest_due.currency()), |acc, t| {
                let t_periods = frequency_periods_per_year(t.payment_frequency);
                let interest = Money::new(
                    t.current_balance.amount() * t.coupon.current_rate(context.as_of) / t_periods,
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

        Ok(TestResult {
            test_id,
            current_ratio: ratio,
            is_passing,
            cure_amount: None,
        })
    }
}

/// Context needed to calculate coverage tests.
#[derive(Debug)]
pub struct TestContext<'a> {
    /// Pool reference.
    pub pool: &'a Pool,
    /// Tranche structure reference.
    pub tranches: &'a TrancheStructure,
    /// Target tranche ID.
    pub tranche_id: &'a str,
    /// As-of date.
    pub as_of: finstack_core::dates::Date,
    /// Cash balance.
    pub cash_balance: Money,
    /// Interest collections.
    pub interest_collections: Money,
    /// Optional rating haircuts for collateral.
    pub haircuts: Option<&'a HashMap<CreditRating, f64>>,
    /// Optional par value threshold (ratio).
    pub par_value_threshold: Option<f64>,
}

/// Result of a coverage test calculation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TestResult {
    /// Test identifier.
    pub test_id: String,
    /// Current calculated ratio.
    pub current_ratio: f64,
    /// Whether test is currently passing.
    pub is_passing: bool,
    /// Cure amount if failing (OC tests only).
    pub cure_amount: Option<Money>,
}

fn collateral_balance_with_haircuts(
    pool: &Pool,
    performing_only: bool,
    haircuts: Option<&HashMap<CreditRating, f64>>,
) -> Result<Money> {
    if haircuts.map(|h| h.is_empty()).unwrap_or(true) {
        return Ok(if performing_only {
            pool.performing_balance()?
        } else {
            pool.total_balance()?
        });
    }

    let mut total = Money::new(0.0, pool.base_currency());
    for asset in &pool.assets {
        if performing_only && asset.is_defaulted {
            continue;
        }

        let mut amount = asset.balance.amount();
        if let Some(map) = haircuts {
            let haircut = asset
                .credit_quality
                .and_then(|rating| map.get(&rating).copied())
                .or_else(|| map.get(&CreditRating::NR).copied())
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            amount *= 1.0 - haircut;
        }

        total = total.checked_add(Money::new(amount, total.currency()))?;
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::structured_credit::types::{
        DealType, Pool, Seniority, Tranche, TrancheCoupon, TrancheStructure,
    };
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use time::Month;

    #[test]
    fn test_oc_test_creation() {
        let test = CoverageTest::new_oc(1.15);
        assert_eq!(test.required_level(), 1.15);
    }

    #[test]
    fn test_oc_test_calculation() {
        let pool = Pool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_oc(1.25);

        let tranche = Tranche::new(
            "TEST_TRANCHE",
            0.0,
            100.0,
            Seniority::Senior,
            Money::new(100_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid date"),
        )
        .expect("Valid tranche");

        let tranches = TrancheStructure::new(vec![tranche]).expect("Valid tranche structure");

        let context = TestContext {
            pool: &pool,
            tranches: &tranches,
            tranche_id: "TEST_TRANCHE",
            as_of: Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(0.0, Currency::USD),
            haircuts: None,
            par_value_threshold: None,
        };

        let result = test
            .calculate(&context)
            .expect("calculation should succeed");

        assert_eq!(result.current_ratio, 0.0);
        assert!(!result.is_passing);
    }

    #[test]
    fn test_ic_test_calculation() {
        let pool = Pool::new("TEST", DealType::CLO, Currency::USD);
        let test = CoverageTest::new_ic(1.20);

        let tranche = Tranche::new(
            "TEST_TRANCHE",
            0.0,
            100.0,
            Seniority::Senior,
            Money::new(100_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).expect("Valid date"),
        )
        .expect("Valid tranche");

        let tranches = TrancheStructure::new(vec![tranche]).expect("Valid tranche structure");

        let context = TestContext {
            pool: &pool,
            tranches: &tranches,
            tranche_id: "TEST_TRANCHE",
            as_of: Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"),
            cash_balance: Money::new(0.0, Currency::USD),
            interest_collections: Money::new(1_500.0, Currency::USD),
            haircuts: None,
            par_value_threshold: None,
        };

        let result = test
            .calculate(&context)
            .expect("calculation should succeed");

        assert!((result.current_ratio - 1.2).abs() < 0.01);
        assert!(result.is_passing);
    }
}
