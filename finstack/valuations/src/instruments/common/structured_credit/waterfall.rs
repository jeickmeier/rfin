//! Waterfall logic for structured credit instruments.
//!
//! Adapts the existing private equity waterfall patterns for CLO/ABS structures,
//! with proper handling of coverage tests, payment diversion, and sequential triggers.

use finstack_core::config::{results_meta, FinstackConfig, ResultsMeta};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;
use indexmap::IndexMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::coverage_tests::TestResults;
use super::pool::AssetPool;
use super::tranches::TrancheStructure;
use super::types::{PaymentMode, TriggerConsequence};

/// Individual step in the waterfall distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WaterfallStep {
    /// Pay trustee and administrative fees
    TrusteeFees { amount: Money },

    /// Pay senior management fees
    SeniorManagementFee { rate: F, base_calculation: FeeBase },

    /// Pay hedge counterparty (if applicable)
    HedgePayments,

    /// Pay tranche interest (in priority order)
    TrancheInterest {
        tranche_id: String,
        include_deferred: bool,
    },

    /// Coverage test checkpoint with potential diversion
    CoverageTest {
        test_names: Vec<String>,
        diversion_target: Option<String>, // Where to divert if breached
    },

    /// Principal payment to tranche
    TranchePrincipal {
        tranche_id: String,
        payment_type: PrincipalPaymentType,
    },

    /// Pay subordinated management fees
    SubordinatedManagementFee { rate: F, base_calculation: FeeBase },

    /// Build or release reserve account
    ReserveAccount {
        target_amount: Money,
        floor_amount: Money,
    },

    /// Distribute residual to equity holders
    EquityDistribution,

    /// Custom step for deal-specific rules
    Custom { description: String, priority: u32 },
}

/// Base for fee calculations
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FeeBase {
    /// Based on total tranche balances
    TranchePrincipal,
    /// Based on pool principal
    PoolPrincipal,
    /// Based on net asset value
    NetAssetValue,
    /// Fixed amount
    FixedAmount(Money),
}

/// Type of principal payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PrincipalPaymentType {
    /// Pro-rata across tranches of same seniority
    ProRata,
    /// Sequential (pay in full before next tranche)
    Sequential,
    /// Turbo payment due to trigger breach
    Turbo,
}

/// Structured credit waterfall engine
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructuredCreditWaterfall {
    /// Current payment mode
    pub payment_mode: PaymentMode,

    /// Interest waterfall steps (always sequential)
    pub interest_waterfall: Vec<WaterfallStep>,

    /// Principal waterfall steps (can be pro-rata or sequential)
    pub principal_waterfall: Vec<WaterfallStep>,

    /// Excess spread waterfall steps
    pub excess_spread_waterfall: Vec<WaterfallStep>,
}

impl StructuredCreditWaterfall {
    /// Create new waterfall with default CLO structure
    pub fn default_clo() -> Self {
        Self {
            payment_mode: PaymentMode::ProRata,
            interest_waterfall: Self::default_clo_interest_waterfall(),
            principal_waterfall: Self::default_clo_principal_waterfall(),
            excess_spread_waterfall: Self::default_clo_excess_spread_waterfall(),
        }
    }

    /// Default CLO interest waterfall
    fn default_clo_interest_waterfall() -> Vec<WaterfallStep> {
        vec![
            WaterfallStep::TrusteeFees {
                amount: Money::new(50_000.0, finstack_core::currency::Currency::USD), // Example amount
            },
            WaterfallStep::SeniorManagementFee {
                rate: 0.004, // 40 bps
                base_calculation: FeeBase::TranchePrincipal,
            },
            WaterfallStep::HedgePayments,
            // Note: Actual tranche interest steps would be added dynamically based on tranche structure
        ]
    }

    /// Default CLO principal waterfall
    fn default_clo_principal_waterfall() -> Vec<WaterfallStep> {
        vec![
            // Coverage tests would be inserted here
            // Tranche principal payments would be added based on structure
        ]
    }

    /// Default CLO excess spread waterfall
    fn default_clo_excess_spread_waterfall() -> Vec<WaterfallStep> {
        vec![
            WaterfallStep::SubordinatedManagementFee {
                rate: 0.002, // 20 bps
                base_calculation: FeeBase::TranchePrincipal,
            },
            WaterfallStep::ReserveAccount {
                target_amount: Money::new(10_000_000.0, finstack_core::currency::Currency::USD),
                floor_amount: Money::new(5_000_000.0, finstack_core::currency::Currency::USD),
            },
            WaterfallStep::EquityDistribution,
        ]
    }

