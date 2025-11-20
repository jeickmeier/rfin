//! SABR volatility model bindings for WASM.
//!
//! Provides SABR (Stochastic Alpha Beta Rho) model parameters and calibration
//! support for implied volatility surface modeling.

use finstack_valuations::calibration::{
    SABRCalibrationDerivatives, SABRMarketData, SABRModelParams,
};
use wasm_bindgen::prelude::*;

/// SABR model parameters for volatility surface calibration.
///
/// The SABR model is a stochastic volatility model widely used for modeling
/// implied volatility smiles in options markets.
///
/// Parameters:
/// - `alpha`: Initial volatility level (must be positive)
/// - `nu`: Volatility of volatility (vol-of-vol, typically 0.1 to 1.0)
/// - `rho`: Correlation between forward price and volatility (typically -0.9 to 0.9)
/// - `beta`: CEV exponent (0.0 for normal, 1.0 for lognormal, 0.5 for rates)
///
/// @example
/// ```typescript
/// // Equity market standard (beta=1.0)
/// const params = SABRModelParams.equityStandard(0.2, 0.4, -0.3);
/// console.log(params.beta); // 1.0
///
/// // Interest rate market standard (beta=0.5)
/// const ratesParams = SABRModelParams.ratesStandard(0.01, 0.2, 0.1);
/// console.log(ratesParams.beta); // 0.5
/// ```
#[wasm_bindgen(js_name = SABRModelParams)]
#[derive(Clone, Debug)]
pub struct JsSABRModelParams {
    inner: SABRModelParams,
}

impl JsSABRModelParams {
    pub(crate) fn from_inner(inner: SABRModelParams) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> SABRModelParams {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = SABRModelParams)]
