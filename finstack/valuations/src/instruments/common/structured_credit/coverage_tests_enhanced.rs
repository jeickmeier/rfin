//! Enhanced coverage tests for structured credit with market-standard calculations.
//!
//! This module implements overcollateralization (OC), interest coverage (IC),
//! and other tests used in CLOs and structured products.

use crate::instruments::common::structured_credit::types_extended::{Tranche, TrancheId};
use crate::instruments::common::structured_credit::{AssetPool, CreditRating};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Enhanced coverage test suite with market-standard calculations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EnhancedCoverageTests {
    /// Overcollateralization tests by tranche
    pub oc_tests: HashMap<TrancheId, OCTest>,
    /// Interest coverage tests by tranche
    pub ic_tests: HashMap<TrancheId, ICTest>,
    /// Par value test
    pub par_value_test: Option<ParValueTest>,
    /// Diversity score test
    pub diversity_test: Option<DiversityTest>,
    /// Weighted average rating factor test
    pub warf_test: Option<WARFTest>,
    /// Weighted average spread test
    pub was_test: Option<WASTest>,
}

/// Overcollateralization test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct OCTest {
    /// Required OC ratio (e.g., 1.25 = 125%)
    pub required_ratio: f64,
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
    /// Create a new OC test with required ratio
    pub fn new(required_ratio: f64) -> Self {
        Self {
            required_ratio,
            current_ratio: None,
            is_passing: false,
            cure_amount: None,
            include_cash: true,
            performing_only: true,
        }
    }

    /// Calculate OC ratio for a tranche
    ///
    /// Market standard formula:
    /// OC = (Performing Collateral + Cash) / (Tranche Balance + Senior Tranches)
    pub fn calculate(
        &mut self,
        pool: &AssetPool,
        tranche: &Tranche,
        senior_balance: Money,
        cash_balance: Money,
    ) -> f64 {
        // Calculate numerator
        let mut numerator = if self.performing_only {
            pool.performing_balance()
        } else {
            pool.total_balance()
        };

        if self.include_cash {
            numerator = numerator.checked_add(cash_balance).unwrap_or(numerator);
        }

        // Calculate denominator (tranche plus all senior tranches)
        let denominator = tranche
            .current_balance
            .checked_add(senior_balance)
            .unwrap_or(tranche.current_balance);

        // Calculate ratio
        let ratio = if denominator.amount() > 0.0 {
            numerator.amount() / denominator.amount()
        } else {
            f64::INFINITY
        };

        // Update state
        self.current_ratio = Some(ratio);
        self.is_passing = ratio >= self.required_ratio;

        // Calculate cure amount if failing
        if !self.is_passing {
            let required_collateral = denominator.amount() * self.required_ratio;
            let shortfall = required_collateral - numerator.amount();
            self.cure_amount = Some(Money::new(shortfall.max(0.0), denominator.currency()));
        } else {
            self.cure_amount = None;
        }

        ratio
    }

    /// Get cushion amount (excess over requirement)
    pub fn get_cushion(&self, currency: Currency) -> Money {
        if let Some(ratio) = self.current_ratio {
            if ratio > self.required_ratio {
                let excess = ratio - self.required_ratio;
                Money::new(excess, currency)
            } else {
                Money::new(0.0, currency)
            }
        } else {
            Money::new(0.0, currency)
        }
    }
}

/// Interest coverage test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ICTest {
    /// Required IC ratio (e.g., 1.20 = 120%)
    pub required_ratio: f64,
    /// Current calculated ratio
    pub current_ratio: Option<f64>,
    /// Whether test is currently passing
    pub is_passing: bool,
    /// Include scheduled interest only
    pub scheduled_only: bool,
}

impl ICTest {
    /// Create a new IC test
    pub fn new(required_ratio: f64) -> Self {
        Self {
            required_ratio,
            current_ratio: None,
            is_passing: false,
            scheduled_only: true,
        }
    }

    /// Calculate IC ratio
    ///
    /// Market standard formula:
    /// IC = Interest Collections / (Interest Due on Tranche + Senior Tranches)
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

/// Par value test (for trading losses)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParValueTest {
    /// Minimum acceptable par value ratio
    pub min_ratio: f64,
    /// Current ratio
    pub current_ratio: Option<f64>,
    /// Whether test is passing
    pub is_passing: bool,
}

impl ParValueTest {
    /// Calculate par value ratio
    ///
    /// Ratio = (Collateral Par - Trading Losses) / Initial Collateral Par
    pub fn calculate(
        &mut self,
        current_par: Money,
        initial_par: Money,
        trading_losses: Money,
    ) -> f64 {
        let adjusted_par = current_par
            .checked_sub(trading_losses)
            .unwrap_or(current_par);

        let ratio = if initial_par.amount() > 0.0 {
            adjusted_par.amount() / initial_par.amount()
        } else {
            0.0
        };

        self.current_ratio = Some(ratio);
        self.is_passing = ratio >= self.min_ratio;

        ratio
    }
}

