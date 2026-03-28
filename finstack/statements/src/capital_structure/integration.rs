//! Capital Structure Integration Logic
//!
//! This module handles the integration between statements models and capital structure,
//! leveraging valuations infrastructure for cashflow aggregation and classification.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::evaluator::EvalWarning;
use crate::types::DebtInstrumentSpec;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::{fx::FxQuery, Money};
use finstack_valuations::cashflow::primitives::CFKind;
use finstack_valuations::cashflow::CashflowProvider;
use finstack_valuations::cashflow::{accrued_interest_amount, AccrualConfig};
use finstack_valuations::instruments::{Bond, InterestRateSwap, TermLoan};
use indexmap::IndexMap;
use serde::Deserialize;
use std::sync::Arc;

/// Snapshot date used for "end of period" quantities under half-open period semantics `[start, end)`.
///
/// We use `end - 1 day` so that cashflows dated exactly on `period.end` are attributed to the
/// *next* period and do not incorrectly affect the prior period's end-of-period balance/accrual.
fn period_snapshot_date(period: &Period) -> Date {
    if period.end <= period.start {
        // Defensive: periods should always have positive length, but clamp to start if malformed.
        return period.start;
    }
    period.end - time::Duration::days(1)
}

/// Calculate contractual flows for a single period.
///
/// This helper extracts flows for a specific period from an instrument's full schedule,
/// returning a CashflowBreakdown for that period. Used for dynamic period-by-period evaluation.
///
/// Periods are treated with half-open semantics `[start, end)`. End-of-period
/// balances and accruals are therefore snapped at `period.end - 1 day` so
/// cashflows occurring exactly on the next period boundary are not attributed
/// to the prior period.
///
/// # Arguments
///
/// * `instrument` - The instrument to calculate flows for
/// * `period` - The period to extract flows for
/// * `opening_balance` - Opening balance at the start of the period
/// * `market_ctx` - Market context for pricing
/// * `as_of` - Valuation date
///
/// # Returns
///
/// Returns a tuple of:
/// - [`CashflowBreakdown`] for the period
/// - closing balance after scheduled flows
/// - evaluation warnings for ignored or unsupported cashflow kinds
///
/// # Errors
///
/// Returns an error if the instrument schedule cannot be built, if currencies
/// are inconsistent, or if accrued interest cannot be computed.
///
/// # References
///
/// - Cashflow discounting and schedule context: `docs/REFERENCES.md#hull-options-futures`
/// - Fixed-income balance/risk interpretation: `docs/REFERENCES.md#tuckman-serrat-fixed-income`
pub fn calculate_period_flows(
    instrument: &dyn CashflowProvider,
    period: &Period,
    opening_balance: Money,
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<(CashflowBreakdown, Money, Vec<EvalWarning>)> {
    let full_schedule = instrument.cashflow_schedule(market_ctx, as_of)?;
    let currency = full_schedule.notional.initial.currency();
    if opening_balance.amount() != 0.0 && opening_balance.currency() != currency {
        return Err(crate::error::Error::currency_mismatch(
            currency,
            opening_balance.currency(),
        ));
    }
    let mut breakdown = CashflowBreakdown::with_currency(currency);
    let mut warnings = Vec::new();
    let snapshot_date = period_snapshot_date(period);
    let outstanding_path = full_schedule.outstanding_by_date()?;
    let scheduled_opening = outstanding_path
        .iter()
        .filter(|(d, _)| *d <= period.start)
        .map(|(_, balance)| {
            if balance.amount() < 0.0 {
                Money::new(-balance.amount(), balance.currency())
            } else {
                *balance
            }
        })
        .next_back()
        .unwrap_or(full_schedule.notional.initial);

    // Use opening balance to scale cashflows when the schedule notional differs from the
    // stateful outstanding (e.g., after applying sweeps). This is an approximation but
    // prevents obviously overstated interest after large paydowns.
    let scale = if opening_balance.amount() == 0.0 {
        if scheduled_opening.amount() == 0.0 {
            1.0
        } else {
            0.0
        }
    } else if scheduled_opening.amount() == 0.0 {
        1.0
    } else {
        opening_balance.amount() / scheduled_opening.amount()
    };

    // Extract flows that fall within this period
    for cf in &full_schedule.flows {
        if cf.date >= period.start && cf.date < period.end {
            let scaled_abs_value =
                Money::new(cf.amount.amount().abs() * scale, cf.amount.currency());

            match cf.kind {
                CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                    breakdown.interest_expense_cash += scaled_abs_value;
                }
                CFKind::Amortization => {
                    breakdown.principal_payment += scaled_abs_value;
                }
                CFKind::PrePayment | CFKind::RevolvingRepayment => {
                    breakdown.principal_payment += scaled_abs_value;
                }
                CFKind::Notional if cf.amount.amount() > 0.0 => {
                    breakdown.principal_payment += scaled_abs_value;
                }
                CFKind::Fee | CFKind::CommitmentFee | CFKind::UsageFee | CFKind::FacilityFee => {
                    breakdown.fees += scaled_abs_value;
                }
                CFKind::PIK => {
                    breakdown.interest_expense_pik += scaled_abs_value;
                }
                CFKind::Notional | CFKind::RevolvingDraw => {
                    // Funding / draw events are not treated as scheduled principal payments in statements.
                }
                CFKind::DefaultedNotional | CFKind::Recovery => {
                    // Credit events are not modeled as part of standard debt service in statements.
                    warnings.push(EvalWarning::CapitalStructureCashflowIgnored {
                        period: period.id,
                        kind: format!("{:?}", cf.kind),
                        cashflow_date: cf.date.to_string(),
                    });
                    tracing::warn!(
                        "Ignoring credit-event CFKind={:?} for period flow calc (date={:?})",
                        cf.kind,
                        cf.date
                    );
                }
                _ => {
                    // CFKind is non-exhaustive; ignore unknown variants to avoid misclassification.
                    warnings.push(EvalWarning::CapitalStructureCashflowIgnored {
                        period: period.id,
                        kind: format!("{:?}", cf.kind),
                        cashflow_date: cf.date.to_string(),
                    });
                    tracing::warn!(
                        "Unhandled CFKind={:?} for period flow calc (date={:?}); ignoring",
                        cf.kind,
                        cf.date
                    );
                }
            }
        }
    }

    // Get closing balance from outstanding_by_date.
    // Find the most recent outstanding balance at or before period end.
    // Note: outstanding_path only has entries on dates when cashflows occur,
    // so we need to find the latest entry <= period.end to get the correct balance.
    let scheduled_closing_balance = outstanding_path
        .iter()
        .rev()
        .find(|(date, _)| *date <= snapshot_date)
        .map(|(_, balance)| {
            if balance.amount() < 0.0 {
                Money::new(-balance.amount(), balance.currency())
            } else {
                *balance
            }
        })
        .unwrap_or_else(|| {
            // If no outstanding entries yet, use initial notional from schedule
            // or fall back to opening balance adjusted by flows
            let initial = full_schedule.notional.initial;
            if initial.amount() < 0.0 {
                Money::new(-initial.amount(), initial.currency())
            } else {
                initial
            }
        });

    let has_new_funding = full_schedule.flows.iter().any(|cf| {
        (cf.date >= period.start
            && cf.date < period.end
            && matches!(cf.kind, CFKind::RevolvingDraw))
            || (cf.date >= period.start
                && cf.date < period.end
                && matches!(cf.kind, CFKind::Notional)
                && cf.amount.amount() <= 0.0)
    });
    let net_new_funding: f64 = full_schedule
        .flows
        .iter()
        .filter(|cf| cf.date >= period.start && cf.date < period.end)
        .filter_map(|cf| match cf.kind {
            CFKind::RevolvingDraw => Some(cf.amount.amount().abs()),
            CFKind::Notional if cf.amount.amount() <= 0.0 => Some(cf.amount.amount().abs()),
            _ => None,
        })
        .sum();
    let closing_balance = if opening_balance.amount() == 0.0 {
        if has_new_funding {
            Money::new(
                scheduled_closing_balance.amount().max(net_new_funding),
                currency,
            )
        } else {
            Money::new(0.0, currency)
        }
    } else {
        scheduled_closing_balance
    };
    breakdown.debt_balance = closing_balance;

    // Calculate accrued interest at period end
    // Note: detailed accrual config (day count, compounding) comes from the schedule itself
    let accrued_scalar =
        accrued_interest_amount(&full_schedule, snapshot_date, &AccrualConfig::default())?;
    let accrued_interest = if opening_balance.amount() == 0.0 && !has_new_funding {
        0.0
    } else {
        accrued_scalar * scale
    };
    breakdown.accrued_interest = Money::new(accrued_interest, currency);

    Ok((breakdown, closing_balance, warnings))
}

