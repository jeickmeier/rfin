use crate::core::error::js_error;
use finstack_core::math::solver::{BrentSolver, NewtonSolver, Solver};
use js_sys::Function;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// Helper to safely call JS functions and convert errors
fn call_js_fn_safe(f: &Function, x: f64) -> Result<f64, JsValue> {
    let result = f.call1(&JsValue::NULL, &JsValue::from_f64(x))?;
    result
        .as_f64()
        .ok_or_else(|| js_error("Callback must return a number"))
}

// Copy-able closure wrapper for JS callbacks
#[derive(Clone, Copy)]
struct JsClosureAdapter<'a> {
    func: &'a Function,
    error_cell: &'a RefCell<Option<JsValue>>,
}

impl JsClosureAdapter<'_> {
    fn invoke(&self, x: f64) -> f64 {
        match call_js_fn_safe(self.func, x) {
            Ok(value) => value,
            Err(err) => {
                *self.error_cell.borrow_mut() = Some(err);
                f64::NAN
            }
        }
    }
}

// Execute a function and return any callback error captured by the adapter.
fn run_with_error_check<R>(
    error_cell: &RefCell<Option<JsValue>>,
    eval: impl FnOnce() -> R,
) -> Result<R, JsValue> {
    let value = eval();
    if let Some(err) = error_cell.borrow_mut().take() {
        Err(err)
    } else {
        Ok(value)
    }
}

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
    /// Create a Newton-Raphson solver with optional configuration.
    ///
    /// Newton-Raphson method uses function value and derivative to find roots.
    /// Fast convergence but requires good initial guess and differentiable function.
    ///
    /// @param {number} [tolerance] - Convergence tolerance (default: 1e-12)
    /// @param {number} [maxIterations] - Maximum iterations (default: 100)
    /// @param {number} [fdStep] - Finite difference step size (default: 1e-8)
    /// @returns {NewtonSolver} Configured Newton-Raphson solver
    ///
    /// @example
    /// ```javascript
    /// // Default configuration
    /// const solver1 = new NewtonSolver();
    ///
    /// // Custom tolerance and iterations
    /// const solver2 = new NewtonSolver(1e-10, 50, 1e-6);
    ///
    /// // Find root of f(x) = x² - 2 (√2 ≈ 1.414)
    /// const root = solver2.solve(x => x*x - 2, 1.0);
    /// console.log(root);  // ~1.414213562373095
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        fd_step: Option<f64>,
    ) -> JsNewtonSolver {
        Self::new_with_options(tolerance, max_iterations, fd_step)
    }

    /// Convergence tolerance for root finding.
    ///
    /// @type {number}
    /// @default 1e-12
    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Set convergence tolerance for root finding.
    ///
    /// @param {number} value - New tolerance value
    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    /// Maximum number of iterations before giving up.
    ///
    /// @type {number}
    /// @default 100
    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    /// Set maximum number of iterations.
    ///
    /// @param {number} value - New maximum iterations
    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    /// Finite difference step size for derivative approximation.
    ///
    /// @type {number}
    /// @default 1e-8
    #[wasm_bindgen(getter, js_name = fdStep)]
    pub fn fd_step(&self) -> f64 {
        self.inner.fd_step
    }

    /// Set finite difference step size.
    ///
    /// @param {number} value - New step size
    #[wasm_bindgen(setter, js_name = fdStep)]
    pub fn set_fd_step(&mut self, value: f64) {
        self.inner.fd_step = value;
    }

    /// Find a root of the given function using Newton-Raphson method.
    ///
    /// @param {Function} func - Function to find root of (takes number, returns number)
    /// @param {number} initial_guess - Starting point for iteration
    /// @returns {number} Approximate root
    /// @throws {Error} If convergence fails or function evaluation fails
    ///
    /// @example
    /// ```javascript
    /// const solver = new NewtonSolver(1e-10, 50);
    ///
    /// // Find root of polynomial: x³ - x - 1
    /// const root = solver.solve(x => x*x*x - x - 1, 1.0);
    /// console.log(root);  // ~1.324717957244746 (real root)
    ///
    /// // Find intersection: cos(x) = x
    /// const intersection = solver.solve(x => Math.cos(x) - x, 0.5);
    /// console.log(intersection);  // ~0.7390851332151607
    /// ```
    #[wasm_bindgen(js_name = solve)]
    pub fn solve(&self, func: &JsValue, initial_guess: f64) -> Result<f64, JsValue> {
        let func = func
            .dyn_ref::<Function>()
            .ok_or_else(|| js_error("Expected a JavaScript function"))?;
        let error_cell = RefCell::new(None);
        let adapter = JsClosureAdapter {
            func,
            error_cell: &error_cell,
        };
        let result = run_with_error_check(&error_cell, || {
            Solver::solve(&self.inner, |x| adapter.invoke(x), initial_guess)
        })?;
        result.map_err(|err| js_error(err.to_string()))
    }

    /// String representation of the solver configuration.
    ///
    /// @returns {string} Human-readable description
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
    /// Create a Brent's method solver with optional configuration.
    ///
    /// Brent's method combines bisection, secant, and inverse quadratic interpolation.
    /// Guaranteed to converge and robust for most functions.
    ///
    /// @param {number} [tolerance] - Convergence tolerance (default: 1e-12)
    /// @param {number} [maxIterations] - Maximum iterations (default: 100)
    /// @param {number} [bracketExpansion] - Bracket expansion factor (default: 1.6)
    /// @param {number} [initialBracketSize] - Initial bracket size (default: auto)
    /// @returns {BrentSolver} Configured Brent's method solver
    ///
    /// @example
    /// ```javascript
    /// // Default configuration
    /// const solver1 = new BrentSolver();
    ///
    /// // Custom tolerance and bracket expansion
    /// const solver2 = new BrentSolver(1e-10, 50, 2.0, 0.1);
    ///
    /// // Find root of f(x) = x² - 2 (√2 ≈ 1.414)
    /// const root = solver2.solve(x => x*x - 2, 1.0);
    /// console.log(root);  // ~1.414213562373095
    /// ```
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

    /// Convergence tolerance for root finding.
    ///
    /// @type {number}
    /// @default 1e-12
    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Set convergence tolerance for root finding.
    ///
    /// @param {number} value - New tolerance value
    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    /// Maximum number of iterations before giving up.
    ///
    /// @type {number}
    /// @default 100
    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    /// Set maximum number of iterations.
    ///
    /// @param {number} value - New maximum iterations
    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    /// Bracket expansion factor for root bracketing.
    ///
    /// @type {number}
    /// @default 1.6
    #[wasm_bindgen(getter, js_name = bracketExpansion)]
    pub fn bracket_expansion(&self) -> f64 {
        self.inner.bracket_expansion
    }

    /// Set bracket expansion factor.
    ///
    /// @param {number} value - New expansion factor
    #[wasm_bindgen(setter, js_name = bracketExpansion)]
    pub fn set_bracket_expansion(&mut self, value: f64) {
        self.inner.bracket_expansion = value;
    }

    /// Initial bracket size for root bracketing.
    ///
    /// @type {number|null}
    /// @default null (auto-detect)
    #[wasm_bindgen(getter, js_name = initialBracketSize)]
    pub fn initial_bracket_size(&self) -> Option<f64> {
        self.inner.initial_bracket_size
    }

    /// Set initial bracket size.
    ///
    /// @param {number|null} value - New initial bracket size
    #[wasm_bindgen(setter, js_name = initialBracketSize)]
    pub fn set_initial_bracket_size(&mut self, value: Option<f64>) {
        self.inner.initial_bracket_size = value;
    }

    /// Find a root of the given function using Brent's method.
    ///
    /// @param {Function} func - Function to find root of (takes number, returns number)
    /// @param {number} initial_guess - Starting point for iteration
    /// @returns {number} Approximate root
    /// @throws {Error} If convergence fails or function evaluation fails
    ///
    /// @example
    /// ```javascript
    /// const solver = new BrentSolver(1e-10, 50);
    ///
    /// // Find root of polynomial: x³ - x - 1
    /// const root = solver.solve(x => x*x*x - x - 1, 1.0);
    /// console.log(root);  // ~1.324717957244746 (real root)
    ///
    /// // Find intersection: cos(x) = x
    /// const intersection = solver.solve(x => Math.cos(x) - x, 0.5);
    /// console.log(intersection);  // ~0.7390851332151607
    /// ```
    #[wasm_bindgen(js_name = solve)]
    pub fn solve(&self, func: &JsValue, initial_guess: f64) -> Result<f64, JsValue> {
        let func = func
            .dyn_ref::<Function>()
            .ok_or_else(|| js_error("Expected a JavaScript function"))?;
        let error_cell = RefCell::new(None);
        let adapter = JsClosureAdapter {
            func,
            error_cell: &error_cell,
        };
        let result = run_with_error_check(&error_cell, || {
            Solver::solve(&self.inner, |x| adapter.invoke(x), initial_guess)
        })?;
        result.map_err(|err| js_error(err.to_string()))
    }

    /// String representation of the solver configuration.
    ///
    /// @returns {string} Human-readable description
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

