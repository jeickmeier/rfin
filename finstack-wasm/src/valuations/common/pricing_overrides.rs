//! WASM bindings for instrument pricing overrides.
//!
//! These types allow JavaScript consumers to specify market quotes, model
//! configuration, bump sizes, and scenario adjustments that override
//! default pricing behavior.

use crate::core::error::{core_to_js, js_error};
use finstack_valuations::instruments::pricing_overrides::{
    BumpConfig, InstrumentPricingOverrides, MarketQuoteOverrides, MetricPricingOverrides,
    ModelConfig, PricingOverrides, ScenarioPricingOverrides,
};
use wasm_bindgen::prelude::*;

// =============================================================================
// MarketQuoteOverrides
// =============================================================================

/// Overrides for market-quoted values (prices, vols, spreads, upfront payments).
#[wasm_bindgen(js_name = MarketQuoteOverrides)]
#[derive(Clone)]
pub struct JsMarketQuoteOverrides {
    pub(crate) inner: MarketQuoteOverrides,
}

impl Default for JsMarketQuoteOverrides {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = MarketQuoteOverrides)]
impl JsMarketQuoteOverrides {
    /// Create empty market quote overrides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MarketQuoteOverrides::default(),
        }
    }

    /// Set quoted clean price for bond yield calculations.
    #[wasm_bindgen(js_name = withCleanPrice)]
    pub fn with_clean_price(mut self, price: f64) -> Self {
        self.inner.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility (overrides vol surface).
    #[wasm_bindgen(js_name = withImpliedVol)]
    pub fn with_implied_vol(mut self, vol: f64) -> Self {
        self.inner.implied_volatility = Some(vol);
        self
    }

    /// Set quoted spread in basis points (for credit instruments).
    #[wasm_bindgen(js_name = withSpreadBp)]
    pub fn with_spread_bp(mut self, spread_bp: f64) -> Self {
        self.inner.quoted_spread_bp = Some(spread_bp);
        self
    }
}

// =============================================================================
// BumpConfig
// =============================================================================

/// Bump sizes for finite-difference sensitivity calculations.
#[wasm_bindgen(js_name = BumpConfig)]
#[derive(Clone)]
pub struct JsBumpConfig {
    pub(crate) inner: BumpConfig,
}

impl Default for JsBumpConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = BumpConfig)]
impl JsBumpConfig {
    /// Create default bump config.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: BumpConfig::default(),
        }
    }

    /// Set rate bump size in basis points.
    #[wasm_bindgen(js_name = withRateBumpBp)]
    pub fn with_rate_bump_bp(mut self, bp: f64) -> Self {
        self.inner.rate_bump_bp = Some(bp);
        self
    }

    /// Set credit spread bump size in basis points.
    #[wasm_bindgen(js_name = withCreditSpreadBumpBp)]
    pub fn with_credit_spread_bump_bp(mut self, bp: f64) -> Self {
        self.inner.credit_spread_bump_bp = Some(bp);
        self
    }

    /// Set spot bump size as percentage (0.01 = 1%).
    #[wasm_bindgen(js_name = withSpotBumpPct)]
    pub fn with_spot_bump_pct(mut self, pct: f64) -> Self {
        self.inner.spot_bump_pct = Some(pct);
        self
    }

    /// Set volatility bump size as absolute vol (0.01 = 1% vol).
    #[wasm_bindgen(js_name = withVolBumpPct)]
    pub fn with_vol_bump_pct(mut self, pct: f64) -> Self {
        self.inner.vol_bump_pct = Some(pct);
        self
    }

    /// Enable adaptive bump sizes based on volatility and moneyness.
    #[wasm_bindgen(js_name = withAdaptiveBumps)]
    pub fn with_adaptive_bumps(mut self, enable: bool) -> Self {
        self.inner.adaptive_bumps = enable;
        self
    }
}

// =============================================================================
// ModelConfig
// =============================================================================

/// Model selection and tree pricing parameters.
#[wasm_bindgen(js_name = ModelConfig)]
#[derive(Clone)]
pub struct JsModelConfig {
    pub(crate) inner: ModelConfig,
}

impl Default for JsModelConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = ModelConfig)]
impl JsModelConfig {
    /// Create default model config.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: ModelConfig::default(),
        }
    }

    /// Set number of time steps for tree-based pricing.
    #[wasm_bindgen(js_name = withTreeSteps)]
    pub fn with_tree_steps(mut self, steps: usize) -> Self {
        self.inner.tree_steps = Some(steps);
        self
    }

    /// Set volatility for tree-based pricing (annualized).
    #[wasm_bindgen(js_name = withTreeVolatility)]
    pub fn with_tree_volatility(mut self, vol: f64) -> Self {
        self.inner.tree_volatility = Some(vol);
        self
    }

    /// Set call exercise friction in cents per 100 of par.
    #[wasm_bindgen(js_name = withCallFrictionCents)]
    pub fn with_call_friction_cents(mut self, cents: f64) -> Self {
        self.inner.call_friction_cents = Some(cents);
        self
    }

    /// Set mean reversion speed for Hull-White tree model.
    #[wasm_bindgen(js_name = withMeanReversion)]
    pub fn with_mean_reversion(mut self, mr: f64) -> Self {
        self.inner.mean_reversion = Some(mr);
        self
    }

    /// Set Monte Carlo path count for path-dependent pricers.
    #[wasm_bindgen(js_name = withMcPaths)]
    pub fn with_mc_paths(mut self, paths: usize) -> Self {
        self.inner.mc_paths = Some(paths);
        self
    }
}

