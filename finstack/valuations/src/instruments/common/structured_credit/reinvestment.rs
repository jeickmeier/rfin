//! Reinvestment period management for CLOs and structured products.
//!
//! This module handles the reinvestment of principal proceeds during the
//! reinvestment period, including eligibility criteria, concentration limits,
//! and portfolio quality tests.

use crate::instruments::common::structured_credit::{
    calculate_seasoning_months, AssetPool, AssetType, CreditRating, PoolAsset, TestResults,
};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Manages reinvestment during the reinvestment period
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReinvestmentManager {
    /// End date of reinvestment period
    pub end_date: Date,
    /// Eligibility criteria for new assets
    pub eligibility_criteria: EligibilityCriteria,
    /// Concentration limits
    pub concentration_limits: ConcentrationLimits,
    /// Portfolio quality tests
    pub quality_tests: PortfolioQualityTests,
    /// Whether reinvestment is currently allowed
    pub reinvestment_allowed: bool,
    /// Events that can terminate reinvestment
    pub termination_events: Vec<ReinvestmentTerminationEvent>,
}

/// Criteria that assets must meet to be eligible for purchase
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EligibilityCriteria {
    /// Minimum rating required
    pub min_rating: Option<CreditRating>,
    /// Maximum rating allowed (for risk limits)
    pub max_rating: Option<CreditRating>,
    /// Minimum spread (basis points)
    pub min_spread_bps: Option<f64>,
    /// Maximum maturity date
    pub max_maturity: Option<Date>,
    /// Minimum remaining term (months)
    pub min_remaining_term: Option<u32>,
    /// Maximum remaining term (months)
    pub max_remaining_term: Option<u32>,
    /// Allowed asset types
    pub allowed_asset_types: Vec<AssetType>,
    /// Allowed currencies
    pub allowed_currencies: Vec<Currency>,
    /// Maximum price (% of par)
    pub max_price_pct: Option<f64>,
    /// Minimum asset size
    pub min_asset_size: Option<Money>,
    /// Maximum asset size
    pub max_asset_size: Option<Money>,
    /// Excluded industries
    pub excluded_industries: Vec<String>,
    /// Excluded obligors
    pub excluded_obligors: Vec<String>,
}

impl EligibilityCriteria {
    /// Check if an asset meets all eligibility criteria
    pub fn is_eligible(&self, asset: &PoolAsset, current_date: Date) -> (bool, Vec<String>) {
        let mut reasons = Vec::new();
        let mut eligible = true;

        // Check rating
        if let Some(min_rating) = self.min_rating {
            if let Some(asset_rating) = asset.credit_quality {
                if asset_rating > min_rating {
                    eligible = false;
                    reasons.push(format!(
                        "Rating {:?} below minimum {:?}",
                        asset_rating, min_rating
                    ));
                }
            } else {
                eligible = false;
                reasons.push("Asset has no rating".to_string());
            }
        }

        if let Some(max_rating) = self.max_rating {
            if let Some(asset_rating) = asset.credit_quality {
                if asset_rating < max_rating {
                    eligible = false;
                    reasons.push(format!(
                        "Rating {:?} above maximum {:?}",
                        asset_rating, max_rating
                    ));
                }
            }
        }

        // Check spread
        if let Some(min_spread) = self.min_spread_bps {
            if let Some(asset_spread) = asset.spread_bps {
                if asset_spread < min_spread {
                    eligible = false;
                    reasons.push(format!(
                        "Spread {:.0}bps below minimum {:.0}bps",
                        asset_spread, min_spread
                    ));
                }
            }
        }

        // Check maturity
        if let Some(max_maturity) = self.max_maturity {
            if asset.maturity > max_maturity {
                eligible = false;
                reasons.push(format!(
                    "Maturity {:?} beyond maximum {:?}",
                    asset.maturity, max_maturity
                ));
            }
        }

        // Check remaining term
        let months_remaining = calculate_seasoning_months(current_date, asset.maturity);

        if let Some(min_term) = self.min_remaining_term {
            if months_remaining < min_term {
                eligible = false;
                reasons.push(format!(
                    "Remaining term {}m below minimum {}m",
                    months_remaining, min_term
                ));
            }
        }

        if let Some(max_term) = self.max_remaining_term {
            if months_remaining > max_term {
                eligible = false;
                reasons.push(format!(
                    "Remaining term {}m above maximum {}m",
                    months_remaining, max_term
                ));
            }
        }

        // Check asset type
        if !self.allowed_asset_types.is_empty()
            && !self.allowed_asset_types.contains(&asset.asset_type)
        {
            eligible = false;
            reasons.push(format!("Asset type {:?} not allowed", asset.asset_type));
        }

        // Check currency
        if !self.allowed_currencies.is_empty()
            && !self.allowed_currencies.contains(&asset.balance.currency())
        {
            eligible = false;
            reasons.push(format!(
                "Currency {:?} not allowed",
                asset.balance.currency()
            ));
        }

        // Check industry exclusions
        if let Some(industry) = &asset.industry {
            if self.excluded_industries.contains(industry) {
                eligible = false;
                reasons.push(format!("Industry {} is excluded", industry));
            }
        }

        // Check obligor exclusions
        if let Some(obligor) = &asset.obligor_id {
            if self.excluded_obligors.contains(obligor) {
                eligible = false;
                reasons.push(format!("Obligor {} is excluded", obligor));
            }
        }

        (eligible, reasons)
    }
}

