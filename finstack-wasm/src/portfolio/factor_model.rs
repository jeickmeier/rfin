//! Portfolio factor-model risk decomposition bindings for WASM.
//!
//! Wraps the portfolio-level factor-model engine including assignment,
//! sensitivity computation, risk decomposition, what-if analysis, and
//! factor stress testing.

use crate::core::dates::FsDate;
use crate::core::error::core_to_js;
use crate::core::factor_model::{JsFactorDefinition, JsFactorModelConfig};
use crate::core::market_data::context::JsMarketContext;
use crate::portfolio::positions::JsPortfolio;
use crate::valuations::factor_model::JsSensitivityMatrix;
use finstack_core::factor_model::FactorId;
use finstack_portfolio::factor_model::{
    FactorContribution, FactorContributionDelta, FactorModel, FactorModelBuilder,
    PositionChange, RiskDecomposition, StressResult, WhatIfEngine, WhatIfResult,
};
use finstack_portfolio::PositionId;
use js_sys::Array;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn portfolio_to_js(err: finstack_portfolio::Error) -> JsValue {
    core_to_js(err.into())
}

// ---------------------------------------------------------------------------
// FactorContribution
// ---------------------------------------------------------------------------

/// Contribution of a single factor to portfolio risk.
#[wasm_bindgen(js_name = FactorContribution)]
#[derive(Clone)]
pub struct JsFactorContribution {
    inner: FactorContribution,
}

impl JsFactorContribution {
    pub(crate) fn from_inner(inner: FactorContribution) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = FactorContribution)]
impl JsFactorContribution {
    /// Factor identifier.
    #[wasm_bindgen(getter, js_name = factorId)]
    pub fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    /// Absolute contribution to the chosen risk measure.
    #[wasm_bindgen(getter, js_name = absoluteRisk)]
    pub fn absolute_risk(&self) -> f64 {
        self.inner.absolute_risk
    }

    /// Contribution as share of total portfolio risk.
    #[wasm_bindgen(getter, js_name = relativeRisk)]
    pub fn relative_risk(&self) -> f64 {
        self.inner.relative_risk
    }

    /// Marginal sensitivity of portfolio risk to the factor.
    #[wasm_bindgen(getter, js_name = marginalRisk)]
    pub fn marginal_risk(&self) -> f64 {
        self.inner.marginal_risk
    }
}

// ---------------------------------------------------------------------------
// RiskDecomposition
// ---------------------------------------------------------------------------

/// Portfolio-level decomposition of total risk across common factors.
#[wasm_bindgen(js_name = RiskDecomposition)]
#[derive(Clone)]
pub struct JsRiskDecomposition {
    inner: RiskDecomposition,
}

#[allow(dead_code)]
impl JsRiskDecomposition {
    pub(crate) fn from_inner(inner: RiskDecomposition) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &RiskDecomposition {
        &self.inner
    }
}

#[wasm_bindgen(js_class = RiskDecomposition)]
impl JsRiskDecomposition {
    /// Total portfolio risk under the selected measure.
    #[wasm_bindgen(getter, js_name = totalRisk)]
    pub fn total_risk(&self) -> f64 {
        self.inner.total_risk
    }

    /// Risk measure name.
    #[wasm_bindgen(getter)]
    pub fn measure(&self) -> String {
        use finstack_core::factor_model::RiskMeasure;
        match self.inner.measure {
            RiskMeasure::Variance => "Variance".to_string(),
            RiskMeasure::Volatility => "Volatility".to_string(),
            RiskMeasure::VaR { .. } => "VaR".to_string(),
            RiskMeasure::ExpectedShortfall { .. } => "ExpectedShortfall".to_string(),
        }
    }

    /// Factor-level risk contributions.
    #[wasm_bindgen(getter, js_name = factorContributions)]
    pub fn factor_contributions(&self) -> Array {
        self.inner
            .factor_contributions
            .iter()
            .cloned()
            .map(|c| JsValue::from(JsFactorContribution::from_inner(c)))
            .collect()
    }

    /// Unattributed residual risk.
    #[wasm_bindgen(getter, js_name = residualRisk)]
    pub fn residual_risk(&self) -> f64 {
        self.inner.residual_risk
    }
}

// ---------------------------------------------------------------------------
// FactorContributionDelta
// ---------------------------------------------------------------------------

/// Change in factor contribution from a what-if analysis.
#[wasm_bindgen(js_name = FactorContributionDelta)]
#[derive(Clone)]
pub struct JsFactorContributionDelta {
    inner: FactorContributionDelta,
}

