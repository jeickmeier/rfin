//! WASM bindings for structured credit waterfall engine.
//!
//! This module exposes the generalized waterfall engine to TypeScript/JavaScript, including:
//! - WaterfallTier, AllocationMode, PaymentType
//! - JSON serialization/deserialization

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::waterfall::{
    CoverageTestRules, CoverageTrigger as WaterfallCoverageTrigger,
};
use finstack_valuations::instruments::fixed_income::structured_credit::{
    execute_waterfall, execute_waterfall_with_explanation, WaterfallContext,
};
use finstack_valuations::instruments::fixed_income::structured_credit::{
    AllocationMode as RustAllocationMode, PaymentCalculation, PaymentType as RustPaymentType,
    Recipient, RecipientType, TrancheStructure, Waterfall, WaterfallDistribution,
    WaterfallTier as RustWaterfallTier,
};
use wasm_bindgen::prelude::*;

// ============================================================================
// ENUMS
// ============================================================================

/// Allocation mode within a tier.
///
/// Values:
/// - 0: Sequential - Pay recipients in order
/// - 1: ProRata - Distribute proportionally by weight
#[wasm_bindgen(js_name = AllocationMode)]
#[derive(Clone, Copy, Debug)]
pub enum JsAllocationMode {
    Sequential = 0,
    ProRata = 1,
}

impl From<JsAllocationMode> for RustAllocationMode {
    fn from(value: JsAllocationMode) -> Self {
        match value {
            JsAllocationMode::Sequential => RustAllocationMode::Sequential,
            JsAllocationMode::ProRata => RustAllocationMode::ProRata,
        }
    }
}

impl From<RustAllocationMode> for JsAllocationMode {
    fn from(value: RustAllocationMode) -> Self {
        match value {
            RustAllocationMode::Sequential => JsAllocationMode::Sequential,
            RustAllocationMode::ProRata => JsAllocationMode::ProRata,
            _ => unreachable!("unknown AllocationMode variant"),
        }
    }
}

/// Payment type classification.
///
/// Values:
/// - 0: Fee - Fee payment
/// - 1: Interest - Interest payment
/// - 2: Principal - Principal payment
/// - 3: Residual - Residual/equity distribution
#[wasm_bindgen(js_name = PaymentType)]
#[derive(Clone, Copy, Debug)]
pub enum JsPaymentType {
    Fee = 0,
    Interest = 1,
    Principal = 2,
    Residual = 3,
}

impl From<JsPaymentType> for RustPaymentType {
    fn from(value: JsPaymentType) -> Self {
        match value {
            JsPaymentType::Fee => RustPaymentType::Fee,
            JsPaymentType::Interest => RustPaymentType::Interest,
            JsPaymentType::Principal => RustPaymentType::Principal,
            JsPaymentType::Residual => RustPaymentType::Residual,
        }
    }
}

impl From<RustPaymentType> for JsPaymentType {
    fn from(value: RustPaymentType) -> Self {
        match value {
            RustPaymentType::Fee => JsPaymentType::Fee,
            RustPaymentType::Interest => JsPaymentType::Interest,
            RustPaymentType::Principal => JsPaymentType::Principal,
            RustPaymentType::Residual => JsPaymentType::Residual,
            _ => unreachable!("unknown PaymentType variant"),
        }
    }
}

// ============================================================================
// WATERFALL TIER
// ============================================================================

/// Waterfall tier with multiple recipients (JSON-based API).
///
/// Use JSON to create tiers with recipients and allocation modes.
///
/// Example:
/// ```javascript
/// const tier = WaterfallTier.fromJson(JSON.stringify({
///   id: "fees",
///   priority: 1,
///   recipients: [...],
///   payment_type: "Fee",
///   allocation_mode: "Sequential",
///   divertible: false
/// }));
/// ```
#[wasm_bindgen(js_name = WaterfallTier)]
#[derive(Clone, Debug)]
pub struct JsWaterfallTier {
    pub(crate) inner: RustWaterfallTier,
}

#[wasm_bindgen(js_class = WaterfallTier)]
impl JsWaterfallTier {
    /// Create from JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsWaterfallTier, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsWaterfallTier { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    /// Convert to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Convert to JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Get tier ID.
    #[wasm_bindgen(getter, js_name = tierId)]
    pub fn tier_id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get priority.
    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> usize {
        self.inner.priority
    }

    /// Get number of recipients.
    #[wasm_bindgen(getter, js_name = recipientCount)]
    pub fn recipient_count(&self) -> usize {
        self.inner.recipients.len()
    }

    /// Get payment type as number.
    #[wasm_bindgen(getter, js_name = paymentType)]
    pub fn payment_type(&self) -> u8 {
        JsPaymentType::from(self.inner.payment_type) as u8
    }

