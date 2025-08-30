//! Policy implementations for private credit covenant and workout functionality.
//!
//! This module provides concrete policy implementations for:
//! - Grid margin policy with bucket-based spreads
//! - Index fallback policy for missing fixings
//! - DSCR sweep policies
//! - Integration with the global function registry

use crate::metrics::MetricContext;
use crate::policy::registry::FunctionParam;
use finstack_core::prelude::*;
use finstack_core::F;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Grid margin policy for determining credit spreads.
#[derive(Clone, Debug)]
pub struct GridMarginPolicy {
    /// Base spreads by rating bucket
    pub base_spreads: HashMap<String, F>,
    /// Sector adjustments
    pub sector_adjustments: HashMap<String, F>,
    /// Maturity adjustments (years -> bps)
    pub maturity_buckets: Vec<(F, F)>,
    /// Floor spread in bps
    pub floor_spread: F,
    /// Cap spread in bps
    pub cap_spread: F,
}

impl Default for GridMarginPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl GridMarginPolicy {
    /// Create a new grid margin policy.
    pub fn new() -> Self {
        let mut base_spreads = HashMap::new();
        base_spreads.insert("AAA".to_string(), 10.0);
        base_spreads.insert("AA".to_string(), 25.0);
        base_spreads.insert("A".to_string(), 50.0);
        base_spreads.insert("BBB".to_string(), 100.0);
        base_spreads.insert("BB".to_string(), 200.0);
        base_spreads.insert("B".to_string(), 400.0);
        base_spreads.insert("CCC".to_string(), 800.0);

        let mut sector_adjustments = HashMap::new();
        sector_adjustments.insert("Technology".to_string(), -25.0);
        sector_adjustments.insert("Energy".to_string(), 50.0);
        sector_adjustments.insert("Retail".to_string(), 25.0);
        sector_adjustments.insert("Healthcare".to_string(), -10.0);

        let maturity_buckets = vec![
            (1.0, 0.0),
            (3.0, 25.0),
            (5.0, 50.0),
            (7.0, 75.0),
            (10.0, 100.0),
        ];

        Self {
            base_spreads,
            sector_adjustments,
            maturity_buckets,
            floor_spread: 5.0,
            cap_spread: 1000.0,
        }
    }

    /// Calculate spread for given parameters.
    pub fn calculate_spread(&self, rating: &str, sector: Option<&str>, maturity_years: F) -> F {
        // Start with base spread
        let mut spread = self.base_spreads.get(rating).copied().unwrap_or(500.0);

        // Add sector adjustment
        if let Some(s) = sector {
            if let Some(adj) = self.sector_adjustments.get(s) {
                spread += adj;
            }
        }

        // Add maturity adjustment - find the highest applicable bucket
        let mut maturity_adj = 0.0;
        for &(years, adj) in &self.maturity_buckets {
            if maturity_years >= years {
                maturity_adj = adj;
            }
        }
        spread += maturity_adj;

        // Apply floor and cap
        spread.max(self.floor_spread).min(self.cap_spread)
    }
}

/// Index fallback policy for handling missing rate fixings.
#[derive(Clone, Debug)]
pub struct IndexFallbackPolicy {
    /// Fallback hierarchy (primary -> fallback index)
    pub fallback_chain: HashMap<String, Vec<String>>,
    /// Static fallback rates
    pub static_rates: HashMap<String, F>,
    /// Spread adjustments when using fallback
    pub fallback_spreads: HashMap<String, F>,
}

impl Default for IndexFallbackPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexFallbackPolicy {
    /// Create a new index fallback policy.
    pub fn new() -> Self {
        let mut fallback_chain = HashMap::new();
        fallback_chain.insert(
            "USD-LIBOR-3M".to_string(),
            vec!["USD-SOFR-3M".to_string(), "USD-OIS".to_string()],
        );
        fallback_chain.insert("GBP-LIBOR-3M".to_string(), vec!["GBP-SONIA-3M".to_string()]);

        let mut static_rates = HashMap::new();
        static_rates.insert("USD-LIBOR-3M".to_string(), 0.05);
        static_rates.insert("USD-SOFR-3M".to_string(), 0.045);
        static_rates.insert("GBP-LIBOR-3M".to_string(), 0.04);

        let mut fallback_spreads = HashMap::new();
        fallback_spreads.insert("USD-LIBOR-3M_to_USD-SOFR-3M".to_string(), 26.161); // ISDA spread
        fallback_spreads.insert("GBP-LIBOR-3M_to_GBP-SONIA-3M".to_string(), 27.66);

        Self {
            fallback_chain,
            static_rates,
            fallback_spreads,
        }
    }

