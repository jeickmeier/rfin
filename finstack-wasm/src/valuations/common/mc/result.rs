//! WASM bindings for Monte Carlo result wrapper.

use super::paths::JsPathDataset;
use crate::core::money::JsMoney;
use finstack_valuations::instruments::common::models::monte_carlo::results::MonteCarloResult;
use wasm_bindgen::prelude::*;

/// Monte Carlo result with optional path data.
#[wasm_bindgen(js_name = MonteCarloResult)]
#[derive(Clone)]
pub struct JsMonteCarloResult {
    inner: MonteCarloResult,
}

#[wasm_bindgen(js_class = MonteCarloResult)]
impl JsMonteCarloResult {
    #[wasm_bindgen(getter)]
    pub fn estimate(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.estimate.mean)
    }

    #[wasm_bindgen(getter)]
    pub fn stderr(&self) -> f64 {
        self.inner.estimate.stderr
    }

    #[wasm_bindgen(getter, js_name = ci95)]
    pub fn ci_95(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        arr.push(&JsMoney::from_inner(self.inner.estimate.ci_95.0).into());
        arr.push(&JsMoney::from_inner(self.inner.estimate.ci_95.1).into());
        arr
    }

    #[wasm_bindgen(getter, js_name = numPaths)]
    pub fn num_paths(&self) -> usize {
        self.inner.estimate.num_paths
    }

    #[wasm_bindgen(getter)]
    pub fn paths(&self) -> Option<JsPathDataset> {
        self.inner
            .paths
            .as_ref()
            .map(|p| JsPathDataset::from_inner(p.clone()))
    }

    #[wasm_bindgen(js_name = hasPaths)]
    pub fn has_paths(&self) -> bool {
        self.inner.has_paths()
    }

    #[wasm_bindgen(js_name = numCapturedPaths)]
    pub fn num_captured_paths(&self) -> usize {
        self.inner.num_captured_paths()
    }

    pub fn mean(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.estimate.mean)
    }

    #[wasm_bindgen(js_name = relativeStderr)]
    pub fn relative_stderr(&self) -> f64 {
        self.inner.estimate.relative_stderr()
    }
}

impl JsMonteCarloResult {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: MonteCarloResult) -> Self {
        Self { inner }
    }
}
