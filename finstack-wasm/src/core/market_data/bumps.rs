//! WASM bindings for market bump specifications.
//!
//! Provides types for applying parallel shocks and bumps to market data.
//! Used for risk metrics (DV01, CS01), scenario analysis, and stress tests.

use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use crate::utils::json::to_js_value;
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::{BumpMode, BumpSpec, BumpType, BumpUnits, MarketBump};
use finstack_core::types::CurveId;
use wasm_bindgen::prelude::*;

/// Mode of applying a bump.
#[wasm_bindgen(js_name = BumpMode)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsBumpMode {
    /// Additive bump in normalized fractional form (e.g., 100bp = 0.01).
    Additive = 0,
    /// Multiplicative bump as a factor (e.g., 1.1 = +10%).
    Multiplicative = 1,
}

impl From<JsBumpMode> for BumpMode {
    fn from(mode: JsBumpMode) -> Self {
        match mode {
            JsBumpMode::Additive => BumpMode::Additive,
            JsBumpMode::Multiplicative => BumpMode::Multiplicative,
        }
    }
}

impl From<BumpMode> for JsBumpMode {
    fn from(mode: BumpMode) -> Self {
        match mode {
            BumpMode::Additive => JsBumpMode::Additive,
            BumpMode::Multiplicative => JsBumpMode::Multiplicative,
            _ => JsBumpMode::Additive, // Default for future variants
        }
    }
}

/// Units for the bump magnitude.
#[wasm_bindgen(js_name = BumpUnits)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsBumpUnits {
    /// Basis points for rates/spreads (100bp = 0.01).
    RateBp = 0,
    /// Percent units (2.0 = 2%).
    Percent = 1,
    /// Direct fraction (0.02 = 2%).
    Fraction = 2,
    /// Direct factor (1.10 = +10%). Only valid for Multiplicative mode.
    Factor = 3,
}

impl From<JsBumpUnits> for BumpUnits {
    fn from(units: JsBumpUnits) -> Self {
        match units {
            JsBumpUnits::RateBp => BumpUnits::RateBp,
            JsBumpUnits::Percent => BumpUnits::Percent,
            JsBumpUnits::Fraction => BumpUnits::Fraction,
            JsBumpUnits::Factor => BumpUnits::Factor,
        }
    }
}

impl From<BumpUnits> for JsBumpUnits {
    fn from(units: BumpUnits) -> Self {
        match units {
            BumpUnits::RateBp => JsBumpUnits::RateBp,
            BumpUnits::Percent => JsBumpUnits::Percent,
            BumpUnits::Fraction => JsBumpUnits::Fraction,
            BumpUnits::Factor => JsBumpUnits::Factor,
            _ => JsBumpUnits::Fraction, // Default for future variants
        }
    }
}

/// Type of bump to apply (parallel or key-rate).
#[wasm_bindgen(js_name = BumpType)]
pub struct JsBumpType {
    inner: BumpType,
}

#[wasm_bindgen(js_class = BumpType)]
impl JsBumpType {
    /// Create a parallel bump type.
    #[wasm_bindgen(js_name = parallel)]
    pub fn parallel() -> JsBumpType {
        JsBumpType {
            inner: BumpType::Parallel,
        }
    }

    /// Create a triangular key-rate bump type with explicit bucket neighbors.
    ///
    /// # Arguments
    /// * `prev_bucket` - Previous bucket time in years (use 0.0 for first bucket)
    /// * `target_bucket` - Target bucket time in years (peak of the triangle)
    /// * `next_bucket` - Next bucket time in years (use Infinity for last bucket)
    #[wasm_bindgen(js_name = triangularKeyRate)]
    pub fn triangular_key_rate(
        prev_bucket: f64,
        target_bucket: f64,
        next_bucket: f64,
    ) -> JsBumpType {
        JsBumpType {
            inner: BumpType::TriangularKeyRate {
                prev_bucket,
                target_bucket,
                next_bucket,
            },
        }
    }

    /// Check if this is a triangular key-rate bump.
    #[wasm_bindgen(getter, js_name = isTriangularKeyRate)]
    pub fn is_triangular_key_rate(&self) -> bool {
        matches!(self.inner, BumpType::TriangularKeyRate { .. })
    }