    /// Get rate with fallback logic.
    pub fn get_rate(
        &self,
        index: &str,
        available_rates: &HashMap<String, F>,
    ) -> finstack_core::Result<F> {
        // Try primary index
        if let Some(&rate) = available_rates.get(index) {
            return Ok(rate);
        }

        // Try fallback chain
        if let Some(fallbacks) = self.fallback_chain.get(index) {
            for fallback in fallbacks {
                if let Some(&rate) = available_rates.get(fallback) {
                    // Apply fallback spread adjustment
                    let key = format!("{}_to_{}", index, fallback);
                    let spread = self.fallback_spreads.get(&key).copied().unwrap_or(0.0);
                    return Ok(rate + spread / 10000.0); // Convert bps to decimal
                }
            }
        }

        // Use static rate as last resort
        self.static_rates
            .get(index)
            .copied()
            .ok_or_else(|| finstack_core::error::InputError::NotFound.into())
    }
}

/// DSCR (Debt Service Coverage Ratio) sweep policy.
#[derive(Clone, Debug)]
pub struct DSCRSweepPolicy {
    /// DSCR thresholds and corresponding sweep percentages
    pub sweep_schedule: Vec<(F, F)>,
    /// Minimum DSCR before any sweep
    pub min_dscr: F,
    /// Maximum sweep percentage
    pub max_sweep: F,
}

impl Default for DSCRSweepPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl DSCRSweepPolicy {
    /// Create a new DSCR sweep policy.
    pub fn new() -> Self {
        let sweep_schedule = vec![
            (1.0, 0.0),  // DSCR >= 1.0: 0% sweep (baseline)
            (1.1, 0.25), // DSCR >= 1.1: 25% sweep
            (1.3, 0.50), // DSCR >= 1.3: 50% sweep
            (1.6, 0.75), // DSCR >= 1.6: 75% sweep
            (2.0, 1.0),  // DSCR >= 2.0: 100% sweep
        ];

        Self {
            sweep_schedule,
            min_dscr: 1.0,
            max_sweep: 1.0,
        }
    }

    /// Calculate sweep percentage based on DSCR.
    pub fn calculate_sweep(&self, dscr: F) -> F {
        if dscr < self.min_dscr {
            return 0.0;
        }

        // Find the highest threshold that DSCR meets or exceeds
        let mut sweep = 0.0;
        for &(threshold, pct) in &self.sweep_schedule {
            if dscr >= threshold {
                sweep = pct;
            }
        }

        sweep.min(self.max_sweep)
    }

    /// Calculate cash available for sweep.
    pub fn calculate_sweep_amount(
        &self,
        dscr: F,
        excess_cash: Money,
        minimum_cash: Money,
    ) -> Money {
        if excess_cash.currency() != minimum_cash.currency() {
            return Money::new(0.0, excess_cash.currency());
        }

        let available = (excess_cash.amount() - minimum_cash.amount()).max(0.0);
        let sweep_pct = self.calculate_sweep(dscr);

        Money::new(available * sweep_pct, excess_cash.currency())
    }
}

/// Parameters for grid margin policy function.
#[derive(Clone, Debug)]
pub struct GridMarginParams {
    pub rating: String,
    pub sector: Option<String>,
    pub maturity_years: F,
}

impl FunctionParam for GridMarginParams {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn FunctionParam> {
        Box::new(self.clone())
    }