    /// Distribute available cash through the waterfall
    pub fn distribute(
        &self,
        available_cash: Money,
        pool: &AssetPool,
        tranches: &TrancheStructure,
        coverage_results: &TestResults,
        payment_date: Date,
    ) -> finstack_core::Result<WaterfallAllocation> {
        let mut allocation = WaterfallAllocation::new(payment_date, available_cash);
        let mut remaining_cash = available_cash;

        // 1. Interest waterfall (always runs first)
        remaining_cash =
            self.run_interest_waterfall(remaining_cash, pool, tranches, &mut allocation)?;

        // 2. Check if payment mode should change due to triggers
        let effective_mode = self.determine_payment_mode(coverage_results);

        // 3. Principal waterfall (depends on payment mode)
        remaining_cash = self.run_principal_waterfall(
            remaining_cash,
            pool,
            tranches,
            &effective_mode,
            &mut allocation,
        )?;

        // 4. Excess spread waterfall (any remaining cash)
        remaining_cash =
            self.run_excess_spread_waterfall(remaining_cash, pool, tranches, &mut allocation)?;

        // Any truly remaining cash goes to equity
        if remaining_cash.amount() > 0.0 {
            allocation.add_payment("EQUITY_RESIDUAL", remaining_cash, "Residual distribution");
        }

        Ok(allocation)
    }

    /// Run interest waterfall
    fn run_interest_waterfall(
        &self,
        mut available: Money,
        _pool: &AssetPool,
        tranches: &TrancheStructure,
        allocation: &mut WaterfallAllocation,
    ) -> finstack_core::Result<Money> {
        for step in &self.interest_waterfall {
            if available.amount() <= 0.0 {
                break;
            }

            available = self.process_waterfall_step(step, available, tranches, allocation)?;
        }

        Ok(available)
    }

    /// Run principal waterfall
    fn run_principal_waterfall(
        &self,
        mut available: Money,
        _pool: &AssetPool,
        tranches: &TrancheStructure,
        payment_mode: &PaymentMode,
        allocation: &mut WaterfallAllocation,
    ) -> finstack_core::Result<Money> {
        // Adjust waterfall steps based on payment mode
        let effective_steps = match payment_mode {
            PaymentMode::ProRata => &self.principal_waterfall,
            PaymentMode::Sequential { .. } => {
                // In sequential mode, prioritize senior tranches
                &self.principal_waterfall // Would be modified in real implementation
            }
            PaymentMode::Hybrid { .. } => &self.principal_waterfall,
        };

        for step in effective_steps {
            if available.amount() <= 0.0 {
                break;
            }

            available = self.process_waterfall_step(step, available, tranches, allocation)?;
        }

        Ok(available)
    }

    /// Run excess spread waterfall
    fn run_excess_spread_waterfall(
        &self,
        mut available: Money,
        _pool: &AssetPool,
        tranches: &TrancheStructure,
        allocation: &mut WaterfallAllocation,
    ) -> finstack_core::Result<Money> {
        for step in &self.excess_spread_waterfall {
            if available.amount() <= 0.0 {
                break;
            }

            available = self.process_waterfall_step(step, available, tranches, allocation)?;
        }

        Ok(available)
    }

    /// Process individual waterfall step
    fn process_waterfall_step(
        &self,
        step: &WaterfallStep,
        available: Money,
        tranches: &TrancheStructure,
        allocation: &mut WaterfallAllocation,
    ) -> finstack_core::Result<Money> {
        match step {
            WaterfallStep::TrusteeFees { amount } => {
                let payment = available.amount().min(amount.amount());
                let payment_amount = Money::new(payment, available.currency());
                allocation.add_payment("TRUSTEE_FEES", payment_amount, "Trustee and admin fees");
                Ok(Money::new(
                    available.amount() - payment,
                    available.currency(),
                ))
            }

            WaterfallStep::TrancheInterest {
                tranche_id,
                include_deferred,
            } => {
                if let Some(tranche) = tranches
                    .tranches
                    .iter()
                    .find(|t| t.id.as_str() == tranche_id)
                {
                    let current_rate = tranche.coupon.current_rate(
                        Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
                    );

                    // Calculate quarterly interest due (simplified)
                    let mut interest_due = tranche.current_balance.amount() * current_rate / 4.0;

                    // Add deferred interest if requested
                    if *include_deferred {
                        interest_due += tranche.deferred_interest.amount();
                    }

                    let payment = available.amount().min(interest_due);
                    let payment_amount = Money::new(payment, available.currency());

                    allocation.add_payment(
                        &format!("{}_INTEREST", tranche_id),
                        payment_amount,
                        "Tranche interest payment",
                    );

                    return Ok(Money::new(
                        available.amount() - payment,
                        available.currency(),
                    ));
                }
                Ok(available) // Tranche not found
            }

            WaterfallStep::TranchePrincipal {
                tranche_id,
                payment_type,
            } => {
                if let Some(tranche) = tranches
                    .tranches
                    .iter()
                    .find(|t| t.id.as_str() == tranche_id)
                {
                    let payment = match payment_type {
                        PrincipalPaymentType::Sequential => {
                            // Pay full tranche balance if available
                            available.amount().min(tranche.current_balance.amount())
                        }
                        PrincipalPaymentType::ProRata => {
                            // Pro-rata allocation (simplified - would need more complex logic)
                            available.amount()
                                * (tranche.current_balance.amount() / tranches.total_size.amount())
                        }
                        PrincipalPaymentType::Turbo => {
                            // Accelerated payment
                            available.amount().min(tranche.current_balance.amount())
                        }
                    };

                    let payment_amount = Money::new(payment, available.currency());
                    allocation.add_payment(
                        &format!("{}_PRINCIPAL", tranche_id),
                        payment_amount,
                        "Tranche principal payment",
                    );

                    return Ok(Money::new(
                        available.amount() - payment,
                        available.currency(),
                    ));
                }
                Ok(available)
            }

            WaterfallStep::EquityDistribution => {
                allocation.add_payment("EQUITY_DISTRIBUTION", available, "Equity distribution");
                Ok(Money::new(0.0, available.currency()))
            }

            _ => {
                // Placeholder for other step types
                Ok(available)
            }
        }
    }