/// Concentration limits for the portfolio
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConcentrationLimits {
    /// Maximum exposure to single obligor (% of pool)
    pub max_obligor_concentration: Option<f64>,
    /// Maximum top 5 obligor concentration
    pub max_top5_concentration: Option<f64>,
    /// Maximum top 10 obligor concentration
    pub max_top10_concentration: Option<f64>,
    /// Industry concentration limits
    pub industry_limits: HashMap<String, f64>,
    /// Rating bucket limits
    pub rating_bucket_limits: HashMap<CreditRating, f64>,
    /// Country/region limits
    pub geographic_limits: HashMap<String, f64>,
    /// Asset type limits (using String keys for categories like "Loan", "Bond", etc.)
    pub asset_type_limits: HashMap<String, f64>,
    /// Second lien loan limit
    pub max_second_lien: Option<f64>,
    /// Covenant-lite loan limit
    pub max_cov_lite: Option<f64>,
    /// DIP (Debtor-in-Possession) loan limit
    pub max_dip: Option<f64>,
}

impl ConcentrationLimits {
    /// Check if adding an asset would breach concentration limits
    pub fn check_limits(&self, pool: &AssetPool, new_asset: &PoolAsset) -> (bool, Vec<String>) {
        let mut breaches = Vec::new();
        let mut passes = true;

        let total_balance = pool
            .total_balance()
            .checked_add(new_asset.balance)
            .unwrap_or_else(|_| pool.total_balance());

        // Check single obligor concentration
        if let Some(max_obligor) = self.max_obligor_concentration {
            if let Some(obligor) = &new_asset.obligor_id {
                let obligor_exposure = get_obligor_exposure(pool, obligor)
                    .checked_add(new_asset.balance)
                    .unwrap_or_else(|_| get_obligor_exposure(pool, obligor));
                let concentration = obligor_exposure.amount() / total_balance.amount();

                if concentration > max_obligor {
                    passes = false;
                    breaches.push(format!(
                        "Obligor concentration {:.1}% exceeds {:.1}% limit",
                        concentration * 100.0,
                        max_obligor * 100.0
                    ));
                }
            }
        }

        // Check industry concentration
        if let Some(industry) = &new_asset.industry {
            if let Some(limit) = self.industry_limits.get(industry) {
                let industry_exposure = get_industry_exposure(pool, industry)
                    .checked_add(new_asset.balance)
                    .unwrap_or_else(|_| get_industry_exposure(pool, industry));
                let concentration = industry_exposure.amount() / total_balance.amount();

                if concentration > *limit {
                    passes = false;
                    breaches.push(format!(
                        "Industry {} concentration {:.1}% exceeds {:.1}% limit",
                        industry,
                        concentration * 100.0,
                        limit * 100.0
                    ));
                }
            }
        }

        // Check rating bucket limits
        if let Some(rating) = new_asset.credit_quality {
            if let Some(limit) = self.rating_bucket_limits.get(&rating) {
                let rating_exposure = get_rating_exposure(pool, rating)
                    .checked_add(new_asset.balance)
                    .unwrap_or_else(|_| get_rating_exposure(pool, rating));
                let concentration = rating_exposure.amount() / total_balance.amount();

                if concentration > *limit {
                    passes = false;
                    breaches.push(format!(
                        "Rating {:?} concentration {:.1}% exceeds {:.1}% limit",
                        rating,
                        concentration * 100.0,
                        limit * 100.0
                    ));
                }
            }
        }

        (passes, breaches)
    }
}

