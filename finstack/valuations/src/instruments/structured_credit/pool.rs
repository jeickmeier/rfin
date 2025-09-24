//! Asset pool structures for structured credit instruments.

use crate::instruments::bond::Bond;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::F;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::types::{AssetType, CreditRating, DealType};

/// Individual asset in the structured credit pool
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PoolAsset {
    /// Unique asset identifier
    pub id: InstrumentId,
    /// Asset classification
    pub asset_type: AssetType,
    /// Current outstanding balance
    pub balance: Money,
    /// Current interest rate
    pub rate: F,
    /// Maturity date
    pub maturity: Date,
    /// Credit quality
    pub credit_quality: Option<CreditRating>,
    /// Industry classification
    pub industry: Option<String>,
    /// Obligor/borrower identifier
    pub obligor_id: Option<String>,
    /// Default status
    pub is_defaulted: bool,
    /// Recovery amount (if defaulted)
    pub recovery_amount: Option<Money>,
    /// Purchase price (for trading gain/loss)
    pub purchase_price: Option<Money>,
    /// Acquisition date
    pub acquisition_date: Option<Date>,
}

impl PoolAsset {
    // Loan support removed from library

    /// Create new pool asset from existing bond
    pub fn from_bond(bond: &Bond, industry: Option<String>) -> Self {
        Self {
            id: bond.id.clone(),
            asset_type: AssetType::Bond {
                bond_type: super::types::BondType::HighYield, // Default assumption
                industry: industry.clone(),
            },
            balance: bond.notional,
            rate: bond.coupon,
            maturity: bond.maturity,
            credit_quality: None,
            industry,
            obligor_id: None,
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: bond
                .pricing_overrides
                .quoted_clean_price
                .map(|p| bond.notional * p),
            acquisition_date: Some(bond.issue),
        }
    }

    /// Current yield of the asset
    pub fn current_yield(&self) -> F {
        self.rate
    }

    /// Remaining term to maturity in years
    pub fn remaining_term(&self, as_of: Date, day_count: DayCount) -> finstack_core::Result<F> {
        day_count.year_fraction(
            as_of,
            self.maturity,
            finstack_core::dates::DayCountCtx::default(),
        )
    }

    /// Mark asset as defaulted with recovery
    pub fn default_with_recovery(&mut self, recovery_amount: Money, _default_date: Date) {
        self.is_defaulted = true;
        self.recovery_amount = Some(recovery_amount);
        // Could store default_date in additional field if needed
    }
}

/// Eligibility criteria for pool assets
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EligibilityCriteria {
    /// Minimum credit rating
    pub min_credit_rating: Option<CreditRating>,
    /// Maximum maturity date
    pub max_maturity: Option<Date>,
    /// Eligible currencies
    pub eligible_currencies: Vec<Currency>,
    /// Excluded industries
    pub excluded_industries: Vec<String>,
    /// Minimum spread over benchmark
    pub min_spread_bp: Option<F>,
    /// Maximum asset size
    pub max_asset_size: Option<Money>,
    /// Minimum asset size
    pub min_asset_size: Option<Money>,
}

impl Default for EligibilityCriteria {
    fn default() -> Self {
        Self {
            min_credit_rating: Some(CreditRating::CCC),
            max_maturity: None,
            eligible_currencies: vec![Currency::USD],
            excluded_industries: Vec::new(),
            min_spread_bp: Some(100.0), // 1% minimum spread
            max_asset_size: None,
            min_asset_size: None,
        }
    }
}

/// Concentration limits for risk management
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConcentrationLimits {
    /// Maximum percentage of pool from single obligor
    pub max_obligor_concentration: F,
    /// Maximum percentage from single industry
    pub max_industry_concentration: F,
    /// Maximum percentage of CCC-rated assets
    pub max_ccc_assets: F,
    /// Minimum diversity score
    pub min_diversity_score: Option<F>,
    /// Maximum weighted average life
    pub max_weighted_avg_life: Option<F>,
    /// Maximum single asset size
    pub max_single_asset_pct: F,
}

impl Default for ConcentrationLimits {
    fn default() -> Self {
        Self {
            max_obligor_concentration: 2.0,   // 2%
            max_industry_concentration: 15.0, // 15%
            max_ccc_assets: 7.5,              // 7.5%
            min_diversity_score: Some(30.0),  // Moody's diversity score
            max_weighted_avg_life: Some(7.0), // 7 years
            max_single_asset_pct: 1.0,        // 1%
        }
    }
}

/// Reinvestment period and rules
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReinvestmentPeriod {
    /// End date of reinvestment period
    pub end_date: Date,
    /// Whether reinvestment is currently active
    pub is_active: bool,
    /// Criteria for new investments
    pub criteria: ReinvestmentCriteria,
}