    /// Determine effective payment mode based on coverage test results
    fn determine_payment_mode(&self, coverage_results: &TestResults) -> PaymentMode {
        if !coverage_results.breached_tests.is_empty() {
            // Check if any breach triggers sequential payment
            for breach in &coverage_results.breached_tests {
                if breach
                    .consequences_applied
                    .contains(&TriggerConsequence::AccelerateAmortization)
                {
                    return PaymentMode::Sequential {
                        triggered_by: breach.test_name.clone(),
                        trigger_date: breach.breach_date,
                    };
                }
            }
        }

        self.payment_mode.clone()
    }
}

/// Result of waterfall distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallAllocation {
    /// Payment date
    pub payment_date: Date,
    /// Total available cash at start
    pub total_available: Money,
    /// Individual payments by recipient
    pub payments: IndexMap<String, PaymentDetail>,
    /// Total distributed
    pub total_distributed: Money,
    /// Any remaining undistributed cash
    pub remaining: Money,
    /// Metadata about the allocation
    pub metadata: ResultsMeta,
}

/// Details of an individual payment in the waterfall
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentDetail {
    /// Amount paid
    pub amount: Money,
    /// Description/reason for payment
    pub description: String,
    /// Step in waterfall where payment occurred
    pub waterfall_step: u32,
}

impl WaterfallAllocation {
    /// Create new allocation result
    pub fn new(payment_date: Date, total_available: Money) -> Self {
        let config = FinstackConfig::default();
        Self {
            payment_date,
            total_available,
            payments: IndexMap::new(),
            total_distributed: Money::new(0.0, total_available.currency()),
            remaining: total_available,
            metadata: results_meta(&config),
        }
    }

    /// Add a payment to the allocation
    pub fn add_payment(&mut self, recipient: &str, amount: Money, description: &str) {
        let step = self.payments.len() as u32 + 1;

        self.payments.insert(
            recipient.to_string(),
            PaymentDetail {
                amount,
                description: description.to_string(),
                waterfall_step: step,
            },
        );

        self.total_distributed = self
            .total_distributed
            .checked_add(amount)
            .unwrap_or(self.total_distributed);

        self.remaining = Money::new(
            (self.remaining.amount() - amount.amount()).max(0.0),
            self.remaining.currency(),
        );
    }

    /// Get payment to specific tranche
    pub fn tranche_payment(&self, tranche_id: &str) -> Option<Money> {
        // Sum both interest and principal payments
        let interest_key = format!("{}_INTEREST", tranche_id);
        let principal_key = format!("{}_PRINCIPAL", tranche_id);

        let interest = self
            .payments
            .get(&interest_key)
            .map(|p| p.amount)
            .unwrap_or_else(|| Money::new(0.0, self.total_available.currency()));

        let principal = self
            .payments
            .get(&principal_key)
            .map(|p| p.amount)
            .unwrap_or_else(|| Money::new(0.0, self.total_available.currency()));

        interest.checked_add(principal).ok()
    }

    /// Check if any cash was diverted due to triggers
    pub fn has_diversions(&self) -> bool {
        self.payments.iter().any(|(_, detail)| {
            detail.description.contains("diverted") || detail.description.contains("turbo")
        })
    }
}

