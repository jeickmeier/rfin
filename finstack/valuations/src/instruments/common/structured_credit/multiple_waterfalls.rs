//! Multiple concurrent waterfall support for structured credit.
//!
//! Hastructure supports multiple waterfall types:
//! - Clean up waterfall 
//! - Pre/Post Enforcement waterfall
//! - Pool collection waterfall
//!
//! This module provides a framework for running multiple waterfall engines
//! concurrently based on deal state and trigger conditions.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

use super::{TrancheStructure, WaterfallEngine, WaterfallResult};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Types of waterfalls that can run concurrently
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WaterfallType {
    /// Normal payment waterfall
    Normal,
    /// Cleanup waterfall (deal termination)
    Cleanup,
    /// Pre-enforcement waterfall (before trigger events)
    PreEnforcement,
    /// Post-enforcement waterfall (after trigger events)
    PostEnforcement,
    /// Pool collection waterfall (asset servicing)
    PoolCollection,
    /// Liquidation waterfall (distressed situations)
    Liquidation,
}

/// Condition that determines which waterfall to use
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WaterfallCondition {
    /// Always use this waterfall
    Always,
    /// Use after specific date
    AfterDate { date: Date },
    /// Use when pool factor falls below threshold
    PoolFactorBelow { threshold: f64 },
    /// Use when coverage tests are breached
    CoverageTestsBreach { test_ids: Vec<String> },
    /// Use during cleanup/wind-down
    DuringCleanup,
    /// Use when deal is in default
    EventOfDefault,
    /// Custom condition (would integrate with expression engine)
    CustomFormula { expression: String },
}

/// Waterfall configuration with conditions
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallConfiguration {
    /// Waterfall type
    pub waterfall_type: WaterfallType,
    /// The actual waterfall engine
    pub engine: WaterfallEngine,
    /// Condition for when this waterfall applies
    pub condition: WaterfallCondition,
    /// Priority when multiple conditions are met (lower = higher priority)
    pub priority: u32,
    /// Whether this waterfall is currently active
    pub is_active: bool,
}

impl WaterfallConfiguration {
    /// Create new waterfall configuration
    pub fn new(
        waterfall_type: WaterfallType,
        engine: WaterfallEngine,
        condition: WaterfallCondition,
        priority: u32,
    ) -> Self {
        Self {
            waterfall_type,
            engine,
            condition,
            priority,
            is_active: false,
        }
    }

    /// Check if this waterfall's condition is met
    pub fn check_condition(&self, context: &WaterfallSelectionContext) -> bool {
        match &self.condition {
            WaterfallCondition::Always => true,
            WaterfallCondition::AfterDate { date } => context.payment_date >= *date,
            WaterfallCondition::PoolFactorBelow { threshold } => {
                let current_balance = context.pool_balance.amount();
                if let Some(original) = context.original_pool_balance {
                    let factor = current_balance / original.amount();
                    factor < *threshold
                } else {
                    false
                }
            },
            WaterfallCondition::CoverageTestsBreach { test_ids } => {
                test_ids.iter().any(|test_id| {
                    context.breached_tests.contains(test_id)
                })
            },
            WaterfallCondition::DuringCleanup => context.is_cleanup_period,
            WaterfallCondition::EventOfDefault => context.is_event_of_default,
            WaterfallCondition::CustomFormula { expression: _ } => {
                // TODO: Integrate with expression engine
                false
            },
        }
    }
}

/// Context for waterfall selection
#[derive(Debug)]
pub struct WaterfallSelectionContext {
    /// Current payment date
    pub payment_date: Date,
    /// Current pool balance
    pub pool_balance: Money,
    /// Original pool balance (for factor calculations)
    pub original_pool_balance: Option<Money>,
    /// Currently breached coverage test IDs
    pub breached_tests: Vec<String>,
    /// Whether in cleanup period
    pub is_cleanup_period: bool,
    /// Whether event of default has occurred
    pub is_event_of_default: bool,
    /// Additional context for custom formulas
    pub custom_context: HashMap<String, f64>,
}

/// Multiple waterfall manager
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MultipleWaterfallManager {
    /// All configured waterfalls
    pub waterfalls: Vec<WaterfallConfiguration>,
    /// Currently active waterfall type
    pub active_waterfall_type: Option<WaterfallType>,
    /// History of waterfall switches
    pub switch_history: Vec<WaterfallSwitch>,
}

/// Record of waterfall switch event
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallSwitch {
    /// Date of switch
    pub switch_date: Date,
    /// Previous waterfall type
    pub from_waterfall: WaterfallType,
    /// New waterfall type
    pub to_waterfall: WaterfallType,
    /// Reason for switch
    pub reason: String,
}

impl MultipleWaterfallManager {
    /// Create new multiple waterfall manager
    pub fn new() -> Self {
        Self {
            waterfalls: Vec::new(),
            active_waterfall_type: None,
            switch_history: Vec::new(),
        }
    }

    /// Add waterfall configuration
    pub fn add_waterfall(mut self, config: WaterfallConfiguration) -> Self {
        self.waterfalls.push(config);
        // Sort by priority
        self.waterfalls.sort_by_key(|w| w.priority);
        self
    }