/// Aggregate cashflows from instruments by period using valuations infrastructure.
///
/// The integration leverages `cashflow_schedule()` for CFKind-aware classification and
/// `outstanding_by_date()` for accurate debt balances. Results are normalized into
/// [`CapitalStructureCashflows`] so downstream code (including the DSL via `cs.*`) can
/// consume totals or per-instrument breakdowns.
///
/// # Arguments
/// * `instruments` - Map of instrument IDs to `CashflowProvider` trait objects
/// * `periods` - Ordered list of model periods used for bucketing cashflows
/// * `market_ctx` - Market context containing curves and other pricing data
/// * `as_of` - Valuation date used when generating cashflows
///
/// # Example
///
/// ```rust,no_run
/// use finstack_statements::capital_structure::{aggregate_instrument_cashflows, CapitalStructureCashflows};
/// use finstack_statements::CapitalStructureSpec;
/// use finstack_core::dates::build_periods;
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_valuations::cashflow::CashflowProvider;
/// use indexmap::IndexMap;
/// use std::sync::Arc;
/// use time::macros::date;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let periods = build_periods("2025Q1..2025Q4", None)?.periods;
/// let instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> = IndexMap::new();
/// let spec = CapitalStructureSpec {
///     debt_instruments: vec![],
///     equity_instruments: vec![],
///     meta: IndexMap::new(),
///     reporting_currency: None,
///     fx_policy: None,
///     waterfall: None,
/// };
/// let cashflows: CapitalStructureCashflows =
///     aggregate_instrument_cashflows(&spec, &instruments, &periods, &MarketContext::new(), date!(2025-01-01))?;
/// assert!(cashflows.totals.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn aggregate_instrument_cashflows(
    spec: &crate::types::CapitalStructureSpec,
    instruments: &IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>>,
    periods: &[Period],
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<CapitalStructureCashflows> {
    let mut result = CapitalStructureCashflows::new();

    // Determine reporting currency: explicit override > FX pivot > set later if single-currency
    let fx_matrix = market_ctx.fx();
    let mut reporting_currency = spec.reporting_currency;
    if reporting_currency.is_none() {
        reporting_currency = fx_matrix.map(|fx| fx.config().pivot_currency);
    }
    // FX policy override (default CashflowDate)
    let fx_policy = spec
        .fx_policy
        .unwrap_or(finstack_core::money::fx::FxConversionPolicy::CashflowDate);

    // Initialize reporting totals if we know the reporting currency up-front
    let mut reporting_totals: Option<IndexMap<PeriodId, CashflowBreakdown>> = reporting_currency
        .map(|rc| {
            let mut map = IndexMap::new();
            for period in periods {
                map.insert(period.id, CashflowBreakdown::with_currency(rc));
            }
            map
        });

    // Always accumulate per-currency totals to avoid panics on mixed-currency portfolios
    let mut totals_by_currency: IndexMap<Currency, IndexMap<PeriodId, CashflowBreakdown>> =
        IndexMap::new();

    // Process each instrument
    for (instrument_id, instrument) in instruments {
        // Use enhanced cashflow_schedule() for precise CFKind classification
        let full_schedule = instrument.cashflow_schedule(market_ctx, as_of)?;

        // Determine currency from first cashflow (all cashflows should be same currency)
        let currency = full_schedule.notional.initial.currency();

        // Initialize period map for this instrument
        let mut instrument_periods: IndexMap<PeriodId, CashflowBreakdown> = IndexMap::new();
        for period in periods {
            instrument_periods.insert(period.id, CashflowBreakdown::with_currency(currency));
        }

        // Pre-initialize per-currency totals for this currency
        totals_by_currency.entry(currency).or_insert_with(|| {
            let mut map = IndexMap::new();
            for period in periods {
                map.insert(period.id, CashflowBreakdown::with_currency(currency));
            }
            map
        });

        // Classify cashflows using precise CFKind information (NO MORE HEURISTICS!)
        // Note: We use full_schedule.flows directly rather than aggregate_by_period because
        // we need access to CFKind metadata for precise classification, which is preserved
        // in the full schedule but lost in simple aggregation.
        for cf in &full_schedule.flows {
            if let Some(period_id) = periods
                .iter()
                .find(|p| cf.date >= p.start && cf.date < p.end)
                .map(|p| p.id)
            {
                if let Some(breakdown) = instrument_periods.get_mut(&period_id) {
                    // Keep as Money, convert to issuer perspective (absolute value)
                    let abs_value = if cf.amount.amount() < 0.0 {
                        finstack_core::money::Money::new(-cf.amount.amount(), cf.amount.currency())
                    } else {
                        cf.amount
                    };

                    let converted_abs = convert_to_reporting(
                        abs_value,
                        cf.date,
                        reporting_currency,
                        fx_matrix,
                        fx_policy,
                    )?;

                    match cf.kind {
                        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                            // Cash interest payments (coupons, floating resets)
                            breakdown.interest_expense_cash += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.interest_expense_cash += money;
                                }
                            }
                        }
                        CFKind::Amortization => {
                            // Principal amortization payments
                            breakdown.principal_payment += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.principal_payment += money;
                                }
                            }
                        }
                        CFKind::PrePayment | CFKind::RevolvingRepayment => {
                            // Principal repayments (unscheduled prepayments, revolving repayments)
                            breakdown.principal_payment += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.principal_payment += money;
                                }
                            }
                        }
                        CFKind::Notional if cf.amount.amount() > 0.0 => {
                            // Principal redemption (bullet payment)
                            breakdown.principal_payment += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.principal_payment += money;
                                }
                            }
                        }
                        CFKind::Fee
                        | CFKind::CommitmentFee
                        | CFKind::UsageFee
                        | CFKind::FacilityFee => {
                            // Commitment fees, facility fees, etc.
                            breakdown.fees += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.fees += money;
                                }
                            }
                        }
                        CFKind::PIK => {
                            // PIK (payment-in-kind) interest accrued but not paid in cash
                            // This increases the outstanding balance and is tracked separately
                            breakdown.interest_expense_pik += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                if let Some(total) = map.get_mut(&period_id) {
                                    total.interest_expense_pik += money;
                                }
                            }
                        }
                        CFKind::DefaultedNotional | CFKind::Recovery => {
                            // Credit events are not modeled as part of standard debt service in statements.
                            tracing::warn!(
                                "Ignoring credit-event CFKind={:?} in CS aggregation (instrument={}, date={:?})",
                                cf.kind,
                                instrument_id,
                                cf.date
                            );
                        }
                        CFKind::Notional if cf.amount.amount() <= 0.0 => {
                            // Negative notional flows (initial exchange) - typically netted against principal
                            // For simplicity, we ignore these as they represent the initial funding, not ongoing cashflows
                            // The debt_balance is tracked separately via outstanding_by_date()
                        }
                        CFKind::RevolvingDraw => {
                            // Funding / draws are not treated as principal payments in statements.
                            // The debt_balance is tracked separately via outstanding_by_date().
                        }
                        _ => {
                            // CFKind is non-exhaustive; ignore unknown variants to avoid misclassification.
                            tracing::warn!(
                                "Unhandled CFKind={:?} in CS aggregation (instrument={}, date={:?}); ignoring",
                                cf.kind,
                                instrument_id,
                                cf.date
                            );
                        }
                    }
                }
            }
        }

        // Build outstanding path for debt balance lookups.
        // Note: outstanding_path only has entries on dates when cashflows occur,
        // so we need to interpolate for periods without explicit entries.
        let outstanding_path = full_schedule.outstanding_by_date()?;

        // Calculate accrued interest and debt balance for ALL periods, not just
        // periods with cashflow entries. This ensures proper accrual accumulation
        // between coupon payment dates.
        for period in periods {
            let period_id = period.id;
            let snapshot_date = period_snapshot_date(period);

            if let Some(breakdown) = instrument_periods.get_mut(&period_id) {
                // Find the most recent outstanding balance at or before period end.
                // Use rev().find() to efficiently get the latest entry <= snapshot_date.
                let outstanding_at_period = outstanding_path
                    .iter()
                    .rev()
                    .find(|(date, _)| *date <= snapshot_date)
                    .map(|(_, amount)| *amount)
                    .unwrap_or(full_schedule.notional.initial);

                // Keep as Money, use absolute value for issuer perspective
                let issuer_balance = if outstanding_at_period.amount() < 0.0 {
                    finstack_core::money::Money::new(
                        -outstanding_at_period.amount(),
                        outstanding_at_period.currency(),
                    )
                } else {
                    outstanding_at_period
                };
                breakdown.debt_balance = issuer_balance;

                // Accrued interest is measured at the period snapshot date (`end - 1 day`)
                // to align with half-open `[start, end)` attribution.
                let accrued_scalar = accrued_interest_amount(
                    &full_schedule,
                    snapshot_date,
                    &AccrualConfig::default(),
                )?;
                let accrued_money = Money::new(accrued_scalar, currency);
                breakdown.accrued_interest = accrued_money;

                // Convert to reporting currency for totals
                if let (Some(map), Some(money)) = (
                    reporting_totals.as_mut(),
                    convert_to_reporting(
                        issuer_balance,
                        snapshot_date,
                        reporting_currency,
                        fx_matrix,
                        fx_policy,
                    )?,
                ) {
                    if let Some(total) = map.get_mut(&period_id) {
                        total.debt_balance += money;
                    }
                }

                if let (Some(map), Some(money)) = (
                    reporting_totals.as_mut(),
                    convert_to_reporting(
                        accrued_money,
                        snapshot_date,
                        reporting_currency,
                        fx_matrix,
                        fx_policy,
                    )?,
                ) {
                    if let Some(total) = map.get_mut(&period_id) {
                        total.accrued_interest += money;
                    }
                }
            }
        }

        // Store instrument's period breakdown
        result
            .by_instrument
            .insert(instrument_id.clone(), instrument_periods.clone());

        // Aggregate into totals (handling Money addition which returns Result)
        if let Some(currency_totals) = totals_by_currency.get_mut(&currency) {
            for (period_id, breakdown) in &instrument_periods {
                if let Some(total) = currency_totals.get_mut(period_id) {
                    total.interest_expense_cash += breakdown.interest_expense_cash;
                    total.interest_expense_pik += breakdown.interest_expense_pik;
                    total.principal_payment += breakdown.principal_payment;
                    total.debt_balance += breakdown.debt_balance;
                    total.fees += breakdown.fees;
                    total.accrued_interest += breakdown.accrued_interest;
                }
            }
        }
    }

    // Finalize totals and reporting currency selection
    result.totals_by_currency = totals_by_currency;

    if let Some(reporting_totals) = reporting_totals {
        result.reporting_currency = reporting_currency;
        result.totals = reporting_totals;
    } else if result.totals_by_currency.len() == 1 {
        if let Some((ccy, per_period)) = result.totals_by_currency.first() {
            result.reporting_currency = Some(*ccy);
            result.totals = per_period.clone();
        }
    }

    Ok(result)
}