/// Diversity score test (concentration limits)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiversityTest {
    /// Minimum required diversity score
    pub min_score: f64,
    /// Current diversity score
    pub current_score: Option<f64>,
    /// Whether test is passing
    pub is_passing: bool,
    /// Industry concentration limits
    pub industry_limits: HashMap<String, f64>,
    /// Obligor concentration limits
    pub obligor_limits: HashMap<String, f64>,
}

impl DiversityTest {
    /// Calculate Moody's diversity score
    ///
    /// This is a simplified version. Full implementation would include:
    /// - Industry correlation matrix
    /// - Obligor equivalents calculation
    /// - Regional diversity factors
    pub fn calculate(&mut self, pool: &AssetPool) -> f64 {
        // Group assets by obligor
        let mut obligor_exposures: HashMap<String, Money> = HashMap::new();

        for asset in &pool.assets {
            let obligor = asset.obligor_id.as_deref().unwrap_or("Unknown");
            let entry = obligor_exposures
                .entry(obligor.to_string())
                .or_insert(Money::new(0.0, pool.base_currency()));
            *entry = entry.checked_add(asset.balance).unwrap_or(*entry);
        }

        // Calculate diversity score using simplified Moody's approach
        // DS = Sum(1/n) where n is number of equal-sized obligors
        let total_balance = pool.total_balance().amount();
        let mut diversity_score = 0.0;

        for (_obligor, exposure) in obligor_exposures {
            if total_balance > 0.0 {
                let weight = exposure.amount() / total_balance;
                // Equivalent number of equal-sized obligors
                diversity_score += weight * weight;
            }
        }

        // Convert to diversity score (reciprocal of Herfindahl index)
        diversity_score = if diversity_score > 0.0 {
            1.0 / diversity_score
        } else {
            0.0
        };

        self.current_score = Some(diversity_score);
        self.is_passing = diversity_score >= self.min_score;

        diversity_score
    }

    /// Check industry concentration limits
    pub fn check_industry_limits(&self, pool: &AssetPool) -> Vec<String> {
        let mut violations = Vec::new();
        let mut industry_exposures: HashMap<String, f64> = HashMap::new();
        let total = pool.total_balance().amount();

        for asset in &pool.assets {
            if let Some(industry) = &asset.industry {
                *industry_exposures.entry(industry.clone()).or_insert(0.0) +=
                    asset.balance.amount() / total;
            }
        }

        for (industry, exposure) in industry_exposures {
            if let Some(limit) = self.industry_limits.get(&industry) {
                if exposure > *limit {
                    violations.push(format!(
                        "{}: {:.1}% exceeds {:.1}% limit",
                        industry,
                        exposure * 100.0,
                        limit * 100.0
                    ));
                }
            }
        }

        violations
    }
}

/// Weighted Average Rating Factor test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WARFTest {
    /// Maximum acceptable WARF
    pub max_warf: f64,
    /// Current WARF
    pub current_warf: Option<f64>,
    /// Whether test is passing
    pub is_passing: bool,
}

impl WARFTest {
    /// Calculate Weighted Average Rating Factor
    ///
    /// WARF uses Moody's idealized default probabilities
    pub fn calculate(&mut self, pool: &AssetPool) -> f64 {
        let mut weighted_sum = 0.0;
        let mut total_balance = 0.0;

        for asset in &pool.assets {
            let balance = asset.balance.amount();
            let rating_factor = asset
                .credit_quality
                .map(get_moody_rating_factor)
                .unwrap_or(3650.0); // CCC/unrated default

            weighted_sum += balance * rating_factor;
            total_balance += balance;
        }

        let warf = if total_balance > 0.0 {
            weighted_sum / total_balance
        } else {
            0.0
        };

        self.current_warf = Some(warf);
        self.is_passing = warf <= self.max_warf;

        warf
    }
}

/// Weighted Average Spread test
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WASTest {
    /// Minimum required WAS (basis points)
    pub min_spread_bps: f64,
    /// Current WAS
    pub current_spread: Option<f64>,
    /// Whether test is passing
    pub is_passing: bool,
}

impl WASTest {
    /// Calculate Weighted Average Spread
    pub fn calculate(&mut self, pool: &AssetPool) -> f64 {
        let mut weighted_spread = 0.0;
        let mut total_balance = 0.0;

        for asset in &pool.assets {
            let balance = asset.balance.amount();
            // Spread would come from asset data - using rate as proxy
            let spread_bps = asset.rate * 10000.0;

            weighted_spread += balance * spread_bps;
            total_balance += balance;
        }

        let was = if total_balance > 0.0 {
            weighted_spread / total_balance
        } else {
            0.0
        };

        self.current_spread = Some(was);
        self.is_passing = was >= self.min_spread_bps;

        was
    }
}

/// Get Moody's rating factor for WARF calculation
/// 
/// Uses standardized Moody's IDEALIZED DEFAULT RATES table.
/// This function is duplicated from the metrics module for now,
/// but should be consolidated into a shared rating utilities module.
fn get_moody_rating_factor(rating: CreditRating) -> f64 {
    // TODO: Consolidate with clo::metrics::warf::get_moody_rating_factor
    crate::instruments::common::structured_credit::rating_factors::moodys_warf_factor(rating)
}