// =============================================================================
// MetricPricingOverrides
// =============================================================================

/// Metric-time overrides (bump sizes, theta period, MC seed scenario).
#[wasm_bindgen(js_name = MetricPricingOverrides)]
#[derive(Clone)]
pub struct JsMetricPricingOverrides {
    pub(crate) inner: MetricPricingOverrides,
}

impl Default for JsMetricPricingOverrides {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = MetricPricingOverrides)]
impl JsMetricPricingOverrides {
    /// Create default metric overrides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: MetricPricingOverrides::default(),
        }
    }

    /// Set theta period for time decay calculations (e.g., "1D", "1W", "1M").
    #[wasm_bindgen(js_name = withThetaPeriod)]
    pub fn with_theta_period(mut self, period: String) -> Self {
        self.inner.theta_period = Some(period);
        self
    }

    /// Set MC seed scenario for deterministic greeks.
    #[wasm_bindgen(js_name = withMcSeedScenario)]
    pub fn with_mc_seed_scenario(mut self, scenario: String) -> Self {
        self.inner.mc_seed_scenario = Some(scenario);
        self
    }
}

// =============================================================================
// InstrumentPricingOverrides
// =============================================================================

/// Instrument-owned pricing inputs that can materially change valuation.
#[wasm_bindgen(js_name = InstrumentPricingOverrides)]
#[derive(Clone)]
pub struct JsInstrumentPricingOverrides {
    pub(crate) inner: InstrumentPricingOverrides,
}

impl Default for JsInstrumentPricingOverrides {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = InstrumentPricingOverrides)]
impl JsInstrumentPricingOverrides {
    /// Create default instrument pricing overrides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: InstrumentPricingOverrides::default(),
        }
    }

    /// Create from a JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsInstrumentPricingOverrides, JsValue> {
        let inner: InstrumentPricingOverrides =
            serde_json::from_str(json).map_err(|e| js_error(format!("Invalid JSON: {}", e)))?;
        inner.validate().map_err(core_to_js)?;
        Ok(JsInstrumentPricingOverrides { inner })
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }
}

// =============================================================================
// ScenarioPricingOverrides
// =============================================================================

/// Scenario-only valuation adjustments (price/spread shocks).
#[wasm_bindgen(js_name = ScenarioPricingOverrides)]
#[derive(Clone)]
pub struct JsScenarioPricingOverrides {
    pub(crate) inner: ScenarioPricingOverrides,
}

impl Default for JsScenarioPricingOverrides {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = ScenarioPricingOverrides)]
impl JsScenarioPricingOverrides {
    /// Create empty scenario overrides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: ScenarioPricingOverrides::default(),
        }
    }

    /// Set scenario price shock as decimal percentage (-0.05 = -5%).
    #[wasm_bindgen(js_name = withPriceShock)]
    pub fn with_price_shock(mut self, shock_pct: f64) -> Self {
        self.inner.scenario_price_shock_pct = Some(shock_pct);
        self
    }

    /// Set scenario spread shock in basis points (50.0 = +50bp).
    #[wasm_bindgen(js_name = withSpreadShock)]
    pub fn with_spread_shock(mut self, shock_bp: f64) -> Self {
        self.inner.scenario_spread_shock_bp = Some(shock_bp);
        self
    }
}

// =============================================================================
// PricingOverrides (top-level)
// =============================================================================

/// Complete pricing overrides combining market quotes, model config, metrics, and scenarios.
#[wasm_bindgen(js_name = PricingOverrides)]
#[derive(Clone)]
pub struct JsPricingOverrides {
    pub(crate) inner: PricingOverrides,
}

impl Default for JsPricingOverrides {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = PricingOverrides)]
impl JsPricingOverrides {
    /// Create empty pricing overrides.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: PricingOverrides::default(),
        }
    }

    /// Create from a JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json: &str) -> Result<JsPricingOverrides, JsValue> {
        let inner: PricingOverrides =
            serde_json::from_str(json).map_err(|e| js_error(format!("Invalid JSON: {}", e)))?;
        Ok(JsPricingOverrides { inner })
    }

    /// Serialize to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }

    /// Set quoted clean price.
    #[wasm_bindgen(js_name = withCleanPrice)]
    pub fn with_clean_price(mut self, price: f64) -> Self {
        self.inner.market_quotes.quoted_clean_price = Some(price);
        self
    }

    /// Set implied volatility.
    #[wasm_bindgen(js_name = withImpliedVol)]
    pub fn with_implied_vol(mut self, vol: f64) -> Self {
        self.inner.market_quotes.implied_volatility = Some(vol);
        self
    }

    /// Set tree steps for tree-based pricing.
    #[wasm_bindgen(js_name = withTreeSteps)]
    pub fn with_tree_steps(mut self, steps: usize) -> Self {
        self.inner.model_config.tree_steps = Some(steps);
        self
    }

    /// Set tree volatility.
    #[wasm_bindgen(js_name = withTreeVolatility)]
    pub fn with_tree_volatility(mut self, vol: f64) -> Self {
        self.inner.model_config.tree_volatility = Some(vol);
        self
    }

    /// Set MC paths for path-dependent pricing.
    #[wasm_bindgen(js_name = withMcPaths)]
    pub fn with_mc_paths(mut self, paths: usize) -> Self {
        self.inner.model_config.mc_paths = Some(paths);
        self
    }

    /// Get rho bump in basis points.
    #[wasm_bindgen(getter, js_name = rhoBumpBp)]
    pub fn rho_bump_bp(&self) -> f64 {
        self.inner.rho_bump_bp()
    }
}
