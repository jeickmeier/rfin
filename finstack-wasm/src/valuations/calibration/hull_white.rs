//! Hull-White one-factor model calibration bindings for WASM.
//!
//! Calibrates mean reversion (κ) and short rate volatility (σ) by minimising
//! squared swaption price errors using the Jamshidian (1989) decomposition.

use crate::core::error::{core_to_js, js_error};
use finstack_valuations::calibration::hull_white::{
    calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess, HullWhiteParams,
    SwapFrequency, SwaptionQuote,
};
use wasm_bindgen::prelude::*;

// =============================================================================
// HullWhiteParams
// =============================================================================

/// Hull-White one-factor model parameters (κ, σ).
#[wasm_bindgen(js_name = HullWhiteParams)]
#[derive(Clone, Copy)]
pub struct JsHullWhiteParams {
    inner: HullWhiteParams,
}

#[wasm_bindgen(js_class = HullWhiteParams)]
impl JsHullWhiteParams {
    /// Create validated Hull-White parameters.
    ///
    /// Both `kappa` (mean reversion) and `sigma` (short rate vol) must be positive.
    #[wasm_bindgen(constructor)]
    pub fn new(kappa: f64, sigma: f64) -> Result<JsHullWhiteParams, JsValue> {
        HullWhiteParams::new(kappa, sigma)
            .map(|inner| JsHullWhiteParams { inner })
            .map_err(core_to_js)
    }

    /// Mean reversion speed (κ).
    #[wasm_bindgen(getter)]
    pub fn kappa(&self) -> f64 {
        self.inner.kappa
    }

    /// Short rate volatility (σ).
    #[wasm_bindgen(getter)]
    pub fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    /// Compute B(t₁, t₂) = (1 − e^{−κ(t₂−t₁)}) / κ.
    #[wasm_bindgen(js_name = bFunction)]
    pub fn b_function(&self, t1: f64, t2: f64) -> f64 {
        self.inner.b_function(t1, t2)
    }

    /// Zero-coupon bond option volatility under HW1F.
    #[wasm_bindgen(js_name = bondOptionVol)]
    pub fn bond_option_vol(&self, t: f64, big_t: f64, s: f64) -> f64 {
        self.inner.bond_option_vol(t, big_t, s)
    }
}

// =============================================================================
// SwaptionQuote
// =============================================================================

/// Market quote for a European swaption used in HW1F calibration.
#[wasm_bindgen(js_name = SwaptionQuote)]
#[derive(Clone, Copy)]
pub struct JsSwaptionQuote {
    inner: SwaptionQuote,
}

#[wasm_bindgen(js_class = SwaptionQuote)]
impl JsSwaptionQuote {
    /// Create a validated swaption market quote.
    ///
    /// * `expiry` - Swaption expiry in years.
    /// * `tenor` - Underlying swap tenor in years.
    /// * `volatility` - Market-quoted volatility.
    /// * `is_normal_vol` - `true` for Bachelier (normal) vol, `false` for Black-76 (lognormal).
    #[wasm_bindgen(constructor)]
    pub fn new(
        expiry: f64,
        tenor: f64,
        volatility: f64,
        is_normal_vol: bool,
    ) -> Result<JsSwaptionQuote, JsValue> {
        SwaptionQuote::try_new(expiry, tenor, volatility, is_normal_vol)
            .map(|inner| JsSwaptionQuote { inner })
            .map_err(core_to_js)
    }

    /// Swaption expiry in years.
    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> f64 {
        self.inner.expiry
    }

    /// Underlying swap tenor in years.
    #[wasm_bindgen(getter)]
    pub fn tenor(&self) -> f64 {
        self.inner.tenor
    }

    /// Market-quoted volatility.
    #[wasm_bindgen(getter)]
    pub fn volatility(&self) -> f64 {
        self.inner.volatility
    }

    /// Whether this is a normal (Bachelier) vol.
    #[wasm_bindgen(getter, js_name = isNormalVol)]
    pub fn is_normal_vol(&self) -> bool {
        self.inner.is_normal_vol
    }
}

// =============================================================================
// SwapFrequency
// =============================================================================

