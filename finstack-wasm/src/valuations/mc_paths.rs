//! WASM bindings for Monte Carlo path data structures.

use finstack_valuations::instruments::common::mc::path_data::{
    PathDataset, PathPoint, SimulatedPath,
};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// A single point along a Monte Carlo path.
#[wasm_bindgen(js_name = PathPoint)]
#[derive(Clone)]
pub struct JsPathPoint {
    inner: PathPoint,
}

#[wasm_bindgen(js_class = PathPoint)]
impl JsPathPoint {
    #[wasm_bindgen(getter)]
    pub fn step(&self) -> usize {
        self.inner.step
    }

    #[wasm_bindgen(getter)]
    pub fn time(&self) -> f64 {
        self.inner.time
    }

    #[wasm_bindgen(getter, js_name = stateVars)]
    pub fn state_vars(&self) -> JsValue {
        let obj = Object::new();
        for (key, value) in &self.inner.state_vars {
            let _ = Reflect::set(&obj, &JsValue::from_str(key), &JsValue::from_f64(*value));
        }
        obj.into()
    }

    #[wasm_bindgen(getter, js_name = payoffValue)]
    pub fn payoff_value(&self) -> Option<f64> {
        self.inner.payoff_value
    }

    #[wasm_bindgen(js_name = getVar)]
    pub fn get_var(&self, key: &str) -> Option<f64> {
        self.inner.get_var(key)
    }

    /// Get the spot price (convenience method).
    pub fn spot(&self) -> Option<f64> {
        self.inner.spot()
    }

    /// Get the variance (convenience method).
    pub fn variance(&self) -> Option<f64> {
        self.inner.variance()
    }

    /// Get the short rate (convenience method).
    #[wasm_bindgen(js_name = shortRate)]
    pub fn short_rate(&self) -> Option<f64> {
        self.inner.short_rate()
    }
}

impl JsPathPoint {
    pub(crate) fn from_inner(inner: PathPoint) -> Self {
        Self { inner }
    }
}

/// A complete simulated Monte Carlo path.
#[wasm_bindgen(js_name = SimulatedPath)]
#[derive(Clone)]
pub struct JsSimulatedPath {
    inner: SimulatedPath,
}

#[wasm_bindgen(js_class = SimulatedPath)]
impl JsSimulatedPath {
    #[wasm_bindgen(getter, js_name = pathId)]
    pub fn path_id(&self) -> usize {
        self.inner.path_id
    }

    #[wasm_bindgen(getter)]
    pub fn points(&self) -> Array {
        let arr = Array::new();
        for point in &self.inner.points {
            arr.push(&JsPathPoint::from_inner(point.clone()).into());
        }
        arr
    }

    #[wasm_bindgen(getter, js_name = finalValue)]
    pub fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    #[wasm_bindgen(js_name = numSteps)]
    pub fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    pub fn point(&self, step: usize) -> Option<JsPathPoint> {
        self.inner.point(step).map(|p| JsPathPoint::from_inner(p.clone()))
    }

    #[wasm_bindgen(js_name = initialPoint)]
    pub fn initial_point(&self) -> Option<JsPathPoint> {
        self.inner.initial_point().map(|p| JsPathPoint::from_inner(p.clone()))
    }

    #[wasm_bindgen(js_name = terminalPoint)]
    pub fn terminal_point(&self) -> Option<JsPathPoint> {
        self.inner.terminal_point().map(|p| JsPathPoint::from_inner(p.clone()))
    }
}

impl JsSimulatedPath {
    pub(crate) fn from_inner(inner: SimulatedPath) -> Self {
        Self { inner }
    }
}

/// Collection of simulated paths with metadata.
#[wasm_bindgen(js_name = PathDataset)]
#[derive(Clone)]
pub struct JsPathDataset {
    pub(crate) inner: PathDataset,
}

#[wasm_bindgen(js_class = PathDataset)]
impl JsPathDataset {
    #[wasm_bindgen(getter, js_name = numPaths)]
    pub fn num_paths(&self) -> usize {
        self.inner.paths.len()
    }

    #[wasm_bindgen(getter)]
    pub fn paths(&self) -> Array {
        let arr = Array::new();
        for path in &self.inner.paths {
            arr.push(&JsSimulatedPath::from_inner(path.clone()).into());
        }
        arr
    }

    #[wasm_bindgen(js_name = getPath)]
    pub fn get_path(&self, index: usize) -> Option<JsSimulatedPath> {
        self.inner.paths.get(index).map(|p| JsSimulatedPath::from_inner(p.clone()))
    }

    #[wasm_bindgen(js_name = processParams)]
    pub fn process_params(&self) -> JsValue {
        // Return process params as JSON-like object
        let obj = Object::new();
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("processType"),
            &JsValue::from_str(&self.inner.process_params.process_type),
        );
        let params_obj = Object::new();
        for (key, value) in &self.inner.process_params.parameters {
            let _ = Reflect::set(
                &params_obj,
                &JsValue::from_str(key),
                &JsValue::from_f64(*value),
            );
        }
        let _ = Reflect::set(&obj, &JsValue::from_str("parameters"), &params_obj);
        obj.into()
    }
}

impl JsPathDataset {
    pub(crate) fn from_inner(inner: PathDataset) -> Self {
        Self { inner }
    }
}

