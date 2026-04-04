//! WASM bindings for capital structure cashflow aggregation.
//!
//! Wraps `finstack_statements::capital_structure` types and exposes
//! functions for aggregating instrument cashflows across a capital structure.

use crate::core::dates::FsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::statements::types::model::JsCapitalStructureSpec;
use finstack_core::dates::PeriodId;
use finstack_statements::capital_structure::{self, CapitalStructureCashflows, CashflowBreakdown};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Per-period cashflow breakdown for a single instrument or aggregate.
///
/// Contains interest (cash and PIK), principal, fees, balance, and accrued interest.
#[wasm_bindgen(js_name = CashflowBreakdown)]
pub struct JsCashflowBreakdown {
    inner: CashflowBreakdown,
}

#[wasm_bindgen(js_class = CashflowBreakdown)]
impl JsCashflowBreakdown {
    /// Cash portion of interest expense.
    #[wasm_bindgen(getter, js_name = interestExpenseCash)]
    pub fn interest_expense_cash(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.interest_expense_cash)
    }

    /// PIK (payment-in-kind) portion of interest expense.
    #[wasm_bindgen(getter, js_name = interestExpensePik)]
    pub fn interest_expense_pik(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.interest_expense_pik)
    }

    /// Total interest expense (cash + PIK).
    #[wasm_bindgen(getter, js_name = interestExpenseTotal)]
    pub fn interest_expense_total(&self) -> Result<JsMoney, JsValue> {
        self.inner
            .interest_expense_total()
            .map(JsMoney::from_inner)
            .map_err(|e| js_error(format!("Interest total error: {e}")))
    }

    /// Principal payment for the period.
    #[wasm_bindgen(getter, js_name = principalPayment)]
    pub fn principal_payment(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.principal_payment)
    }

    /// Fees for the period.
    #[wasm_bindgen(getter)]
    pub fn fees(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.fees)
    }

    /// Outstanding debt balance at period end.
    #[wasm_bindgen(getter, js_name = debtBalance)]
    pub fn debt_balance(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.debt_balance)
    }

    /// Accrued but unpaid interest.
    #[wasm_bindgen(getter, js_name = accruedInterest)]
    pub fn accrued_interest(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.accrued_interest)
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        let interest = self
            .inner
            .interest_expense_total()
            .map_or_else(|e| format!("<error: {e}>"), |m| m.to_string());
        format!(
            "CashflowBreakdown(interest={}, principal={}, balance={})",
            interest, self.inner.principal_payment, self.inner.debt_balance
        )
    }
}

/// Aggregated capital structure cashflows across all instruments and periods.
///
/// Provides accessors for per-instrument and aggregate (total) flows.
#[wasm_bindgen(js_name = CapitalStructureCashflows)]
pub struct JsCapitalStructureCashflows {
    inner: CapitalStructureCashflows,
}

#[wasm_bindgen(js_class = CapitalStructureCashflows)]
impl JsCapitalStructureCashflows {
    /// List of instrument IDs in the capital structure.
    #[wasm_bindgen(getter, js_name = instrumentIds)]
    pub fn instrument_ids(&self) -> Vec<String> {
        self.inner.by_instrument.keys().cloned().collect()
    }

    /// List of period IDs that have aggregate totals.
    #[wasm_bindgen(getter, js_name = periodIds)]
    pub fn period_ids(&self) -> Vec<String> {
        self.inner
            .totals
            .keys()
            .map(|pid| pid.to_string())
            .collect()
    }