/// Convert a money amount into the reporting currency when FX data is available.
pub(crate) fn convert_to_reporting(
    money: finstack_core::money::Money,
    on: Date,
    reporting_currency: Option<Currency>,
    fx_matrix: Option<&Arc<finstack_core::money::fx::FxMatrix>>,
    fx_policy: finstack_core::money::fx::FxConversionPolicy,
) -> Result<Option<finstack_core::money::Money>> {
    let Some(rc) = reporting_currency else {
        return Ok(None);
    };

    if rc == money.currency() {
        return Ok(Some(money));
    }

    let Some(fx) = fx_matrix else {
        return Err(crate::error::Error::capital_structure(format!(
            "Cannot convert {} to reporting currency {} on {}: no FX matrix present. \
             Supply FX in MarketContext (or remove reporting_currency / keep single-currency portfolios).",
            money.currency(),
            rc,
            on
        )));
    };

    let rate = fx
        .rate(FxQuery::with_policy(money.currency(), rc, on, fx_policy))?
        .rate;
    Ok(Some(finstack_core::money::Money::new(
        money.amount() * rate,
        rc,
    )))
}

/// Build a [`Bond`] instrument from a [`DebtInstrumentSpec`].
///
/// # Arguments
/// * `spec` - Debt instrument specification sourced from the model
///
/// # Errors
/// Returns an error when the payload cannot be deserialized as a `Bond`.
pub fn build_bond_from_spec(spec: &DebtInstrumentSpec) -> Result<Bond> {
    match spec {
        DebtInstrumentSpec::Bond {
            id,
            spec: json_spec,
        } => Bond::deserialize(json_spec)
            .map_err(|e| crate::error::Error::build(format!(
                "Failed to deserialize bond '{}': {}. Ensure the JSON spec matches the Bond structure.",
                id, e
            ))),
        _ => Err(crate::error::Error::build(
            "Expected Bond variant in DebtInstrumentSpec, but got a different variant"
        )),
    }
}