/// Criteria for reinvestment during revolving period
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReinvestmentCriteria {
    /// Maximum purchase price (% of par)
    pub max_price: F,
    /// Minimum yield requirement
    pub min_yield: F,
    /// Must maintain credit quality distribution
    pub maintain_credit_quality: bool,
    /// Must maintain weighted average life
    pub maintain_wal: bool,
    /// Must satisfy eligibility criteria
    pub apply_eligibility_criteria: bool,
}

impl Default for ReinvestmentCriteria {
    fn default() -> Self {
        Self {
            max_price: 100.0, // 100% of par
            min_yield: 0.0,
            maintain_credit_quality: true,
            maintain_wal: true,
            apply_eligibility_criteria: true,
        }
    }
}

/// Pool-level performance statistics
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PoolStats {
    /// Weighted average coupon
    pub weighted_avg_coupon: F,
    /// Weighted average spread
    pub weighted_avg_spread: F,
    /// Weighted average life
    pub weighted_avg_life: F,
    /// Weighted average rating factor
    pub weighted_avg_rating_factor: F,
    /// Diversity score (Moody's methodology)
    pub diversity_score: F,
    /// Number of obligors
    pub num_obligors: usize,
    /// Number of industries
    pub num_industries: usize,
    /// Cumulative default rate
    pub cumulative_default_rate: F,
    /// Recovery rate on defaults
    pub recovery_rate: F,
    /// Prepayment rate (annualized)
    pub prepayment_rate: F,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            weighted_avg_coupon: 0.0,
            weighted_avg_spread: 0.0,
            weighted_avg_life: 0.0,
            weighted_avg_rating_factor: 0.0,
            diversity_score: 0.0,
            num_obligors: 0,
            num_industries: 0,
            cumulative_default_rate: 0.0,
            recovery_rate: 0.4, // 40% default
            prepayment_rate: 0.0,
        }
    }
}

/// Main asset pool structure
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AssetPool {
    /// Pool identifier
    pub id: InstrumentId,

    /// Deal type classification
    pub deal_type: DealType,

    /// Underlying assets
    pub assets: Vec<PoolAsset>,

    /// Pool governance
    pub eligibility_criteria: EligibilityCriteria,
    pub concentration_limits: ConcentrationLimits,

    /// Performance tracking
    pub cumulative_defaults: Money,
    pub cumulative_recoveries: Money,
    pub cumulative_prepayments: Money,

    /// Reinvestment management
    pub reinvestment_period: Option<ReinvestmentPeriod>,

    /// Pool-level accounts
    pub collection_account: Money,
    pub reserve_account: Money,
    pub excess_spread_account: Money,

    /// Cached statistics (updated periodically)
    pub stats: PoolStats,
}

impl AssetPool {
    /// Create new asset pool
    pub fn new(id: impl Into<InstrumentId>, deal_type: DealType, base_currency: Currency) -> Self {
        let zero_money = Money::new(0.0, base_currency);
        Self {
            id: id.into(),
            deal_type,
            assets: Vec::new(),
            eligibility_criteria: EligibilityCriteria::default(),
            concentration_limits: ConcentrationLimits::default(),
            cumulative_defaults: zero_money,
            cumulative_recoveries: zero_money,
            cumulative_prepayments: zero_money,
            reinvestment_period: None,
            collection_account: zero_money,
            reserve_account: zero_money,
            excess_spread_account: zero_money,
            stats: PoolStats::default(),
        }
    }

    // Loan add removed

    /// Add asset from existing bond
    pub fn add_bond(&mut self, bond: &Bond, industry: Option<String>) -> &mut Self {
        let asset = PoolAsset::from_bond(bond, industry);
        self.assets.push(asset);
        self
    }

    /// Total pool balance
    pub fn total_balance(&self) -> Money {
        self.assets
            .iter()
            .try_fold(Money::new(0.0, self.base_currency()), |acc, asset| {
                acc.checked_add(asset.balance)
            })
            .unwrap_or_else(|_| Money::new(0.0, self.base_currency()))
    }

    /// Total pool balance excluding defaulted assets
    pub fn performing_balance(&self) -> Money {
        self.assets
            .iter()
            .filter(|a| !a.is_defaulted)
            .try_fold(Money::new(0.0, self.base_currency()), |acc, asset| {
                acc.checked_add(asset.balance)
            })
            .unwrap_or_else(|_| Money::new(0.0, self.base_currency()))
    }

    /// Calculate weighted average coupon
    pub fn weighted_avg_coupon(&self) -> F {
        let total_balance = self.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        let weighted_sum = self
            .assets
            .iter()
            .map(|a| a.rate * a.balance.amount())
            .sum::<F>();

        weighted_sum / total_balance
    }

