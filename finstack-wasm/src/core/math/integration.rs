use crate::core::math::callable::JsCallable;
use crate::core::utils::js_error;
use finstack_core::math::integration as core_integration;
use finstack_core::math::integration::GaussHermiteQuadrature;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = GaussHermiteQuadrature)]
pub struct JsGaussHermiteQuadrature {
    inner: GaussHermiteQuadrature,
}

impl JsGaussHermiteQuadrature {
    fn from_inner(inner: GaussHermiteQuadrature) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = GaussHermiteQuadrature)]
impl JsGaussHermiteQuadrature {
    #[wasm_bindgen(constructor)]
    pub fn new(order: usize) -> Result<JsGaussHermiteQuadrature, JsValue> {
        match order {
            5 => Ok(Self::from_inner(GaussHermiteQuadrature::order_5())),
            7 => Ok(Self::from_inner(GaussHermiteQuadrature::order_7())),
            10 => Ok(Self::from_inner(GaussHermiteQuadrature::order_10())),
            _ => Err(js_error("Supported orders are 5, 7, or 10")),
        }
    }

    #[wasm_bindgen(js_name = order5)]
    pub fn order_5() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_5())
    }

    #[wasm_bindgen(js_name = order7)]
    pub fn order_7() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_7())
    }

    #[wasm_bindgen(js_name = order10)]
    pub fn order_10() -> JsGaussHermiteQuadrature {
        Self::from_inner(GaussHermiteQuadrature::order_10())
    }

    #[wasm_bindgen(getter)]
    pub fn order(&self) -> usize {
        self.inner.points.len()
    }

    #[wasm_bindgen(js_name = points)]
    pub fn points(&self) -> Vec<f64> {
        self.inner.points.to_vec()
    }

    #[wasm_bindgen(js_name = weights)]
    pub fn weights(&self) -> Vec<f64> {
        self.inner.weights.to_vec()
    }

    #[wasm_bindgen(js_name = integrate)]
    pub fn integrate(&self, func: &JsValue) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_value(|| self.inner.integrate(closure))
    }

    #[wasm_bindgen(js_name = integrateAdaptive)]
    pub fn integrate_adaptive(&self, func: &JsValue, tolerance: f64) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_value(|| self.inner.integrate_adaptive(closure, tolerance))
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("GaussHermiteQuadrature(order={})", self.order())
    }
}

#[wasm_bindgen(js_name = simpsonRule)]
pub fn simpson_rule(func: &JsValue, a: f64, b: f64, intervals: usize) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::simpson_rule(closure, a, b, intervals),
        |err| js_error(err.to_string()),
    )
}

#[wasm_bindgen(js_name = adaptiveSimpson)]
pub fn adaptive_simpson(
    func: &JsValue,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::adaptive_simpson(closure, a, b, tol, max_depth),
        |err| js_error(err.to_string()),
    )
}

#[wasm_bindgen(js_name = adaptiveQuadrature)]
pub fn adaptive_quadrature(
    func: &JsValue,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    adaptive_simpson(func, a, b, tol, max_depth)
}

#[wasm_bindgen(js_name = gaussLegendreIntegrate)]
pub fn gauss_legendre_integrate(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::gauss_legendre_integrate(closure, a, b, order),
        |err| js_error(err.to_string()),
    )
}

#[wasm_bindgen(js_name = gaussLegendreIntegrateComposite)]
pub fn gauss_legendre_integrate_composite(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
    panels: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::gauss_legendre_integrate_composite(closure, a, b, order, panels),
        |err| js_error(err.to_string()),
    )
}

#[wasm_bindgen(js_name = gaussLegendreIntegrateAdaptive)]
pub fn gauss_legendre_integrate_adaptive(
    func: &JsValue,
    a: f64,
    b: f64,
    order: usize,
    tol: f64,
    max_depth: usize,
) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || {
            core_integration::gauss_legendre_integrate_adaptive(
                closure, a, b, order, tol, max_depth,
            )
        },
        |err| js_error(err.to_string()),
    )
}

#[wasm_bindgen(js_name = trapezoidalRule)]
pub fn trapezoidal_rule(func: &JsValue, a: f64, b: f64, intervals: usize) -> Result<f64, JsValue> {
    let callable = JsCallable::new(func)?;
    let closure = callable.closure();
    callable.run_core(
        || core_integration::trapezoidal_rule(closure, a, b, intervals),
        |err| js_error(err.to_string()),
    )
}