impl JsSABRModelParams {
    /// Create SABR model parameters with explicit values.
    ///
    /// @param {number} alpha - Initial volatility level (must be positive)
    /// @param {number} nu - Volatility of volatility (typically 0.1 to 1.0)
    /// @param {number} rho - Correlation between forward and volatility (-1 to 1)
    /// @param {number} beta - CEV exponent (0.0 to 1.0)
    /// @returns {SABRModelParams} Configured SABR parameters
    /// @throws {Error} If parameters are out of reasonable ranges
    ///
    /// @example
    /// ```typescript
    /// const params = new SABRModelParams(0.2, 0.4, -0.3, 1.0);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(alpha: f64, nu: f64, rho: f64, beta: f64) -> Result<JsSABRModelParams, JsValue> {
        if alpha <= 0.0 {
            return Err(JsValue::from_str("alpha must be positive"));
        }
        if nu < 0.0 {
            return Err(JsValue::from_str("nu must be non-negative"));
        }
        if !(-1.0..=1.0).contains(&rho) {
            return Err(JsValue::from_str("rho must be in [-1, 1]"));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(JsValue::from_str("beta must be in [0, 1]"));
        }
        Ok(Self::from_inner(SABRModelParams::new(alpha, nu, rho, beta)))
    }

    /// Create SABR parameters with equity market standard (beta=1.0).
    ///
    /// Equity options typically use lognormal dynamics with beta=1.0.
    ///
    /// @param {number} alpha - Initial volatility level
    /// @param {number} nu - Volatility of volatility
    /// @param {number} rho - Correlation parameter
    /// @returns {SABRModelParams} Parameters with beta=1.0
    ///
    /// @example
    /// ```typescript
    /// const params = SABRModelParams.equityStandard(0.2, 0.4, -0.3);
    /// ```
    #[wasm_bindgen(js_name = equityStandard)]
    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> JsSABRModelParams {
        Self::from_inner(SABRModelParams::equity_standard(alpha, nu, rho))
    }

    /// Create SABR parameters with interest rate market standard (beta=0.5).
    ///
    /// Interest rate options typically use beta=0.5 to capture the behavior
    /// of interest rate volatilities.
    ///
    /// @param {number} alpha - Initial volatility level
    /// @param {number} nu - Volatility of volatility
    /// @param {number} rho - Correlation parameter
    /// @returns {SABRModelParams} Parameters with beta=0.5
    ///
    /// @example
    /// ```typescript
    /// const params = SABRModelParams.ratesStandard(0.01, 0.2, 0.1);
    /// ```
    #[wasm_bindgen(js_name = ratesStandard)]
    pub fn rates_standard(alpha: f64, nu: f64, rho: f64) -> JsSABRModelParams {
        Self::from_inner(SABRModelParams::rates_standard(alpha, nu, rho))
    }

    /// Create SABR parameters with custom beta (e.g., 0.7 for FX markets).
    ///
    /// @param {number} alpha - Initial volatility level
    /// @param {number} nu - Volatility of volatility
    /// @param {number} rho - Correlation parameter
    /// @param {number} beta - Custom beta value
    /// @returns {SABRModelParams} Parameters with specified beta
    ///
    /// @example
    /// ```typescript
    /// // FX market example with beta=0.7
    /// const params = SABRModelParams.withBeta(0.1, 0.3, -0.2, 0.7);
    /// ```
    #[wasm_bindgen(js_name = withBeta)]
    pub fn with_beta(
        alpha: f64,
        nu: f64,
        rho: f64,
        beta: f64,
    ) -> Result<JsSABRModelParams, JsValue> {
        if !(0.0..=1.0).contains(&beta) {
            return Err(JsValue::from_str("beta must be in [0, 1]"));
        }
        Ok(Self::from_inner(SABRModelParams::new(alpha, nu, rho, beta)))
    }

    // Getters

    /// Initial volatility level.
    #[wasm_bindgen(getter)]
    pub fn alpha(&self) -> f64 {
        self.inner.alpha
    }

    /// Volatility of volatility (vol-of-vol).
    #[wasm_bindgen(getter)]
    pub fn nu(&self) -> f64 {
        self.inner.nu
    }

    /// Correlation between forward price and volatility.
    #[wasm_bindgen(getter)]
    pub fn rho(&self) -> f64 {
        self.inner.rho
    }

    /// CEV exponent parameter.
    #[wasm_bindgen(getter)]
    pub fn beta(&self) -> f64 {
        self.inner.beta
    }

    /// Convert to JSON object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        #[derive(serde::Serialize)]
        struct SABRJson {
            alpha: f64,
            nu: f64,
            rho: f64,
            beta: f64,
        }

        let data = SABRJson {
            alpha: self.inner.alpha,
            nu: self.inner.nu,
            rho: self.inner.rho,
            beta: self.inner.beta,
        };

        serde_wasm_bindgen::to_value(&data)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "SABRModelParams(alpha={:.4}, nu={:.4}, rho={:.3}, beta={:.1})",
            self.inner.alpha, self.inner.nu, self.inner.rho, self.inner.beta
        )
    }
}

/// SABR market data for calibration.
///
/// Contains market observables (forward, strikes, implied vols, expiry)
/// used for calibrating SABR parameters.
///
/// @example
/// ```typescript
/// const marketData = new SABRMarketData(
///   100.0,                  // forward
///   1.0,                     // time to expiry in years
///   [90, 95, 100, 105, 110], // strikes
///   [0.22, 0.20, 0.19, 0.20, 0.22], // market vols
///   0.5                      // beta
/// );
/// ```
#[wasm_bindgen(js_name = SABRMarketData)]
#[derive(Clone, Debug)]
pub struct JsSABRMarketData {
    inner: SABRMarketData,
}

impl JsSABRMarketData {
    pub(crate) fn from_inner(inner: SABRMarketData) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> SABRMarketData {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = SABRMarketData)]
