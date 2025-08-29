//! Workout engine for managing loan restructuring and recovery.
//!
//! This module provides a state machine for managing workout processes:
//! - State transitions (performing, stressed, default, workout, recovered)
//! - Policy-based workout strategies
//! - Penalty and recovery flow generation
//! - Waterfall application for recoveries

use finstack_core::prelude::*;
use finstack_core::F;
use crate::cashflow::primitives::CashFlow;
use crate::instruments::loan::term_loan::{Loan, InterestSpec};
use std::collections::HashMap;

/// Workout state for a loan.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkoutState {
    /// Loan is performing normally
    Performing,
    /// Loan is showing signs of stress but not in default
    Stressed {
        /// Stress indicators (e.g., "late_payment", "covenant_breach")
        indicators: Vec<String>,
    },
    /// Loan is in default
    Default {
        /// Date of default
        default_date: Date,
        /// Reason for default
        reason: String,
    },
    /// Loan is in workout/restructuring
    Workout {
        /// Start date of workout
        start_date: Date,
        /// Type of workout (e.g., "forbearance", "restructure", "foreclosure")
        workout_type: String,
    },
    /// Loan has recovered from workout
    Recovered {
        /// Date of recovery
        recovery_date: Date,
        /// Recovery percentage
        recovery_rate: F,
    },
    /// Loan has been written off
    WrittenOff {
        /// Write-off date
        writeoff_date: Date,
        /// Loss amount
        loss_amount: Money,
    },
}

/// Workout policy specification.
#[derive(Clone, Debug)]
pub struct WorkoutPolicy {
    /// Name of the policy
    pub name: String,
    /// Stress thresholds (metric -> threshold)
    pub stress_thresholds: HashMap<String, F>,
    /// Default triggers
    pub default_triggers: Vec<DefaultTrigger>,
    /// Workout strategies by loan type
    pub workout_strategies: HashMap<String, WorkoutStrategy>,
    /// Recovery waterfall
    pub recovery_waterfall: RecoveryWaterfall,
}

/// Default trigger conditions.
#[derive(Clone, Debug)]
pub enum DefaultTrigger {
    /// Payment delay in days
    PaymentDelay { days: i32 },
    /// Covenant breach without cure
    CovenantBreach { covenant_type: String },
    /// Cross-default from another facility
    CrossDefault { facility_id: String },
    /// Bankruptcy filing
    Bankruptcy,
    /// Material adverse change
    MaterialAdverseChange,
}

/// Workout strategy specification.
#[derive(Clone, Debug)]
pub struct WorkoutStrategy {
    /// Strategy name
    pub name: String,
    /// Forbearance period (if applicable)
    pub forbearance_months: Option<i32>,
    /// Rate modification
    pub rate_modification: Option<RateModification>,
    /// Principal modification
    pub principal_modification: Option<PrincipalModification>,
    /// Maturity extension
    pub maturity_extension_months: Option<i32>,
    /// Required collateral
    pub additional_collateral: Option<String>,
    /// Exit fee percentage
    pub exit_fee_pct: Option<F>,
}

/// Rate modification options.
#[derive(Clone, Debug)]
pub enum RateModification {
    /// Reduce rate by fixed amount
    ReduceBy { bps: F },
    /// Set to new fixed rate
    SetTo { rate: F },
    /// Convert to PIK
    ConvertToPIK { pik_rate: F },
    /// Split between cash and PIK
    SplitCashPIK { cash_rate: F, pik_rate: F },
}

/// Principal modification options.
#[derive(Clone, Debug)]
pub enum PrincipalModification {
    /// Forgive percentage of principal
    Forgive { percentage: F },
    /// Defer percentage to balloon
    Defer { percentage: F },
    /// Amortize over new schedule
    Reamortize { months: i32 },
}

/// Recovery waterfall specification.
#[derive(Clone, Debug)]
pub struct RecoveryWaterfall {
    /// Waterfall tiers in order of priority
    pub tiers: Vec<RecoveryTier>,
}

/// Recovery tier in waterfall.
#[derive(Clone, Debug)]
pub struct RecoveryTier {
    /// Tier name
    pub name: String,
    /// Claim type (e.g., "expenses", "senior_debt", "subordinated_debt", "equity")
    pub claim_type: String,
    /// Claim amount or calculation
    pub claim_amount: ClaimAmount,
    /// Recovery percentage at this tier
    pub recovery_pct: F,
}

/// Claim amount specification.
#[derive(Clone, Debug)]
pub enum ClaimAmount {
    /// Fixed amount
    Fixed(Money),
    /// Percentage of outstanding
    PercentOfOutstanding(F),
    /// Calculated from formula
    Calculated(String),
}

/// Workout event tracking.
#[derive(Clone, Debug)]
pub struct WorkoutEvent {
    /// Event date
    pub date: Date,
    /// Previous state
    pub from_state: WorkoutState,
    /// New state
    pub to_state: WorkoutState,
    /// Event description
    pub description: String,
    /// Financial impact
    pub impact: Option<Money>,
}