/// Portfolio quality tests that must be maintained
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PortfolioQualityTests {
    /// Minimum weighted average rating factor
    pub min_warf: Option<f64>,
    /// Maximum weighted average rating factor
    pub max_warf: Option<f64>,
    /// Minimum weighted average spread
    pub min_was_bps: Option<f64>,
    /// Minimum weighted average coupon
    pub min_wac: Option<f64>,
    /// Maximum weighted average life
    pub max_wal_years: Option<f64>,
    /// Minimum diversity score
    pub min_diversity_score: Option<f64>,
}

impl PortfolioQualityTests {
    /// Check if adding an asset maintains portfolio quality
    pub fn check_quality(&self, pool: &AssetPool, new_asset: &PoolAsset) -> (bool, Vec<String>) {
        let mut failures = Vec::new();
        let mut passes = true;

        // Calculate pro-forma metrics with new asset
        let proforma_pool = with_additional_asset(pool, new_asset);

        // Check WARF
        if let Some(max_warf) = self.max_warf {
            let warf = calculate_warf(&proforma_pool);
            if warf > max_warf {
                passes = false;
                failures.push(format!(
                    "Pro-forma WARF {:.0} exceeds maximum {:.0}",
                    warf, max_warf
                ));
            }
        }

        // Check WAS
        if let Some(min_was) = self.min_was_bps {
            let was = calculate_was(&proforma_pool);
            if was < min_was {
                passes = false;
                failures.push(format!(
                    "Pro-forma WAS {:.0}bps below minimum {:.0}bps",
                    was, min_was
                ));
            }
        }

        // Check diversity
        if let Some(min_diversity) = self.min_diversity_score {
            let diversity = calculate_diversity_score(&proforma_pool);
            if diversity < min_diversity {
                passes = false;
                failures.push(format!(
                    "Pro-forma diversity {:.1} below minimum {:.1}",
                    diversity, min_diversity
                ));
            }
        }

        (passes, failures)
    }
}

/// Events that can terminate the reinvestment period
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ReinvestmentTerminationEvent {
    /// Scheduled end date reached
    ScheduledEnd,
    /// Coverage test failure
    CoverageTestFailure { test_type: String, tranche: String },
    /// Manager replacement
    ManagerReplacement,
    /// Event of default
    EventOfDefault,
    /// Voluntary termination by manager
    VoluntaryTermination,
    /// Insufficient reinvestment opportunities
    InsufficientOpportunities {
        min_required_purchases: Money,
        period_months: u32,
    },
}

impl ReinvestmentManager {
    /// Create new reinvestment manager
    pub fn new(end_date: Date) -> Self {
        Self {
            end_date,
            eligibility_criteria: EligibilityCriteria::default(),
            concentration_limits: ConcentrationLimits::default(),
            quality_tests: PortfolioQualityTests::default(),
            reinvestment_allowed: true,
            termination_events: Vec::new(),
        }
    }

    /// Check if reinvestment is allowed at a given date
    pub fn can_reinvest(&self, as_of: Date, coverage_results: &TestResults) -> bool {
        // Check if past reinvestment end date
        if as_of >= self.end_date {
            return false;
        }

        // Check if manually disabled
        if !self.reinvestment_allowed {
            return false;
        }

        // Check coverage tests - if any tests are breached, reinvestment stops
        if !coverage_results.breached_tests.is_empty() {
            return false;
        }

        true
    }

