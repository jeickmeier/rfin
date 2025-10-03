//! Capital Structure Integration Logic
//!
//! This module handles the integration between statements models and capital structure,
//! including cashflow aggregation by period.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::types::DebtInstrumentSpec;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::market_data::MarketContext;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::{Bond, InterestRateSwap};
use indexmap::IndexMap;
use std::sync::Arc;

/// Aggregate cashflows from instruments by period.
///
/// This function takes a list of instruments and periods, generates cashflows
/// for each instrument, and aggregates them by period.
///
/// # Arguments
/// * `instruments` - Map of instrument_id → instrument trait object
/// * `periods` - Model periods to aggregate into
/// * `market_ctx` - Market context with discount/forward curves
/// * `as_of` - Valuation date
///
/// # Returns
/// Aggregated cashflows by instrument and period
pub fn aggregate_instrument_cashflows(
    instruments: &IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>>,
    periods: &[Period],
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<CapitalStructureCashflows> {
    let mut result = CapitalStructureCashflows::new();
    
    // Initialize period maps for totals
    for period in periods {
        result.totals.insert(period.id, CashflowBreakdown::default());
    }
    
    // Process each instrument
    for (instrument_id, instrument) in instruments {
        // Build cashflow schedule
        let flows = instrument.build_schedule(market_ctx, as_of)?;
        
        // Initialize period map for this instrument
        let mut instrument_periods: IndexMap<PeriodId, CashflowBreakdown> = IndexMap::new();
        for period in periods {
            instrument_periods.insert(period.id, CashflowBreakdown::default());
        }
        
        // Track outstanding balance for each period
        let mut outstanding_balance = 0.0;
        
        // Aggregate cashflows into periods
        for (flow_date, amount) in &flows {
            // Find the period containing this cashflow
            if let Some(period) = find_period_containing_date(periods, *flow_date) {
                let breakdown = instrument_periods.get_mut(&period.id).unwrap();
                
                // FIXME: Simplified cashflow classification using sign-based heuristics
                // TODO: Use CFKind from cashflow schedule for precise classification
                // Current limitations:
                // - Cannot distinguish between interest and principal payments accurately
                // - Assumes negative = interest, positive = principal receipt
                // - Should use CFKind::Interest, CFKind::Principal from schedule
                // See PHASE6_SUMMARY.md for details
                
                let value = amount.amount();
                if value < 0.0 {
                    // Outflow - classify as interest (we'll refine this later)
                    breakdown.interest_expense += -value;
                } else {
                    // Inflow (e.g., bond redemption) - principal receipt
                    // For debt from company perspective, this is a payment
                    breakdown.principal_payment += value;
                }
            }
        }
        
        // FIXME: Simplified debt balance tracking
        // TODO: Track actual notional schedule from instrument amortization spec
        // Current limitations:
        // - Uses simple balance = previous_balance - principal_payment
        // - Should track actual notional amortization schedule
        // - Doesn't handle revolving facilities (draws/repayments)
        // See PHASE6_SUMMARY.md for details
        for period in periods {
            let breakdown = instrument_periods.get_mut(&period.id).unwrap();
            
            // Simple model: balance decreases by principal payments
            if outstanding_balance == 0.0 {
                // Initialize from notional if first flow
                if let Some((_, first_amount)) = flows.first() {
                    outstanding_balance = first_amount.amount().abs();
                }
            }
            
            outstanding_balance -= breakdown.principal_payment;
            breakdown.debt_balance = outstanding_balance.max(0.0);
        }
        
        // Store instrument's period breakdown
        result.by_instrument.insert(instrument_id.clone(), instrument_periods.clone());
        
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

/// Find the period that contains a given date.
fn find_period_containing_date(periods: &[Period], date: Date) -> Option<&Period> {
    periods.iter().find(|period| {
        // Check if date is within [start, end) for the period
        date >= period.start && date < period.end
    })
}

/// Build a Bond instrument from a DebtInstrumentSpec.
pub fn build_bond_from_spec(
    spec: &DebtInstrumentSpec,
) -> Result<Bond> {
    match spec {
        DebtInstrumentSpec::Bond { spec: json_spec, .. } => {
            serde_json::from_value(json_spec.clone())
                .map_err(|e| crate::error::Error::build(format!("Failed to deserialize bond: {}", e)))
        }
        _ => Err(crate::error::Error::build("Expected Bond spec")),
    }
}

/// Build an InterestRateSwap instrument from a DebtInstrumentSpec.
pub fn build_swap_from_spec(
    spec: &DebtInstrumentSpec,
) -> Result<InterestRateSwap> {
    match spec {
        DebtInstrumentSpec::Swap { spec: json_spec, .. } => {
            serde_json::from_value(json_spec.clone())
                .map_err(|e| crate::error::Error::build(format!("Failed to deserialize swap: {}", e)))
        }
        _ => Err(crate::error::Error::build("Expected Swap spec")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;
    
    #[test]
    fn test_find_period_containing_date() {
        let q1_start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let q1_end = Date::from_calendar_date(2025, Month::April, 1).unwrap();
        let q2_start = q1_end;
        let q2_end = Date::from_calendar_date(2025, Month::July, 1).unwrap();
        
        let periods = vec![
            Period {
                id: PeriodId::quarter(2025, 1),
                start: q1_start,
                end: q1_end,
                is_actual: true,
            },
            Period {
                id: PeriodId::quarter(2025, 2),
                start: q2_start,
                end: q2_end,
                is_actual: false,
            },
        ];
        
        // Date in Q1
        let jan_15 = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let period = find_period_containing_date(&periods, jan_15);
        assert!(period.is_some());
        assert_eq!(period.unwrap().id, PeriodId::quarter(2025, 1));
        
        // Date in Q2
        let apr_15 = Date::from_calendar_date(2025, Month::April, 15).unwrap();
        let period = find_period_containing_date(&periods, apr_15);
        assert!(period.is_some());
        assert_eq!(period.unwrap().id, PeriodId::quarter(2025, 2));
        
        // Date outside range
        let dec_15 = Date::from_calendar_date(2024, Month::December, 15).unwrap();
        let period = find_period_containing_date(&periods, dec_15);
        assert!(period.is_none());
    }
    
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