/// Coupon frequency for the underlying swap in HW1F calibration.
#[wasm_bindgen(js_name = SwapFrequency)]
#[derive(Clone, Copy)]
pub struct JsSwapFrequency {
    inner: SwapFrequency,
}

#[wasm_bindgen(js_class = SwapFrequency)]
impl JsSwapFrequency {
    /// Annual (1 payment/year, EUR/GBP standard).
    #[wasm_bindgen(getter, js_name = ANNUAL)]
    pub fn annual() -> JsSwapFrequency {
        JsSwapFrequency {
            inner: SwapFrequency::Annual,
        }
    }

    /// Semi-annual (2 payments/year, USD standard).
    #[wasm_bindgen(getter, js_name = SEMI_ANNUAL)]
    pub fn semi_annual() -> JsSwapFrequency {
        JsSwapFrequency {
            inner: SwapFrequency::SemiAnnual,
        }
    }

    /// Quarterly (4 payments/year).
    #[wasm_bindgen(getter, js_name = QUARTERLY)]
    pub fn quarterly() -> JsSwapFrequency {
        JsSwapFrequency {
            inner: SwapFrequency::Quarterly,
        }
    }
}

// =============================================================================
// Calibration function
// =============================================================================

/// Calibrate Hull-White 1-factor parameters to European swaption market data.
///
/// Fits κ (mean reversion) and σ (short rate volatility) by minimising
/// squared differences between model and market swaption prices.
///
/// * `discount_factors` - Array of `[time, df]` pairs defining the discount curve.
/// * `quotes` - Array of swaption quotes.
/// * `frequency` - Coupon frequency of the underlying swap.
///
/// Returns a JavaScript object with `params` (HullWhiteParams) and `report` fields.
#[wasm_bindgen(js_name = calibrateHullWhite)]
pub fn calibrate_hull_white(
    discount_factors: JsValue,
    quotes_js: JsValue,
    frequency: &JsSwapFrequency,
) -> Result<JsValue, JsValue> {
    let df_pairs: Vec<(f64, f64)> = serde_wasm_bindgen::from_value(discount_factors)
        .map_err(|e| js_error(format!("Invalid discount factors: {}", e)))?;

    let quotes: Vec<SwaptionQuote> =
        serde_wasm_bindgen::from_value::<Vec<SwaptionQuoteInput>>(quotes_js)
            .map_err(|e| js_error(format!("Invalid quotes: {}", e)))?
            .into_iter()
            .map(|q| {
                SwaptionQuote::try_new(q.expiry, q.tenor, q.volatility, q.is_normal_vol)
                    .map_err(core_to_js)
            })
            .collect::<Result<Vec<_>, _>>()?;

    let df_fn = build_df_interpolator(&df_pairs);

    let (params, report) = calibrate_hull_white_to_swaptions_with_frequency_and_initial_guess(
        &df_fn,
        &quotes,
        frequency.inner,
        None,
    )
    .map_err(core_to_js)?;

    let result = serde_json::json!({
        "params": { "kappa": params.kappa, "sigma": params.sigma },
        "report": {
            "success": report.success,
            "iterations": report.iterations,
            "objective_value": report.objective_value,
            "convergence_reason": report.convergence_reason,
        },
    });

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| js_error(format!("Result serialization failed: {}", e)))
}

#[derive(serde::Deserialize)]
struct SwaptionQuoteInput {
    expiry: f64,
    tenor: f64,
    volatility: f64,
    #[serde(default)]
    is_normal_vol: bool,
}

fn build_df_interpolator(pairs: &[(f64, f64)]) -> impl Fn(f64) -> f64 + '_ {
    move |t: f64| {
        if pairs.is_empty() {
            return 1.0;
        }
        if t <= pairs[0].0 {
            return pairs[0].1;
        }
        if t >= pairs[pairs.len() - 1].0 {
            return pairs[pairs.len() - 1].1;
        }
        for window in pairs.windows(2) {
            let (t0, df0) = window[0];
            let (t1, df1) = window[1];
            if t >= t0 && t <= t1 {
                let frac = (t - t0) / (t1 - t0);
                let ln0 = df0.ln();
                let ln1 = df1.ln();
                return (ln0 + frac * (ln1 - ln0)).exp();
            }
        }
        pairs[pairs.len() - 1].1
    }
}
