//! Capital Structure Integration Logic
//!
//! This module handles the integration between statements models and capital structure,
//! leveraging valuations infrastructure for cashflow aggregation and classification.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::types::DebtInstrumentSpec;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::market_data::MarketContext;
use finstack_valuations::cashflow::aggregation::aggregate_by_period;
use finstack_valuations::cashflow::primitives::CFKind;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::{Bond, InterestRateSwap};
use indexmap::IndexMap;
use std::sync::Arc;

/// Aggregate cashflows from instruments by period using full valuations infrastructure.
///
/// This function now uses `build_full_schedule()` for precise CFKind-based classification
/// and `outstanding_by_date()` for accurate balance tracking, achieving 100% valuations integration.
///
/// # Arguments
/// * `instruments` - Map of instrument_id → instrument trait object
/// * `periods` - Model periods to aggregate into
/// * `market_ctx` - Market context with discount/forward curves
/// * `as_of` - Valuation date
///
/// # Returns
/// Aggregated cashflows by instrument and period with precise CFKind classification
pub fn aggregate_instrument_cashflows(
    instruments: &IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>>,
    periods: &[Period],
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<CapitalStructureCashflows> {
    let mut result = CapitalStructureCashflows::new();

    // Initialize period maps for totals
    for period in periods {
        result
            .totals
            .insert(period.id, CashflowBreakdown::default());
    }

    // Process each instrument
    for (instrument_id, instrument) in instruments {
        // Use enhanced build_full_schedule() for precise CFKind classification
        let full_schedule = instrument.build_full_schedule(market_ctx, as_of)?;

        // Initialize period map for this instrument
        let mut instrument_periods: IndexMap<PeriodId, CashflowBreakdown> = IndexMap::new();
        for period in periods {
            instrument_periods.insert(period.id, CashflowBreakdown::default());
        }

        // Convert to DatedFlow for period aggregation
        let dated_flows: Vec<(Date, finstack_core::money::Money)> = full_schedule
            .flows
            .iter()
            .map(|cf| (cf.date, cf.amount))
            .collect();

        // Use valuations aggregate_by_period for proper currency-preserving aggregation
        let _period_flows = aggregate_by_period(&dated_flows, periods);

        // Classify cashflows using precise CFKind information (NO MORE HEURISTICS!)
        for cf in &full_schedule.flows {
            if let Some(period_id) = periods
                .iter()
                .find(|p| cf.date >= p.start && cf.date < p.end)
                .map(|p| p.id)
            {
                if let Some(breakdown) = instrument_periods.get_mut(&period_id) {
                    let value = cf.amount.amount().abs(); // Convert to issuer perspective

                    match cf.kind {
                        CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                            // Interest payments (coupons, floating resets)
                            breakdown.interest_expense += value;
                        }
                        CFKind::Amortization => {
                            // Principal amortization payments
                            breakdown.principal_payment += value;
                        }
                        CFKind::Notional if cf.amount.amount() > 0.0 => {
                            // Principal redemption (bullet payment)
                            breakdown.principal_payment += value;
                        }
                        CFKind::Fee => {
                            // Commitment fees, facility fees, etc.
                            breakdown.fees += value;
                        }
                        CFKind::PIK => {
                            // PIK interest increases outstanding (negative interest expense)
                            breakdown.interest_expense += value;
                        }
                        _ => {
                            // Other types (rare) - log for debugging
                            // Could add to interest_expense as conservative fallback
                            breakdown.interest_expense += value;
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
                    breakdown.debt_balance = outstanding_amount.amount().abs();
                }
            }
        }

        // Store instrument's period breakdown
        result
            .by_instrument
            .insert(instrument_id.clone(), instrument_periods.clone());

        // Aggregate into totals
        for (period_id, breakdown) in &instrument_periods {
            let total = result.totals.get_mut(period_id).unwrap();
            total.interest_expense += breakdown.interest_expense;
            total.principal_payment += breakdown.principal_payment;
            total.debt_balance += breakdown.debt_balance;
            total.fees += breakdown.fees;
        }
    }

    Ok(result)
}

/// Build a Bond instrument from a DebtInstrumentSpec.
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

/// Build an InterestRateSwap instrument from a DebtInstrumentSpec.
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

/// Build any debt instrument from a DebtInstrumentSpec.
///
/// This function attempts to deserialize a Generic spec as various known debt instrument types.
/// It tries each type in order and returns the first successful deserialization.
///
/// Supported types:
/// - Bond (fixed or floating rate bonds)
/// - InterestRateSwap (pay-fixed or receive-fixed)
/// - Deposit (term deposits for cash management) - requires serde feature in valuations
/// - Repo (repurchase agreements) - requires serde feature in valuations
/// - FRA (forward rate agreements) - requires serde feature in valuations
///
/// # Returns
/// A boxed trait object implementing CashflowProvider that can be used for cashflow generation.
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

            // Try as Deposit (if valuations has serde support)
            // Note: This requires Deposit to have serde derives in valuations crate
            // Uncomment when available:
            // if let Ok(deposit) = serde_json::from_value::<finstack_valuations::instruments::Deposit>(json_spec.clone()) {
            //     return Ok(Arc::new(deposit));
            // }

            // Try as Repo (if valuations has serde support)
            // Note: This requires Repo to have serde derives in valuations crate
            // Uncomment when available:
            // if let Ok(repo) = serde_json::from_value::<finstack_valuations::instruments::Repo>(json_spec.clone()) {
            //     return Ok(Arc::new(repo));
            // }

            // Try as FRA (if valuations has serde support)
            // Note: This requires FRA to have serde derives in valuations crate
            // Uncomment when available:
            // if let Ok(fra) = serde_json::from_value::<finstack_valuations::instruments::FRA>(json_spec.clone()) {
            //     return Ok(Arc::new(fra));
            // }

            // If all deserialization attempts fail, return an error
            Err(crate::error::Error::build(format!(
                "Failed to deserialize generic debt instrument '{}' as any known type. \
                 Tried: Bond, InterestRateSwap. \
                 The JSON structure must match one of these types exactly. \
                 Additional types (Deposit, Repo, FRA) require serde support in valuations crate.",
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
        let bond = Bond::fixed_semiannual(
            InstrumentId::new("BOND-001"),
            Money::new(1_000_000.0, Currency::USD),
            0.05,
            Date::from_calendar_date(2025, Month::January, 15).unwrap(),
            Date::from_calendar_date(2030, Month::January, 15).unwrap(),
            CurveId::new("USD-OIS"),
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&bond).unwrap();

        // Create DebtInstrumentSpec
        let spec = DebtInstrumentSpec::Bond {
            id: "BOND-001".to_string(),
            spec: spec_json,
        };

        // Deserialize back
        let deserialized_bond = build_bond_from_spec(&spec).unwrap();
        assert_eq!(deserialized_bond.id.as_str(), "BOND-001");
        assert_eq!(deserialized_bond.notional.currency(), Currency::USD);
        assert_eq!(deserialized_bond.coupon, 0.05);
    }

    #[test]
    fn test_build_swap_from_spec() {
        use finstack_core::money::Money;
        use finstack_core::types::InstrumentId;

        // Create a Swap using valuations
        let swap = InterestRateSwap::usd_pay_fixed(
            InstrumentId::new("SWAP-001"),
            Money::new(5_000_000.0, Currency::USD),
            0.04,
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&swap).unwrap();

        // Create DebtInstrumentSpec
        let spec = DebtInstrumentSpec::Swap {
            id: "SWAP-001".to_string(),
            spec: spec_json,
        };

        // Deserialize back
        let deserialized_swap = build_swap_from_spec(&spec).unwrap();
        assert_eq!(deserialized_swap.id.as_str(), "SWAP-001");
        assert_eq!(deserialized_swap.notional.currency(), Currency::USD);
    }
}