    /// Get allocation mode as number.
    #[wasm_bindgen(getter, js_name = allocationMode)]
    pub fn allocation_mode(&self) -> u8 {
        JsAllocationMode::from(self.inner.allocation_mode) as u8
    }

    /// Is tier divertible.
    #[wasm_bindgen(getter)]
    pub fn divertible(&self) -> bool {
        self.inner.divertible
    }

    /// Add a recipient to this tier using JSON payloads for type/calculation.
    ///
    /// @param {string} recipientId - Unique recipient identifier
    /// @param {string} recipientTypeJson - JSON for RecipientType
    /// @param {string} calculationJson - JSON for PaymentCalculation
    #[wasm_bindgen(js_name = addRecipient)]
    pub fn add_recipient(
        &mut self,
        recipient_id: &str,
        recipient_type_json: &str,
        calculation_json: &str,
    ) -> Result<JsWaterfallTier, JsValue> {
        let recipient_type: RecipientType = serde_json::from_str(recipient_type_json)
            .map_err(|e| js_error(format!("Invalid recipient_type JSON: {e}")))?;
        let calc: PaymentCalculation = serde_json::from_str(calculation_json)
            .map_err(|e| js_error(format!("Invalid calculation JSON: {e}")))?;

        self.inner
            .recipients
            .push(Recipient::new(recipient_id, recipient_type, calc));
        Ok(self.clone())
    }

    /// Add a fixed-fee recipient helper.
    #[wasm_bindgen(js_name = addFixedFee)]
    pub fn add_fixed_fee(
        &mut self,
        recipient_id: &str,
        provider_name: &str,
        amount: f64,
        currency: &str,
    ) -> Result<JsWaterfallTier, JsValue> {
        let curr: Currency = currency
            .parse()
            .map_err(|e| js_error(format!("Invalid currency '{currency}': {e}")))?;
        let recipient = Recipient::fixed_fee(recipient_id, provider_name, Money::new(amount, curr));
        self.inner.recipients.push(recipient);
        Ok(self.clone())
    }

    /// Add a tranche interest recipient helper.
    #[wasm_bindgen(js_name = addTrancheInterest)]
    pub fn add_tranche_interest(
        &mut self,
        recipient_id: &str,
        tranche_id: &str,
    ) -> Result<JsWaterfallTier, JsValue> {
        self.inner
            .recipients
            .push(Recipient::tranche_interest(recipient_id, tranche_id));
        Ok(self.clone())
    }

    /// Add a tranche principal recipient helper (no target balance).
    #[wasm_bindgen(js_name = addTranchePrincipal)]
    pub fn add_tranche_principal(
        &mut self,
        recipient_id: &str,
        tranche_id: &str,
    ) -> Result<JsWaterfallTier, JsValue> {
        self.inner
            .recipients
            .push(Recipient::tranche_principal(recipient_id, tranche_id, None));
        Ok(self.clone())
    }

    /// Set allocation mode (sequential / pro-rata).
    #[wasm_bindgen(js_name = setAllocationMode)]
    pub fn set_allocation_mode(&mut self, mode: JsAllocationMode) -> JsWaterfallTier {
        self.inner.allocation_mode = mode.into();
        self.clone()
    }

    /// Mark this tier as divertible.
    #[wasm_bindgen(js_name = setDivertible)]
    pub fn set_divertible(&mut self, divertible: bool) -> JsWaterfallTier {
        self.inner.divertible = divertible;
        self.clone()
    }
}

// ============================================================================
// WATERFALL ENGINE WRAPPERS
// ============================================================================

/// Full waterfall engine wrapper.
#[wasm_bindgen(js_name = WaterfallEngine)]
#[derive(Clone, Debug)]
pub struct JsWaterfall {
    pub(crate) inner: Waterfall,
}

#[wasm_bindgen(js_class = WaterfallEngine)]
impl JsWaterfall {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsWaterfall, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsWaterfall { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Add a tier to the engine.
    #[wasm_bindgen(js_name = addTier)]
    pub fn add_tier(&mut self, tier: &JsWaterfallTier) -> JsWaterfall {
        self.inner = self.inner.clone().add_tier(tier.inner.clone());
        self.clone()
    }
}

/// Coverage trigger (OC/IC) wrapper.
#[wasm_bindgen(js_name = CoverageTrigger)]
#[derive(Clone, Debug)]
pub struct JsCoverageTrigger {
    pub(crate) inner: WaterfallCoverageTrigger,
}

#[wasm_bindgen(js_class = CoverageTrigger)]
impl JsCoverageTrigger {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsCoverageTrigger, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsCoverageTrigger { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }
}

/// Coverage rules (haircuts, thresholds).
#[wasm_bindgen(js_name = CoverageTestRules)]
#[derive(Clone, Debug)]
pub struct JsCoverageTestRules {
    pub(crate) inner: CoverageTestRules,
}

#[wasm_bindgen(js_class = CoverageTestRules)]
impl JsCoverageTestRules {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsCoverageTestRules, JsValue> {
        serde_json::from_str(json_str)
            .map(|inner| JsCoverageTestRules { inner })
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }
}

/// Waterfall distribution result.
#[wasm_bindgen(js_name = WaterfallDistribution)]
#[derive(Clone, Debug)]
pub struct JsWaterfallDistribution {
    inner: WaterfallDistribution,
}

#[wasm_bindgen(js_class = WaterfallDistribution)]
impl JsWaterfallDistribution {
    /// Convert to JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Convert to JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Payment date.
    #[wasm_bindgen(getter, js_name = paymentDate)]
    pub fn payment_date(&self) -> JsDate {
        JsDate::from_core(self.inner.payment_date)
    }