/// Build an [`InterestRateSwap`] instrument from a [`DebtInstrumentSpec`].
///
/// # Arguments
/// * `spec` - Debt instrument specification sourced from the model
///
/// # Errors
/// Returns an error when the payload cannot be deserialized as an `InterestRateSwap`.
pub fn build_swap_from_spec(spec: &DebtInstrumentSpec) -> Result<InterestRateSwap> {
    match spec {
        DebtInstrumentSpec::Swap {
            id,
            spec: json_spec,
        } => InterestRateSwap::deserialize(json_spec)
            .map_err(|e| crate::error::Error::build(format!(
                "Failed to deserialize swap '{}': {}. Ensure the JSON spec matches the InterestRateSwap structure.",
                id, e
            ))),
        _ => Err(crate::error::Error::build(
            "Expected Swap variant in DebtInstrumentSpec, but got a different variant"
        )),
    }
}

/// Build a [`TermLoan`] instrument from a [`DebtInstrumentSpec`].
///
/// # Arguments
/// * `spec` - Debt instrument specification sourced from the model
///
/// # Errors
/// Returns an error when the payload cannot be deserialized as a `TermLoan`.
pub fn build_term_loan_from_spec(spec: &DebtInstrumentSpec) -> Result<TermLoan> {
    match spec {
        DebtInstrumentSpec::TermLoan {
            id,
            spec: json_spec,
        } => TermLoan::deserialize(json_spec)
            .map_err(|e| crate::error::Error::build(format!(
                "Failed to deserialize term loan '{}': {}. Ensure the JSON spec matches the TermLoan structure.",
                id, e
            ))),
        _ => Err(crate::error::Error::build(
            "Expected TermLoan variant in DebtInstrumentSpec, but got a different variant"
        )),
    }
}

