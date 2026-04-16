//! Core types for credit event restructuring analysis.
//!
//! Defines the claim hierarchy, collateral allocation, and allocation
//! modes used across the recovery waterfall, exchange offer, and LME
//! modules.

use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

/// Seniority class in the bankruptcy/recovery priority stack.
///
/// Ordered from highest priority (first to be paid) to lowest.
/// Follows the standard US Chapter 11 absolute priority rule.
///
/// The discriminant ordering matches payment priority: lower
/// discriminant = higher priority. This enables `Ord`-based
/// sorting for waterfall execution.
///
/// # References
///
/// US Bankruptcy Code ss. 507, 1129(b) (absolute priority rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ClaimSeniority {
    /// Debtor-in-possession financing (super-priority administrative).
    DipFinancing,
    /// Administrative claims (professional fees, post-petition trade).
    Administrative,
    /// Priority claims (taxes, wages up to statutory cap).
    Priority,
    /// First-lien secured (with collateral allocation).
    FirstLienSecured,
    /// Second-lien secured.
    SecondLienSecured,
    /// Third-lien / junior secured.
    JuniorSecured,
    /// Senior unsecured (including deficiency claims from undersecured lenders).
    SeniorUnsecured,
    /// Senior subordinated.
    SeniorSubordinated,
    /// Subordinated.
    Subordinated,
    /// Mezzanine / deeply subordinated.
    Mezzanine,
    /// Preferred equity.
    PreferredEquity,
    /// Common equity.
    CommonEquity,
}

/// A single claim in the recovery waterfall.
///
/// Represents one creditor class's full claim against the estate,
/// including principal, accrued interest, and any penalties or
/// make-whole amounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// Unique identifier for this claim class.
    pub id: String,
    /// Human-readable label (e.g., "First Lien Term Loan B").
    pub label: String,
    /// Priority in the recovery waterfall.
    pub seniority: ClaimSeniority,
    /// Outstanding principal amount.
    pub principal: Money,
    /// Accrued and unpaid interest as of petition/valuation date.
    pub accrued_interest: Money,
    /// Make-whole premium, prepayment penalty, or other contractual damages.
    pub penalties: Money,
    /// Optional reference to the originating instrument.
    pub instrument_id: Option<String>,
    /// Collateral allocated to this claim (secured claims only).
    ///
    /// When set, recovery on this claim is first sourced from collateral
    /// value before participating in the general unsecured pool.
    pub collateral: Option<CollateralAllocation>,
    /// Allocation mode within this claim class when multiple holders exist.
    #[serde(default)]
    pub intra_class_allocation: AllocationMode,
}

impl Claim {
    /// Total claim amount (principal + accrued + penalties).
    ///
    /// Returns an error if currencies are mismatched across the three
    /// components.
    pub fn total_claim(&self) -> crate::Result<Money> {
        let sum = self
            .principal
            .checked_add(self.accrued_interest)
            .map_err(crate::Error::Core)?
            .checked_add(self.penalties)
            .map_err(crate::Error::Core)?;
        Ok(sum)
    }
}

/// Collateral backing a secured claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralAllocation {
    /// Description of collateral (e.g., "All domestic assets", "IP portfolio").
    pub description: String,
    /// Estimated collateral value as of valuation date.
    pub value: Money,
    /// Haircut applied to collateral value for recovery analysis (0.0 - 1.0).
    pub haircut: f64,
    /// Whether this collateral is shared with other claim classes.
    #[serde(default)]
    pub shared: bool,
    /// IDs of other claims sharing this collateral pool (empty if exclusive).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_with: Vec<String>,
}

impl CollateralAllocation {
    /// Net collateral value after haircut.
    pub fn net_value(&self) -> Money {
        Money::new(
            self.value.amount() * (1.0 - self.haircut),
            self.value.currency(),
        )
    }
}

/// How to allocate recoveries within a single claim class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AllocationMode {
    /// Distribute proportionally by claim size (standard for most classes).
    #[default]
    ProRata,
    /// Pay in strict order within the class (rare; some inter-creditor agreements).
    StrictPriority,
}