    /// Total available cash at start of distribution.
    #[wasm_bindgen(getter, js_name = totalAvailable)]
    pub fn total_available(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.total_available)
    }

    /// Tier allocations as [tier_id, amount] tuples.
    #[wasm_bindgen(getter, js_name = tierAllocations)]
    pub fn tier_allocations(&self) -> js_sys::Array {
        self.inner
            .tier_allocations
            .iter()
            .map(|(id, amt)| {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_str(id));
                arr.push(&JsValue::from_f64(amt.amount()));
                arr
            })
            .collect()
    }

    /// Number of payment records in this distribution.
    #[wasm_bindgen(getter, js_name = paymentRecordCount)]
    pub fn payment_record_count(&self) -> usize {
        self.inner.payment_records.len()
    }

    /// Coverage test results as [test_name, value, passed] tuples.
    #[wasm_bindgen(getter, js_name = coverageTests)]
    pub fn coverage_tests(&self) -> js_sys::Array {
        self.inner
            .coverage_tests
            .iter()
            .map(|(name, value, passed)| {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_str(name));
                arr.push(&JsValue::from_f64(*value));
                arr.push(&JsValue::from_bool(*passed));
                arr
            })
            .collect()
    }

    /// Get all coverage test names.
    #[wasm_bindgen(getter, js_name = coverageTestNames)]
    pub fn coverage_test_names(&self) -> js_sys::Array {
        self.inner
            .coverage_tests
            .iter()
            .map(|(name, _, _)| JsValue::from_str(name))
            .collect()
    }

    /// Check if all coverage tests passed.
    #[wasm_bindgen(getter, js_name = allCoverageTestsPassed)]
    pub fn all_coverage_tests_passed(&self) -> bool {
        self.inner
            .coverage_tests
            .iter()
            .all(|(_, _, passed)| *passed)
    }
}

/// Waterfall execution entrypoint.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = executeWaterfall)]
pub fn execute_waterfall_js(
    engine: &JsWaterfall,
    tranches_json: &str,
    pool_json: &str,
    available_cash: &JsMoney,
    interest_collections: &JsMoney,
    payment_date: &JsDate,
    period_start: &JsDate,
    pool_balance: &JsMoney,
    market: &JsMarketContext,
    coverage_rules_json: Option<String>,
    coverage_triggers_json: Option<String>,
    explain: Option<bool>,
) -> Result<JsWaterfallDistribution, JsValue> {
    let mut waterfall = engine.inner.clone();

    if let Some(json) = coverage_rules_json {
        let rules: CoverageTestRules =
            serde_json::from_str(&json).map_err(|e| js_error(format!("coverage_rules: {e}")))?;
        waterfall = waterfall.with_coverage_rules(rules);
    }

    if let Some(json) = coverage_triggers_json {
        let triggers: Vec<WaterfallCoverageTrigger> =
            serde_json::from_str(&json).map_err(|e| js_error(format!("coverage_triggers: {e}")))?;
        for trig in triggers {
            waterfall = waterfall.add_coverage_trigger(trig);
        }
    }

    let tranches: TrancheStructure =
        serde_json::from_str(tranches_json).map_err(|e| js_error(format!("tranches JSON: {e}")))?;
    let pool: finstack_valuations::instruments::fixed_income::structured_credit::Pool =
        serde_json::from_str(pool_json).map_err(|e| js_error(format!("pool JSON: {e}")))?;

    let ctx = WaterfallContext {
        available_cash: available_cash.inner(),
        interest_collections: interest_collections.inner(),
        payment_date: payment_date.inner(),
        period_start: period_start.inner(),
        pool_balance: pool_balance.inner(),
        market: market.inner(),
        tranche_balances: None,
        reserve_balance: finstack_core::money::Money::new(0.0, pool.base_currency()),
    };

    let distribution = if explain.unwrap_or(false) {
        execute_waterfall_with_explanation(&waterfall, &tranches, &pool, ctx, Default::default())
    } else {
        execute_waterfall(&waterfall, &tranches, &pool, ctx)
    }
    .map_err(|e| js_error(e.to_string()))?;

    Ok(JsWaterfallDistribution {
        inner: distribution,
    })
}