    /// Get the target bucket for key-rate bumps, or null for parallel bumps.
    #[wasm_bindgen(getter, js_name = targetBucket)]
    pub fn target_bucket(&self) -> Option<f64> {
        match self.inner {
            BumpType::TriangularKeyRate { target_bucket, .. } => Some(target_bucket),
            BumpType::Parallel => None,
        }
    }
}

/// Unified bump specification capturing mode, units, and value.
#[wasm_bindgen(js_name = BumpSpec)]
pub struct JsBumpSpec {
    inner: BumpSpec,
}

#[wasm_bindgen(js_class = BumpSpec)]
impl JsBumpSpec {
    /// Create a new bump specification.
    ///
    /// # Arguments
    /// * `mode` - How the bump is applied (additive or multiplicative)
    /// * `units` - Units for interpreting the bump magnitude
    /// * `value` - Magnitude of the bump
    /// * `bump_type` - Optional bump type (defaults to Parallel)
    #[wasm_bindgen(constructor)]
    pub fn new(
        mode: JsBumpMode,
        units: JsBumpUnits,
        value: f64,
        bump_type: Option<JsBumpType>,
    ) -> JsBumpSpec {
        let bt = bump_type.map(|t| t.inner).unwrap_or(BumpType::Parallel);
        JsBumpSpec {
            inner: BumpSpec {
                mode: mode.into(),
                units: units.into(),
                value,
                bump_type: bt,
            },
        }
    }

    /// Create an additive bump specified in basis points (e.g., 100.0 = 100bp = 1%).
    #[wasm_bindgen(js_name = parallelBp)]
    pub fn parallel_bp(bump_bp: f64) -> JsBumpSpec {
        JsBumpSpec {
            inner: BumpSpec::parallel_bp(bump_bp),
        }
    }

    /// Create a triangular key-rate bump with explicit bucket neighbors.
    ///
    /// # Arguments
    /// * `prev_bucket` - Previous bucket time in years (use 0.0 for first bucket)
    /// * `target_bucket` - Target bucket time in years (peak of the triangle)
    /// * `next_bucket` - Next bucket time in years (use Infinity for last bucket)
    /// * `bump_bp` - Bump size in basis points
    #[wasm_bindgen(js_name = triangularKeyRateBp)]
    pub fn triangular_key_rate_bp(
        prev_bucket: f64,
        target_bucket: f64,
        next_bucket: f64,
        bump_bp: f64,
    ) -> JsBumpSpec {
        JsBumpSpec {
            inner: BumpSpec::triangular_key_rate_bp(
                prev_bucket,
                target_bucket,
                next_bucket,
                bump_bp,
            ),
        }
    }

    /// Create a multiplicative bump given as a factor (e.g., 1.1 = +10%).
    #[wasm_bindgen(js_name = multiplier)]
    pub fn multiplier(factor: f64) -> JsBumpSpec {
        JsBumpSpec {
            inner: BumpSpec::multiplier(factor),
        }
    }

    /// Create an additive inflation shift specified in percent (e.g., 2.0 = +2%).
    #[wasm_bindgen(js_name = inflationShiftPct)]
    pub fn inflation_shift_pct(bump_pct: f64) -> JsBumpSpec {
        JsBumpSpec {
            inner: BumpSpec::inflation_shift_pct(bump_pct),
        }
    }

    /// Create an additive correlation shift specified in percent (e.g., 10.0 = +10%).
    #[wasm_bindgen(js_name = correlationShiftPct)]
    pub fn correlation_shift_pct(bump_pct: f64) -> JsBumpSpec {
        JsBumpSpec {
            inner: BumpSpec::correlation_shift_pct(bump_pct),
        }
    }

    /// Get the bump mode.
    #[wasm_bindgen(getter)]
    pub fn mode(&self) -> JsBumpMode {
        self.inner.mode.into()
    }

    /// Get the bump units.
    #[wasm_bindgen(getter)]
    pub fn units(&self) -> JsBumpUnits {
        self.inner.units.into()
    }

