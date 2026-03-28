//! CMO waterfall engine.
//!
//! This module implements the waterfall logic for distributing collateral
//! cashflows to CMO tranches according to their priority and type.

use super::tranches::pac_support::{allocate_pac_support, PacSchedule};
use super::types::{CmoTranche, CmoTrancheType, CmoWaterfall};
use finstack_core::money::Money;
use finstack_core::HashMap;

/// Cashflow allocation for a single period.
#[derive(Debug, Clone)]
pub struct TrancheAllocation {
    /// Tranche ID
    pub tranche_id: String,
    /// Principal allocated
    pub principal: f64,
    /// Scheduled principal allocated
    pub scheduled_principal: f64,
    /// Prepayment principal allocated
    pub prepayment_principal: f64,
    /// Interest allocated
    pub interest: f64,
    /// Beginning balance
    pub beginning_balance: f64,
    /// Ending balance
    pub ending_balance: f64,
}

/// Waterfall execution result for a single period.
#[derive(Debug, Clone)]
pub struct WaterfallPeriodResult {
    /// Allocations by tranche
    pub allocations: Vec<TrancheAllocation>,
    /// Total principal distributed
    pub total_principal: f64,
    /// Total scheduled principal distributed
    pub total_scheduled_principal: f64,
    /// Total prepayment principal distributed
    pub total_prepayment_principal: f64,
    /// Total interest distributed
    pub total_interest: f64,
    /// Residual principal (if any)
    pub residual_principal: f64,
    /// Residual interest (if any)
    pub residual_interest: f64,
}

/// Execute waterfall for a single period.
///
/// Distributes principal and interest from collateral to tranches
/// according to waterfall rules.
///
/// # Arguments
///
/// * `waterfall` - Waterfall configuration with tranches
/// * `available_principal` - Total principal available for distribution
/// * `available_interest` - Total interest available for distribution
///
/// # Returns
///
/// Waterfall execution result with allocations by tranche
/// Optional PAC context for waterfall execution.
#[derive(Debug, Clone, Default)]
pub struct PacContext {
    /// PAC schedule for scheduled payment lookup.
    pub schedule: Option<PacSchedule>,
    /// Current period index into the schedule.
    pub period_index: usize,
    /// Actual PSA speed for collar check.
    pub actual_psa: f64,
}

/// Execute waterfall for a single period (backward-compatible entry point).
pub fn execute_waterfall(
    waterfall: &mut CmoWaterfall,
    available_principal: f64,
    available_interest: f64,
) -> WaterfallPeriodResult {
    execute_waterfall_with_pac(waterfall, available_principal, available_interest, None)
}

/// Execute waterfall with optional PAC schedule context.
pub fn execute_waterfall_with_pac(
    waterfall: &mut CmoWaterfall,
    available_principal: f64,
    available_interest: f64,
    pac_context: Option<&PacContext>,
) -> WaterfallPeriodResult {
    execute_waterfall_with_principal_breakdown(
        waterfall,
        available_principal,
        0.0,
        available_interest,
        pac_context,
    )
}

