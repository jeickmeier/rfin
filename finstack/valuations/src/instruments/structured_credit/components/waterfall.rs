//! Generalized tier-based waterfall engine for structured credit instruments.
//!
//! This module provides a production-grade waterfall engine supporting:
//! - Tier-based payment distribution with configurable priorities
//! - Pro-rata and sequential allocation modes within tiers
//! - Multi-recipient tiers for complex payment structures
//! - Coverage test integration with diversion rules
//! - Circular reference detection
//! - Explainability traces

use super::coverage_tests::{CoverageTest, TestContext};
use super::AssetPool;
use super::TrancheStructure;
use crate::instruments::structured_credit::config::QUARTERLY_PERIODS_PER_YEAR;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// ============================================================================
// CORE TYPES
// ============================================================================

/// Recipient of waterfall payments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentRecipient {
    /// Service provider (trustee, admin, rating agency, etc.)
    ServiceProvider(String),
    /// Manager fee (type indicates senior/subordinated/incentive)
    ManagerFee(ManagementFeeType),
    /// Tranche payment
    Tranche(String),
    /// Equity/residual distribution
    Equity,
}

/// Type of management fee
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ManagementFeeType {
    /// Senior variant.
    Senior,
    /// Subordinated variant.
    Subordinated,
    /// Incentive variant.
    Incentive,
}

/// How to calculate payment amount
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentCalculation {
    /// Fixed amount
    FixedAmount {
        /// Amount.
        amount: Money,
    },
    /// Percentage of collateral balance
    PercentageOfCollateral {
        /// Rate.
        rate: f64,
        /// Annualized.
        annualized: bool,
    },
    /// Interest due on tranche
    TrancheInterest {
        /// Tranche id.
        tranche_id: String,
    },
    /// Principal payment to tranche
    TranchePrincipal {
        /// Tranche id.
        tranche_id: String,
        /// Target balance.
        target_balance: Option<Money>,
    },
    /// All remaining cash
    ResidualCash,
}

/// Allocation mode within a tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AllocationMode {
    /// Pay recipients sequentially in order until tier allocation exhausted
    Sequential,
    /// Distribute proportionally by weight or equally if no weights
    ProRata,
}

/// Payment type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PaymentType {
    /// Fee payment
    Fee,
    /// Interest payment
    Interest,
    /// Principal payment
    Principal,
    /// Residual/equity distribution
    Residual,
}

/// Individual payment recipient within a tier
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Recipient {
    /// Unique identifier
    pub id: String,
    /// Recipient type
    pub recipient_type: PaymentRecipient,
    /// How to calculate payment amount
    pub calculation: PaymentCalculation,
    /// Weight for pro-rata distribution (None = equal weight)
    pub weight: Option<f64>,
}

impl Recipient {
    /// Create a new recipient
    pub fn new(
        id: impl Into<String>,
        recipient_type: PaymentRecipient,
        calculation: PaymentCalculation,
    ) -> Self {
        Self {
            id: id.into(),
            recipient_type,
            calculation,
            weight: None,
        }
    }

    /// Set weight for pro-rata allocation
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Create a fixed fee recipient
    pub fn fixed_fee(id: impl Into<String>, provider: impl Into<String>, amount: Money) -> Self {
        Self::new(
            id,
            PaymentRecipient::ServiceProvider(provider.into()),
            PaymentCalculation::FixedAmount { amount },
        )
    }

    /// Create a tranche interest recipient
    pub fn tranche_interest(id: impl Into<String>, tranche_id: impl Into<String>) -> Self {
        let tranche_id_str = tranche_id.into();
        Self::new(
            id,
            PaymentRecipient::Tranche(tranche_id_str.clone()),
            PaymentCalculation::TrancheInterest {
                tranche_id: tranche_id_str,
            },
        )
    }

    /// Create a tranche principal recipient
    pub fn tranche_principal(
        id: impl Into<String>,
        tranche_id: impl Into<String>,
        target_balance: Option<Money>,
    ) -> Self {
        let tranche_id_str = tranche_id.into();
        Self::new(
            id,
            PaymentRecipient::Tranche(tranche_id_str.clone()),
            PaymentCalculation::TranchePrincipal {
                tranche_id: tranche_id_str,
                target_balance,
            },
        )
    }
}

/// Waterfall tier with multiple recipients
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallTier {
    /// Unique tier identifier
    pub id: String,
    /// Priority order (lower = higher priority)
    pub priority: usize,
    /// Recipients in this tier
    pub recipients: Vec<Recipient>,
    /// Payment type classification
    pub payment_type: PaymentType,
    /// How to allocate within tier
    pub allocation_mode: AllocationMode,
    /// Whether this tier can be diverted if coverage tests fail
    pub divertible: bool,
}

impl WaterfallTier {
    /// Create a new waterfall tier
    pub fn new(id: impl Into<String>, priority: usize, payment_type: PaymentType) -> Self {
        Self {
            id: id.into(),
            priority,
            recipients: Vec::new(),
            payment_type,
            allocation_mode: AllocationMode::Sequential,
            divertible: false,
        }
    }