/// Levenberg-Marquardt solver for non-linear least squares optimization.
///
/// Combines Gauss-Newton and gradient descent methods using an adaptive damping
/// parameter. Particularly effective for calibrating financial models with
/// multiple parameters (SABR, Heston, multi-curve bootstrapping).
///
/// @example
/// ```javascript
/// const solver = new LevenbergMarquardtSolver(1e-8, 100);
///
/// // Minimize (x-2)^2 + (y-3)^2
/// const result = solver.minimize(
///   params => Math.pow(params[0] - 2, 2) + Math.pow(params[1] - 3, 2),
///   new Float64Array([0, 0])
/// );
/// console.log(result);  // ~[2, 3]
/// ```
#[wasm_bindgen(js_name = LevenbergMarquardtSolver)]
pub struct JsLevenbergMarquardtSolver {
    inner: finstack_core::math::solver_multi::LevenbergMarquardtSolver,
}

#[wasm_bindgen(js_class = LevenbergMarquardtSolver)]
impl JsLevenbergMarquardtSolver {
    /// Create a Levenberg-Marquardt solver with optional configuration.
    ///
    /// @param {number} [tolerance] - Convergence tolerance (default: 1e-8)
    /// @param {number} [maxIterations] - Maximum iterations (default: 100)
    /// @param {number} [lambdaInit] - Initial damping parameter (default: 1e-3)
    /// @param {number} [fdStep] - Finite difference step size (default: 1e-8)
    #[wasm_bindgen(constructor)]
    pub fn new(
        tolerance: Option<f64>,
        max_iterations: Option<usize>,
        lambda_init: Option<f64>,
        fd_step: Option<f64>,
    ) -> JsLevenbergMarquardtSolver {
        let mut inner = finstack_core::math::solver_multi::LevenbergMarquardtSolver::new();
        if let Some(tol) = tolerance {
            inner.tolerance = tol;
        }
        if let Some(iter) = max_iterations {
            inner.max_iterations = iter;
        }
        if let Some(lambda) = lambda_init {
            inner.lambda_init = lambda;
        }
        if let Some(step) = fd_step {
            inner.fd_step = step;
        }
        Self { inner }
    }

