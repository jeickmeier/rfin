//! Asset pool structures for structured credit instruments.

use crate::instruments::bond::Bond;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::enums::{AssetType, CreditRating, DealType};
use super::reinvestment::{ConcentrationLimits, EligibilityCriteria};

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
    /// Current interest rate (all-in coupon)
    pub rate: f64,
    /// Spread over index in basis points (for floating rate assets)
    /// For WAS calculation: use this field, not the all-in rate
    pub spread_bps: Option<f64>,
    /// Reference index for floating rate (e.g., "SOFR-3M", "LIBOR-3M")
    pub index_id: Option<String>,
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
    /// Create new pool asset from existing bond
    pub fn from_bond(bond: &Bond, industry: Option<String>) -> Self {
        Self {
            id: bond.id.to_owned(),
            asset_type: AssetType::HighYieldBond {
                industry: industry.clone(),
            },
            balance: bond.notional,
            rate: bond.coupon,
            spread_bps: None, // Bond doesn't track spread separately
            index_id: None,
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

    /// Create a floating rate loan asset with explicit spread tracking
    ///
    /// This helper ensures spread_bps is properly populated for WAS calculations.
    ///
    /// # Arguments
    /// * `id` - Unique asset identifier
    /// * `balance` - Current outstanding balance
    /// * `index_id` - Reference rate (e.g., "SOFR-3M", "LIBOR-3M")
    /// * `spread_bps` - Spread over index in basis points
    /// * `maturity` - Maturity date
    ///
    /// # Example
    /// ```ignore
    /// let asset = PoolAsset::floating_rate_loan(
    ///     "LOAN001",
    ///     Money::new(10_000_000.0, Currency::USD),
    ///     "SOFR-3M",
    ///     450.0,  // 450bps spread
    ///     maturity_date,
    /// );
    /// // asset.rate will be 0.0 initially (set after index fixings)
    /// // asset.spread_bps will be Some(450.0) for WAS calculation
    /// ```
    pub fn floating_rate_loan(
        id: impl Into<InstrumentId>,
        balance: Money,
        index_id: impl Into<String>,
        spread_bps: f64,
        maturity: Date,
    ) -> Self {
        Self {
            id: id.into(),
            asset_type: AssetType::FirstLienLoan { industry: None },
            balance,
            rate: spread_bps / super::constants::BASIS_POINTS_DIVISOR, // Initialize with spread only
            spread_bps: Some(spread_bps),
            index_id: Some(index_id.into()),
            maturity,
            credit_quality: None,
            industry: None,
            obligor_id: None,
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: None,
        }
    }

    /// Create a fixed rate bond asset
    ///
    /// For fixed rate assets, spread_bps is None (WAS falls back to rate).
    pub fn fixed_rate_bond(
        id: impl Into<InstrumentId>,
        balance: Money,
        rate: f64,
        maturity: Date,
    ) -> Self {
        Self {
            id: id.into(),
            asset_type: AssetType::HighYieldBond { industry: None },
            balance,
            rate,
            spread_bps: None, // Fixed rate - no separate spread
            index_id: None,
            maturity,
            credit_quality: None,
            industry: None,
            obligor_id: None,
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: None,
            acquisition_date: None,
        }
    }

    /// Set credit quality
    pub fn with_rating(mut self, rating: CreditRating) -> Self {
        self.credit_quality = Some(rating);
        self
    }

    /// Set industry classification
    pub fn with_industry(mut self, industry: impl Into<String>) -> Self {
        self.industry = Some(industry.into());
        self
    }

    /// Set obligor identifier
    pub fn with_obligor(mut self, obligor_id: impl Into<String>) -> Self {
        self.obligor_id = Some(obligor_id.into());
        self
    }

    /// Current yield of the asset
    pub fn current_yield(&self) -> f64 {
        self.rate
    }

    /// Get spread component in basis points
    ///
    /// Returns the explicit spread if available, otherwise derives from rate.
    pub fn spread_bps(&self) -> f64 {
        self.spread_bps
            .unwrap_or(self.rate * super::constants::BASIS_POINTS_DIVISOR)
    }

    /// Remaining term to maturity in years
    pub fn remaining_term(&self, as_of: Date, day_count: DayCount) -> finstack_core::Result<f64> {
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
    pub max_price: f64,
    /// Minimum yield requirement
    pub min_yield: f64,
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
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PoolStats {
    /// Weighted average coupon
    pub weighted_avg_coupon: f64,
    /// Weighted average spread
    pub weighted_avg_spread: f64,
    /// Weighted average life (approximation using WAM)
    /// For accurate WAL, use weighted_avg_life_from_cashflows()
    pub weighted_avg_life: f64,
    /// Weighted average rating factor
    pub weighted_avg_rating_factor: f64,
    /// Diversity score (Moody's methodology)
    pub diversity_score: f64,
    /// Number of obligors
    pub num_obligors: usize,
    /// Number of industries
    pub num_industries: usize,
    /// Cumulative default rate
    pub cumulative_default_rate: f64,
    /// Recovery rate on defaults
    pub recovery_rate: f64,
    /// Prepayment rate (annualized)
    pub prepayment_rate: f64,
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
            eligibility_criteria: super::reinvestment::EligibilityCriteria::default(),
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
    pub fn weighted_avg_coupon(&self) -> f64 {
        let total_balance = self.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        let weighted_sum = self
            .assets
            .iter()
            .map(|a| a.rate * a.balance.amount())
            .sum::<f64>();

        weighted_sum / total_balance
    }

    /// Calculate weighted average maturity (WAM)
    ///
    /// This calculates the balance-weighted average time to maturity.
    /// Note: This is NOT the same as Weighted Average Life (WAL).
    /// WAL requires cashflow schedules and is calculated from principal payments.
    pub fn weighted_avg_maturity(&self, as_of: Date) -> f64 {
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
            .sum::<f64>();

        weighted_sum / total_balance
    }

    /// Calculate true weighted average life from cashflow schedule
    ///
    /// This is the market-standard calculation that should be used when
    /// full cashflow schedules are available.
    pub fn weighted_avg_life_from_cashflows(
        &self,
        cashflows: &[(Date, Money)],
        as_of: Date,
    ) -> f64 {
        let mut wal_numerator = 0.0;
        let mut total_principal = 0.0;

        for (date, amount) in cashflows {
            if *date > as_of {
                let years = (*date - as_of).whole_days() as f64 / super::constants::DAYS_PER_YEAR;
                wal_numerator += amount.amount() * years;
                total_principal += amount.amount();
            }
        }

        if total_principal > 0.0 {
            wal_numerator / total_principal
        } else {
            0.0
        }
    }

    /// Calculate diversity score (simplified Moody's approach)
    pub fn diversity_score(&self) -> f64 {
        let mut obligor_balances: HashMap<String, f64> = HashMap::new();
        let total_balance = self.total_balance().amount();

        if total_balance == 0.0 {
            return 0.0;
        }

        // Group by obligor
        for asset in &self.assets {
            if let Some(ref obligor) = asset.obligor_id {
                *obligor_balances.entry(obligor.to_owned()).or_insert(0.0) += asset.balance.amount();
            }
        }

        // Calculate diversity score = (sum of balances)^2 / sum of (balance^2)
        let sum_balances: f64 = obligor_balances.values().sum();
        let sum_squares: f64 = obligor_balances.values().map(|b| b * b).sum();

        if sum_squares > 0.0 {
            (sum_balances * sum_balances) / sum_squares
        } else {
            0.0
        }
    }

    /// Check eligibility of an asset for the pool
    pub fn is_eligible(&self, asset: &PoolAsset, as_of: Date) -> bool {
        let (eligible, _reasons) = self.eligibility_criteria.is_eligible(asset, as_of);
        eligible
    }

    /// Check concentration limits compliance
    pub fn check_concentration_limits(&self) -> ConcentrationCheckResult {
        let mut violations = Vec::new();
        let total_balance = self.total_balance().amount();

        if total_balance == 0.0 {
            return ConcentrationCheckResult { violations };
        }

        // Check obligor concentration
        if let Some(max_obligor) = self.concentration_limits.max_obligor_concentration {
            let mut obligor_concentrations: HashMap<String, f64> = HashMap::new();
            for asset in &self.assets {
                if let Some(ref obligor) = asset.obligor_id {
                    *obligor_concentrations.entry(obligor.to_owned()).or_insert(0.0) +=
                        asset.balance.amount() / total_balance;
                }
            }

            for (obligor, concentration) in &obligor_concentrations {
                if *concentration > max_obligor {
                    violations.push(ConcentrationViolation {
                        violation_type: "obligor_concentration".to_string(),
                        identifier: obligor.clone(),
                        current_level: *concentration * 100.0,
                        limit: max_obligor * 100.0,
                    });
                }
            }
        }

        // Check industry concentration using industry_limits
        let mut industry_concentrations: HashMap<String, f64> = HashMap::new();
        for asset in &self.assets {
            if let Some(ref industry) = asset.industry {
                *industry_concentrations
                    .entry(industry.to_owned())
                    .or_insert(0.0) += asset.balance.amount() / total_balance;
            }
        }

        for (industry, concentration) in &industry_concentrations {
            if let Some(&limit) = self.concentration_limits.industry_limits.get(industry) {
                if *concentration > limit {
                    violations.push(ConcentrationViolation {
                        violation_type: "industry_concentration".to_string(),
                        identifier: industry.clone(),
                        current_level: *concentration * 100.0,
                        limit: limit * 100.0,
                    });
                }
            }
        }

        // Check rating bucket concentration
        for rating in [CreditRating::CCC, CreditRating::CC, CreditRating::C] {
            if let Some(&limit) = self.concentration_limits.rating_bucket_limits.get(&rating) {
                let rating_balance: f64 = self
                    .assets
                    .iter()
                    .filter(|a| a.credit_quality == Some(rating))
                    .map(|a| a.balance.amount())
                    .sum();
                let rating_concentration = rating_balance / total_balance;

                if rating_concentration > limit {
                    violations.push(ConcentrationViolation {
                        violation_type: "rating_concentration".to_string(),
                        identifier: format!("{:?}", rating),
                        current_level: rating_concentration * 100.0,
                        limit: limit * 100.0,
                    });
                }
            }
        }

        ConcentrationCheckResult { violations }
    }

    /// Update pool statistics
    pub fn update_stats(&mut self, as_of: Date) {
        self.stats.weighted_avg_coupon = self.weighted_avg_coupon();
        // Note: Using WAM as approximation for WAL in stats
        // For true WAL, use weighted_avg_life_from_cashflows with actual cashflows
        self.stats.weighted_avg_life = self.weighted_avg_maturity(as_of);
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
        let defaulted_balance: f64 = self
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

    /// Calculate weighted average spread (WAS) in basis points
    ///
    /// Market standard: uses spread component only for floating rate assets.
    pub fn weighted_avg_spread(&self) -> f64 {
        let total_balance = self.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        let weighted_spread = self
            .assets
            .iter()
            .map(|a| a.spread_bps() * a.balance.amount())
            .sum::<f64>();

        weighted_spread / total_balance
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
    pub current_level: f64,
    pub limit: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_pool_creation() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);
        assert_eq!(pool.id.as_str(), "TEST_POOL");
        assert_eq!(pool.deal_type, DealType::CLO);
        assert_eq!(pool.base_currency(), Currency::USD);
    }
}