impl JsFactorContributionDelta {
    pub(crate) fn from_inner(inner: FactorContributionDelta) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = FactorContributionDelta)]
impl JsFactorContributionDelta {
    /// Factor identifier.
    #[wasm_bindgen(getter, js_name = factorId)]
    pub fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    /// Absolute risk change.
    #[wasm_bindgen(getter, js_name = absoluteChange)]
    pub fn absolute_change(&self) -> f64 {
        self.inner.absolute_change
    }

    /// Relative risk change.
    #[wasm_bindgen(getter, js_name = relativeChange)]
    pub fn relative_change(&self) -> f64 {
        self.inner.relative_change
    }
}

// ---------------------------------------------------------------------------
// WhatIfResult
// ---------------------------------------------------------------------------

/// Result of a what-if position change analysis.
#[wasm_bindgen(js_name = WhatIfResult)]
#[derive(Clone)]
pub struct JsWhatIfResult {
    inner: WhatIfResult,
}

impl JsWhatIfResult {
    pub(crate) fn from_inner(inner: WhatIfResult) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = WhatIfResult)]
impl JsWhatIfResult {
    /// Risk decomposition before the change.
    #[wasm_bindgen(getter)]
    pub fn before(&self) -> JsRiskDecomposition {
        JsRiskDecomposition::from_inner(self.inner.before.clone())
    }

    /// Risk decomposition after the change.
    #[wasm_bindgen(getter)]
    pub fn after(&self) -> JsRiskDecomposition {
        JsRiskDecomposition::from_inner(self.inner.after.clone())
    }

    /// Per-factor contribution deltas.
    #[wasm_bindgen(getter)]
    pub fn delta(&self) -> Array {
        self.inner
            .delta
            .iter()
            .cloned()
            .map(|d| JsValue::from(JsFactorContributionDelta::from_inner(d)))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// StressResult
// ---------------------------------------------------------------------------

/// Result of a factor stress test.
#[wasm_bindgen(js_name = StressResult)]
#[derive(Clone)]
pub struct JsStressResult {
    inner: StressResult,
}

impl JsStressResult {
    pub(crate) fn from_inner(inner: StressResult) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = StressResult)]
impl JsStressResult {
    /// Total P&L from the stress.
    #[wasm_bindgen(getter, js_name = totalPnl)]
    pub fn total_pnl(&self) -> f64 {
        self.inner.total_pnl
    }

    /// Stressed risk decomposition.
    #[wasm_bindgen(getter, js_name = stressedDecomposition)]
    pub fn stressed_decomposition(&self) -> JsRiskDecomposition {
        JsRiskDecomposition::from_inner(self.inner.stressed_decomposition.clone())
    }
}

// ---------------------------------------------------------------------------
// FactorModelBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a FactorModel from configuration.
///
/// @example
/// ```javascript
/// const model = new FactorModelBuilder()
///   .config(myConfig)
///   .build();
/// ```
#[wasm_bindgen(js_name = FactorModelBuilder)]
pub struct JsFactorModelBuilder {
    inner: Option<FactorModelBuilder>,
}

impl Default for JsFactorModelBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FactorModelBuilder)]
impl JsFactorModelBuilder {
    /// Create a new builder.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Some(FactorModelBuilder::new()),
        }
    }

    /// Set the factor model configuration.
    pub fn config(mut self, config: &JsFactorModelConfig) -> Self {
        if let Some(builder) = self.inner.take() {
            self.inner = Some(builder.config(config.inner().clone()));
        }
        self
    }

    /// Build the factor model.
    pub fn build(mut self) -> Result<JsFactorModel, JsValue> {
        let builder = self.inner.take().ok_or_else(|| {
            crate::core::error::js_error("Builder already consumed")
        })?;
        let model = builder.build().map_err(portfolio_to_js)?;
        Ok(JsFactorModel {
            inner: Arc::new(model),
        })
    }
}

// ---------------------------------------------------------------------------
// FactorModel
// ---------------------------------------------------------------------------

/// Portfolio factor-model engine for risk decomposition and what-if analysis.
///
/// @example
/// ```javascript
/// const model = new FactorModelBuilder().config(config).build();
/// const decomp = model.analyze(portfolio, market, asOf);
/// console.log(decomp.totalRisk);
/// ```
#[wasm_bindgen(js_name = FactorModel)]
#[derive(Clone)]
pub struct JsFactorModel {
    inner: Arc<FactorModel>,
}