/// Build a concrete instrument from a [`DebtInstrumentSpec`].
///
/// Generic specs are attempted against a known set of instrument implementations
/// (bonds, swaps, deposits, FRAs, repos) and the first successful deserialization is used.
///
/// # Arguments
/// * `spec` - Debt instrument specification from the model
///
/// # Returns
/// A boxed [`CashflowProvider`] trait object ready for cashflow generation.
///
/// # Errors
/// Returns an error when the specification cannot be matched to any supported instrument type.
pub fn build_any_instrument_from_spec(
    spec: &DebtInstrumentSpec,
) -> Result<Arc<dyn CashflowProvider + Send + Sync>> {
    match spec {
        DebtInstrumentSpec::Bond { .. } => {
            let bond = build_bond_from_spec(spec)?;
            Ok(Arc::new(bond))
        }
        DebtInstrumentSpec::Swap { .. } => {
            let swap = build_swap_from_spec(spec)?;
            Ok(Arc::new(swap))
        }
        DebtInstrumentSpec::TermLoan { .. } => {
            let term_loan = build_term_loan_from_spec(spec)?;
            Ok(Arc::new(term_loan))
        }
        DebtInstrumentSpec::Generic {
            id,
            spec: json_spec,
        } => {
            // Try to deserialize as known types in order of likelihood and
            // surface a helpful error if none match.
            let mut attempts: Vec<String> = Vec::new();

            match Bond::deserialize(json_spec) {
                Ok(bond) => return Ok(Arc::new(bond)),
                Err(e) => attempts.push(format!("Bond: {e}")),
            }

            match InterestRateSwap::deserialize(json_spec) {
                Ok(swap) => return Ok(Arc::new(swap)),
                Err(e) => attempts.push(format!("InterestRateSwap: {e}")),
            }

            match TermLoan::deserialize(json_spec) {
                Ok(term_loan) => return Ok(Arc::new(term_loan)),
                Err(e) => attempts.push(format!("TermLoan: {e}")),
            }

            match finstack_valuations::instruments::Deposit::deserialize(json_spec) {
                Ok(deposit) => return Ok(Arc::new(deposit)),
                Err(e) => attempts.push(format!("Deposit: {e}")),
            }

            match finstack_valuations::instruments::ForwardRateAgreement::deserialize(json_spec) {
                Ok(fra) => return Ok(Arc::new(fra)),
                Err(e) => attempts.push(format!("ForwardRateAgreement: {e}")),
            }

            match finstack_valuations::instruments::Repo::deserialize(json_spec) {
                Ok(repo) => return Ok(Arc::new(repo)),
                Err(e) => attempts.push(format!("Repo: {e}")),
            }

            // Commented out until revolving_credit module is implemented
            /*
            if let Ok(rcf) =
                serde_json::from_value::<RevolvingCreditFacility>(json_spec.clone())
            {
                return Ok(Arc::new(rcf));
            }
            */

            // If all deserialization attempts fail, return an error
            Err(crate::error::Error::build(format!(
                "Failed to deserialize generic debt instrument '{}' as any known type. \
                 Tried: Bond, InterestRateSwap, TermLoan, Deposit, ForwardRateAgreement, Repo. \
                 The JSON structure must match one of these types exactly. Errors: {}",
                id,
                attempts.join("; ")
            )))
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::cashflow::CashFlow;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, PeriodId};
    use finstack_core::money::Money;
    use finstack_valuations::cashflow::builder::{CashFlowMeta, CashFlowSchedule, Notional};
    use finstack_valuations::cashflow::primitives::CFKind;
    use time::Month;

    struct SignedFlowInstrument {
        schedule: CashFlowSchedule,
    }

    impl CashflowProvider for SignedFlowInstrument {
        fn cashflow_schedule(
            &self,
            _curves: &MarketContext,
            _as_of: Date,
        ) -> finstack_core::Result<CashFlowSchedule> {
            Ok(self.schedule.clone())
        }
    }

    #[test]
    fn calculate_period_flows_normalizes_interest_to_issuer_outflow() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");
        let period = Period {
            id: PeriodId::quarter(2025, 1),
            start,
            end,
            is_actual: false,
        };

        let instrument = SignedFlowInstrument {
            schedule: CashFlowSchedule {
                flows: vec![CashFlow {
                    date: Date::from_calendar_date(2025, Month::February, 15).expect("valid date"),
                    reset_date: None,
                    amount: Money::new(-50_000.0, Currency::USD),
                    kind: CFKind::Fixed,
                    accrual_factor: 0.25,
                    rate: None,
                }],
                notional: Notional::par(1_000_000.0, Currency::USD),
                day_count: DayCount::Act365F,
                meta: CashFlowMeta::default(),
            },
        };

        let market_ctx = MarketContext::new();
        let (breakdown, _, warnings) = calculate_period_flows(
            &instrument,
            &period,
            Money::new(1_000_000.0, Currency::USD),
            &market_ctx,
            start,
        )
        .expect("period flow calculation should succeed");

        assert!(warnings.is_empty());
        assert_eq!(breakdown.interest_expense_cash.amount(), 50_000.0);
    }

    #[test]
    fn calculate_period_flows_zero_opening_balance_zeroes_contractual_flows() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");
        let period = Period {
            id: PeriodId::quarter(2025, 1),
            start,
            end,
            is_actual: false,
        };

        let instrument = SignedFlowInstrument {
            schedule: CashFlowSchedule {
                flows: vec![
                    CashFlow {
                        date: Date::from_calendar_date(2025, Month::February, 15)
                            .expect("valid date"),
                        reset_date: None,
                        amount: Money::new(-50_000.0, Currency::USD),
                        kind: CFKind::Fixed,
                        accrual_factor: 0.25,
                        rate: None,
                    },
                    CashFlow {
                        date: Date::from_calendar_date(2025, Month::March, 15).expect("valid date"),
                        reset_date: None,
                        amount: Money::new(-100_000.0, Currency::USD),
                        kind: CFKind::Amortization,
                        accrual_factor: 0.0,
                        rate: None,
                    },
                ],
                notional: Notional::par(1_000_000.0, Currency::USD),
                day_count: DayCount::Act365F,
                meta: CashFlowMeta::default(),
            },
        };

        let market_ctx = MarketContext::new();
        let (breakdown, closing_balance, warnings) = calculate_period_flows(
            &instrument,
            &period,
            Money::new(0.0, Currency::USD),
            &market_ctx,
            start,
        )
        .expect("period flow calculation should succeed");

        assert!(warnings.is_empty());
        assert_eq!(breakdown.interest_expense_cash.amount(), 0.0);
        assert_eq!(breakdown.principal_payment.amount(), 0.0);
        assert_eq!(breakdown.accrued_interest.amount(), 0.0);
        assert_eq!(breakdown.debt_balance.amount(), 0.0);
        assert_eq!(closing_balance.amount(), 0.0);
    }

    #[test]
    fn calculate_period_flows_zero_opening_balance_preserves_new_draws() {
        let start = Date::from_calendar_date(2025, Month::January, 1).expect("valid date");
        let end = Date::from_calendar_date(2025, Month::April, 1).expect("valid date");
        let period = Period {
            id: PeriodId::quarter(2025, 1),
            start,
            end,
            is_actual: false,
        };

        let instrument = SignedFlowInstrument {
            schedule: CashFlowSchedule {
                flows: vec![CashFlow {
                    date: Date::from_calendar_date(2025, Month::February, 15).expect("valid date"),
                    reset_date: None,
                    amount: Money::new(100_000.0, Currency::USD),
                    kind: CFKind::RevolvingDraw,
                    accrual_factor: 0.0,
                    rate: None,
                }],
                notional: Notional::par(0.0, Currency::USD),
                day_count: DayCount::Act365F,
                meta: CashFlowMeta::default(),
            },
        };

        let market_ctx = MarketContext::new();
        let (breakdown, closing_balance, warnings) = calculate_period_flows(
            &instrument,
            &period,
            Money::new(0.0, Currency::USD),
            &market_ctx,
            start,
        )
        .expect("period flow calculation should succeed");

        assert!(warnings.is_empty());
        assert_eq!(breakdown.interest_expense_cash.amount(), 0.0);
        assert_eq!(breakdown.principal_payment.amount(), 0.0);
        assert_eq!(breakdown.debt_balance.amount(), 100_000.0);
        assert_eq!(closing_balance.amount(), 100_000.0);
    }
}