    /// Calculate weighted average life (simplified)
    pub fn weighted_avg_life(&self, as_of: Date) -> F {
        let total_balance = self.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        let weighted_sum = self
            .assets
            .iter()
            .filter_map(|a| {
                a.remaining_term(as_of, DayCount::Act365F)
                    .ok()
                    .map(|term| term * a.balance.amount())
            })
            .sum::<F>();

        weighted_sum / total_balance
    }

    /// Calculate diversity score (simplified Moody's approach)
    pub fn diversity_score(&self) -> F {
        let mut obligor_balances: HashMap<String, F> = HashMap::new();
        let total_balance = self.total_balance().amount();

        if total_balance == 0.0 {
            return 0.0;
        }

        // Group by obligor
        for asset in &self.assets {
            if let Some(ref obligor) = asset.obligor_id {
                *obligor_balances.entry(obligor.clone()).or_insert(0.0) += asset.balance.amount();
            }
        }

        // Calculate diversity score = (sum of balances)^2 / sum of (balance^2)
        let sum_balances: F = obligor_balances.values().sum();
        let sum_squares: F = obligor_balances.values().map(|b| b * b).sum();

        if sum_squares > 0.0 {
            (sum_balances * sum_balances) / sum_squares
        } else {
            0.0
        }
    }

    /// Check eligibility of an asset for the pool
    pub fn is_eligible(&self, asset: &PoolAsset, _as_of: Date) -> bool {
        let criteria = &self.eligibility_criteria;

        // Check credit rating
        if let (Some(min_rating), Some(asset_rating)) =
            (criteria.min_credit_rating, asset.credit_quality)
        {
            if asset_rating < min_rating {
                return false;
            }
        }

        // Check maturity
        if let Some(max_maturity) = criteria.max_maturity {
            if asset.maturity > max_maturity {
                return false;
            }
        }

        // Check currency
        if !criteria
            .eligible_currencies
            .contains(&asset.balance.currency())
        {
            return false;
        }

        // Check excluded industries
        if let Some(ref industry) = asset.industry {
            if criteria.excluded_industries.contains(industry) {
                return false;
            }
        }

        // Check minimum spread (simplified)
        if let Some(min_spread) = criteria.min_spread_bp {
            if asset.rate * 10_000.0 < min_spread {
                return false;
            }
        }

        true
    }

    /// Check concentration limits compliance
    pub fn check_concentration_limits(&self) -> ConcentrationCheckResult {
        let mut violations = Vec::new();
        let total_balance = self.total_balance().amount();

        if total_balance == 0.0 {
            return ConcentrationCheckResult { violations };
        }

        // Check obligor concentration
        let mut obligor_concentrations: HashMap<String, F> = HashMap::new();
        for asset in &self.assets {
            if let Some(ref obligor) = asset.obligor_id {
                *obligor_concentrations.entry(obligor.clone()).or_insert(0.0) +=
                    asset.balance.amount() / total_balance * 100.0;
            }
        }

        for (obligor, concentration) in &obligor_concentrations {
            if *concentration > self.concentration_limits.max_obligor_concentration {
                violations.push(ConcentrationViolation {
                    violation_type: "obligor_concentration".to_string(),
                    identifier: obligor.clone(),
                    current_level: *concentration,
                    limit: self.concentration_limits.max_obligor_concentration,
                });
            }
        }

        // Check industry concentration
        let mut industry_concentrations: HashMap<String, F> = HashMap::new();
        for asset in &self.assets {
            if let Some(ref industry) = asset.industry {
                *industry_concentrations
                    .entry(industry.clone())
                    .or_insert(0.0) += asset.balance.amount() / total_balance * 100.0;
            }
        }

        for (industry, concentration) in &industry_concentrations {
            if *concentration > self.concentration_limits.max_industry_concentration {
                violations.push(ConcentrationViolation {
                    violation_type: "industry_concentration".to_string(),
                    identifier: industry.clone(),
                    current_level: *concentration,
                    limit: self.concentration_limits.max_industry_concentration,
                });
            }
        }

        // Check CCC asset concentration
        let ccc_balance: F = self
            .assets
            .iter()
            .filter(|a| {
                matches!(
                    a.credit_quality,
                    Some(CreditRating::CCC | CreditRating::CC | CreditRating::C)
                )
            })
            .map(|a| a.balance.amount())
            .sum();
        let ccc_concentration = ccc_balance / total_balance * 100.0;

        if ccc_concentration > self.concentration_limits.max_ccc_assets {
            violations.push(ConcentrationViolation {
                violation_type: "ccc_concentration".to_string(),
                identifier: "CCC_BUCKET".to_string(),
                current_level: ccc_concentration,
                limit: self.concentration_limits.max_ccc_assets,
            });
        }

        ConcentrationCheckResult { violations }
    }