    /// Get the bump value.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.inner.value
    }

    /// Get the bump type.
    #[wasm_bindgen(getter, js_name = bumpType)]
    pub fn bump_type(&self) -> JsBumpType {
        JsBumpType {
            inner: self.inner.bump_type,
        }
    }

    /// Serialize the bump spec to a JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize the bump spec to a JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Deserialize a bump spec from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsBumpSpec, JsValue> {
        let inner: BumpSpec =
            serde_json::from_str(json_str).map_err(|e| js_error(e.to_string()))?;
        Ok(JsBumpSpec { inner })
    }
}

/// Unified bump description spanning curves, surfaces, FX, and scalar prices.
#[wasm_bindgen(js_name = MarketBump)]
pub struct JsMarketBump {
    inner: MarketBump,
}

#[wasm_bindgen(js_class = MarketBump)]
impl JsMarketBump {
    /// Create a curve bump.
    ///
    /// # Arguments
    /// * `id` - Identifier of the curve/surface/price entry
    /// * `spec` - How to bump the entry
    #[wasm_bindgen(js_name = curve)]
    pub fn curve(id: &str, spec: &JsBumpSpec) -> JsMarketBump {
        JsMarketBump {
            inner: MarketBump::Curve {
                id: CurveId::from(id),
                spec: spec.inner,
            },
        }
    }

    /// Create an FX rate percentage shock.
    ///
    /// # Arguments
    /// * `base` - Base currency
    /// * `quote` - Quote currency
    /// * `pct` - Percentage change (e.g., 5.0 = +5%)
    /// * `as_of` - Valuation date (YYYY-MM-DD string)
    #[wasm_bindgen(js_name = fxPct)]
    pub fn fx_pct(
        base: &JsCurrency,
        quote: &JsCurrency,
        pct: f64,
        as_of: &str,
    ) -> Result<JsMarketBump, JsValue> {
        let date = Date::parse(as_of, &time::format_description::well_known::Iso8601::DATE)
            .map_err(|e| js_error(format!("Invalid date: {}", e)))?;
        Ok(JsMarketBump {
            inner: MarketBump::FxPct {
                base: base.inner(),
                quote: quote.inner(),
                pct,
                as_of: date,
            },
        })
    }

    /// Create a volatility surface bucket bump (percentage multiplier).
    ///
    /// # Arguments
    /// * `surface_id` - Surface identifier
    /// * `pct` - Percentage change to apply
    /// * `expiries` - Optional expiry filters (year fractions)
    /// * `strikes` - Optional strike filters
    #[wasm_bindgen(js_name = volBucketPct)]
    pub fn vol_bucket_pct(
        surface_id: &str,
        pct: f64,
        expiries: Option<Vec<f64>>,
        strikes: Option<Vec<f64>>,
    ) -> JsMarketBump {
        JsMarketBump {
            inner: MarketBump::VolBucketPct {
                surface_id: CurveId::from(surface_id),
                expiries,
                strikes,
                pct,
            },
        }
    }

    /// Create a base correlation bucket bump (additive points).
    ///
    /// # Arguments
    /// * `surface_id` - Curve identifier
    /// * `points` - Absolute correlation points to add (0.02 = +2 points)
    /// * `detachments` - Optional detachment filters (percent)
    #[wasm_bindgen(js_name = baseCorrBucketPts)]
    pub fn base_corr_bucket_pts(
        surface_id: &str,
        points: f64,
        detachments: Option<Vec<f64>>,
    ) -> JsMarketBump {
        JsMarketBump {
            inner: MarketBump::BaseCorrBucketPts {
                surface_id: CurveId::from(surface_id),
                detachments,
                points,
            },
        }
    }

    /// Get the kind of bump ("curve", "fxPct", "volBucketPct", "baseCorrBucketPts").
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match &self.inner {
            MarketBump::Curve { .. } => "curve".to_string(),
            MarketBump::FxPct { .. } => "fxPct".to_string(),
            MarketBump::VolBucketPct { .. } => "volBucketPct".to_string(),
            MarketBump::BaseCorrBucketPts { .. } => "baseCorrBucketPts".to_string(),
        }
    }

    /// Serialize the market bump to a JavaScript object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize the market bump to a JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Deserialize a market bump from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsMarketBump, JsValue> {
        let inner: MarketBump =
            serde_json::from_str(json_str).map_err(|e| js_error(e.to_string()))?;
        Ok(JsMarketBump { inner })
    }
}