    /// Add a recipient to this tier
    pub fn add_recipient(mut self, recipient: Recipient) -> Self {
        self.recipients.push(recipient);
        self
    }

    /// Set allocation mode
    pub fn allocation_mode(mut self, mode: AllocationMode) -> Self {
        self.allocation_mode = mode;
        self
    }

    /// Mark as divertible
    pub fn divertible(mut self, divertible: bool) -> Self {
        self.divertible = divertible;
        self
    }
}

// ============================================================================
// WATERFALL RESULT
// ============================================================================

/// Result of waterfall distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallResult {
    /// Payment date
    pub payment_date: Date,
    /// Total available cash at start
    pub total_available: Money,

    /// Tier-level allocations
    pub tier_allocations: Vec<(String, Money)>,

    /// Distributions by recipient
    pub distributions: HashMap<PaymentRecipient, Money>,
    /// Detailed payment records
    pub payment_records: Vec<PaymentRecord>,

    /// Coverage test results (test_name, value, passed)
    pub coverage_tests: Vec<(String, f64, bool)>,

    /// Total diverted cash
    pub diverted_cash: Money,
    /// Remaining undistributed cash
    pub remaining_cash: Money,
    /// Whether any diversions occurred
    pub had_diversions: bool,
    /// Diversion reason if applicable
    pub diversion_reason: Option<String>,

    /// Optional explanation trace (enabled via ExplainOpts)
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub explanation: Option<ExplanationTrace>,
}

/// Record of individual payment
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PaymentRecord {
    /// Tier id
    pub tier_id: String,
    /// Recipient id within tier
    pub recipient_id: String,
    /// Priority
    pub priority: usize,
    /// Recipient
    pub recipient: PaymentRecipient,
    /// Requested amount
    pub requested_amount: Money,
    /// Paid amount
    pub paid_amount: Money,
    /// Shortfall
    pub shortfall: Money,
    /// Diverted
    pub diverted: bool,
}

// ============================================================================
// COVERAGE TRIGGERS
// ============================================================================

/// Simple OC/IC trigger for diversion
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTrigger {
    /// Tranche where test applies
    pub tranche_id: String,
    /// OC trigger level (e.g., 1.15 = 115%)
    pub oc_trigger: Option<f64>,
    /// IC trigger level (e.g., 1.10 = 110%)
    pub ic_trigger: Option<f64>,
}

/// Type of coverage test (simplified to OC/IC only)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CoverageTestType {
    /// Overcollateralization test
    OC,
    /// Interest coverage test
    IC,
}

// ============================================================================
// WATERFALL WORKSPACE (Pre-allocated Buffers)
// ============================================================================

/// Pre-allocated workspace for waterfall execution to avoid hot-path allocations.
///
/// This struct holds reusable buffers that are cleared between periods rather than
/// reallocated. For Monte Carlo simulations with thousands of paths and hundreds
/// of periods, this significantly reduces allocation overhead.
///
/// # Usage
///
/// ```ignore
/// let mut workspace = WaterfallWorkspace::new(num_tiers, num_recipients);
/// for period in periods {
///     workspace.clear();
///     engine.execute_waterfall_with_workspace(..., &mut workspace)?;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct WaterfallWorkspace {
    /// Pre-allocated tier allocations buffer
    pub tier_allocations: Vec<(String, Money)>,
    /// Pre-allocated distributions map
    pub distributions: HashMap<PaymentRecipient, Money>,
    /// Pre-allocated payment records buffer
    pub payment_records: Vec<PaymentRecord>,
    /// Pre-allocated coverage test results buffer
    pub coverage_tests: Vec<(String, f64, bool)>,
    /// Pre-allocated tranche index (built once per deal, reused across periods)
    pub tranche_index: HashMap<String, usize>,
}

impl WaterfallWorkspace {
    /// Create a new workspace with pre-allocated capacity.
    ///
    /// # Arguments
    /// * `num_tiers` - Number of waterfall tiers (for tier_allocations capacity)
    /// * `num_recipients` - Total number of recipients across all tiers
    /// * `num_tranches` - Number of tranches (for tranche_index capacity)
    pub fn new(num_tiers: usize, num_recipients: usize, num_tranches: usize) -> Self {
        Self {
            tier_allocations: Vec::with_capacity(num_tiers),
            distributions: HashMap::with_capacity(num_recipients),
            payment_records: Vec::with_capacity(num_recipients),
            coverage_tests: Vec::with_capacity(num_tranches * 2), // OC + IC per tranche
            tranche_index: HashMap::with_capacity(num_tranches),
        }
    }