    /// Update pool statistics
    pub fn update_stats(&mut self, as_of: Date) {
        self.stats.weighted_avg_coupon = self.weighted_avg_coupon();
        self.stats.weighted_avg_life = self.weighted_avg_life(as_of);
        self.stats.diversity_score = self.diversity_score();

        // Count unique obligors and industries
        let mut obligors = std::collections::HashSet::new();
        let mut industries = std::collections::HashSet::new();

        for asset in &self.assets {
            if let Some(ref obligor) = asset.obligor_id {
                obligors.insert(obligor.clone());
            }
            if let Some(ref industry) = asset.industry {
                industries.insert(industry.clone());
            }
        }

        self.stats.num_obligors = obligors.len();
        self.stats.num_industries = industries.len();

        // Calculate default rate
        let defaulted_balance: F = self
            .assets
            .iter()
            .filter(|a| a.is_defaulted)
            .map(|a| a.balance.amount())
            .sum();
        self.stats.cumulative_default_rate =
            defaulted_balance / self.total_balance().amount() * 100.0;
    }

    /// Base currency of the pool (from first asset)
    pub fn base_currency(&self) -> Currency {
        self.assets
            .first()
            .map(|a| a.balance.currency())
            .unwrap_or(Currency::USD)
    }

    /// Get assets by industry
    pub fn assets_by_industry(&self, industry: &str) -> Vec<&PoolAsset> {
        self.assets
            .iter()
            .filter(|a| a.industry.as_deref() == Some(industry))
            .collect()
    }

    /// Get assets by obligor
    pub fn assets_by_obligor(&self, obligor_id: &str) -> Vec<&PoolAsset> {
        self.assets
            .iter()
            .filter(|a| a.obligor_id.as_deref() == Some(obligor_id))
            .collect()
    }
}

/// Result of concentration limit checking
#[derive(Debug, Clone)]
pub struct ConcentrationCheckResult {
    pub violations: Vec<ConcentrationViolation>,
}

impl ConcentrationCheckResult {
    /// Check if any limits are violated
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }
}

/// Individual concentration limit violation
#[derive(Debug, Clone)]
pub struct ConcentrationViolation {
    pub violation_type: String,
    pub identifier: String,
    pub current_level: F,
    pub limit: F,
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[allow(dead_code)]
    fn test_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    #[test]
    fn test_pool_creation() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
        assert_eq!(pool.id.as_str(), "TEST_POOL");
        assert_eq!(pool.deal_type, DealType::CLO);
        assert_eq!(pool.base_currency(), Currency::USD);
    }

    // #[test]
    // fn test_pool_stats_calculation() {
    //     let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

    //     // Add test assets
    //     let asset1 = PoolAsset {
    //         id: "ASSET1".to_string(),
    //         asset_type: AssetType::Loan {
    //             loan_type: LoanType::FirstLien,
    //             industry: Some("Technology".to_string()),
    //         },
    //         balance: Money::new(100_000.0, Currency::USD),
    //         rate: 0.08,
    //         maturity: test_date(),
    //         credit_quality: Some(CreditRating::B),
    //         industry: Some("Technology".to_string()),
    //         obligor_id: Some("OBLIGOR1".to_string()),
    //         is_defaulted: false,
    //         recovery_amount: None,
    //         purchase_price: None,
    //         acquisition_date: None,
    //     };

    //     pool.assets.push(asset1);
    //     pool.update_stats(test_date());

    //     assert_eq!(pool.stats.weighted_avg_coupon, 0.08);
    //     assert_eq!(pool.stats.num_obligors, 1);
    //     assert_eq!(pool.stats.num_industries, 1);
    // }

    // #[test]
    // fn test_concentration_limits() {
    //     let mut pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

    //     // Add asset that violates obligor concentration
    //     let large_asset = PoolAsset {
    //         id: "LARGE_ASSET".to_string(),
    //         asset_type: AssetType::Loan {
    //             loan_type: LoanType::FirstLien,
    //             industry: Some("Technology".to_string()),
    //         },
    //         balance: Money::new(1_000_000.0, Currency::USD), // Large asset
    //         rate: 0.08,
    //         maturity: test_date(),
    //         credit_quality: Some(CreditRating::B),
    //         industry: Some("Technology".to_string()),
    //         obligor_id: Some("BIG_OBLIGOR".to_string()),
    //         is_defaulted: false,
    //         recovery_amount: None,
    //         purchase_price: None,
    //         acquisition_date: None,
    //     };

    //     pool.assets.push(large_asset);

    //     // Set strict limit for testing
    //     pool.concentration_limits.max_obligor_concentration = 50.0; // 50%

    //     let result = pool.check_concentration_limits();
    //     assert!(result.has_violations());
    // }
}
