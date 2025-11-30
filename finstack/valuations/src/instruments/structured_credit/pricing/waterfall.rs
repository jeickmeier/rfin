//! Waterfall execution functions for structured credit instruments.
//!
//! This module contains pure functions for executing waterfall distributions.
//! All type definitions are in `types::waterfall`.

use super::coverage_tests::{CoverageTest, TestContext};
use crate::instruments::structured_credit::types::constants::QUARTERLY_PERIODS_PER_YEAR;
use crate::instruments::structured_credit::types::{
    AllocationMode, PaymentCalculation, PaymentRecord, PaymentType, Pool, Recipient, RecipientType,
    TrancheStructure, Waterfall, WaterfallDistribution, WaterfallTier, WaterfallWorkspace,
};
use crate::instruments::structured_credit::utils::frequency_periods_per_year;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::explain::{ExplainOpts, ExplanationTrace, TraceEntry};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use std::collections::HashMap;

// ============================================================================
// CURRENCY PRECISION HELPERS
// ============================================================================

/// Returns the number of decimal places for currency-aware penny-safe allocation.
#[inline]
fn currency_decimal_places(currency: Currency) -> u32 {
    match currency {
        Currency::JPY => 0,
        _ => 2,
    }
}

/// Returns the scaling factor for converting amounts to smallest currency units.
#[inline]
fn currency_scale_factor(currency: Currency) -> f64 {
    let decimals = currency_decimal_places(currency);
    10_f64.powi(decimals as i32)
}

// ============================================================================
// MAIN EXECUTION FUNCTIONS
// ============================================================================