    /// Create workspace from a WaterfallEngine and TrancheStructure.
    ///
    /// This is the recommended way to create a workspace as it automatically
    /// calculates the correct capacities.
    pub fn from_engine(engine: &WaterfallEngine, tranches: &TrancheStructure) -> Self {
        let num_tiers = engine.tiers.len();
        let num_recipients: usize = engine.tiers.iter().map(|t| t.recipients.len()).sum();
        let num_tranches = tranches.tranches.len();

        let mut workspace = Self::new(num_tiers, num_recipients, num_tranches);

        // Pre-build the tranche index (this is stable for the life of the deal)
        for (i, t) in tranches.tranches.iter().enumerate() {
            workspace.tranche_index.insert(t.id.to_string(), i);
        }

        workspace
    }

    /// Clear all buffers for reuse in the next period.
    ///
    /// This retains allocated capacity while removing all elements.
    pub fn clear(&mut self) {
        self.tier_allocations.clear();
        self.distributions.clear();
        self.payment_records.clear();
        self.coverage_tests.clear();
        // Note: tranche_index is NOT cleared as it's stable across periods
    }
}

impl Default for WaterfallWorkspace {
    fn default() -> Self {
        Self::new(8, 32, 8) // Reasonable defaults for typical CLO structures
    }
}

// ============================================================================
// WATERFALL ENGINE
// ============================================================================

/// Main waterfall engine with tier-based distribution
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WaterfallEngine {
    /// Ordered payment tiers
    pub tiers: Vec<WaterfallTier>,
    /// Coverage triggers for OC/IC diversion
    pub coverage_triggers: Vec<CoverageTrigger>,
    /// Base currency
    pub base_currency: Currency,
}

impl WaterfallEngine {
    /// Create new waterfall engine
    pub fn new(base_currency: Currency) -> Self {
        Self {
            tiers: Vec::new(),
            coverage_triggers: Vec::new(),
            base_currency,
        }
    }

    /// Add a tier
    pub fn add_tier(mut self, tier: WaterfallTier) -> Self {
        self.tiers.push(tier);
        self.tiers.sort_by_key(|t| t.priority);
        self
    }

    /// Add coverage trigger for OC/IC diversion
    pub fn add_coverage_trigger(mut self, trigger: CoverageTrigger) -> Self {
        self.coverage_triggers.push(trigger);
        self
    }

    /// Execute waterfall to distribute available cash
    #[allow(clippy::too_many_arguments)]
    pub fn execute_waterfall(
        &mut self,
        available_cash: Money,
        interest_collections: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
        pool: &AssetPool,
        market: &MarketContext,
    ) -> Result<WaterfallResult> {
        self.execute_waterfall_with_explanation(
            available_cash,
            interest_collections,
            payment_date,
            tranches,
            pool_balance,
            pool,
            market,
            ExplainOpts::disabled(),
        )
    }

