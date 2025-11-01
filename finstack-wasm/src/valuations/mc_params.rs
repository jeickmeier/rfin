//! WASM bindings for Monte Carlo process parameters.

use finstack_valuations::instruments::common::mc::path_data::ProcessParams;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

/// Process parameters and metadata for Monte Carlo simulation.
#[wasm_bindgen(js_name = ProcessParams)]
#[derive(Clone)]
pub struct JsProcessParams {
    inner: ProcessParams,
}

#[wasm_bindgen(js_class = ProcessParams)]
impl JsProcessParams {
    #[wasm_bindgen(getter, js_name = processType)]
    pub fn process_type(&self) -> String {
        self.inner.process_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn parameters(&self) -> JsValue {
        let obj = Object::new();
        for (key, value) in &self.inner.parameters {
            let _ = Reflect::set(&obj, &JsValue::from_str(key), &JsValue::from_f64(*value));
        }
        obj.into()
    }

    #[wasm_bindgen(getter)]
    pub fn correlation(&self) -> Option<Array> {
        self.inner.correlation.as_ref().map(|corr| {
            let arr = Array::new_with_length(corr.len() as u32);
            for (i, &val) in corr.iter().enumerate() {
                arr.set(i as u32, JsValue::from_f64(val));
            }
            arr
        })
    }

    #[wasm_bindgen(getter, js_name = factorNames)]
    pub fn factor_names(&self) -> Array {
        let arr = Array::new_with_length(self.inner.factor_names.len() as u32);
        for (i, name) in self.inner.factor_names.iter().enumerate() {
            arr.set(i as u32, JsValue::from_str(name));
        }
        arr
    }

    #[wasm_bindgen(js_name = getParam)]
    pub fn get_param(&self, key: &str) -> Option<f64> {
        self.inner.parameters.get(key).copied()
    }

    pub fn dim(&self) -> Option<usize> {
        self.inner.dim()
    }

    #[wasm_bindgen(js_name = correlationMatrix)]
    pub fn correlation_matrix(&self) -> Option<Array> {
        if let Some(ref corr_flat) = self.inner.correlation {
            let dim = (corr_flat.len() as f64).sqrt() as usize;
            let rows = Array::new();

            for i in 0..dim {
                let row = Array::new();
                for j in 0..dim {
                    row.push(&JsValue::from_f64(corr_flat[i * dim + j]));
                }
                rows.push(&row);
            }

            Some(rows)
        } else {
            None
        }
    }
}

impl JsProcessParams {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: ProcessParams) -> Self {
        Self { inner }
    }
}