impl JsSABRMarketData {
    /// Create SABR market data.
    ///
    /// @param {number} forward - Forward price
    /// @param {number} timeToExpiry - Time to expiry in years
    /// @param {Array<number>} strikes - Strike prices
    /// @param {Array<number>} marketVols - Market implied volatilities (same length as strikes)
    /// @param {number} beta - Beta parameter (typically 0.5 for rates, 1.0 for equity)
    /// @returns {SABRMarketData} Market data for calibration
    /// @throws {Error} If arrays have mismatched lengths or invalid values
    ///
    /// @example
    /// ```typescript
    /// const data = new SABRMarketData(
    ///   100, 1.0, [90, 95, 100, 105, 110],
    ///   [0.22, 0.20, 0.19, 0.20, 0.22], 0.5
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        forward: f64,
        time_to_expiry: f64,
        strikes: Vec<f64>,
        market_vols: Vec<f64>,
        beta: f64,
    ) -> Result<JsSABRMarketData, JsValue> {
        if strikes.len() != market_vols.len() {
            return Err(JsValue::from_str(
                "Strikes and market vols must have the same length",
            ));
        }
        if strikes.is_empty() {
            return Err(JsValue::from_str(
                "Must provide at least one strike/vol pair",
            ));
        }
        if forward <= 0.0 {
            return Err(JsValue::from_str("Forward must be positive"));
        }
        if time_to_expiry <= 0.0 {
            return Err(JsValue::from_str("Time to expiry must be positive"));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(JsValue::from_str("Beta must be in [0, 1]"));
        }

        Ok(Self::from_inner(SABRMarketData {
            forward,
            time_to_expiry,
            strikes,
            market_vols,
            beta,
            shift: None, // Default to None, could be exposed in future if needed
        }))
    }

    /// Forward price.
    #[wasm_bindgen(getter)]
    pub fn forward(&self) -> f64 {
        self.inner.forward
    }

    /// Strike prices.
    #[wasm_bindgen(getter)]
    pub fn strikes(&self) -> Vec<f64> {
        self.inner.strikes.clone()
    }

    /// Market implied volatilities.
    #[wasm_bindgen(getter, js_name = marketVols)]
    pub fn market_vols(&self) -> Vec<f64> {
        self.inner.market_vols.clone()
    }

    /// Time to expiry in years.
    #[wasm_bindgen(getter, js_name = timeToExpiry)]
    pub fn time_to_expiry(&self) -> f64 {
        self.inner.time_to_expiry
    }

    /// Beta parameter.
    #[wasm_bindgen(getter)]
    pub fn beta(&self) -> f64 {
        self.inner.beta
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "SABRMarketData(forward={:.2}, strikes={}, time_to_expiry={:.2}y, beta={:.1})",
            self.inner.forward,
            self.inner.strikes.len(),
            self.inner.time_to_expiry,
            self.inner.beta
        )
    }
}

/// SABR calibration derivatives for optimization.
///
/// Provides analytical derivatives of SABR implied volatility formula
/// for efficient numerical calibration.
///
/// @example
/// ```typescript
/// const marketData = new SABRMarketData(...);
/// const derivatives = new SABRCalibrationDerivatives(marketData);
/// ```
#[wasm_bindgen(js_name = SABRCalibrationDerivatives)]
pub struct JsSABRCalibrationDerivatives {
    #[allow(dead_code)]
    inner: SABRCalibrationDerivatives,
}

impl JsSABRCalibrationDerivatives {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: SABRCalibrationDerivatives) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = SABRCalibrationDerivatives)]
impl JsSABRCalibrationDerivatives {
    /// Create SABR calibration derivatives helper.
    ///
    /// @param {SABRMarketData} marketData - Market data for calibration
    /// @returns {SABRCalibrationDerivatives} Derivatives helper
    #[wasm_bindgen(constructor)]
    pub fn new(market_data: &JsSABRMarketData) -> JsSABRCalibrationDerivatives {
        Self {
            inner: SABRCalibrationDerivatives::new(market_data.inner()),
        }
    }
}