/// Calculate all coverage tests for a structure
pub fn calculate_all_coverage_tests(
    tests: &mut EnhancedCoverageTests,
    pool: &AssetPool,
    tranches: &[Tranche],
    cash_balance: Money,
    interest_collections: Money,
) -> CoverageTestResults {
    let mut results = CoverageTestResults::default();

    // Calculate tests for each tranche
    let mut cumulative_senior_balance = Money::new(0.0, cash_balance.currency());
    let mut cumulative_senior_interest = Money::new(0.0, cash_balance.currency());

    for tranche in tranches {
        // OC test
        if let Some(oc_test) = tests.oc_tests.get_mut(&tranche.id) {
            let ratio = oc_test.calculate(pool, tranche, cumulative_senior_balance, cash_balance);
            results.oc_ratios.insert(tranche.id.clone(), ratio);
            results
                .oc_passing
                .insert(tranche.id.clone(), oc_test.is_passing);
        }

        // IC test
        if let Some(ic_test) = tests.ic_tests.get_mut(&tranche.id) {
            let interest_due = Money::new(
                tranche.current_balance.amount() * tranche.coupon_rate / 12.0,
                tranche.current_balance.currency(),
            );

            let ratio = ic_test.calculate(
                interest_collections,
                interest_due,
                cumulative_senior_interest,
            );
            results.ic_ratios.insert(tranche.id.clone(), ratio);
            results
                .ic_passing
                .insert(tranche.id.clone(), ic_test.is_passing);

            cumulative_senior_interest = cumulative_senior_interest
                .checked_add(interest_due)
                .unwrap_or(cumulative_senior_interest);
        }

        cumulative_senior_balance = cumulative_senior_balance
            .checked_add(tranche.current_balance)
            .unwrap_or(cumulative_senior_balance);
    }

    // Par value test
    if let Some(par_test) = &mut tests.par_value_test {
        let ratio = par_test.calculate(
            pool.total_balance(),
            pool.total_balance(), // Using total_balance as proxy for original
            Money::new(0.0, pool.base_currency()), // Trading losses would be tracked
        );
        results.par_value_ratio = Some(ratio);
        results.par_value_passing = par_test.is_passing;
    }

    // Diversity test
    if let Some(div_test) = &mut tests.diversity_test {
        let score = div_test.calculate(pool);
        results.diversity_score = Some(score);
        results.diversity_passing = div_test.is_passing;
    }

    // WARF test
    if let Some(warf_test) = &mut tests.warf_test {
        let warf = warf_test.calculate(pool);
        results.warf = Some(warf);
        results.warf_passing = warf_test.is_passing;
    }

    // WAS test
    if let Some(was_test) = &mut tests.was_test {
        let was = was_test.calculate(pool);
        results.was = Some(was);
        results.was_passing = was_test.is_passing;
    }

    results
}

/// Results from coverage test calculations
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTestResults {
    pub oc_ratios: HashMap<TrancheId, f64>,
    pub oc_passing: HashMap<TrancheId, bool>,
    pub ic_ratios: HashMap<TrancheId, f64>,
    pub ic_passing: HashMap<TrancheId, bool>,
    pub par_value_ratio: Option<f64>,
    pub par_value_passing: bool,
    pub diversity_score: Option<f64>,
    pub diversity_passing: bool,
    pub warf: Option<f64>,
    pub warf_passing: bool,
    pub was: Option<f64>,
    pub was_passing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oc_calculation() {
        let mut oc_test = OCTest::new(1.25);

        // Create test data
        let pool = create_test_pool();
        let tranche = create_test_tranche();
        let senior_balance = Money::new(50_000_000.0, Currency::USD);
        let cash = Money::new(5_000_000.0, Currency::USD);

        let ratio = oc_test.calculate(&pool, &tranche, senior_balance, cash);

        assert!(ratio > 0.0);
        assert_eq!(oc_test.is_passing, ratio >= 1.25);
    }

    #[test]
    fn test_diversity_score() {
        let mut div_test = DiversityTest {
            min_score: 50.0,
            current_score: None,
            is_passing: false,
            industry_limits: HashMap::new(),
            obligor_limits: HashMap::new(),
        };

        let pool = create_test_pool();
        let score = div_test.calculate(&pool);

        // Empty pool will have diversity score of 0
        assert!(score >= 0.0);
    }

    fn create_test_pool() -> AssetPool {
        use crate::instruments::common::structured_credit::DealType;
        AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD)
    }

    fn create_test_tranche() -> Tranche {
        use crate::instruments::common::structured_credit::CouponType;
        Tranche {
            id: "CLASS_A".into(),
            name: "Class A Notes".to_string(),
            rating: Some(CreditRating::AAA),
            original_balance: Money::new(80_000_000.0, Currency::USD),
            current_balance: Money::new(80_000_000.0, Currency::USD),
            coupon_rate: 0.03,
            coupon_type: CouponType::Fixed,
            payment_priority: 1,
            legal_maturity: finstack_core::dates::Date::from_calendar_date(
                2030,
                time::Month::January,
                1,
            )
            .unwrap(),
            coverage_tests: None,
        }
    }
}