    /// Convergence tolerance.
    #[wasm_bindgen(getter)]
    pub fn tolerance(&self) -> f64 {
        self.inner.tolerance
    }

    /// Set convergence tolerance.
    #[wasm_bindgen(setter)]
    pub fn set_tolerance(&mut self, value: f64) {
        self.inner.tolerance = value;
    }

    /// Maximum number of iterations.
    #[wasm_bindgen(getter, js_name = maxIterations)]
    pub fn max_iterations(&self) -> usize {
        self.inner.max_iterations
    }

    /// Set maximum iterations.
    #[wasm_bindgen(setter, js_name = maxIterations)]
    pub fn set_max_iterations(&mut self, value: usize) {
        self.inner.max_iterations = value;
    }

    /// Initial damping parameter (lambda).
    #[wasm_bindgen(getter, js_name = lambdaInit)]
    pub fn lambda_init(&self) -> f64 {
        self.inner.lambda_init
    }

    /// Set initial damping parameter.
    #[wasm_bindgen(setter, js_name = lambdaInit)]
    pub fn set_lambda_init(&mut self, value: f64) {
        self.inner.lambda_init = value;
    }

    /// Finite difference step size.
    #[wasm_bindgen(getter, js_name = fdStep)]
    pub fn fd_step(&self) -> f64 {
        self.inner.fd_step
    }