    /// Get the total interest expense for a specific instrument and period.
    #[wasm_bindgen(js_name = getInterest)]
    pub fn get_interest(&self, instrument_id: &str, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_interest(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the cash interest expense for a specific instrument and period.
    #[wasm_bindgen(js_name = getInterestCash)]
    pub fn get_interest_cash(&self, instrument_id: &str, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_interest_cash(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the PIK interest expense for a specific instrument and period.
    #[wasm_bindgen(js_name = getInterestPik)]
    pub fn get_interest_pik(&self, instrument_id: &str, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_interest_pik(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the principal payment for a specific instrument and period.
    #[wasm_bindgen(js_name = getPrincipal)]
    pub fn get_principal(&self, instrument_id: &str, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_principal(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the debt balance for a specific instrument and period.
    #[wasm_bindgen(js_name = getDebtBalance)]
    pub fn get_debt_balance(&self, instrument_id: &str, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_debt_balance(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the accrued interest for a specific instrument and period.
    #[wasm_bindgen(js_name = getAccruedInterest)]
    pub fn get_accrued_interest(
        &self,
        instrument_id: &str,
        period_id: &str,
    ) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_accrued_interest(instrument_id, &pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total interest expense across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalInterest)]
    pub fn get_total_interest(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_interest(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total cash interest expense across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalInterestCash)]
    pub fn get_total_interest_cash(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_interest_cash(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total PIK interest expense across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalInterestPik)]
    pub fn get_total_interest_pik(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_interest_pik(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total principal payment across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalPrincipal)]
    pub fn get_total_principal(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_principal(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total debt balance across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalDebtBalance)]
    pub fn get_total_debt_balance(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_debt_balance(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the total accrued interest across all instruments for a period.
    #[wasm_bindgen(js_name = getTotalAccruedInterest)]
    pub fn get_total_accrued_interest(&self, period_id: &str) -> Result<f64, JsValue> {
        let pid = parse_period_id(period_id)?;
        self.inner
            .get_total_accrued_interest(&pid)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the full breakdown for a given instrument and period.
    #[wasm_bindgen(js_name = getBreakdown)]
    pub fn get_breakdown(
        &self,
        instrument_id: &str,
        period_id: &str,
    ) -> Result<Option<JsCashflowBreakdown>, JsValue> {
        let pid = parse_period_id(period_id)?;
        Ok(self
            .inner
            .by_instrument
            .get(instrument_id)
            .and_then(|periods| periods.get(&pid))
            .map(|b| JsCashflowBreakdown { inner: b.clone() }))
    }

    /// Get the aggregate breakdown for a given period.
    #[wasm_bindgen(js_name = getTotalBreakdown)]
    pub fn get_total_breakdown(
        &self,
        period_id: &str,
    ) -> Result<Option<JsCashflowBreakdown>, JsValue> {
        let pid = parse_period_id(period_id)?;
        Ok(self
            .inner
            .totals
            .get(&pid)
            .map(|b| JsCashflowBreakdown { inner: b.clone() }))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner).map_err(|e| {
            js_error(format!(
                "Failed to serialize CapitalStructureCashflows: {e}"
            ))
        })
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CapitalStructureCashflows(instruments={}, periods={})",
            self.inner.by_instrument.len(),
            self.inner.totals.len()
        )
    }
}

/// Aggregate cashflows across all instruments in a capital structure.
///
/// Builds instruments from the spec's debt instrument definitions, then
/// computes period-by-period cashflow breakdowns for each instrument
/// and rolls up aggregate totals.
#[wasm_bindgen(js_name = aggregateInstrumentCashflows)]
pub fn aggregate_instrument_cashflows(
    spec: &JsCapitalStructureSpec,
    period_ids: Vec<String>,
    market: &JsMarketContext,
    as_of: &FsDate,
) -> Result<JsCapitalStructureCashflows, JsValue> {
    let core_periods: Vec<finstack_core::dates::Period> = period_ids
        .iter()
        .map(|s| {
            let pid = PeriodId::from_str(s)
                .map_err(|e| js_error(format!("Invalid period ID '{s}': {e}")))?;
            Ok(finstack_core::dates::Period {
                id: pid,
                start: finstack_core::dates::Date::MIN,
                end: finstack_core::dates::Date::MIN,
                is_actual: false,
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;

    let mut instruments = indexmap::IndexMap::new();
    for debt_spec in &spec.inner.debt_instruments {
        let id = match debt_spec {
            finstack_statements::types::DebtInstrumentSpec::Bond { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::Swap { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::TermLoan { id, .. }
            | finstack_statements::types::DebtInstrumentSpec::Generic { id, .. } => id.clone(),
        };
        let inst = capital_structure::build_any_instrument_from_spec(debt_spec)
            .map_err(|e| js_error(format!("Failed to build instrument '{id}': {e}")))?;
        instruments.insert(id, inst);
    }

    let result = capital_structure::aggregate_instrument_cashflows(
        &spec.inner,
        &instruments,
        &core_periods,
        market.inner(),
        as_of.inner(),
    )
    .map_err(|e| js_error(format!("Failed to aggregate cashflows: {e}")))?;

    Ok(JsCapitalStructureCashflows { inner: result })
}

/// Waterfall specification for dynamic cash flow allocation.
///
/// Defines payment priorities and optional sweep/PIK controls.
/// Construct via `fromJSON`.
#[wasm_bindgen(js_name = WaterfallSpec)]
pub struct JsWaterfallSpec {
    pub(crate) inner: finstack_statements::capital_structure::WaterfallSpec,
}

#[wasm_bindgen(js_class = WaterfallSpec)]
impl JsWaterfallSpec {
    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsWaterfallSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsWaterfallSpec { inner })
            .map_err(|e| js_error(format!("Failed to deserialize WaterfallSpec: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize WaterfallSpec: {e}")))
    }
}

/// Excess Cash Flow (ECF) sweep specification.
///
/// Defines how to calculate ECF and what percentage to sweep to pay down debt.
/// Construct via `fromJSON`.
#[wasm_bindgen(js_name = EcfSweepSpec)]
pub struct JsEcfSweepSpec {
    pub(crate) inner: finstack_statements::capital_structure::EcfSweepSpec,
}

#[wasm_bindgen(js_class = EcfSweepSpec)]
impl JsEcfSweepSpec {
    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsEcfSweepSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsEcfSweepSpec { inner })
            .map_err(|e| js_error(format!("Failed to deserialize EcfSweepSpec: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize EcfSweepSpec: {e}")))
    }
}

/// PIK toggle specification for switching between cash and PIK interest modes.
///
/// Construct via `fromJSON`.
#[wasm_bindgen(js_name = PikToggleSpec)]
pub struct JsPikToggleSpec {
    pub(crate) inner: finstack_statements::capital_structure::PikToggleSpec,
}

#[wasm_bindgen(js_class = PikToggleSpec)]
impl JsPikToggleSpec {
    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsPikToggleSpec, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsPikToggleSpec { inner })
            .map_err(|e| js_error(format!("Failed to deserialize PikToggleSpec: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize PikToggleSpec: {e}")))
    }
}

fn parse_period_id(s: &str) -> Result<PeriodId, JsValue> {
    PeriodId::from_str(s).map_err(|e| js_error(format!("Invalid period ID '{s}': {e}")))
}