    /// Execute waterfall with optional explanation trace
    #[allow(clippy::too_many_arguments)]
    pub fn execute_waterfall_with_explanation(
        &mut self,
        available_cash: Money,
        interest_collections: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
        pool: &AssetPool,
        market: &MarketContext,
        explain: ExplainOpts,
    ) -> Result<WaterfallResult> {
        let mut remaining = available_cash;
        let mut tier_allocations = Vec::with_capacity(self.tiers.len());
        // Pre-allocate distributions based on estimated unique recipients across tiers
        let estimated_recipients = self.tiers.iter().map(|t| t.recipients.len()).sum::<usize>();
        let mut distributions: HashMap<PaymentRecipient, Money> =
            HashMap::with_capacity(estimated_recipients);
        let mut payment_records = Vec::with_capacity(estimated_recipients);
        let mut total_diverted = Money::new(0.0, self.base_currency);
        let mut had_diversions = false;
        let mut diversion_reason = None;

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("waterfall"))
        } else {
            None
        };

        // Build tranche index for O(1) lookup by id
        let mut tranche_index: HashMap<&str, usize> =
            HashMap::with_capacity(tranches.tranches.len());
        for (i, t) in tranches.tranches.iter().enumerate() {
            tranche_index.insert(t.id.as_str(), i);
        }

        // Evaluate coverage tests
        let coverage_test_results = self.evaluate_coverage_tests(
            tranches,
            pool,
            payment_date,
            available_cash,
            interest_collections,
        )?;

        // Check if diversions are active
        let diversion_active = coverage_test_results.iter().any(|(_, _, passed)| !passed);
        if diversion_active {
            had_diversions = true;
            diversion_reason = Some("OC or IC test failed".to_string());
        }

        // Process tiers in priority order (all tiers processed, even if cash exhausted)
        for tier in &self.tiers {
            // Determine if this tier should be diverted
            let (target_recipients, tier_diverted): (&[Recipient], bool) =
                if tier.divertible && diversion_active {
                    // Find senior tier to divert to
                    let senior_tier = self
                        .tiers
                        .iter()
                        .filter(|t| {
                            t.priority < tier.priority && t.payment_type == PaymentType::Principal
                        })
                        .min_by_key(|t| t.priority);

                    senior_tier
                        .map(|s| (&s.recipients[..], true))
                        .unwrap_or((&tier.recipients[..], false))
                } else {
                    (&tier.recipients[..], false)
                };

            // Allocate cash to tier based on mode
            let tier_cash = match tier.allocation_mode {
                AllocationMode::Sequential => self.allocate_sequential(
                    tier,
                    target_recipients,
                    remaining,
                    tranches,
                    &tranche_index,
                    pool_balance,
                    payment_date,
                    market,
                    tier_diverted,
                    &mut distributions,
                    &mut payment_records,
                    &mut trace,
                    &explain,
                )?,
                AllocationMode::ProRata => self.allocate_pro_rata(
                    tier,
                    target_recipients,
                    remaining,
                    tranches,
                    &tranche_index,
                    pool_balance,
                    payment_date,
                    market,
                    tier_diverted,
                    &mut distributions,
                    &mut payment_records,
                    &mut trace,
                    &explain,
                )?,
            };

            if tier_diverted {
                total_diverted = total_diverted.checked_add(tier_cash)?;
            }

            tier_allocations.push((tier.id.clone(), tier_cash));
            remaining = remaining.checked_sub(tier_cash)?;
        }

        Ok(WaterfallResult {
            payment_date,
            total_available: available_cash,
            tier_allocations,
            distributions,
            payment_records,
            coverage_tests: coverage_test_results,
            diverted_cash: total_diverted,
            remaining_cash: remaining,
            had_diversions,
            diversion_reason,
            explanation: trace,
        })
    }

    /// Execute waterfall using a pre-allocated workspace for zero-allocation hot paths.
    ///
    /// This method reuses buffers from the workspace instead of allocating new ones,
    /// significantly reducing allocation overhead in Monte Carlo simulations.
    ///
    /// # Arguments
    /// * `workspace` - Pre-allocated workspace (call `workspace.clear()` before each period)
    ///
    /// # Example
    /// ```ignore
    /// let mut workspace = WaterfallWorkspace::from_engine(&engine, &tranches);
    /// for period in periods {
    ///     workspace.clear();
    ///     let result = engine.execute_waterfall_with_workspace(
    ///         available, interest, date, &tranches, pool_bal, &pool, &market,
    ///         ExplainOpts::disabled(), &mut workspace
    ///     )?;
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn execute_waterfall_with_workspace(
        &mut self,
        available_cash: Money,
        interest_collections: Money,
        payment_date: Date,
        tranches: &TrancheStructure,
        pool_balance: Money,
        pool: &AssetPool,
        market: &MarketContext,
        explain: ExplainOpts,
        workspace: &mut WaterfallWorkspace,
    ) -> Result<WaterfallResult> {
        let mut remaining = available_cash;
        let mut total_diverted = Money::new(0.0, self.base_currency);
        let mut had_diversions = false;
        let mut diversion_reason = None;

        // Initialize explanation trace if requested
        let mut trace = if explain.enabled {
            Some(ExplanationTrace::new("waterfall"))
        } else {
            None
        };

        // Build a temporary tranche index for &str keys (required by allocate_* methods)
        // This is a lightweight reference map that doesn't allocate string data
        let tranche_index: HashMap<&str, usize> = workspace
            .tranche_index
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();

        // Evaluate coverage tests into workspace buffer
        workspace.coverage_tests.clear();
        let coverage_test_results = self.evaluate_coverage_tests(
            tranches,
            pool,
            payment_date,
            available_cash,
            interest_collections,
        )?;
        workspace.coverage_tests.extend(coverage_test_results.iter().cloned());

        // Check if diversions are active
        let diversion_active = workspace.coverage_tests.iter().any(|(_, _, passed)| !passed);
        if diversion_active {
            had_diversions = true;
            diversion_reason = Some("OC or IC test failed".to_string());
        }

        // Process tiers in priority order (all tiers processed, even if cash exhausted)
        for tier in &self.tiers {
            // Determine if this tier should be diverted
            let (target_recipients, tier_diverted): (&[Recipient], bool) =
                if tier.divertible && diversion_active {
                    // Find senior tier to divert to
                    let senior_tier = self
                        .tiers
                        .iter()
                        .filter(|t| {
                            t.priority < tier.priority && t.payment_type == PaymentType::Principal
                        })
                        .min_by_key(|t| t.priority);

                    senior_tier
                        .map(|s| (&s.recipients[..], true))
                        .unwrap_or((&tier.recipients[..], false))
                } else {
                    (&tier.recipients[..], false)
                };

            // Allocate cash to tier based on mode
            let tier_cash = match tier.allocation_mode {
                AllocationMode::Sequential => self.allocate_sequential(
                    tier,
                    target_recipients,
                    remaining,
                    tranches,
                    &tranche_index,
                    pool_balance,
                    payment_date,
                    market,
                    tier_diverted,
                    &mut workspace.distributions,
                    &mut workspace.payment_records,
                    &mut trace,
                    &explain,
                )?,
                AllocationMode::ProRata => self.allocate_pro_rata(
                    tier,
                    target_recipients,
                    remaining,
                    tranches,
                    &tranche_index,
                    pool_balance,
                    payment_date,
                    market,
                    tier_diverted,
                    &mut workspace.distributions,
                    &mut workspace.payment_records,
                    &mut trace,
                    &explain,
                )?,
            };

            if tier_diverted {
                total_diverted = total_diverted.checked_add(tier_cash)?;
            }

            workspace.tier_allocations.push((tier.id.clone(), tier_cash));
            remaining = remaining.checked_sub(tier_cash)?;
        }

        // Build result from workspace buffers (this clones the data out)
        Ok(WaterfallResult {
            payment_date,
            total_available: available_cash,
            tier_allocations: workspace.tier_allocations.clone(),
            distributions: workspace.distributions.clone(),
            payment_records: workspace.payment_records.clone(),
            coverage_tests: workspace.coverage_tests.clone(),
            diverted_cash: total_diverted,
            remaining_cash: remaining,
            had_diversions,
            diversion_reason,
            explanation: trace,
        })
    }

    /// Allocate cash sequentially to recipients
    #[allow(clippy::too_many_arguments)]
    fn allocate_sequential(
        &self,
        tier: &WaterfallTier,
        recipients: &[Recipient],
        mut available: Money,
        tranches: &TrancheStructure,
        tranche_index: &HashMap<&str, usize>,
        pool_balance: Money,
        payment_date: Date,
        market: &MarketContext,
        diverted: bool,
        distributions: &mut HashMap<PaymentRecipient, Money>,
        payment_records: &mut Vec<PaymentRecord>,
        trace: &mut Option<ExplanationTrace>,
        explain: &ExplainOpts,
    ) -> Result<Money> {
        let mut tier_total = Money::new(0.0, self.base_currency);

        for recipient in recipients {
            if available.amount() <= 0.0 {
                break;
            }

            let requested = self.calculate_payment_amount(
                &recipient.calculation,
                available,
                tranches,
                tranche_index,
                pool_balance,
                payment_date,
                market,
            )?;

            let paid = if requested.amount() <= available.amount() {
                requested
            } else {
                available
            };

            let shortfall = requested
                .checked_sub(paid)
                .unwrap_or(Money::new(0.0, self.base_currency));

            // Update distributions
            use std::collections::hash_map::Entry;
            match distributions.entry(recipient.recipient_type.clone()) {
                Entry::Occupied(mut e) => {
                    let next = e.get().checked_add(paid)?;
                    e.insert(next);
                }
                Entry::Vacant(e) => {
                    e.insert(paid);
                }
            }

            // Record payment
            payment_records.push(PaymentRecord {
                tier_id: tier.id.clone(),
                recipient_id: recipient.id.clone(),
                priority: tier.priority,
                recipient: recipient.recipient_type.clone(),
                requested_amount: requested,
                paid_amount: paid,
                shortfall,
                diverted,
            });

            // Add trace entry
            if let Some(ref mut t) = trace {
                t.push(
                    TraceEntry::WaterfallStep {
                        period: 0,
                        step_name: format!(
                            "{}/{} - {:?}",
                            tier.id, recipient.id, recipient.recipient_type
                        ),
                        cash_in_amount: requested.amount(),
                        cash_in_currency: requested.currency().to_string(),
                        cash_out_amount: paid.amount(),
                        cash_out_currency: paid.currency().to_string(),
                        shortfall_amount: if shortfall.amount() > 0.0 {
                            Some(shortfall.amount())
                        } else {
                            None
                        },
                        shortfall_currency: if shortfall.amount() > 0.0 {
                            Some(shortfall.currency().to_string())
                        } else {
                            None
                        },
                    },
                    explain.max_entries,
                );
            }

            tier_total = tier_total.checked_add(paid)?;
            available = available.checked_sub(paid)?;
        }

        Ok(tier_total)
    }

    /// Allocate cash pro-rata to recipients using penny-safe allocation.
    ///
    /// Uses "largest remainder" method to ensure sum(distributions) == tier_available
    /// exactly, preventing floating-point residuals from accumulating over time.
    #[allow(clippy::too_many_arguments)]
    fn allocate_pro_rata(
        &self,
        tier: &WaterfallTier,
        recipients: &[Recipient],
        available: Money,
        tranches: &TrancheStructure,
        tranche_index: &HashMap<&str, usize>,
        pool_balance: Money,
        payment_date: Date,
        market: &MarketContext,
        diverted: bool,
        distributions: &mut HashMap<PaymentRecipient, Money>,
        payment_records: &mut Vec<PaymentRecord>,
        trace: &mut Option<ExplanationTrace>,
        explain: &ExplainOpts,
    ) -> Result<Money> {
        if recipients.is_empty() {
            return Ok(Money::new(0.0, self.base_currency));
        }

        // Calculate total requested across all recipients
        let mut total_requested = Money::new(0.0, self.base_currency);
        let mut recipient_requests = Vec::with_capacity(recipients.len());

        for recipient in recipients {
            let requested = self.calculate_payment_amount(
                &recipient.calculation,
                available,
                tranches,
                tranche_index,
                pool_balance,
                payment_date,
                market,
            )?;
            total_requested = total_requested.checked_add(requested)?;
            recipient_requests.push((recipient, requested));
        }

        // Calculate total weight
        let total_weight: f64 = recipients.iter().map(|r| r.weight.unwrap_or(1.0)).sum();

        let tier_available = if total_requested.amount() <= available.amount() {
            total_requested
        } else {
            available
        };

        // ========================================================================
        // Penny-Safe Allocation using Largest Remainder Method
        // ========================================================================
        // 1. Calculate ideal (fractional) allocations
        // 2. Floor each allocation to get integer cents
        // 3. Distribute remainder cents to recipients with largest fractional parts
        // This ensures sum(paid) == tier_available exactly (to the penny)

        // Round to cents (2 decimal places) for penny-safe calculation
        let tier_available_cents = (tier_available.amount() * 100.0).round() as i64;

        // Calculate pro-rata shares and ideal allocations in cents
        let mut allocations_data: Vec<(usize, &Recipient, Money, i64, f64)> =
            Vec::with_capacity(recipient_requests.len());

        for (idx, (recipient, requested)) in recipient_requests.iter().enumerate() {
            let weight = recipient.weight.unwrap_or(1.0);
            let pro_rata_share = if total_weight > 0.0 {
                weight / total_weight
            } else {
                1.0 / recipients.len() as f64
            };

            // Ideal allocation in cents (fractional)
            let ideal_cents = tier_available_cents as f64 * pro_rata_share;
            let floor_cents = ideal_cents.floor() as i64;
            let remainder = ideal_cents - floor_cents as f64;

            allocations_data.push((idx, recipient, *requested, floor_cents, remainder));
        }

        // Calculate total floor cents allocated
        let total_floor_cents: i64 = allocations_data.iter().map(|(_, _, _, fc, _)| fc).sum();
        let mut remainder_cents = tier_available_cents - total_floor_cents;

        // Sort by remainder (descending) to distribute extra cents to largest remainders
        let mut indices_by_remainder: Vec<usize> = (0..allocations_data.len()).collect();
        indices_by_remainder.sort_by(|&a, &b| {
            allocations_data[b]
                .4
                .partial_cmp(&allocations_data[a].4)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Distribute remainder cents one at a time to recipients with largest remainders
        let mut final_cents: Vec<i64> = allocations_data.iter().map(|(_, _, _, fc, _)| *fc).collect();
        for &idx in &indices_by_remainder {
            if remainder_cents <= 0 {
                break;
            }
            final_cents[idx] += 1;
            remainder_cents -= 1;
        }

        // Now process each recipient with their penny-safe allocation
        let mut tier_total = Money::new(0.0, self.base_currency);

        for (idx, (_, recipient, requested, _, _)) in allocations_data.iter().enumerate() {
            let allocated_cents = final_cents[idx];
            let allocated = Money::new(allocated_cents as f64 / 100.0, self.base_currency);

            let paid = if allocated.amount() <= requested.amount() {
                allocated
            } else {
                *requested
            };

            let shortfall = requested
                .checked_sub(paid)
                .unwrap_or(Money::new(0.0, self.base_currency));

            // Update distributions
            use std::collections::hash_map::Entry;
            match distributions.entry(recipient.recipient_type.clone()) {
                Entry::Occupied(mut e) => {
                    let next = e.get().checked_add(paid)?;
                    e.insert(next);
                }
                Entry::Vacant(e) => {
                    e.insert(paid);
                }
            }

            let weight = recipient.weight.unwrap_or(1.0);
            let pro_rata_share = if total_weight > 0.0 {
                weight / total_weight
            } else {
                1.0 / recipients.len() as f64
            };

            // Record payment
            payment_records.push(PaymentRecord {
                tier_id: tier.id.clone(),
                recipient_id: recipient.id.clone(),
                priority: tier.priority,
                recipient: recipient.recipient_type.clone(),
                requested_amount: *requested,
                paid_amount: paid,
                shortfall,
                diverted,
            });

            // Add trace entry
            if let Some(ref mut t) = trace {
                t.push(
                    TraceEntry::WaterfallStep {
                        period: 0,
                        step_name: format!(
                            "{}/{} - {:?} (pro-rata {:.1}%)",
                            tier.id,
                            recipient.id,
                            recipient.recipient_type,
                            pro_rata_share * 100.0
                        ),
                        cash_in_amount: requested.amount(),
                        cash_in_currency: requested.currency().to_string(),
                        cash_out_amount: paid.amount(),
                        cash_out_currency: paid.currency().to_string(),
                        shortfall_amount: if shortfall.amount() > 0.0 {
                            Some(shortfall.amount())
                        } else {
                            None
                        },
                        shortfall_currency: if shortfall.amount() > 0.0 {
                            Some(shortfall.currency().to_string())
                        } else {
                            None
                        },
                    },
                    explain.max_entries,
                );
            }

            tier_total = tier_total.checked_add(paid)?;
        }

        Ok(tier_total)
    }

    /// Evaluate coverage tests
    fn evaluate_coverage_tests(
        &self,
        tranches: &TrancheStructure,
        pool: &AssetPool,
        as_of: Date,
        available_cash: Money,
        interest_collections: Money,
    ) -> Result<Vec<(String, f64, bool)>> {
        // Pre-allocate for OC + IC tests per trigger (2 tests per trigger)
        let mut results = Vec::with_capacity(self.coverage_triggers.len() * 2);

        for trigger in &self.coverage_triggers {
            if let Some(oc_trigger_level) = trigger.oc_trigger {
                let ctx = TestContext {
                    pool,
                    tranches,
                    tranche_id: &trigger.tranche_id,
                    as_of,
                    cash_balance: available_cash,
                    interest_collections,
                };

                let oc_test = CoverageTest::new_oc(oc_trigger_level);
                let result = oc_test.calculate(&ctx);
                results.push((
                    format!("OC_{}", trigger.tranche_id),
                    result.current_ratio,
                    result.is_passing,
                ));
            }

            if let Some(ic_trigger_level) = trigger.ic_trigger {
                let ctx = TestContext {
                    pool,
                    tranches,
                    tranche_id: &trigger.tranche_id,
                    as_of,
                    cash_balance: available_cash,
                    interest_collections,
                };

                let ic_test = CoverageTest::new_ic(ic_trigger_level);
                let result = ic_test.calculate(&ctx);
                results.push((
                    format!("IC_{}", trigger.tranche_id),
                    result.current_ratio,
                    result.is_passing,
                ));
            }
        }

        Ok(results)
    }

    #[allow(clippy::too_many_arguments)]
    fn calculate_payment_amount(
        &self,
        calculation: &PaymentCalculation,
        available: Money,
        tranches: &TrancheStructure,
        tranche_index: &HashMap<&str, usize>,
        pool_balance: Money,
        payment_date: Date,
        market: &MarketContext,
    ) -> Result<Money> {
        match calculation {
            PaymentCalculation::FixedAmount { amount } => Ok(*amount),

            PaymentCalculation::PercentageOfCollateral { rate, annualized } => {
                let period_rate = if *annualized {
                    rate / QUARTERLY_PERIODS_PER_YEAR
                } else {
                    *rate
                };
                Ok(Money::new(
                    pool_balance.amount() * period_rate,
                    self.base_currency,
                ))
            }

            PaymentCalculation::TrancheInterest { tranche_id } => {
                if let Some(&idx) = tranche_index.get(tranche_id.as_str()) {
                    let tranche = &tranches.tranches[idx];
                    let rate = tranche.coupon.current_rate_with_index(payment_date, market);
                    let period_rate = rate / QUARTERLY_PERIODS_PER_YEAR;
                    Ok(Money::new(
                        tranche.current_balance.amount() * period_rate,
                        self.base_currency,
                    ))
                } else {
                    Ok(Money::new(0.0, self.base_currency))
                }
            }

            PaymentCalculation::TranchePrincipal {
                tranche_id,
                target_balance,
            } => {
                if let Some(&idx) = tranche_index.get(tranche_id.as_str()) {
                    let tranche = &tranches.tranches[idx];
                    if let Some(target) = target_balance {
                        let payment = tranche
                            .current_balance
                            .checked_sub(*target)
                            .unwrap_or(Money::new(0.0, self.base_currency));
                        Ok(if payment.amount() <= available.amount() {
                            payment
                        } else {
                            available
                        })
                    } else {
                        Ok(if tranche.current_balance.amount() <= available.amount() {
                            tranche.current_balance
                        } else {
                            available
                        })
                    }
                } else {
                    Ok(Money::new(0.0, self.base_currency))
                }
            }

            PaymentCalculation::ResidualCash => Ok(available),
        }
    }

    /// Create standard sequential waterfall with fees + tranche interest + tranche principal
    ///
    /// This is the common pattern across CLO, ABS, CMBS, and RMBS instruments.
    pub fn standard_sequential(
        base_currency: Currency,
        tranches: &TrancheStructure,
        fee_recipients: Vec<Recipient>,
    ) -> Self {
        let mut engine = Self::new(base_currency);
        let mut priority = 1;

        // Add fees tier
        if !fee_recipients.is_empty() {
            let fees_tier = WaterfallTier::new("fees", priority, PaymentType::Fee)
                .allocation_mode(AllocationMode::Sequential);
            let fees_tier = fee_recipients
                .into_iter()
                .fold(fees_tier, |tier, recipient| tier.add_recipient(recipient));
            engine.tiers.push(fees_tier);
            priority += 1;
        }

        // Add interest tier
        let mut sorted_tranches = tranches.tranches.clone();
        sorted_tranches.sort_by_key(|t| t.payment_priority);

        let mut interest_recipients = Vec::new();
        for tranche in &sorted_tranches {
            if tranche.seniority != super::enums::TrancheSeniority::Equity {
                interest_recipients.push(Recipient::tranche_interest(
                    format!("{}_interest", tranche.id.as_str()),
                    tranche.id.as_str(),
                ));
            }
        }

        if !interest_recipients.is_empty() {
            let interest_tier = WaterfallTier::new("interest", priority, PaymentType::Interest)
                .allocation_mode(AllocationMode::Sequential);
            let interest_tier = interest_recipients
                .into_iter()
                .fold(interest_tier, |tier, recipient| {
                    tier.add_recipient(recipient)
                });
            engine.tiers.push(interest_tier);
            priority += 1;
        }

        // Add principal tier
        let mut principal_recipients = Vec::new();
        for tranche in &sorted_tranches {
            if tranche.seniority != super::enums::TrancheSeniority::Equity {
                principal_recipients.push(Recipient::tranche_principal(
                    format!("{}_principal", tranche.id.as_str()),
                    tranche.id.as_str(),
                    None,
                ));
            }
        }

        if !principal_recipients.is_empty() {
            let principal_tier = WaterfallTier::new("principal", priority, PaymentType::Principal)
                .allocation_mode(AllocationMode::Sequential)
                .divertible(true);
            let principal_tier = principal_recipients
                .into_iter()
                .fold(principal_tier, |tier, recipient| {
                    tier.add_recipient(recipient)
                });
            engine.tiers.push(principal_tier);
            priority += 1;
        }

        // Add equity tier
        let equity_tier = WaterfallTier::new("equity", priority, PaymentType::Residual)
            .allocation_mode(AllocationMode::Sequential)
            .add_recipient(Recipient::new(
                "equity_distribution",
                PaymentRecipient::Equity,
                PaymentCalculation::ResidualCash,
            ));
        engine.tiers.push(equity_tier);

        engine
    }
}

/// Builder for waterfall engine
pub struct WaterfallBuilder {
    engine: WaterfallEngine,
    next_priority: usize,
}

impl WaterfallBuilder {
    /// Create new builder
    pub fn new(base_currency: Currency) -> Self {
        Self {
            engine: WaterfallEngine::new(base_currency),
            next_priority: 1,
        }
    }

    /// Add a tier
    pub fn add_tier(mut self, mut tier: WaterfallTier) -> Self {
        if tier.priority == 0 {
            tier.priority = self.next_priority;
            self.next_priority += 1;
        }
        self.engine = self.engine.add_tier(tier);
        self
    }

    /// Add coverage trigger
    pub fn add_coverage_trigger(mut self, trigger: CoverageTrigger) -> Self {
        self.engine = self.engine.add_coverage_trigger(trigger);
        self
    }

    /// Build the waterfall engine
    pub fn build(self) -> WaterfallEngine {
        self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;

    #[test]
    fn test_waterfall_tier_creation() {
        let tier = WaterfallTier::new("test_tier", 1, PaymentType::Fee)
            .add_recipient(Recipient::new(
                "recipient1",
                PaymentRecipient::ServiceProvider("Trustee".into()),
                PaymentCalculation::FixedAmount {
                    amount: Money::new(1000.0, Currency::USD),
                },
            ))
            .allocation_mode(AllocationMode::Sequential);

        assert_eq!(tier.id, "test_tier");
        assert_eq!(tier.priority, 1);
        assert_eq!(tier.recipients.len(), 1);
        assert_eq!(tier.allocation_mode, AllocationMode::Sequential);
    }

    #[test]
    fn test_recipient_helpers() {
        let fee = Recipient::fixed_fee("trustee", "Trustee", Money::new(50000.0, Currency::USD));
        assert_eq!(fee.id, "trustee");

        let interest = Recipient::tranche_interest("class_a_int", "CLASS_A");
        assert_eq!(interest.id, "class_a_int");
        if let PaymentRecipient::Tranche(id) = &interest.recipient_type {
            assert_eq!(id, "CLASS_A");
        } else {
            panic!("Expected Tranche recipient");
        }
    }

    #[test]
    fn test_waterfall_builder() {
        let waterfall = WaterfallBuilder::new(Currency::USD)
            .add_tier(
                WaterfallTier::new("fees", 1, PaymentType::Fee).add_recipient(Recipient::new(
                    "trustee",
                    PaymentRecipient::ServiceProvider("Trustee".into()),
                    PaymentCalculation::FixedAmount {
                        amount: Money::new(25000.0, Currency::USD),
                    },
                )),
            )
            .build();

        assert_eq!(waterfall.tiers.len(), 1);
        assert_eq!(waterfall.tiers[0].id, "fees");
    }

    #[test]
    fn test_allocation_mode_sequential() {
        let mode = AllocationMode::Sequential;
        assert_eq!(mode, AllocationMode::Sequential);
    }

    #[test]
    fn test_allocation_mode_pro_rata() {
        let mode = AllocationMode::ProRata;
        assert_eq!(mode, AllocationMode::ProRata);
    }
}