/// Execute waterfall to distribute available cash.
#[allow(clippy::too_many_arguments)]
pub fn execute_waterfall(
    waterfall: &Waterfall,
    available_cash: Money,
    interest_collections: Money,
    payment_date: Date,
    tranches: &TrancheStructure,
    pool_balance: Money,
    pool: &Pool,
    market: &MarketContext,
) -> Result<WaterfallDistribution> {
    execute_waterfall_with_explanation(
        waterfall,
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

/// Execute waterfall with optional explanation trace.
#[allow(clippy::too_many_arguments)]
pub fn execute_waterfall_with_explanation(
    waterfall: &Waterfall,
    available_cash: Money,
    interest_collections: Money,
    payment_date: Date,
    tranches: &TrancheStructure,
    pool_balance: Money,
    pool: &Pool,
    market: &MarketContext,
    explain: ExplainOpts,
) -> Result<WaterfallDistribution> {
    let mut remaining = available_cash;
    let mut tier_allocations = Vec::with_capacity(waterfall.tiers.len());
    let estimated_recipients = waterfall
        .tiers
        .iter()
        .map(|t| t.recipients.len())
        .sum::<usize>();
    let mut distributions: HashMap<RecipientType, Money> =
        HashMap::with_capacity(estimated_recipients);
    let mut payment_records = Vec::with_capacity(estimated_recipients);
    let mut total_diverted = Money::new(0.0, waterfall.base_currency);
    let mut had_diversions = false;
    let mut diversion_reason = None;

    let mut trace = if explain.enabled {
        Some(ExplanationTrace::new("waterfall"))
    } else {
        None
    };

    // Build tranche index for O(1) lookup by id
    let mut tranche_index: HashMap<&str, usize> = HashMap::with_capacity(tranches.tranches.len());
    for (i, t) in tranches.tranches.iter().enumerate() {
        tranche_index.insert(t.id.as_str(), i);
    }

    // Evaluate coverage tests
    let coverage_test_results = evaluate_coverage_tests(
        waterfall,
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

    // Process tiers in priority order
    for tier in &waterfall.tiers {
        let (target_recipients, tier_diverted): (&[Recipient], bool) =
            if tier.divertible && diversion_active {
                let senior_tier = waterfall
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

        let tier_cash = match tier.allocation_mode {
            AllocationMode::Sequential => allocate_sequential(
                waterfall.base_currency,
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
            AllocationMode::ProRata => allocate_pro_rata(
                waterfall.base_currency,
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

    Ok(WaterfallDistribution {
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
#[allow(clippy::too_many_arguments)]
pub fn execute_waterfall_with_workspace(
    waterfall: &Waterfall,
    available_cash: Money,
    interest_collections: Money,
    payment_date: Date,
    tranches: &TrancheStructure,
    pool_balance: Money,
    pool: &Pool,
    market: &MarketContext,
    explain: ExplainOpts,
    workspace: &mut WaterfallWorkspace,
) -> Result<WaterfallDistribution> {
    let mut remaining = available_cash;
    let mut total_diverted = Money::new(0.0, waterfall.base_currency);
    let mut had_diversions = false;
    let mut diversion_reason = None;

    let mut trace = if explain.enabled {
        Some(ExplanationTrace::new("waterfall"))
    } else {
        None
    };

    let tranche_index: HashMap<&str, usize> = workspace
        .tranche_index
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    // Evaluate coverage tests into workspace buffer
    workspace.coverage_tests.clear();
    let coverage_test_results = evaluate_coverage_tests(
        waterfall,
        tranches,
        pool,
        payment_date,
        available_cash,
        interest_collections,
    )?;
    workspace
        .coverage_tests
        .extend(coverage_test_results.iter().cloned());

    let diversion_active = workspace
        .coverage_tests
        .iter()
        .any(|(_, _, passed)| !passed);
    if diversion_active {
        had_diversions = true;
        diversion_reason = Some("OC or IC test failed".to_string());
    }

    for tier in &waterfall.tiers {
        let (target_recipients, tier_diverted): (&[Recipient], bool) =
            if tier.divertible && diversion_active {
                let senior_tier = waterfall
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

        let tier_cash = match tier.allocation_mode {
            AllocationMode::Sequential => allocate_sequential(
                waterfall.base_currency,
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
            AllocationMode::ProRata => allocate_pro_rata(
                waterfall.base_currency,
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

        workspace
            .tier_allocations
            .push((tier.id.clone(), tier_cash));
        remaining = remaining.checked_sub(tier_cash)?;
    }

    Ok(WaterfallDistribution {
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

// ============================================================================
// ALLOCATION FUNCTIONS
// ============================================================================

/// Allocate cash sequentially to recipients.
#[allow(clippy::too_many_arguments)]
fn allocate_sequential(
    base_currency: Currency,
    tier: &WaterfallTier,
    recipients: &[Recipient],
    mut available: Money,
    tranches: &TrancheStructure,
    tranche_index: &HashMap<&str, usize>,
    pool_balance: Money,
    payment_date: Date,
    market: &MarketContext,
    diverted: bool,
    distributions: &mut HashMap<RecipientType, Money>,
    payment_records: &mut Vec<PaymentRecord>,
    trace: &mut Option<ExplanationTrace>,
    explain: &ExplainOpts,
) -> Result<Money> {
    let mut tier_total = Money::new(0.0, base_currency);

    for recipient in recipients {
        if available.amount() <= 0.0 {
            break;
        }

        let requested = calculate_payment_amount(
            base_currency,
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
            .unwrap_or(Money::new(0.0, base_currency));

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
#[allow(clippy::too_many_arguments)]
fn allocate_pro_rata(
    base_currency: Currency,
    tier: &WaterfallTier,
    recipients: &[Recipient],
    available: Money,
    tranches: &TrancheStructure,
    tranche_index: &HashMap<&str, usize>,
    pool_balance: Money,
    payment_date: Date,
    market: &MarketContext,
    diverted: bool,
    distributions: &mut HashMap<RecipientType, Money>,
    payment_records: &mut Vec<PaymentRecord>,
    trace: &mut Option<ExplanationTrace>,
    explain: &ExplainOpts,
) -> Result<Money> {
    if recipients.is_empty() {
        return Ok(Money::new(0.0, base_currency));
    }

    // Calculate total requested across all recipients
    let mut total_requested = Money::new(0.0, base_currency);
    let mut recipient_requests = Vec::with_capacity(recipients.len());

    for recipient in recipients {
        let requested = calculate_payment_amount(
            base_currency,
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

    let total_weight: f64 = recipients.iter().map(|r| r.weight.unwrap_or(1.0)).sum();

    let tier_available = if total_requested.amount() <= available.amount() {
        total_requested
    } else {
        available
    };

    // Penny-safe allocation using largest remainder method
    let scale = currency_scale_factor(base_currency);
    let tier_available_units = (tier_available.amount() * scale).round() as i64;

    let mut allocations_data: Vec<(usize, &Recipient, Money, i64, f64)> =
        Vec::with_capacity(recipient_requests.len());

    for (idx, (recipient, requested)) in recipient_requests.iter().enumerate() {
        let weight = recipient.weight.unwrap_or(1.0);
        let pro_rata_share = if total_weight > 0.0 {
            weight / total_weight
        } else {
            1.0 / recipients.len() as f64
        };

        let ideal_units = tier_available_units as f64 * pro_rata_share;
        let floor_units = ideal_units.floor() as i64;
        let remainder = ideal_units - floor_units as f64;

        allocations_data.push((idx, recipient, *requested, floor_units, remainder));
    }

    let total_floor_units: i64 = allocations_data.iter().map(|(_, _, _, fu, _)| fu).sum();
    let mut remainder_units = tier_available_units - total_floor_units;

    let mut indices_by_remainder: Vec<usize> = (0..allocations_data.len()).collect();
    indices_by_remainder.sort_by(|&a, &b| {
        allocations_data[b]
            .4
            .partial_cmp(&allocations_data[a].4)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut final_units: Vec<i64> = allocations_data
        .iter()
        .map(|(_, _, _, fu, _)| *fu)
        .collect();
    for &idx in &indices_by_remainder {
        if remainder_units <= 0 {
            break;
        }
        final_units[idx] += 1;
        remainder_units -= 1;
    }

    let mut tier_total = Money::new(0.0, base_currency);

    for (idx, (_, recipient, requested, _, _)) in allocations_data.iter().enumerate() {
        let allocated_units = final_units[idx];
        let allocated = Money::new(allocated_units as f64 / scale, base_currency);

        let paid = if allocated.amount() <= requested.amount() {
            allocated
        } else {
            *requested
        };

        let shortfall = requested
            .checked_sub(paid)
            .unwrap_or(Money::new(0.0, base_currency));

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

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Evaluate coverage tests.
fn evaluate_coverage_tests(
    waterfall: &Waterfall,
    tranches: &TrancheStructure,
    pool: &Pool,
    as_of: Date,
    available_cash: Money,
    interest_collections: Money,
) -> Result<Vec<(String, f64, bool)>> {
    let mut results = Vec::with_capacity(waterfall.coverage_triggers.len() * 2);

    for trigger in &waterfall.coverage_triggers {
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

/// Calculate payment amount for a recipient.
#[allow(clippy::too_many_arguments)]
fn calculate_payment_amount(
    base_currency: Currency,
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
                base_currency,
            ))
        }

        PaymentCalculation::TrancheInterest { tranche_id } => {
            if let Some(&idx) = tranche_index.get(tranche_id.as_str()) {
                let tranche = &tranches.tranches[idx];
                let rate = tranche.coupon.current_rate_with_index(payment_date, market);
                // Use tranche's actual payment frequency instead of hardcoded quarterly
                let periods_per_year = frequency_periods_per_year(tranche.payment_frequency);
                let period_rate = rate / periods_per_year;
                Ok(Money::new(
                    tranche.current_balance.amount() * period_rate,
                    base_currency,
                ))
            } else {
                Ok(Money::new(0.0, base_currency))
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
                        .unwrap_or(Money::new(0.0, base_currency));
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
                Ok(Money::new(0.0, base_currency))
            }
        }

        PaymentCalculation::ResidualCash => Ok(available),
    }
}