    /// Select and execute the appropriate waterfall
    pub fn execute_waterfall(
        &mut self,
        available_cash: Money,
        context: &WaterfallSelectionContext,
        tranches: &TrancheStructure,
    ) -> Result<WaterfallResult> {
        // Find the highest priority waterfall whose condition is met
        let selected_index = self.waterfalls
            .iter()
            .enumerate()
            .find(|(_, w)| w.check_condition(context))
            .map(|(i, _)| i)
            .ok_or_else(|| {
                finstack_core::error::InputError::NotFound {
                    id: "applicable_waterfall".to_string(),
                }
            })?;

        let selected_type = self.waterfalls[selected_index].waterfall_type;

        // Check if we need to switch waterfalls
        if let Some(current_type) = self.active_waterfall_type {
            if current_type != selected_type {
                self.switch_history.push(WaterfallSwitch {
                    switch_date: context.payment_date,
                    from_waterfall: current_type,
                    to_waterfall: selected_type,
                    reason: format!("Condition met: {:?}", self.waterfalls[selected_index].condition),
                });
            }
        }

        self.active_waterfall_type = Some(selected_type);

        // Update active status
        for (i, w) in self.waterfalls.iter_mut().enumerate() {
            w.is_active = i == selected_index;
        }

        // Execute the selected waterfall
        self.waterfalls[selected_index].engine.apply_waterfall(
            available_cash,
            context.payment_date,
            tranches,
            context.pool_balance,
        )
    }

    /// Get currently active waterfall type
    pub fn active_waterfall(&self) -> Option<WaterfallType> {
        self.active_waterfall_type
    }

    /// Check if any enforcement waterfalls are active
    pub fn is_enforcement_active(&self) -> bool {
        matches!(
            self.active_waterfall_type, 
            Some(WaterfallType::PostEnforcement) | Some(WaterfallType::Liquidation)
        )
    }

    /// Create standard CLO waterfall configurations
    pub fn standard_clo_waterfalls(base_currency: finstack_core::currency::Currency) -> Self {
        let mut manager = Self::new();

        // Normal waterfall (highest priority when no issues)
        let normal_waterfall = WaterfallEngine::standard_clo(base_currency);
        manager = manager.add_waterfall(WaterfallConfiguration::new(
            WaterfallType::Normal,
            normal_waterfall,
            WaterfallCondition::Always, // Default fallback
            100, // Lower priority - fallback
        ));

        // Post-enforcement waterfall (when coverage tests fail)
        let mut enforcement_waterfall = WaterfallEngine::standard_clo(base_currency);
        // In enforcement, all excess goes to senior principal (turbo)
        enforcement_waterfall.payment_mode = super::enums::PaymentMode::Sequential {
            triggered_by: "coverage_breach".to_string(),
            trigger_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
        };
        
        manager = manager.add_waterfall(WaterfallConfiguration::new(
            WaterfallType::PostEnforcement,
            enforcement_waterfall,
            WaterfallCondition::CoverageTestsBreach { 
                test_ids: vec!["oc_test".to_string(), "ic_test".to_string()] 
            },
            1, // High priority
        ));

        // Cleanup waterfall (when winding down)
        let cleanup_waterfall = WaterfallEngine::standard_clo(base_currency);
        manager = manager.add_waterfall(WaterfallConfiguration::new(
            WaterfallType::Cleanup,
            cleanup_waterfall,
            WaterfallCondition::PoolFactorBelow { threshold: 0.10 }, // 10% cleanup threshold
            2, // High priority
        ));

        manager
    }
}

impl Default for MultipleWaterfallManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).unwrap()
    }

    #[test]
    fn test_waterfall_condition_evaluation() {
        let context = WaterfallSelectionContext {
            payment_date: test_date(),
            pool_balance: Money::new(50_000_000.0, Currency::USD),
            original_pool_balance: Some(Money::new(500_000_000.0, Currency::USD)),
            breached_tests: vec!["oc_test".to_string()],
            is_cleanup_period: false,
            is_event_of_default: false,
            custom_context: HashMap::new(),
        };

        // Test pool factor condition
        let config = WaterfallConfiguration::new(
            WaterfallType::Cleanup,
            WaterfallEngine::new(Currency::USD),
            WaterfallCondition::PoolFactorBelow { threshold: 0.20 }, // 20%
            1,
        );

        // Pool factor = 50M / 500M = 10%, should trigger cleanup
        assert!(config.check_condition(&context));

        // Test coverage breach condition
        let config2 = WaterfallConfiguration::new(
            WaterfallType::PostEnforcement,
            WaterfallEngine::new(Currency::USD),
            WaterfallCondition::CoverageTestsBreach { 
                test_ids: vec!["oc_test".to_string()] 
            },
            1,
        );

        assert!(config2.check_condition(&context));
    }

    #[test]
    fn test_multiple_waterfall_manager() {
        let manager = MultipleWaterfallManager::standard_clo_waterfalls(Currency::USD);
        
        assert_eq!(manager.waterfalls.len(), 3);
        assert!(manager.active_waterfall().is_none());
        
        // After execution, should have an active waterfall
        let _context = WaterfallSelectionContext {
            payment_date: test_date(),
            pool_balance: Money::new(100_000_000.0, Currency::USD),
            original_pool_balance: Some(Money::new(500_000_000.0, Currency::USD)),
            breached_tests: Vec::new(),
            is_cleanup_period: false,
            is_event_of_default: false,
            custom_context: HashMap::new(),
        };

        // Should select normal waterfall since no special conditions
        // (This test would need a proper TrancheStructure to actually execute)
    }
}
