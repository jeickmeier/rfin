//! WASM bindings for [`finstack_core::types`] rate helpers (`Rate`, `Bps`, `Percentage`).

use crate::utils::to_js_err;
use finstack_core::types::{Bps as RustBps, Percentage as RustPercentage, Rate as RustRate};
use wasm_bindgen::prelude::*;

/// Interest or discount rate stored as a decimal (e.g. `0.05` is 5%).
#[wasm_bindgen(js_name = Rate)]
pub struct Rate {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustRate,
}

#[wasm_bindgen(js_class = Rate)]
impl Rate {
    /// Creates a rate from a decimal value (0.05 = 5%). Errors if `decimal` is not finite.
    #[wasm_bindgen(constructor)]
    pub fn new(decimal: f64) -> Result<Rate, JsValue> {
        RustRate::try_from_decimal(decimal)
            .map(|inner| Rate { inner })
            .map_err(to_js_err)
    }

    /// Creates a rate from a percent figure (5.0 = 5%).
    #[wasm_bindgen(js_name = fromPercent)]
    pub fn from_percent(pct: f64) -> Result<Rate, JsValue> {
        if !pct.is_finite() {
            return Err(to_js_err("percent must be finite"));
        }
        RustRate::try_from_decimal(pct / 100.0)
            .map(|inner| Rate { inner })
            .map_err(to_js_err)
    }

    /// Creates a rate from basis points (500 bps = 5%). `bps` is rounded to the nearest integer.
    #[wasm_bindgen(js_name = fromBps)]
    pub fn from_bps(bps: f64) -> Result<Rate, JsValue> {
        let b = RustBps::try_new(bps).map_err(to_js_err)?;
        Ok(Rate { inner: b.as_rate() })
    }

    /// Rate as a decimal (0.05 for 5%).
    #[wasm_bindgen(getter, js_name = asDecimal)]
    pub fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Rate as a percent (5.0 for 5%).
    #[wasm_bindgen(getter, js_name = asPercent)]
    pub fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }

    /// Rate in basis points (rounded to the nearest integer).
    #[wasm_bindgen(getter, js_name = asBps)]
    pub fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }
}

/// Basis points (1 bp = 0.01%, 10_000 bps = 100%).
#[wasm_bindgen(js_name = Bps)]
pub struct Bps {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustBps,
}

#[wasm_bindgen(js_class = Bps)]
impl Bps {
    /// Creates basis points from a floating value; input is rounded to the nearest integer bp.
    #[wasm_bindgen(constructor)]
    pub fn new(value: f64) -> Result<Bps, JsValue> {
        RustBps::try_new(value)
            .map(|inner| Bps { inner })
            .map_err(to_js_err)
    }

    /// Value as a decimal (e.g. 25 bp → 0.0025).
    #[wasm_bindgen(js_name = asDecimal)]
    pub fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Value in whole basis points.
    #[wasm_bindgen(js_name = asBps)]
    pub fn as_bps(&self) -> i32 {
        self.inner.as_bps()
    }
}

/// Percentage stored in percent points (5.0 means 5%).
#[wasm_bindgen(js_name = Percentage)]
pub struct Percentage {
    #[wasm_bindgen(skip)]
    pub(crate) inner: RustPercentage,
}

#[wasm_bindgen(js_class = Percentage)]
impl Percentage {
    /// Creates a percentage; errors if the value is not finite.
    #[wasm_bindgen(constructor)]
    pub fn new(value: f64) -> Result<Percentage, JsValue> {
        RustPercentage::try_new(value)
            .map(|inner| Percentage { inner })
            .map_err(to_js_err)
    }

    /// Value as a decimal (5% → 0.05).
    #[wasm_bindgen(js_name = asDecimal)]
    pub fn as_decimal(&self) -> f64 {
        self.inner.as_decimal()
    }

    /// Value in percent points.
    #[wasm_bindgen(js_name = asPercent)]
    pub fn as_percent(&self) -> f64 {
        self.inner.as_percent()
    }
}
