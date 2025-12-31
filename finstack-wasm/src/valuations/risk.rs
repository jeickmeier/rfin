//! WASM bindings for risk calculations (VaR and ladders).
//!
//! These wrappers perform only JS↔Rust type conversion and delegate all
//! calculations to the underlying finstack_valuations code.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::valuations::instruments::{extract_instrument, JsBond};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::risk::{
    calculate_var, MarketHistory, MarketScenario, RiskFactorShift, RiskFactorType, VarConfig,
    VarMethod, VarResult,
};
use finstack_valuations::metrics::{standard_registry, MetricContext, MetricId};
use js_sys::{Array, Object, Reflect};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// =============================================================================
// VaR Types
// =============================================================================

/// VaR calculation method.
#[wasm_bindgen(js_name = VarMethod)]
#[derive(Clone, Copy)]
pub struct JsVarMethod {
    inner: VarMethod,
}

#[wasm_bindgen(js_class = VarMethod)]
impl JsVarMethod {
    /// Full revaluation VaR.
    #[wasm_bindgen(constructor)]
    pub fn full_revaluation() -> Self {
        Self {
            inner: VarMethod::FullRevaluation,
        }
    }

    /// Taylor approximation VaR.
    #[wasm_bindgen(js_name = taylorApproximation)]
    pub fn taylor_approximation() -> Self {
        Self {
            inner: VarMethod::TaylorApproximation,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// VaR configuration wrapper.
#[wasm_bindgen(js_name = VarConfig)]
#[derive(Clone)]
pub struct JsVarConfig {
    inner: VarConfig,
}

impl JsVarConfig {
    pub(crate) fn inner(&self) -> &VarConfig {
        &self.inner
    }
}

#[wasm_bindgen(js_class = VarConfig)]
impl JsVarConfig {
    /// Create a VaR config.
    #[wasm_bindgen(constructor)]
    pub fn new(confidence_level: f64, method: Option<JsVarMethod>) -> Result<Self, JsValue> {
        if !(0.0..=1.0).contains(&confidence_level) {
            return Err(js_error(
                "confidenceLevel must be between 0.0 and 1.0 (inclusive)",
            ));
        }

        let var_method = method
            .map(|m| m.inner)
            .unwrap_or(VarMethod::FullRevaluation);

        Ok(Self {
            inner: VarConfig::new(confidence_level).with_method(var_method),
        })
    }

    /// Convenience config for 95% VaR.
    #[wasm_bindgen(js_name = var95)]
    pub fn var_95() -> Self {
        Self {
            inner: VarConfig::var_95(),
        }
    }

    /// Convenience config for 99% VaR.
    #[wasm_bindgen(js_name = var99)]
    pub fn var_99() -> Self {
        Self {
            inner: VarConfig::var_99(),
        }
    }

    #[wasm_bindgen(getter, js_name = confidenceLevel)]
    pub fn confidence_level(&self) -> f64 {
        self.inner.confidence_level
    }

    #[wasm_bindgen(getter)]
    pub fn method(&self) -> JsVarMethod {
        JsVarMethod {
            inner: self.inner.method,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "VarConfig(confidence_level={}, method={:?})",
            self.inner.confidence_level, self.inner.method
        )
    }
}

/// VaR result wrapper.
#[wasm_bindgen(js_name = VarResult)]
pub struct JsVarResult {
    inner: VarResult,
}

#[wasm_bindgen(js_class = VarResult)]
impl JsVarResult {
    #[wasm_bindgen(getter)]
    pub fn var(&self) -> f64 {
        self.inner.var
    }

    #[wasm_bindgen(getter, js_name = expectedShortfall)]
    pub fn expected_shortfall(&self) -> f64 {
        self.inner.expected_shortfall
    }

    #[wasm_bindgen(getter, js_name = pnlDistribution)]
    pub fn pnl_distribution(&self) -> Array {
        self.inner
            .pnl_distribution
            .iter()
            .map(|v| JsValue::from_f64(*v))
            .collect()
    }

    #[wasm_bindgen(getter, js_name = confidenceLevel)]
    pub fn confidence_level(&self) -> f64 {
        self.inner.confidence_level
    }

    #[wasm_bindgen(getter, js_name = numScenarios)]
    pub fn num_scenarios(&self) -> usize {
        self.inner.num_scenarios
    }
}

/// Risk factor type wrapper.
#[wasm_bindgen(js_name = RiskFactorType)]
#[derive(Clone)]
pub struct JsRiskFactorType {
    inner: RiskFactorType,
}

#[wasm_bindgen(js_class = RiskFactorType)]
impl JsRiskFactorType {
    #[wasm_bindgen(js_name = discountRate)]
    pub fn discount_rate(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::DiscountRate {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    #[wasm_bindgen(js_name = forwardRate)]
    pub fn forward_rate(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::ForwardRate {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    #[wasm_bindgen(js_name = creditSpread)]
    pub fn credit_spread(curve_id: String, tenor_years: f64) -> Self {
        Self {
            inner: RiskFactorType::CreditSpread {
                curve_id: curve_id.into(),
                tenor_years,
            },
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Single risk factor shift.
#[wasm_bindgen(js_name = RiskFactorShift)]
#[derive(Clone)]
pub struct JsRiskFactorShift {
    inner: RiskFactorShift,
}

#[wasm_bindgen(js_class = RiskFactorShift)]
impl JsRiskFactorShift {
    #[wasm_bindgen(constructor)]
    pub fn new(factor: &JsRiskFactorType, shift: f64) -> Self {
        Self {
            inner: RiskFactorShift {
                factor: factor.inner.clone(),
                shift,
            },
        }
    }

    #[wasm_bindgen(getter)]
    pub fn shift(&self) -> f64 {
        self.inner.shift
    }

    #[wasm_bindgen(getter)]
    pub fn factor(&self) -> JsRiskFactorType {
        JsRiskFactorType {
            inner: self.inner.factor.clone(),
        }
    }
}

/// Historical market scenario.
#[wasm_bindgen(js_name = MarketScenario)]
#[derive(Clone)]
pub struct JsMarketScenario {
    inner: MarketScenario,
}

#[wasm_bindgen(js_class = MarketScenario)]
impl JsMarketScenario {
    #[wasm_bindgen(constructor)]
    pub fn new(date: &JsDate, shifts: Vec<JsRiskFactorShift>) -> JsMarketScenario {
        let shift_inner = shifts.into_iter().map(|s| s.inner).collect();
        JsMarketScenario {
            inner: MarketScenario::new(date.inner(), shift_inner),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn date(&self) -> JsDate {
        JsDate::from_core(self.inner.date)
    }

    #[wasm_bindgen(getter)]
    pub fn shifts(&self) -> Array {
        self.inner
            .shifts
            .iter()
            .cloned()
            .map(|s| JsRiskFactorShift { inner: s })
            .map(JsValue::from)
            .collect()
    }
}

/// Historical market data for VaR.
#[wasm_bindgen(js_name = MarketHistory)]
#[derive(Clone)]
pub struct JsMarketHistory {
    inner: MarketHistory,
}

impl JsMarketHistory {
    pub(crate) fn inner(&self) -> &MarketHistory {
        &self.inner
    }
}

#[wasm_bindgen(js_class = MarketHistory)]
impl JsMarketHistory {
    #[wasm_bindgen(constructor)]
    pub fn new(
        base_date: &JsDate,
        window_days: u32,
        scenarios: Vec<JsMarketScenario>,
    ) -> JsMarketHistory {
        let scenario_inner = scenarios.into_iter().map(|s| s.inner).collect();
        JsMarketHistory {
            inner: MarketHistory::new(base_date.inner(), window_days, scenario_inner),
        }
    }

    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> JsDate {
        JsDate::from_core(self.inner.base_date)
    }

    #[wasm_bindgen(getter, js_name = windowDays)]
    pub fn window_days(&self) -> u32 {
        self.inner.window_days
    }

    #[wasm_bindgen(getter)]
    pub fn scenarios(&self) -> Array {
        self.inner
            .scenarios
            .iter()
            .cloned()
            .map(|s| JsMarketScenario { inner: s })
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name = numScenarios)]
    pub fn num_scenarios(&self) -> usize {
        self.inner.len()
    }
}

// =============================================================================
// VaR Calculation Functions
// =============================================================================

/// Calculate historical VaR for one or more instruments.
#[wasm_bindgen(js_name = calculateVar)]
pub fn calculate_var_js(
    instruments: &JsValue,
    market: &JsMarketContext,
    history: &JsMarketHistory,
    as_of: &JsDate,
    config: &JsVarConfig,
) -> Result<JsVarResult, JsValue> {
    let mut handles: Vec<Box<dyn Instrument>> = Vec::new();
    if instruments.is_instance_of::<Array>() {
        for inst in Array::from(instruments).iter() {
            handles.push(extract_instrument(&inst)?);
        }
    } else {
        handles.push(extract_instrument(instruments)?);
    }

    let refs: Vec<&dyn Instrument> = handles
        .iter()
        .map(|h| h.as_ref() as &dyn Instrument)
        .collect();

    calculate_var(
        &refs,
        market.inner(),
        history.inner(),
        as_of.inner(),
        config.inner(),
    )
    .map(|inner| JsVarResult { inner })
    .map_err(|e| js_error(format!("VaR calculation failed: {}", e)))
}

fn bucketed_metric(
    instrument: &dyn Instrument,
    market: &JsMarketContext,
    as_of: &JsDate,
    metric_id: MetricId,
    value_key: &str,
) -> Result<JsValue, JsValue> {
    let as_of_date = as_of.inner();

    let base_value = instrument
        .value(market.inner(), as_of_date)
        .map_err(|e| js_error(format!("Pricing failed: {}", e)))?;

    let mut context = MetricContext::new(
        Arc::from(instrument.clone_box()),
        Arc::new(market.inner().clone()),
        as_of_date,
        base_value,
        MetricContext::default_config(),
    );

    let registry = standard_registry();
    registry
        .compute(std::slice::from_ref(&metric_id), &mut context)
        .map_err(|e| js_error(format!("Metric computation failed: {}", e)))?;

    let series = context
        .computed_series
        .get(&metric_id)
        .ok_or_else(|| js_error(format!("{} series not available", value_key)))?;

    let bucket_array = Array::from_iter(series.iter().map(|(b, _)| JsValue::from_str(b)));
    let value_array = Array::from_iter(series.iter().map(|(_, v)| JsValue::from_f64(*v)));

    let result = Object::new();
    Reflect::set(&result, &JsValue::from_str("bucket"), &bucket_array)?;
    Reflect::set(&result, &JsValue::from_str(value_key), &value_array)?;

    Ok(result.into())
}

/// Compute Key Rate Duration (KRD) DV01 ladder for a bond using core metrics.
#[wasm_bindgen(js_name = krdDv01Ladder)]
pub fn krd_dv01_ladder(
    bond: &JsBond,
    market: &JsMarketContext,
    as_of: &JsDate,
) -> Result<JsValue, JsValue> {
    let bond_inner = bond.inner_bond();
    bucketed_metric(&bond_inner, market, as_of, MetricId::BucketedDv01, "dv01")
}

/// Compute CS01 ladder for a bond using core metrics.
#[wasm_bindgen(js_name = cs01Ladder)]
pub fn cs01_ladder(
    bond: &JsBond,
    market: &JsMarketContext,
    as_of: &JsDate,
) -> Result<JsValue, JsValue> {
    let bond_inner = bond.inner_bond();
    bucketed_metric(&bond_inner, market, as_of, MetricId::BucketedCs01, "cs01")
}
