//! Utility modules for structured credit instruments.
//!
//! This module combines rating factors and reinvestment management
//! into a unified utility interface.

use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use std::cmp::Ordering;
use std::sync::OnceLock;

use super::components::{AssetPool, PoolAsset};
use super::components::enums::CreditRating;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// RATING FACTORS
// ============================================================================

/// Rating factor table for a specific rating agency methodology
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RatingFactorTable {
    /// Factors by rating
    factors: std::collections::HashMap<CreditRating, f64>,
    /// Agency name
    agency: String,
    /// Methodology description
    methodology: String,
}

impl RatingFactorTable {
    /// Create Moody's standard WARF table
    ///
    /// Based on Moody's IDEALIZED DEFAULT RATES table used in CLO analysis.
    /// These factors represent the expected cumulative default rate over the
    /// life of the asset for each rating category.
    ///
    /// Source: Moody's "Approach to Rating Collateralized Loan Obligations"
    pub fn moodys_standard() -> Self {
        let mut factors = std::collections::HashMap::new();
        factors.insert(CreditRating::AAA, 1.0);
        factors.insert(CreditRating::AA, 10.0);
        factors.insert(CreditRating::A, 40.0);
        factors.insert(CreditRating::BBB, 260.0);
        factors.insert(CreditRating::BB, 1350.0);
        factors.insert(CreditRating::B, 2720.0);
        factors.insert(CreditRating::CCC, 6500.0);
        factors.insert(CreditRating::CC, 8070.0);
        factors.insert(CreditRating::C, 10000.0);
        factors.insert(CreditRating::D, 10000.0);
        factors.insert(CreditRating::NR, 3650.0); // B-/CCC+ equivalent

        Self {
            factors,
            agency: "Moody's".to_string(),
            methodology: "IDEALIZED DEFAULT RATES".to_string(),
        }
    }

    /// Get factor for a specific rating
    pub fn get_factor(&self, rating: CreditRating) -> f64 {
        self.factors.get(&rating).copied().unwrap_or(3650.0)
    }

    /// Get agency name
    pub fn agency(&self) -> &str {
        &self.agency
    }

    /// Get methodology description
    pub fn methodology(&self) -> &str {
        &self.methodology
    }
}

/// Lazily initialized Moody's WARF rating factor table
static MOODYS_WARF_TABLE: OnceLock<RatingFactorTable> = OnceLock::new();

/// Get Moody's WARF factor for a rating (convenience function)
///
/// This is the standard function that should be used throughout the codebase
/// for consistent WARF calculations.
pub fn moodys_warf_factor(rating: CreditRating) -> f64 {
    MOODYS_WARF_TABLE
        .get_or_init(RatingFactorTable::moodys_standard)
        .get_factor(rating)
}

// ============================================================================
// REINVESTMENT MANAGEMENT
// ============================================================================

/// Manages reinvestment during the reinvestment period (price-only selection)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ReinvestmentManager {
    /// End date of reinvestment period
    pub end_date: Date,
    /// Whether reinvestment is currently allowed
    pub reinvestment_allowed: bool,
}

/// Stub eligibility criteria (kept for backward compatibility in pool.rs)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EligibilityCriteria;

/// Stub concentration limits (kept for backward compatibility in pool.rs)
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ConcentrationLimits;

impl ReinvestmentManager {
    /// Create new reinvestment manager
    pub fn new(end_date: Date) -> Self {
        Self {
            end_date,
            reinvestment_allowed: true,
        }
    }

    /// Check if reinvestment is allowed at a given date
    pub fn can_reinvest(&self, as_of: Date) -> bool {
        as_of < self.end_date && self.reinvestment_allowed
    }