/// Execute waterfall while preserving scheduled-principal vs prepayment buckets.
pub fn execute_waterfall_with_principal_breakdown(
    waterfall: &mut CmoWaterfall,
    scheduled_principal: f64,
    prepayment_principal: f64,
    available_interest: f64,
    pac_context: Option<&PacContext>,
) -> WaterfallPeriodResult {
    let mut allocations = Vec::new();
    let mut remaining_principal = scheduled_principal + prepayment_principal;
    let mut remaining_interest = available_interest;
    let mut remaining_scheduled_principal = scheduled_principal;
    let mut remaining_prepayment_principal = prepayment_principal;

    // First pass: distribute interest to interest-bearing tranches
    let _total_interest_notional: f64 = waterfall
        .tranches
        .iter()
        .filter(|t| t.is_interest_bearing())
        .map(|t| t.current_face.amount())
        .sum();

    let mut interest_allocations: HashMap<String, f64> = HashMap::default();

    for tranche in &waterfall.tranches {
        if tranche.is_interest_bearing() && tranche.current_face.amount() > 0.0 {
            // Interest = balance × coupon / 12
            let monthly_interest = tranche.current_face.amount() * tranche.coupon / 12.0;
            let allocated_interest = monthly_interest.min(remaining_interest);
            remaining_interest -= allocated_interest;
            interest_allocations.insert(tranche.id.clone(), allocated_interest);
        }
    }

    // Second pass: distribute principal based on tranche type and priority
    // Group tranches by priority
    let mut priority_groups: HashMap<u32, Vec<&CmoTranche>> = HashMap::default();
    for tranche in &waterfall.tranches {
        if tranche.receives_principal() {
            priority_groups
                .entry(tranche.priority)
                .or_default()
                .push(tranche);
        }
    }

    let mut priorities: Vec<u32> = priority_groups.keys().cloned().collect();
    priorities.sort();

    let mut principal_allocations: HashMap<String, f64> = HashMap::default();

    // Handle PO strips: they receive all principal before other tranches
    for tranche in &waterfall.tranches {
        if tranche.tranche_type == CmoTrancheType::PrincipalOnly && remaining_principal > 0.0 {
            let po_alloc = allocate_po_cashflow(tranche, remaining_principal);
            let actual_alloc = po_alloc.min(tranche.current_face.amount());
            principal_allocations.insert(tranche.id.clone(), actual_alloc);
            remaining_principal -= actual_alloc;
        }
    }

    for priority in priorities {
        if remaining_principal <= 0.0 {
            break;
        }

        // Priority groups are built from tranches above, so get() always succeeds
        if let Some(tranches) = priority_groups.get(&priority) {
            // Determine allocation mode for this priority group
            let allocation = allocate_principal_to_group(
                tranches,
                remaining_principal,
                waterfall.pro_rata_same_priority,
                pac_context,
            );

            for (id, amount) in allocation {
                remaining_principal -= amount;
                principal_allocations.insert(id, amount);
            }
        }
    }

    // Build final allocations and update tranche balances
    let mut total_principal = 0.0;
    let mut total_scheduled_principal = 0.0;
    let mut total_prepayment_principal = 0.0;
    let mut total_interest = 0.0;

    for tranche in &mut waterfall.tranches {
        let principal = principal_allocations
            .get(&tranche.id)
            .cloned()
            .unwrap_or(0.0);
        let interest = interest_allocations
            .get(&tranche.id)
            .cloned()
            .unwrap_or(0.0);
        let scheduled_principal = principal.min(remaining_scheduled_principal);
        remaining_scheduled_principal -= scheduled_principal;
        let prepayment_principal =
            (principal - scheduled_principal).min(remaining_prepayment_principal);
        remaining_prepayment_principal -= prepayment_principal;

        let beginning = tranche.current_face.amount();
        let ending = (beginning - principal).max(0.0);

        // Update tranche balance
        tranche.current_face = Money::new(ending, tranche.current_face.currency());

        allocations.push(TrancheAllocation {
            tranche_id: tranche.id.clone(),
            principal,
            scheduled_principal,
            prepayment_principal,
            interest,
            beginning_balance: beginning,
            ending_balance: ending,
        });

        total_principal += principal;
        total_scheduled_principal += scheduled_principal;
        total_prepayment_principal += prepayment_principal;
        total_interest += interest;
    }

    WaterfallPeriodResult {
        allocations,
        total_principal,
        total_scheduled_principal,
        total_prepayment_principal,
        total_interest,
        residual_principal: remaining_principal,
        residual_interest: remaining_interest,
    }
}

/// Allocate principal to a group of tranches at the same priority.
fn allocate_principal_to_group(
    tranches: &[&CmoTranche],
    available: f64,
    pro_rata: bool,
    pac_context: Option<&PacContext>,
) -> Vec<(String, f64)> {
    let mut allocations = Vec::new();
    let mut remaining = available;

    // Separate PAC from others
    let (pac_tranches, other_tranches): (Vec<&&CmoTranche>, Vec<&&CmoTranche>) = tranches
        .iter()
        .partition(|t| t.tranche_type == CmoTrancheType::Pac);

    // When PAC schedule is available, use proper PAC/Support allocation
    if let Some(ctx) = pac_context {
        if let Some(ref schedule) = ctx.schedule {
            let pac_balance: f64 = pac_tranches.iter().map(|t| t.current_face.amount()).sum();
            let support_balance: f64 = other_tranches.iter().map(|t| t.current_face.amount()).sum();
            let pac_scheduled = schedule.scheduled_at(ctx.period_index);

            let (pac_alloc, support_alloc) = allocate_pac_support(
                remaining,
                pac_balance,
                support_balance,
                pac_scheduled,
                ctx.actual_psa,
                &schedule.collar,
            );

            // Distribute PAC allocation pro-rata among PAC tranches
            if pac_balance > 0.0 && pac_alloc > 0.0 {
                for tranche in &pac_tranches {
                    let proportion = tranche.current_face.amount() / pac_balance;
                    let alloc = pac_alloc * proportion;
                    allocations.push((tranche.id.clone(), alloc));
                }
            }
            // Distribute support allocation among other tranches
            if support_alloc > 0.0 {
                let mut support_remaining = support_alloc;
                for tranche in &other_tranches {
                    if support_remaining <= 0.0 {
                        break;
                    }
                    let balance = tranche.current_face.amount();
                    let alloc = balance.min(support_remaining);
                    allocations.push((tranche.id.clone(), alloc));
                    support_remaining -= alloc;
                }
            }

            return allocations;
        }
    }

    // Fallback: balance-limited allocation when no PAC schedule is available
    for tranche in &pac_tranches {
        if remaining <= 0.0 {
            break;
        }
        let balance = tranche.current_face.amount();
        if balance <= 0.0 {
            continue;
        }
        let allocated = balance.min(remaining);
        allocations.push((tranche.id.clone(), allocated));
        remaining -= allocated;
    }

    // Support tranches absorb excess/shortfall
    // For sequential without PAC, just go in order
    if pro_rata {
        // Pro-rata allocation
        let total_balance: f64 = other_tranches.iter().map(|t| t.current_face.amount()).sum();

        if total_balance > 0.0 {
            for tranche in &other_tranches {
                let balance = tranche.current_face.amount();
                let proportion = balance / total_balance;
                let allocated = (remaining * proportion).min(balance);
                allocations.push((tranche.id.clone(), allocated));
            }
        }
    } else {
        // Sequential allocation (first tranche gets everything first)
        for tranche in &other_tranches {
            if remaining <= 0.0 {
                break;
            }
            let balance = tranche.current_face.amount();
            if balance <= 0.0 {
                continue;
            }
            let allocated = balance.min(remaining);
            allocations.push((tranche.id.clone(), allocated));
            remaining -= allocated;
        }
    }

    allocations
}