/// Workout engine for managing loan workouts.
pub struct WorkoutEngine {
    /// Current state
    pub state: WorkoutState,
    /// Workout policy
    pub policy: WorkoutPolicy,
    /// Event history
    pub events: Vec<WorkoutEvent>,
    /// Cached recovery analysis
    pub recovery_analysis: Option<RecoveryAnalysis>,
}

/// Recovery analysis results.
#[derive(Clone, Debug)]
pub struct RecoveryAnalysis {
    /// Expected recovery amount
    pub expected_recovery: Money,
    /// Recovery rate
    pub recovery_rate: F,
    /// Recovery by tier
    pub tier_recoveries: Vec<(String, Money)>,
    /// Recovery timeline
    pub recovery_schedule: Vec<(Date, Money)>,
}

impl WorkoutEngine {
    /// Create a new workout engine.
    pub fn new(policy: WorkoutPolicy) -> Self {
        Self {
            state: WorkoutState::Performing,
            policy,
            events: Vec::new(),
            recovery_analysis: None,
        }
    }
    
    /// Transition to a new state.
    pub fn transition(
        &mut self,
        new_state: WorkoutState,
        date: Date,
        description: impl Into<String>,
    ) -> finstack_core::Result<()> {
        // Validate transition
        if !self.is_valid_transition(&self.state, &new_state) {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        
        // Record event
        let event = WorkoutEvent {
            date,
            from_state: self.state.clone(),
            to_state: new_state.clone(),
            description: description.into(),
            impact: None,
        };
        self.events.push(event);
        
        // Update state
        self.state = new_state;
        
        // Clear cached analysis on state change
        self.recovery_analysis = None;
        
        Ok(())
    }
    
    /// Apply workout strategy to a loan.
    pub fn apply(
        &mut self,
        loan: &mut Loan,
        strategy_name: &str,
        as_of: Date,
    ) -> finstack_core::Result<WorkoutApplication> {
        // Get strategy and clone it to avoid borrowing issues
        let strategy = self.policy.workout_strategies.get(strategy_name)
            .ok_or(finstack_core::error::InputError::NotFound)?
            .clone();
        
        // Track modifications
        let mut modifications = Vec::new();
        
        // Apply rate modification
        if let Some(ref rate_mod) = strategy.rate_modification {
            self.apply_rate_modification(loan, rate_mod)?;
            modifications.push(format!("Rate modified: {:?}", rate_mod));
        }
        
        // Apply principal modification
        if let Some(ref principal_mod) = strategy.principal_modification {
            self.apply_principal_modification(loan, principal_mod)?;
            modifications.push(format!("Principal modified: {:?}", principal_mod));
        }
        
        // Apply maturity extension
        if let Some(months) = strategy.maturity_extension_months {
            let new_maturity = loan.maturity_date.checked_add(time::Duration::days(months as i64 * 30))
                .ok_or(finstack_core::error::InputError::Invalid)?;
            loan.maturity_date = new_maturity;
            modifications.push(format!("Maturity extended by {} months", months));
        }
        
        // Calculate exit fee if applicable
        let exit_fee = strategy.exit_fee_pct.map(|pct| loan.outstanding * pct);
        
        // Transition to workout state
        self.transition(
            WorkoutState::Workout {
                start_date: as_of,
                workout_type: strategy.name.clone(),
            },
            as_of,
            format!("Applied workout strategy: {}", strategy.name),
        )?;
        
        Ok(WorkoutApplication {
            strategy_name: strategy.name,
            applied_date: as_of,
            modifications,
            exit_fee,
            forbearance_end: strategy.forbearance_months.map(|months| {
                as_of.checked_add(time::Duration::days(months as i64 * 30)).unwrap()
            }),
        })
    }
    
    /// Generate penalty flows for workout.
    pub fn generate_penalty_flows(
        &self,
        loan: &Loan,
        as_of: Date,
    ) -> finstack_core::Result<Vec<CashFlow>> {
        let mut flows = Vec::new();
        
        match &self.state {
            WorkoutState::Default { .. } => {
                // Default interest penalty
                let default_rate = 0.05; // 5% default interest
                let penalty_interest = loan.outstanding * default_rate * 
                    finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
                        as_of, loan.maturity_date, loan.day_count
                    );
                
                flows.push(CashFlow {
                    date: loan.maturity_date,
                    reset_date: None,
                    amount: penalty_interest,
                    kind: crate::cashflow::primitives::CFKind::Fee,
                    accrual_factor: 0.0,
                });
            },
            WorkoutState::Workout { .. } => {
                // Workout fees
                if let Some(strategy) = self.policy.workout_strategies.values().next() {
                    if let Some(exit_fee_pct) = strategy.exit_fee_pct {
                        let exit_fee = loan.outstanding * exit_fee_pct;
                        flows.push(CashFlow {
                            date: loan.maturity_date,
                            reset_date: None,
                            amount: exit_fee,
                            kind: crate::cashflow::primitives::CFKind::Fee,
                            accrual_factor: 0.0,
                        });
                    }
                }
            },
            _ => {}
        }
        
        Ok(flows)
    }
    