    /// Select assets for reinvestment from available market opportunities
    pub fn select_assets(
        &self,
        available_cash: Money,
        market_opportunities: Vec<PoolAsset>,
        _current_pool: &AssetPool,
        _market: &MarketContext,
        _as_of: Date,
    ) -> Vec<PoolAsset> {
        let mut remaining_cash = available_cash;

        // Compute price-per-par for each asset; default to 100% if unknown
        let mut indices: Vec<usize> = (0..market_opportunities.len()).collect();
        indices.sort_unstable_by(|&i, &j| {
            let pi = price_per_par(&market_opportunities[i]);
            let pj = price_per_par(&market_opportunities[j]);
            pi.partial_cmp(&pj).unwrap_or(Ordering::Equal)
        });

        let mut selected = Vec::new();
        for idx in indices {
            let asset = &market_opportunities[idx];
            let price_pct = price_per_par(asset);
            let cost_amount = asset.balance.amount() * price_pct;
            if cost_amount <= remaining_cash.amount() {
                // Reduce cash and take the whole asset
                let cost = Money::new(cost_amount, remaining_cash.currency());
                remaining_cash = remaining_cash
                    .checked_sub(cost)
                    .unwrap_or(remaining_cash);
                selected.push(asset.clone());
            }
        }

        selected
    }
}

#[inline]
fn price_per_par(asset: &PoolAsset) -> f64 {
    // If purchase price provided, use price as a percentage of par
    if let Some(price) = asset.purchase_price {
        if asset.balance.amount() > 0.0 {
            return (price.amount() / asset.balance.amount()).max(0.0);
        }
    }
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    // Rating factors tests
    #[test]
    fn test_moodys_warf_factors() {
        let table = RatingFactorTable::moodys_standard();

        // Verify key rating factors
        assert_eq!(table.get_factor(CreditRating::AAA), 1.0);
        assert_eq!(table.get_factor(CreditRating::A), 40.0);
        assert_eq!(table.get_factor(CreditRating::B), 2720.0);
        assert_eq!(table.get_factor(CreditRating::CCC), 6500.0);
    }

    #[test]
    fn test_convenience_function_matches_table() {
        let table = RatingFactorTable::moodys_standard();

        // Verify convenience function matches table
        assert_eq!(
            moodys_warf_factor(CreditRating::AAA),
            table.get_factor(CreditRating::AAA)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::A),
            table.get_factor(CreditRating::A)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::BBB),
            table.get_factor(CreditRating::BBB)
        );
        assert_eq!(
            moodys_warf_factor(CreditRating::B),
            table.get_factor(CreditRating::B)
        );
    }

    // Reinvestment tests
    #[test]
    fn test_selects_cheapest_assets_first() {
        use super::super::components::{DealType, AssetType};
        
        let mgr = ReinvestmentManager::new(Date::from_calendar_date(2026, time::Month::January, 1).unwrap());
        let pool = AssetPool::new("POOL", DealType::CLO, Currency::USD);

        let a = PoolAsset {
            id: "A".to_string().into(),
            asset_type: AssetType::HighYieldBond { industry: None },
            balance: Money::new(100.0, Currency::USD),
            rate: 0.0,
            spread_bps: None,
            index_id: None,
            maturity: Date::from_calendar_date(2030, time::Month::January, 1).unwrap(),
            credit_quality: None,
            industry: None,
            obligor_id: None,
            is_defaulted: false,
            recovery_amount: None,
            purchase_price: Some(Money::new(95.0, Currency::USD)),
            acquisition_date: None,
        };

        let b = PoolAsset { purchase_price: Some(Money::new(98.0, Currency::USD)), ..a.clone() };
        let c = PoolAsset { purchase_price: Some(Money::new(102.0, Currency::USD)), ..a.clone() };

        let cash = Money::new(195.0, Currency::USD);
        let selected = mgr.select_assets(cash, vec![b.clone(), a.clone(), c.clone()], &pool, &MarketContext::default(), Date::from_calendar_date(2025, time::Month::January, 1).unwrap());

        // Expect picks at 95 and 98 first (total 193) and skips 102 due to budget
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|x| x.purchase_price.unwrap().amount() == 95.0));
        assert!(selected.iter().any(|x| x.purchase_price.unwrap().amount() == 98.0));
    }
}

