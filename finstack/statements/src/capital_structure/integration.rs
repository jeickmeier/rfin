//! Capital Structure Integration Logic
//!
//! This module handles the integration between statements models and capital structure,
//! leveraging valuations infrastructure for cashflow aggregation and classification.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::types::DebtInstrumentSpec;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::market_data::MarketContext;
use finstack_valuations::cashflow::primitives::CFKind;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::{Bond, InterestRateSwap, TermLoan};
use indexmap::IndexMap;
use std::sync::Arc;

/// Aggregate cashflows from instruments by period using valuations infrastructure.
///
/// The integration leverages `build_full_schedule()` for CFKind-aware classification and
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
/// ```rust,ignore
/// use finstack_statements::capital_structure::integration::aggregate_instrument_cashflows;
/// use finstack_statements::capital_structure::types::CapitalStructureCashflows;
/// use finstack_core::dates::build_periods;
/// use indexmap::IndexMap;
///
/// let periods = build_periods("2025Q1..Q4", None)?.periods;
/// let instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> = IndexMap::new();
/// let cashflows: CapitalStructureCashflows =
///     aggregate_instrument_cashflows(&instruments, &periods, &market_ctx, as_of)?;
/// assert!(cashflows.totals.is_empty());
/// ```
pub fn aggregate_instrument_cashflows(
    instruments: &IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>>,
    periods: &[Period],
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<CapitalStructureCashflows> {
    let mut result = CapitalStructureCashflows::new();

    // Determine base currency from first instrument (if any) or default to USD
    // For now, we default to USD for all aggregations
    // In a future implementation, we'll query instrument currency
    let base_currency = finstack_core::currency::Currency::USD;

    // Initialize period maps for totals with base currency
    for period in periods {
        result
            .totals
            .insert(period.id, CashflowBreakdown::with_currency(base_currency));
    }

    // Process each instrument
    for (instrument_id, instrument) in instruments {
        // Use enhanced build_full_schedule() for precise CFKind classification
        let full_schedule = instrument.build_full_schedule(market_ctx, as_of)?;

        // Determine currency from first cashflow (all cashflows should be same currency)
        let currency = full_schedule
            .flows
            .first()
            .map(|cf| cf.amount.currency())
            .unwrap_or(finstack_core::currency::Currency::USD);

        // Initialize period map for this instrument
        let mut instrument_periods: IndexMap<PeriodId, CashflowBreakdown> = IndexMap::new();
        for period in periods {
            instrument_periods.insert(period.id, CashflowBreakdown::with_currency(currency));
        }

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

                    match cf.kind {
                        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                            // Cash interest payments (coupons, floating resets)
                            breakdown.interest_expense_cash += abs_value;
                        }
                        CFKind::Amortization => {
                            // Principal amortization payments
                            breakdown.principal_payment += abs_value;
                        }
                        CFKind::Notional if cf.amount.amount() > 0.0 => {
                            // Principal redemption (bullet payment)
                            breakdown.principal_payment += abs_value;
                        }
                        CFKind::Fee => {
                            // Commitment fees, facility fees, etc.
                            breakdown.fees += abs_value;
                        }
                        CFKind::PIK => {
                            // PIK (payment-in-kind) interest accrued but not paid in cash
                            // This increases the outstanding balance and is tracked separately
                            breakdown.interest_expense_pik += abs_value;
                        }
                        CFKind::Notional if cf.amount.amount() <= 0.0 => {
                            // Negative notional flows (initial exchange) - typically netted against principal
                            // For simplicity, we ignore these as they represent the initial funding, not ongoing cashflows
                            // The debt_balance is tracked separately via outstanding_by_date()
                        }
                        _ => {
                            // CFKind is non-exhaustive, so we need this catch-all for forward compatibility.
                            // If new CFKind variants are added in the future, conservatively treat them as cash interest.
                            // Note: If this case is hit frequently, consider adding explicit handling for the new CFKind.
                            // In production, this should be logged with: tracing::warn!("Unknown CFKind: {:?}", cf.kind)
                            breakdown.interest_expense_cash += abs_value;
                        }
                    }
                }
            }
        }

        // Use precise outstanding balance tracking from valuations
        let outstanding_path = full_schedule.outstanding_by_date();
        for (date, outstanding_amount) in outstanding_path {
            if let Some(period_id) = periods
                .iter()
                .find(|p| date >= p.start && date < p.end)
                .map(|p| p.id)
            {
                if let Some(breakdown) = instrument_periods.get_mut(&period_id) {
                    // Keep as Money, use absolute value for issuer perspective
                    breakdown.debt_balance = if outstanding_amount.amount() < 0.0 {
                        finstack_core::money::Money::new(
                            -outstanding_amount.amount(),
                            outstanding_amount.currency(),
                        )
                    } else {
                        outstanding_amount
                    };
                }
            }
        }

        // Store instrument's period breakdown
        result
            .by_instrument
            .insert(instrument_id.clone(), instrument_periods.clone());

        // Aggregate into totals (handling Money addition which returns Result)
        for (period_id, breakdown) in &instrument_periods {
            // SAFETY: All periods were initialized at function start
            let total = result
                .totals
                .get_mut(period_id)
                .expect("period should exist in totals map");
            // Money += Money unwraps internally (uses AddAssign which panics on currency mismatch)
            total.interest_expense_cash += breakdown.interest_expense_cash;
            total.interest_expense_pik += breakdown.interest_expense_pik;
            total.principal_payment += breakdown.principal_payment;
            total.debt_balance += breakdown.debt_balance;
            total.fees += breakdown.fees;
        }
    }

    Ok(result)
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
        } => serde_json::from_value(json_spec.clone())
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
        } => serde_json::from_value(json_spec.clone())
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
        } => serde_json::from_value(json_spec.clone())
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
            // Try to deserialize as known types in order of likelihood

            // Try as Bond first (most common)
            if let Ok(bond) = serde_json::from_value::<Bond>(json_spec.clone()) {
                return Ok(Arc::new(bond));
            }

            // Try as InterestRateSwap
            if let Ok(swap) = serde_json::from_value::<InterestRateSwap>(json_spec.clone()) {
                return Ok(Arc::new(swap));
            }

            // Try as TermLoan (bank debt)
            if let Ok(term_loan) = serde_json::from_value::<TermLoan>(json_spec.clone()) {
                return Ok(Arc::new(term_loan));
            }

            // Try as Deposit (cash management)
            if let Ok(deposit) = serde_json::from_value::<finstack_valuations::instruments::Deposit>(
                json_spec.clone(),
            ) {
                return Ok(Arc::new(deposit));
            }

            // Try as FRA (forward rate hedge)
            if let Ok(fra) = serde_json::from_value::<
                finstack_valuations::instruments::ForwardRateAgreement,
            >(json_spec.clone())
            {
                return Ok(Arc::new(fra));
            }

            // Try as Repo (repurchase agreement)
            if let Ok(repo) =
                serde_json::from_value::<finstack_valuations::instruments::Repo>(json_spec.clone())
            {
                return Ok(Arc::new(repo));
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
                 The JSON structure must match one of these types exactly.",
                id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_build_bond_from_spec() {
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};

        // Create a Bond using valuations
        let bond = Bond::fixed(
            InstrumentId::new("BOND-001"),
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2025, Month::January, 15).expect("valid date"),
            Date::from_calendar_date(2030, Month::January, 15).expect("valid date"),
            CurveId::new("USD-OIS"),
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&bond).expect("bond should serialize");

        // Create DebtInstrumentSpec
        let spec = DebtInstrumentSpec::Bond {
            id: "BOND-001".to_string(),
            spec: spec_json,
        };

        // Deserialize back
        let deserialized_bond = build_bond_from_spec(&spec).expect("bond should deserialize");
        assert_eq!(deserialized_bond.id.as_str(), "BOND-001");
        assert_eq!(deserialized_bond.notional.currency(), Currency::USD);
        // Check coupon from cashflow_spec
        use finstack_valuations::instruments::bond::CashflowSpec;
        if let CashflowSpec::Fixed(spec) = &deserialized_bond.cashflow_spec {
            assert_eq!(spec.rate, 0.05);
        } else {
            panic!("Expected fixed cashflow spec");
        }
    }

    #[test]
    fn test_build_swap_from_spec() {
        use finstack_core::money::Money;
        use finstack_core::types::InstrumentId;

        use finstack_valuations::instruments::common::parameters::PayReceive;

        // Create a Swap using valuations
        let swap = InterestRateSwap::new(
            InstrumentId::new("SWAP-001"),
            Money::new(5_000_000.0, Currency::USD),
            0.04,
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Date::from_calendar_date(2030, Month::January, 1).expect("valid date"),
            PayReceive::PayFixed,
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&swap).expect("swap should serialize");

        // Create DebtInstrumentSpec
        let spec = DebtInstrumentSpec::Swap {
            id: "SWAP-001".to_string(),
            spec: spec_json,
        };

        // Deserialize back
        let deserialized_swap = build_swap_from_spec(&spec).expect("swap should deserialize");
        assert_eq!(deserialized_swap.id.as_str(), "SWAP-001");
        assert_eq!(deserialized_swap.notional.currency(), Currency::USD);
    }
}