/// Waterfall builder for creating deal-specific waterfalls
pub struct WaterfallBuilder {
    interest_steps: Vec<WaterfallStep>,
    principal_steps: Vec<WaterfallStep>,
    excess_steps: Vec<WaterfallStep>,
    payment_mode: PaymentMode,
}

impl WaterfallBuilder {
    /// Create new waterfall builder
    pub fn new() -> Self {
        Self {
            interest_steps: Vec::new(),
            principal_steps: Vec::new(),
            excess_steps: Vec::new(),
            payment_mode: PaymentMode::ProRata,
        }
    }

    /// Add interest step
    pub fn add_interest_step(mut self, step: WaterfallStep) -> Self {
        self.interest_steps.push(step);
        self
    }

    /// Add principal step
    pub fn add_principal_step(mut self, step: WaterfallStep) -> Self {
        self.principal_steps.push(step);
        self
    }

    /// Add excess spread step
    pub fn add_excess_step(mut self, step: WaterfallStep) -> Self {
        self.excess_steps.push(step);
        self
    }

    /// Set payment mode
    pub fn payment_mode(mut self, mode: PaymentMode) -> Self {
        self.payment_mode = mode;
        self
    }

    /// Build the waterfall
    pub fn build(self) -> StructuredCreditWaterfall {
        StructuredCreditWaterfall {
            payment_mode: self.payment_mode,
            interest_waterfall: self.interest_steps,
            principal_waterfall: self.principal_steps,
            excess_spread_waterfall: self.excess_steps,
        }
    }

    /// Create standard CLO waterfall for given tranches
    pub fn standard_clo(tranches: &TrancheStructure) -> Self {
        let mut builder = Self::new();

        // Add standard fees
        builder = builder.add_interest_step(WaterfallStep::TrusteeFees {
            amount: Money::new(50_000.0, finstack_core::currency::Currency::USD),
        });

        builder = builder.add_interest_step(WaterfallStep::SeniorManagementFee {
            rate: 0.004,
            base_calculation: FeeBase::TranchePrincipal,
        });

        // Add tranche interest payments in priority order
        let mut sorted_tranches = tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);

        for tranche in &sorted_tranches {
            builder = builder.add_interest_step(WaterfallStep::TrancheInterest {
                tranche_id: tranche.id.as_str().to_string(),
                include_deferred: true,
            });
        }

        // Add coverage tests and principal payments
        for tranche in &sorted_tranches {
            if tranche.seniority == super::types::TrancheSeniority::Senior {
                builder = builder.add_principal_step(WaterfallStep::CoverageTest {
                    test_names: vec![format!("{}_OC", tranche.id.as_str())],
                    diversion_target: Some(tranche.id.as_str().to_string()),
                });
            }

            builder = builder.add_principal_step(WaterfallStep::TranchePrincipal {
                tranche_id: tranche.id.as_str().to_string(),
                payment_type: PrincipalPaymentType::Sequential,
            });
        }

        // Add excess spread distribution
        builder = builder.add_excess_step(WaterfallStep::SubordinatedManagementFee {
            rate: 0.002,
            base_calculation: FeeBase::TranchePrincipal,
        });

        builder = builder.add_excess_step(WaterfallStep::EquityDistribution);

        builder
    }
}

impl Default for WaterfallBuilder {
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
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    #[test]
    fn test_waterfall_allocation_creation() {
        let available = Money::new(1_000_000.0, Currency::USD);
        let mut allocation = WaterfallAllocation::new(test_date(), available);

        // Add some payments
        allocation.add_payment("FEES", Money::new(50_000.0, Currency::USD), "Admin fees");
        allocation.add_payment(
            "SENIOR_INTEREST",
            Money::new(100_000.0, Currency::USD),
            "Senior interest",
        );

        assert_eq!(allocation.payments.len(), 2);
        assert_eq!(allocation.total_distributed.amount(), 150_000.0);
        assert_eq!(allocation.remaining.amount(), 850_000.0);
    }

    #[test]
    fn test_waterfall_builder() {
        let builder = WaterfallBuilder::new()
            .add_interest_step(WaterfallStep::TrusteeFees {
                amount: Money::new(50_000.0, Currency::USD),
            })
            .add_interest_step(WaterfallStep::TrancheInterest {
                tranche_id: "SENIOR_A".to_string(),
                include_deferred: true,
            })
            .payment_mode(PaymentMode::ProRata);

        let waterfall = builder.build();
        assert_eq!(waterfall.interest_waterfall.len(), 2);
        assert!(matches!(waterfall.payment_mode, PaymentMode::ProRata));
    }
}
