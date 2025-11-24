//! Capital Structure Integration Logic
//!
//! This module handles the integration between statements models and capital structure,
//! leveraging valuations infrastructure for cashflow aggregation and classification.

use crate::capital_structure::types::*;
use crate::error::Result;
use crate::types::DebtInstrumentSpec;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Period, PeriodId};
use finstack_core::market_data::MarketContext;
use finstack_core::money::{fx::FxQuery, Money};
use finstack_valuations::cashflow::primitives::CFKind;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::{Bond, InterestRateSwap, TermLoan};
use indexmap::IndexMap;
use serde::Deserialize;
use std::sync::Arc;

/// Calculate contractual flows for a single period.
///
/// This helper extracts flows for a specific period from an instrument's full schedule,
/// returning a CashflowBreakdown for that period. Used for dynamic period-by-period evaluation.
///
/// # Arguments
/// * `instrument` - The instrument to calculate flows for
/// * `period` - The period to extract flows for
/// * `opening_balance` - Opening balance at the start of the period
/// * `market_ctx` - Market context for pricing
/// * `as_of` - Valuation date
///
/// # Returns
/// CashflowBreakdown for the period and closing balance
pub fn calculate_period_flows(
    instrument: &dyn CashflowProvider,
    period: &Period,
    opening_balance: Money,
    market_ctx: &MarketContext,
    as_of: Date,
) -> Result<(CashflowBreakdown, Money)> {
    let full_schedule = instrument.build_full_schedule(market_ctx, as_of)?;
    let currency = opening_balance.currency();
    let mut breakdown = CashflowBreakdown::with_currency(currency);

    // Extract flows that fall within this period
    for cf in &full_schedule.flows {
        if cf.date >= period.start && cf.date < period.end {
            let abs_value = if cf.amount.amount() < 0.0 {
                Money::new(-cf.amount.amount(), cf.amount.currency())
            } else {
                cf.amount
            };

            match cf.kind {
                CFKind::Fixed | CFKind::Stub | CFKind::FloatReset => {
                    breakdown.interest_expense_cash += abs_value;
                }
                CFKind::Amortization => {
                    breakdown.principal_payment += abs_value;
                }
                CFKind::Notional if cf.amount.amount() > 0.0 => {
                    breakdown.principal_payment += abs_value;
                }
                CFKind::Fee => {
                    breakdown.fees += abs_value;
                }
                CFKind::PIK => {
                    breakdown.interest_expense_pik += abs_value;
                }
                _ => {}
            }
        }
    }

    // Get closing balance from outstanding_by_date
    let outstanding_path = full_schedule.outstanding_by_date();
    let closing_balance = outstanding_path
        .iter()
        .rev()
        .find(|(date, _)| *date >= period.start && *date < period.end)
        .map(|(_, balance)| {
            if balance.amount() < 0.0 {
                Money::new(-balance.amount(), balance.currency())
            } else {
                *balance
            }
        })
        .unwrap_or_else(|| {
            // If no flows in period, use opening balance adjusted by flows
            opening_balance
                .checked_sub(breakdown.principal_payment)
                .unwrap_or_else(|_| Money::new(0.0, currency))
                .checked_add(breakdown.interest_expense_pik)
                .unwrap_or_else(|_| Money::new(0.0, currency))
        });

    breakdown.debt_balance = closing_balance;

    Ok((breakdown, closing_balance))
}

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
/// let spec = CapitalStructureSpec {
///     debt_instruments: vec![],
///     equity_instruments: vec![],
///     meta: IndexMap::new(),
///     reporting_currency: None,
///     fx_policy: None,
/// };
/// let cashflows: CapitalStructureCashflows =
///     aggregate_instrument_cashflows(&spec, &instruments, &periods, &market_ctx, as_of)?;
/// assert!(cashflows.totals.is_empty());
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
    let fx_matrix = market_ctx.fx.as_ref();
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
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.interest_expense_cash += money;
                            }
                        }
                        CFKind::Amortization => {
                            // Principal amortization payments
                            breakdown.principal_payment += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.principal_payment += money;
                            }
                        }
                        CFKind::Notional if cf.amount.amount() > 0.0 => {
                            // Principal redemption (bullet payment)
                            breakdown.principal_payment += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.principal_payment += money;
                            }
                        }
                        CFKind::Fee => {
                            // Commitment fees, facility fees, etc.
                            breakdown.fees += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.fees += money;
                            }
                        }
                        CFKind::PIK => {
                            // PIK (payment-in-kind) interest accrued but not paid in cash
                            // This increases the outstanding balance and is tracked separately
                            breakdown.interest_expense_pik += abs_value;
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.interest_expense_pik += money;
                            }
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
                            if let (Some(map), Some(money)) =
                                (reporting_totals.as_mut(), converted_abs)
                            {
                                let total = map.get_mut(&period_id).expect("period initialized");
                                total.interest_expense_cash += money;
                            }
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
                    let issuer_balance = if outstanding_amount.amount() < 0.0 {
                        finstack_core::money::Money::new(
                            -outstanding_amount.amount(),
                            outstanding_amount.currency(),
                        )
                    } else {
                        outstanding_amount
                    };
                    breakdown.debt_balance = issuer_balance;

                    if let (Some(map), Some(money)) = (
                        reporting_totals.as_mut(),
                        convert_to_reporting(
                            issuer_balance,
                            date,
                            reporting_currency,
                            fx_matrix,
                            fx_policy,
                        )?,
                    ) {
                        let total = map.get_mut(&period_id).expect("period initialized");
                        total.debt_balance = money;
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
                let total = currency_totals
                    .get_mut(period_id)
                    .expect("period should exist in currency totals");
                total.interest_expense_cash += breakdown.interest_expense_cash;
                total.interest_expense_pik += breakdown.interest_expense_pik;
                total.principal_payment += breakdown.principal_payment;
                total.debt_balance += breakdown.debt_balance;
                total.fees += breakdown.fees;
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
fn convert_to_reporting(
    money: finstack_core::money::Money,
    on: Date,
    reporting_currency: Option<Currency>,
    fx_matrix: Option<&Arc<finstack_core::money::fx::FxMatrix>>,
    fx_policy: finstack_core::money::fx::FxConversionPolicy,
) -> Result<Option<finstack_core::money::Money>> {
    if let (Some(rc), Some(fx)) = (reporting_currency, fx_matrix) {
        if rc == money.currency() {
            return Ok(Some(money));
        }
        let rate = fx
            .rate(FxQuery::with_policy(money.currency(), rc, on, fx_policy))?
            .rate;
        Ok(Some(finstack_core::money::Money::new(
            money.amount() * rate,
            rc,
        )))
    } else {
        Ok(None)
    }
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
            // Try to deserialize as known types in order of likelihood

            // Try as Bond first (most common)
            if let Ok(bond) = Bond::deserialize(json_spec) {
                return Ok(Arc::new(bond));
            }

            // Try as InterestRateSwap
            if let Ok(swap) = InterestRateSwap::deserialize(json_spec) {
                return Ok(Arc::new(swap));
            }

            // Try as TermLoan (bank debt)
            if let Ok(term_loan) = TermLoan::deserialize(json_spec) {
                return Ok(Arc::new(term_loan));
            }

            // Try as Deposit (cash management)
            if let Ok(deposit) = finstack_valuations::instruments::Deposit::deserialize(json_spec) {
                return Ok(Arc::new(deposit));
            }

            // Try as FRA (forward rate hedge)
            if let Ok(fra) =
                finstack_valuations::instruments::ForwardRateAgreement::deserialize(json_spec)
            {
                return Ok(Arc::new(fra));
            }

            // Try as Repo (repurchase agreement)
            if let Ok(repo) = finstack_valuations::instruments::Repo::deserialize(json_spec) {
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

        // Create a USD market-standard swap using valuations
        let swap = InterestRateSwap::create_usd_swap(
            InstrumentId::new("SWAP-001"),
            Money::new(5_000_000.0, Currency::USD),
            0.04,
            Date::from_calendar_date(2025, Month::January, 1).expect("valid date"),
            Date::from_calendar_date(2030, Month::January, 1).expect("valid date"),
            PayReceive::PayFixed,
        )
        .expect("swap should build");

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