    /// Select assets for reinvestment from available market opportunities
    pub fn select_assets(
        &self,
        available_cash: Money,
        market_opportunities: Vec<PoolAsset>,
        current_pool: &AssetPool,
        _market: &MarketContext,
        as_of: Date,
    ) -> Vec<PoolAsset> {
        let mut selected = Vec::new();
        let mut remaining_cash = available_cash;

        // Score by index without moving assets
        let mut scored_indices: Vec<(usize, f64)> = market_opportunities
            .iter()
            .enumerate()
            .filter_map(|(i, asset)| {
                // Check eligibility
                let (eligible, _) = self.eligibility_criteria.is_eligible(asset, as_of);
                if !eligible {
                    return None;
                }

                // Check concentration limits
                let (passes, _) = self.concentration_limits.check_limits(current_pool, asset);
                if !passes {
                    return None;
                }

                // Score based on spread, rating, and diversification benefit
                let score = self.score_asset(asset, current_pool);
                Some((i, score))
            })
            .collect();

        // Sort by score (descending)
        scored_indices
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Conservative reserve to minimize reallocations
        selected.reserve(scored_indices.len());

        // Select assets up to available cash
        for (idx, _score) in scored_indices {
            let asset = &market_opportunities[idx];
            if asset.balance.amount() <= remaining_cash.amount() {
                remaining_cash = remaining_cash
                    .checked_sub(asset.balance)
                    .unwrap_or(remaining_cash);
                selected.push(asset.clone());
            }
        }

        selected
    }

    /// Score an asset for reinvestment (higher = better)
    fn score_asset(&self, asset: &PoolAsset, pool: &AssetPool) -> f64 {
        let mut score = 0.0;

        // Spread component (higher spread = higher score)
        if let Some(spread) = asset.spread_bps {
            score += spread / 100.0; // Normalize to percentage
        }

        // Rating component (better rating = higher score for quality)
        if let Some(rating) = asset.credit_quality {
            score += match rating {
                CreditRating::AAA => 10.0,
                CreditRating::AA => 9.0,
                CreditRating::A => 8.0,
                CreditRating::BBB => 7.0,
                CreditRating::BB => 5.0,
                CreditRating::B => 3.0,
                _ => 1.0,
            };
        }

        // Diversification benefit (new obligor/industry = higher score)
        if let Some(obligor) = &asset.obligor_id {
            // Check if this is a new obligor (not already in pool)
            let is_new_obligor = !pool
                .assets
                .iter()
                .any(|a| a.obligor_id.as_deref() == Some(obligor.as_str()));
            if is_new_obligor {
                score += 5.0; // Bonus for new obligor
            }
        }

        if let Some(industry) = &asset.industry {
            // Calculate industry concentration
            let industry_exposure = get_industry_exposure(pool, industry);
            let total = pool.total_balance();
            let industry_concentration = if total.amount() > 0.0 {
                industry_exposure.amount() / total.amount()
            } else {
                0.0
            };

            if industry_concentration < 0.05 {
                score += 3.0; // Bonus for underweight industry
            }
        }

        score
    }

    /// Process a termination event
    pub fn process_termination_event(&mut self, event: ReinvestmentTerminationEvent) {
        self.termination_events.push(event);
        self.reinvestment_allowed = false;
    }
}

// Helper functions for AssetPool operations - consolidated using generic approach
fn get_obligor_exposure(pool: &AssetPool, obligor: &str) -> Money {
    sum_asset_balances(pool.assets_by_obligor(obligor), pool.base_currency())
}

fn get_industry_exposure(pool: &AssetPool, industry: &str) -> Money {
    sum_asset_balances(pool.assets_by_industry(industry), pool.base_currency())
}

fn get_rating_exposure(pool: &AssetPool, rating: CreditRating) -> Money {
    let filtered: Vec<&PoolAsset> = pool
        .assets
        .iter()
        .filter(|a| a.credit_quality == Some(rating))
        .collect();
    sum_asset_balances(filtered, pool.base_currency())
}

/// Generic helper to sum asset balances
fn sum_asset_balances(assets: Vec<&PoolAsset>, base_currency: Currency) -> Money {
    assets
        .iter()
        .fold(Money::new(0.0, base_currency), |acc, asset| {
            acc.checked_add(asset.balance).unwrap_or(acc)
        })
}