    /// Set finite difference step size.
    #[wasm_bindgen(setter, js_name = fdStep)]
    pub fn set_fd_step(&mut self, value: f64) {
        self.inner.fd_step = value;
    }

    /// Minimize an objective function starting from an initial guess.
    ///
    /// @param {Function} objective - Function mapping Float64Array → number
    /// @param {Float64Array} initial - Initial parameter guess
    /// @param {Float64Array} [boundsLow] - Optional lower bounds for each parameter
    /// @param {Float64Array} [boundsHigh] - Optional upper bounds for each parameter
    /// @returns {Float64Array} Optimal parameter vector
    #[wasm_bindgen(js_name = minimize)]
    pub fn minimize(
        &self,
        objective: &JsValue,
        initial: Vec<f64>,
        bounds_low: Option<Vec<f64>>,
        bounds_high: Option<Vec<f64>>,
    ) -> Result<Vec<f64>, JsValue> {
        use finstack_core::math::solver_multi::MultiSolver;

        let func = objective
            .dyn_ref::<Function>()
            .ok_or_else(|| js_error("Expected a JavaScript function"))?;

        let error_cell = RefCell::new(None);

        let bounds_vec: Option<Vec<(f64, f64)>> = match (bounds_low, bounds_high) {
            (Some(lo), Some(hi)) => {
                if lo.len() != hi.len() || lo.len() != initial.len() {
                    return Err(js_error("Bounds arrays must match initial parameter length"));
                }
                Some(lo.into_iter().zip(hi).collect())
            }
            (None, None) => None,
            _ => return Err(js_error("Both boundsLow and boundsHigh must be provided, or neither")),
        };

        let obj_wrapper = |params: &[f64]| -> f64 {
            let arr = js_sys::Float64Array::from(params);
            match func.call1(&JsValue::NULL, &arr) {
                Ok(val) => val.as_f64().unwrap_or(f64::NAN),
                Err(err) => {
                    *error_cell.borrow_mut() = Some(err);
                    f64::NAN
                }
            }
        };

        let result = run_with_error_check(&error_cell, || {
            self.inner
                .minimize(obj_wrapper, &initial, bounds_vec.as_deref())
        })?;

        result.map_err(|e| js_error(e.to_string()))
    }

    /// String representation of the solver configuration.
    #[allow(clippy::inherent_to_string)]
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        format!(
            "LevenbergMarquardtSolver(tolerance={}, maxIterations={}, lambdaInit={}, fdStep={})",
            self.inner.tolerance, self.inner.max_iterations, self.inner.lambda_init, self.inner.fd_step
        )
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn newton_solver_propagates_javascript_errors() {
        let solver = JsNewtonSolver::new(None, None, None);
        let callback = Function::new_with_args("x", "throw new Error('boom')");

        let err = solver
            .solve(&callback.into(), 1.0)
            .expect_err("callback error should surface as Result::Err");

        let message = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
            .ok()
            .and_then(|value| value.as_string());
        assert_eq!(message.as_deref(), Some("boom"));
    }
}
