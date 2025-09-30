use crate::core::math::callable::JsCallable;
use crate::core::utils::js_error;
use finstack_core::math::solver::{BrentSolver, HybridSolver, NewtonSolver, Solver};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = NewtonSolver)]
pub struct JsNewtonSolver {
    inner: NewtonSolver,
}

impl JsNewtonSolver {
    fn new_with_options(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        fd_step: Option<f64>,
    ) -> Self {
        let mut inner = NewtonSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(iter) = max_iterations {
            inner.max_iterations = iter;
        }
        if let Some(step) = fd_step {
            inner.fd_step = step;
        }
        Self { inner }
    }
}

#[wasm_bindgen(js_class = NewtonSolver)]
impl JsNewtonSolver {
    #[wasm_bindgen(constructor)]
    pub fn new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        fd_step: Option<f64>,
    ) -> JsNewtonSolver {
        Self::new_with_options(tolerance, max_iterations, fd_step)
    }

    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    #[wasm_bindgen(getter, js_name = fdStep)]
    pub fn fd_step(&self) -> f64 {
        self.inner.fd_step
    }

    #[wasm_bindgen(setter, js_name = fdStep)]
    pub fn set_fd_step(&mut self, value: f64) {
        self.inner.fd_step = value;
    }

    #[wasm_bindgen(js_name = solve)]
    pub fn solve(&self, func: &JsValue, initial_guess: f64) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            |err| js_error(err.to_string()),
        )
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "NewtonSolver(tolerance={}, maxIterations={}, fdStep={})",
            self.inner.tolerance, self.inner.max_iterations, self.inner.fd_step
        )
    }
}

#[wasm_bindgen(js_name = BrentSolver)]
pub struct JsBrentSolver {
    inner: BrentSolver,
}

impl JsBrentSolver {
    fn new_with_options(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        bracket_expansion: Option<f64>,
        initial_bracket_size: Option<f64>,
    ) -> Self {
        let mut inner = BrentSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(iter) = max_iterations {
            inner.max_iterations = iter;
        }
        if let Some(expansion) = bracket_expansion {
            inner.bracket_expansion = expansion;
        }
        if initial_bracket_size.is_some() {
            inner.initial_bracket_size = initial_bracket_size;
        }
        Self { inner }
    }
}

#[wasm_bindgen(js_class = BrentSolver)]
impl JsBrentSolver {
    #[wasm_bindgen(constructor)]
    pub fn new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        bracket_expansion: Option<f64>,
        initial_bracket_size: Option<f64>,
    ) -> JsBrentSolver {
        Self::new_with_options(
            tolerance,
            max_iterations,
            bracket_expansion,
            initial_bracket_size,
        )
    }

    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    #[wasm_bindgen(getter, js_name = bracketExpansion)]
    pub fn bracket_expansion(&self) -> f64 {
        self.inner.bracket_expansion
    }

    #[wasm_bindgen(setter, js_name = bracketExpansion)]
    pub fn set_bracket_expansion(&mut self, value: f64) {
        self.inner.bracket_expansion = value;
    }

    #[wasm_bindgen(getter, js_name = initialBracketSize)]
    pub fn initial_bracket_size(&self) -> Option<f64> {
        self.inner.initial_bracket_size
    }

    #[wasm_bindgen(setter, js_name = initialBracketSize)]
    pub fn set_initial_bracket_size(&mut self, value: Option<f64>) {
        self.inner.initial_bracket_size = value;
    }

    #[wasm_bindgen(js_name = solve)]
    pub fn solve(&self, func: &JsValue, initial_guess: f64) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            |err| js_error(err.to_string()),
        )
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "BrentSolver(tolerance={}, maxIterations={}, bracketExpansion={}, initialBracketSize={:?})",
            self.inner.tolerance,
            self.inner.max_iterations,
            self.inner.bracket_expansion,
            self.inner.initial_bracket_size
        )
    }
}

#[wasm_bindgen(js_name = HybridSolver)]
pub struct JsHybridSolver {
    inner: HybridSolver,
    tolerance: f64,
    max_iterations: usize,
}

impl JsHybridSolver {
    fn new_with_options(tolerance: Option<f64>, max_iterations: Option<usize>) -> Self {
        let tol = tolerance.unwrap_or(1e-12);
        let max_iter = max_iterations.unwrap_or(100);
        let inner = HybridSolver::new()
            .with_tolerance(tol)
            .with_max_iterations(max_iter);
        Self {
            inner,
            tolerance: tol,
            max_iterations: max_iter,
        }
    }

    fn rebuild(&mut self) {
        self.inner = HybridSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_iterations(self.max_iterations);
    }
}

#[wasm_bindgen(js_class = HybridSolver)]
impl JsHybridSolver {
    #[wasm_bindgen(constructor)]
    pub fn new(tolerance: Option<f64>, max_iterations: Option<usize>) -> JsHybridSolver {
        Self::new_with_options(tolerance, max_iterations)
    }

    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }

    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.tolerance = value;
        self.rebuild();
    }

    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.max_iterations = value;
        self.rebuild();
    }

    #[wasm_bindgen(js_name = solve)]
    pub fn solve(&self, func: &JsValue, initial_guess: f64) -> Result<f64, JsValue> {
        let callable = JsCallable::new(func)?;
        let closure = callable.closure();
        callable.run_core(
            || Solver::solve(&self.inner, closure, initial_guess),
            |err| js_error(err.to_string()),
        )
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "HybridSolver(tolerance={}, maxIterations={})",
            self.tolerance, self.max_iterations
        )
    }
}