    fn to_json(&self) -> finstack_core::Result<String> {
        Ok(format!(
            r#"{{"rating":"{}","sector":{},"maturity_years":{}}}"#,
            self.rating,
            self.sector
                .as_ref()
                .map(|s| format!(r#""{}""#, s))
                .unwrap_or("null".to_string()),
            self.maturity_years
        ))
    }
}

/// Parameters for DSCR sweep policy function.
#[derive(Clone, Debug)]
pub struct DSCRSweepParams {
    pub dscr: F,
    pub excess_cash: Money,
    pub minimum_cash: Money,
}

impl FunctionParam for DSCRSweepParams {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn FunctionParam> {
        Box::new(self.clone())
    }

    fn to_json(&self) -> finstack_core::Result<String> {
        Ok(format!(
            r#"{{"dscr":{},"excess_cash":{},"minimum_cash":{}}}"#,
            self.dscr,
            self.excess_cash.amount(),
            self.minimum_cash.amount()
        ))
    }
}

/// Custom metric calculator for DSCR.
pub fn calculate_dscr(context: &MetricContext) -> finstack_core::Result<F> {
    // Simplified DSCR calculation
    // In practice, would pull from statements/cashflows

    // Get EBITDA (simplified as base_value * margin)
    let ebitda = context.base_value.amount() * 0.3; // 30% EBITDA margin

    // Get debt service (simplified as 10% of base value)
    let debt_service = context.base_value.amount() * 0.1;

    if debt_service > 0.0 {
        Ok(ebitda / debt_service)
    } else {
        Ok(0.0)
    }
}

/// Custom metric calculator for interest coverage.
pub fn calculate_interest_coverage(context: &MetricContext) -> finstack_core::Result<F> {
    // Simplified interest coverage calculation
    let ebitda = context.base_value.amount() * 0.3;
    let interest_expense = context.base_value.amount() * 0.05; // 5% interest

    if interest_expense > 0.0 {
        Ok(ebitda / interest_expense)
    } else {
        Ok(0.0)
    }
}

/// Register all policy functions with the global registry.
pub fn register_policy_functions() {
    use crate::policy::registry::{register_policy, register_toggle};

    // Register grid margin policy
    let grid_policy = Arc::new(GridMarginPolicy::new());
    register_policy(
        "grid_margin",
        Arc::new(move |params: &dyn Any| {
            if let Some(p) = params.downcast_ref::<GridMarginParams>() {
                let spread =
                    grid_policy.calculate_spread(&p.rating, p.sector.as_deref(), p.maturity_years);
                Ok(Box::new(spread) as Box<dyn Any + Send + Sync>)
            } else {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }),
    );

    // Register DSCR sweep policy
    let dscr_policy = Arc::new(DSCRSweepPolicy::new());
    register_policy(
        "dscr_sweep",
        Arc::new(move |params: &dyn Any| {
            if let Some(p) = params.downcast_ref::<DSCRSweepParams>() {
                let sweep_amount =
                    dscr_policy.calculate_sweep_amount(p.dscr, p.excess_cash, p.minimum_cash);
                Ok(Box::new(sweep_amount) as Box<dyn Any + Send + Sync>)
            } else {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }),
    );

    // Register index fallback policy
    let fallback_policy = Arc::new(IndexFallbackPolicy::new());
    register_policy(
        "index_fallback",
        Arc::new(move |params: &dyn Any| {
            if let Some((index, rates)) = params.downcast_ref::<(&str, HashMap<String, F>)>() {
                let rate = fallback_policy.get_rate(index, rates)?;
                Ok(Box::new(rate) as Box<dyn Any + Send + Sync>)
            } else {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }),
    );

    // Register covenant breach toggle
    register_toggle(
        "covenant_breach",
        Arc::new(|params: &dyn Any| {
            if let Some(dscr) = params.downcast_ref::<F>() {
                Ok(*dscr < 1.25) // Breach if DSCR < 1.25
            } else {
                Err(finstack_core::error::InputError::Invalid.into())
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_margin_policy() {
        let policy = GridMarginPolicy::new();

        // Test base spread
        let spread = policy.calculate_spread("BBB", None, 3.0);
        assert_eq!(spread, 125.0); // 100 base + 25 maturity

        // Test with sector adjustment
        let spread = policy.calculate_spread("BBB", Some("Technology"), 3.0);
        assert_eq!(spread, 100.0); // 100 base - 25 sector + 25 maturity

        // Test floor
        let spread = policy.calculate_spread("AAA", Some("Technology"), 0.5);
        assert!(spread >= policy.floor_spread);
    }

    #[test]
    fn test_index_fallback() {
        let policy = IndexFallbackPolicy::new();
        let mut rates = HashMap::new();
        rates.insert("USD-SOFR-3M".to_string(), 0.045);

        // Test fallback with spread adjustment
        let rate = policy.get_rate("USD-LIBOR-3M", &rates).unwrap();
        assert!((rate - 0.0476161).abs() < 0.0001); // 0.045 + 26.161bps

        // Test static fallback
        let empty_rates = HashMap::new();
        let rate = policy.get_rate("USD-LIBOR-3M", &empty_rates).unwrap();
        assert_eq!(rate, 0.05);
    }

    #[test]
    fn test_dscr_sweep() {
        let policy = DSCRSweepPolicy::new();

        // Test sweep percentages
        assert_eq!(policy.calculate_sweep(0.8), 0.0);
        assert_eq!(policy.calculate_sweep(1.2), 0.25);
        assert_eq!(policy.calculate_sweep(1.4), 0.50);
        assert_eq!(policy.calculate_sweep(1.7), 0.75);
        assert_eq!(policy.calculate_sweep(2.5), 1.0);

        // Test sweep amount calculation
        let excess = Money::new(1000.0, Currency::USD);
        let minimum = Money::new(200.0, Currency::USD);

        let sweep = policy.calculate_sweep_amount(1.5, excess, minimum);
        assert_eq!(sweep.amount(), 400.0); // (1000-200) * 0.5
    }
}
