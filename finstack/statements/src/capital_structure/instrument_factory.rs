//! Build concrete instrument types from `DebtInstrumentSpec` JSON payloads.
//!
//! Extracted from `integration.rs` so the schedule-aggregation pipeline does
//! not have to share a translation unit with serde-driven instrument
//! construction. Each `build_*_from_spec` function targets one concrete type;
//! [`build_any_instrument_from_spec`] dispatches by spec variant and falls
//! back to a "try every known instrument" loop for the `Generic` variant.

use crate::error::Result;
use crate::types::DebtInstrumentSpec;
use finstack_cashflows::CashflowProvider;
use finstack_valuations::instruments::{Bond, InterestRateSwap, TermLoan};
use serde::Deserialize;
use std::sync::Arc;

/// Build a [`Bond`] instrument from a [`DebtInstrumentSpec`].
///
/// # Arguments
/// * `spec` - Debt instrument specification sourced from the model
///
/// # Errors
/// Returns an error when the payload cannot be deserialized as a `Bond`.
pub(crate) fn build_bond_from_spec(spec: &DebtInstrumentSpec) -> Result<Bond> {
    match spec {
        DebtInstrumentSpec::Bond {
            id,
            spec: json_spec,
        } => Bond::deserialize(json_spec).map_err(|e| {
            crate::error::Error::build(format!(
                "Failed to deserialize bond '{}': {}. Ensure the JSON spec matches the Bond structure.",
                id, e
            ))
        }),
        _ => Err(crate::error::Error::build(
            "Expected Bond variant in DebtInstrumentSpec, but got a different variant",
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
pub(crate) fn build_swap_from_spec(spec: &DebtInstrumentSpec) -> Result<InterestRateSwap> {
    match spec {
        DebtInstrumentSpec::Swap {
            id,
            spec: json_spec,
        } => InterestRateSwap::deserialize(json_spec).map_err(|e| {
            crate::error::Error::build(format!(
                "Failed to deserialize swap '{}': {}. Ensure the JSON spec matches the InterestRateSwap structure.",
                id, e
            ))
        }),
        _ => Err(crate::error::Error::build(
            "Expected Swap variant in DebtInstrumentSpec, but got a different variant",
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
pub(crate) fn build_term_loan_from_spec(spec: &DebtInstrumentSpec) -> Result<TermLoan> {
    match spec {
        DebtInstrumentSpec::TermLoan {
            id,
            spec: json_spec,
        } => TermLoan::deserialize(json_spec).map_err(|e| {
            crate::error::Error::build(format!(
                "Failed to deserialize term loan '{}': {}. Ensure the JSON spec matches the TermLoan structure.",
                id, e
            ))
        }),
        _ => Err(crate::error::Error::build(
            "Expected TermLoan variant in DebtInstrumentSpec, but got a different variant",
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
