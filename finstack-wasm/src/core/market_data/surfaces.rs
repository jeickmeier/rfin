use crate::core::error::js_error;
use crate::core::utils::js_array_from_iter;
use finstack_core::market_data::surfaces::VolSurface;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = VolSurface)]
#[derive(Clone)]
pub struct JsVolSurface {
    inner: Arc<VolSurface>,
}

impl JsVolSurface {
    pub(crate) fn inner(&self) -> Arc<VolSurface> {
        Arc::clone(&self.inner)
    }

    pub(crate) fn from_arc(inner: Arc<VolSurface>) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = VolSurface)]
impl JsVolSurface {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        expiries: Vec<f64>,
        strikes: Vec<f64>,
        vols: Vec<f64>,
    ) -> Result<JsVolSurface, JsValue> {
        let surface = VolSurface::from_grid(id, &expiries, &strikes, &vols)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsVolSurface {
            inner: Arc::new(surface),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn expiries(&self) -> js_sys::Array {
        js_array_from_iter(self.inner.expiries().iter().copied().map(JsValue::from_f64))
    }

    #[wasm_bindgen(getter)]
    pub fn strikes(&self) -> js_sys::Array {
        js_array_from_iter(self.inner.strikes().iter().copied().map(JsValue::from_f64))
    }

    #[wasm_bindgen(js_name = gridShape)]
    pub fn grid_shape(&self) -> js_sys::Array {
        let shape = self.inner.grid_shape();
        js_array_from_iter(
            [shape.0 as f64, shape.1 as f64]
                .into_iter()
                .map(JsValue::from_f64),
        )
    }

    /// Interpolated volatility at `(expiry, strike)` with flat extrapolation.
    ///
    /// This method clamps coordinates to grid bounds, providing safe evaluation
    /// that never fails. Use `valueChecked` for explicit error handling or
    /// `valueUnchecked` when bounds are guaranteed.
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_clamped(expiry, strike)
    }

    /// Volatility at `(expiry, strike)`, returning an error if out of bounds.
    ///
    /// Use this method when you need explicit error handling for out-of-bounds
    /// coordinates rather than flat extrapolation.
    #[wasm_bindgen(js_name = valueChecked)]
    pub fn value_checked(&self, expiry: f64, strike: f64) -> Result<f64, JsValue> {
        self.inner
            .value_checked(expiry, strike)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Volatility at `(expiry, strike)`, clamping to the surface domain.
    ///
    /// Alias for `value()` - both use flat extrapolation (clamping to edge values).
    #[wasm_bindgen(js_name = valueClamped)]
    pub fn value_clamped(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_clamped(expiry, strike)
    }

    /// Volatility at `(expiry, strike)` without bounds checking.
    ///
    /// # Panics
    ///
    /// Panics if `expiry` or `strike` is outside the grid bounds.
    /// Use only when bounds are guaranteed by the caller.
    #[wasm_bindgen(js_name = valueUnchecked)]
    pub fn value_unchecked(&self, expiry: f64, strike: f64) -> f64 {
        self.inner.value_unchecked(expiry, strike)
    }
}