/// Allocate IO cashflows.
///
/// IO strips receive interest based on their notional and coupon,
/// but their notional decreases as the underlying pool pays down.
pub fn allocate_io_cashflow(io_tranche: &CmoTranche, collateral_factor: f64) -> f64 {
    // IO payment = notional × factor × coupon / 12
    let adjusted_notional = io_tranche.original_face.amount() * collateral_factor;
    adjusted_notional * io_tranche.coupon / 12.0
}

/// Allocate PO cashflows.
///
/// PO strips receive all principal payments (scheduled + prepay)
/// but no interest.
pub fn allocate_po_cashflow(_po_tranche: &CmoTranche, total_principal: f64) -> f64 {
    // PO gets all principal
    total_principal
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::fixed_income::cmo::types::CmoTranche;
    use finstack_core::currency::Currency;

    fn create_test_waterfall() -> CmoWaterfall {
        let tranches = vec![
            CmoTranche::sequential("A", Money::new(40_000.0, Currency::USD), 0.04, 1),
            CmoTranche::sequential("B", Money::new(30_000.0, Currency::USD), 0.05, 2),
            CmoTranche::sequential("C", Money::new(30_000.0, Currency::USD), 0.06, 3),
        ];

        CmoWaterfall::new(tranches)
    }

    #[test]
    fn test_sequential_waterfall() {
        let mut waterfall = create_test_waterfall();

        // Distribute 10,000 principal, enough interest
        let result = execute_waterfall(&mut waterfall, 10_000.0, 500.0);

        // A should get all principal (it's first priority)
        let a_alloc = result
            .allocations
            .iter()
            .find(|a| a.tranche_id == "A")
            .expect("A tranche allocation not found");
        assert!((a_alloc.principal - 10_000.0).abs() < 1.0);

        // B and C should get nothing yet
        let b_alloc = result
            .allocations
            .iter()
            .find(|a| a.tranche_id == "B")
            .expect("B tranche allocation not found");
        assert!(b_alloc.principal < 1.0);
    }

    #[test]
    fn test_waterfall_payoff_a() {
        let mut waterfall = create_test_waterfall();

        // Distribute enough to pay off A completely plus some to B
        let result = execute_waterfall(&mut waterfall, 50_000.0, 500.0);

        // A should be paid off
        let a_alloc = result
            .allocations
            .iter()
            .find(|a| a.tranche_id == "A")
            .expect("A tranche allocation not found");
        assert!((a_alloc.principal - 40_000.0).abs() < 1.0);
        assert!(a_alloc.ending_balance < 1.0);

        // B should get remaining
        let b_alloc = result
            .allocations
            .iter()
            .find(|a| a.tranche_id == "B")
            .expect("B tranche allocation not found");
        assert!((b_alloc.principal - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn test_interest_allocation() {
        let mut waterfall = create_test_waterfall();

        // Run waterfall with interest
        let result = execute_waterfall(&mut waterfall, 1_000.0, 500.0);

        // Each tranche should get monthly interest based on balance × coupon / 12
        let a_alloc = result
            .allocations
            .iter()
            .find(|a| a.tranche_id == "A")
            .expect("A tranche allocation not found");

        // A: 40,000 × 0.04 / 12 = 133.33
        assert!(a_alloc.interest > 100.0 && a_alloc.interest < 200.0);
    }

    #[test]
    fn test_io_allocation() {
        let io = CmoTranche::io_strip("IO", Money::new(100_000.0, Currency::USD), 0.04);

        // At 100% factor
        let payment = allocate_io_cashflow(&io, 1.0);
        // 100,000 × 0.04 / 12 = 333.33
        assert!((payment - 333.33).abs() < 1.0);

        // At 50% factor
        let payment_half = allocate_io_cashflow(&io, 0.5);
        assert!((payment_half - 166.67).abs() < 1.0);
    }
}