#[wasm_bindgen(js_class = FactorModel)]
impl JsFactorModel {
    /// Get the factor definitions used by this model.
    pub fn factors(&self) -> Array {
        self.inner
            .factors()
            .iter()
            .cloned()
            .map(|f| JsValue::from(JsFactorDefinition::from_inner(f)))
            .collect()
    }

    /// Decompose portfolio risk across configured factors.
    pub fn analyze(
        &self,
        portfolio: &JsPortfolio,
        market: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<JsRiskDecomposition, JsValue> {
        self.inner
            .analyze(&portfolio.inner, market.inner(), as_of.inner())
            .map(JsRiskDecomposition::from_inner)
            .map_err(portfolio_to_js)
    }

    /// Compute the sensitivity matrix for a portfolio.
    #[wasm_bindgen(js_name = computeSensitivities)]
    pub fn compute_sensitivities(
        &self,
        portfolio: &JsPortfolio,
        market: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<JsSensitivityMatrix, JsValue> {
        self.inner
            .compute_sensitivities(&portfolio.inner, market.inner(), as_of.inner())
            .map(JsSensitivityMatrix::from_inner)
            .map_err(portfolio_to_js)
    }

    /// Create a what-if engine for scenario analysis.
    #[wasm_bindgen(js_name = whatIf)]
    pub fn what_if(
        &self,
        base: &JsRiskDecomposition,
        sensitivities: &JsSensitivityMatrix,
        portfolio: &JsPortfolio,
        market: &JsMarketContext,
        as_of: &FsDate,
    ) -> JsWhatIfEngineWrapper {
        JsWhatIfEngineWrapper {
            model: self.inner.clone(),
            base: base.inner.clone(),
            sensitivities: sensitivities.inner().clone(),
            portfolio: portfolio.inner.clone(),
            market: market.inner().clone(),
            as_of: as_of.inner(),
        }
    }
}

// ---------------------------------------------------------------------------
// WhatIfEngine wrapper
// ---------------------------------------------------------------------------

/// Engine for position what-if, factor stress, and optimization analysis.
#[wasm_bindgen(js_name = WhatIfEngine)]
#[derive(Clone)]
pub struct JsWhatIfEngineWrapper {
    model: Arc<FactorModel>,
    base: RiskDecomposition,
    sensitivities: finstack_valuations::factor_model::sensitivity::SensitivityMatrix,
    portfolio: finstack_portfolio::Portfolio,
    market: finstack_core::market_data::context::MarketContext,
    as_of: time::Date,
}

impl JsWhatIfEngineWrapper {
    fn with_inner<T>(
        &self,
        f: impl FnOnce(WhatIfEngine<'_>) -> finstack_portfolio::Result<T>,
    ) -> finstack_portfolio::Result<T> {
        let inner = WhatIfEngine::new(
            self.model.as_ref(),
            &self.base,
            &self.sensitivities,
            &self.portfolio,
            &self.market,
            self.as_of,
        );
        f(inner)
    }
}

#[wasm_bindgen(js_class = WhatIfEngine)]
impl JsWhatIfEngineWrapper {
    /// Evaluate the risk impact of position changes.
    #[wasm_bindgen(js_name = positionWhatIf)]
    pub fn position_what_if(
        &self,
        remove_position_ids: Vec<String>,
    ) -> Result<JsWhatIfResult, JsValue> {
        let changes: Vec<PositionChange> = remove_position_ids
            .into_iter()
            .map(|id| PositionChange::Remove {
                position_id: PositionId::new(id),
            })
            .collect();
        self.with_inner(|inner| inner.position_what_if(&changes))
            .map(JsWhatIfResult::from_inner)
            .map_err(portfolio_to_js)
    }

    /// Apply a factor stress scenario.
    #[wasm_bindgen(js_name = factorStress)]
    pub fn factor_stress(
        &self,
        factor_ids: Vec<String>,
        shifts: Vec<f64>,
    ) -> Result<JsStressResult, JsValue> {
        let stresses: Vec<(FactorId, f64)> = factor_ids
            .into_iter()
            .zip(shifts)
            .map(|(id, shift)| (FactorId::new(id), shift))
            .collect();
        self.with_inner(|inner| inner.factor_stress(&stresses))
            .map(JsStressResult::from_inner)
            .map_err(portfolio_to_js)
    }
}