    /// Generate recovery flows based on waterfall.
    pub fn generate_recovery_flows(
        &mut self,
        outstanding: Money,
        collateral_value: Money,
        as_of: Date,
    ) -> finstack_core::Result<RecoveryAnalysis> {
        let mut available = collateral_value.amount();
        let mut tier_recoveries = Vec::new();
        let mut total_recovery = 0.0;
        
        for tier in &self.policy.recovery_waterfall.tiers {
            let claim = match &tier.claim_amount {
                ClaimAmount::Fixed(amount) => amount.amount(),
                ClaimAmount::PercentOfOutstanding(pct) => outstanding.amount() * pct,
                ClaimAmount::Calculated(_formula) => outstanding.amount(), // Simplified
            };
            
            let recovery = (claim * tier.recovery_pct).min(available);
            available -= recovery;
            total_recovery += recovery;
            
            tier_recoveries.push((tier.name.clone(), Money::new(recovery, outstanding.currency())));
        }
        
        let recovery_rate = total_recovery / outstanding.amount();
        
        // Simple recovery schedule (immediate recovery for now)
        let recovery_schedule = vec![(as_of, Money::new(total_recovery, outstanding.currency()))];
        
        let analysis = RecoveryAnalysis {
            expected_recovery: Money::new(total_recovery, outstanding.currency()),
            recovery_rate,
            tier_recoveries,
            recovery_schedule,
        };
        
        self.recovery_analysis = Some(analysis.clone());
        
        Ok(analysis)
    }
    
    // Helper methods
    
    fn is_valid_transition(&self, from: &WorkoutState, to: &WorkoutState) -> bool {
        matches!(
            (from, to),
            (WorkoutState::Performing, WorkoutState::Stressed { .. }) |
            (WorkoutState::Performing, WorkoutState::Default { .. }) |
            (WorkoutState::Stressed { .. }, WorkoutState::Default { .. }) |
            (WorkoutState::Stressed { .. }, WorkoutState::Performing) |
            (WorkoutState::Default { .. }, WorkoutState::Workout { .. }) |
            (WorkoutState::Workout { .. }, WorkoutState::Recovered { .. }) |
            (WorkoutState::Workout { .. }, WorkoutState::WrittenOff { .. }) |
            (WorkoutState::Default { .. }, WorkoutState::WrittenOff { .. })
        )
    }
    
    fn apply_rate_modification(
        &self,
        loan: &mut Loan,
        modification: &RateModification,
    ) -> finstack_core::Result<()> {
        match modification {
            RateModification::ReduceBy { bps } => {
                if let InterestSpec::Fixed { rate, .. } = &mut loan.interest {
                    *rate -= bps / 10000.0;
                }
            },
            RateModification::SetTo { rate } => {
                loan.interest = InterestSpec::Fixed {
                    rate: *rate,
                    step_ups: None,
                };
            },
            RateModification::ConvertToPIK { pik_rate } => {
                loan.interest = InterestSpec::PIK {
                    rate: *pik_rate,
                };
            },
            RateModification::SplitCashPIK { cash_rate, pik_rate } => {
                loan.interest = InterestSpec::CashPlusPIK {
                    cash_rate: *cash_rate,
                    pik_rate: *pik_rate,
                };
            },
        }
        Ok(())
    }
    
    fn apply_principal_modification(
        &self,
        loan: &mut Loan,
        modification: &PrincipalModification,
    ) -> finstack_core::Result<()> {
        match modification {
            PrincipalModification::Forgive { percentage } => {
                loan.outstanding *= 1.0 - percentage;
            },
            PrincipalModification::Defer { percentage } => {
                // Create balloon payment
                let deferred = loan.outstanding * *percentage;
                loan.outstanding = (loan.outstanding - deferred)?;
                // Note: Would need to track deferred amount separately
            },
            PrincipalModification::Reamortize { months: _ } => {
                // Update amortization schedule
                use crate::cashflow::amortization_notional::AmortizationSpec;
                loan.amortization = AmortizationSpec::LinearTo {
                    final_notional: Money::new(0.0, loan.outstanding.currency()),
                };
            },
        }
        Ok(())
    }
}

/// Result of applying a workout strategy.
#[derive(Clone, Debug)]
pub struct WorkoutApplication {
    /// Strategy name
    pub strategy_name: String,
    /// Date applied
    pub applied_date: Date,
    /// Modifications made
    pub modifications: Vec<String>,
    /// Exit fee if applicable
    pub exit_fee: Option<Money>,
    /// Forbearance end date if applicable
    pub forbearance_end: Option<Date>,
}