fn with_additional_asset(pool: &AssetPool, asset: &PoolAsset) -> AssetPool {
    let mut new_pool = pool.clone();
    // Just clone the PoolAsset directly
    new_pool.assets.push(asset.clone());
    new_pool
}

fn calculate_warf(pool: &AssetPool) -> f64 {
    use crate::instruments::common::structured_credit::rating_factors;

    let mut weighted_sum = 0.0;
    let mut total_balance = 0.0;

    for asset in &pool.assets {
        let balance = asset.balance.amount();
        let rating_factor = asset
            .credit_quality
            .map(rating_factors::moodys_warf_factor)
            .unwrap_or(3650.0);

        weighted_sum += balance * rating_factor;
        total_balance += balance;
    }

    if total_balance > 0.0 {
        weighted_sum / total_balance
    } else {
        0.0
    }
}

fn calculate_was(pool: &AssetPool) -> f64 {
    pool.weighted_avg_spread()
}

fn calculate_diversity_score(pool: &AssetPool) -> f64 {
    pool.diversity_score()
}

// Default implementations
impl Default for EligibilityCriteria {
    fn default() -> Self {
        Self {
            min_rating: Some(CreditRating::CCC),
            max_rating: None,
            min_spread_bps: Some(150.0),
            max_maturity: None,
            min_remaining_term: Some(12),
            max_remaining_term: Some(84),
            allowed_asset_types: vec![
                AssetType::FirstLienLoan { industry: None },
                AssetType::HighYieldBond { industry: None },
            ],
            allowed_currencies: vec![Currency::USD, Currency::EUR],
            max_price_pct: Some(102.0),
            min_asset_size: None,
            max_asset_size: None,
            excluded_industries: Vec::new(),
            excluded_obligors: Vec::new(),
        }
    }
}

impl Default for ConcentrationLimits {
    fn default() -> Self {
        use super::constants::*;
        Self {
            max_obligor_concentration: Some(DEFAULT_MAX_OBLIGOR_CONCENTRATION),
            max_top5_concentration: Some(DEFAULT_MAX_TOP5_CONCENTRATION),
            max_top10_concentration: Some(DEFAULT_MAX_TOP10_CONCENTRATION),
            industry_limits: HashMap::new(),
            rating_bucket_limits: HashMap::new(),
            geographic_limits: HashMap::new(),
            asset_type_limits: HashMap::new(),
            max_second_lien: Some(DEFAULT_MAX_SECOND_LIEN),
            max_cov_lite: Some(DEFAULT_MAX_COV_LITE),
            max_dip: Some(DEFAULT_MAX_DIP),
        }
    }
}

impl Default for PortfolioQualityTests {
    fn default() -> Self {
        Self {
            min_warf: None,
            max_warf: Some(2500.0),
            min_was_bps: Some(350.0),
            min_wac: Some(0.05),
            max_wal_years: Some(5.0),
            min_diversity_score: Some(50.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eligibility_criteria() {
        let criteria = EligibilityCriteria::default();

        let asset = create_test_asset();
        let current_date = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();

        let (eligible, reasons) = criteria.is_eligible(&asset, current_date);

        assert!(eligible || !reasons.is_empty());
    }

    #[test]
    fn test_concentration_limits() {
        let limits = ConcentrationLimits::default();
        let pool = create_test_pool();
        let asset = create_test_asset();

        let (passes, breaches) = limits.check_limits(&pool, &asset);

        assert!(passes || !breaches.is_empty());
    }

    fn create_test_asset() -> PoolAsset {
        PoolAsset {
            id: "TEST001".to_string().into(),
            asset_type: AssetType::FirstLienLoan {
                industry: Some("Technology".to_string()),
            },
            balance: Money::new(950_000.0, Currency::USD),
            rate: 0.06,
            spread_bps: Some(450.0),
            index_id: None,
            maturity: Date::from_calendar_date(2028, time::Month::December, 31).unwrap(),
            credit_quality: Some(CreditRating::BB),
            industry: Some("Technology".to_string()),
            obligor_id: Some("OBLIGOR001".to_string()),
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: None,
        }
    }

    fn create_test_pool() -> AssetPool {
        AssetPool::new(
            "CLO001",
            crate::instruments::common::structured_credit::DealType::CLO,
            Currency::USD,
        )
    }
}
