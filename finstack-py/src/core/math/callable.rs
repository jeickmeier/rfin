use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyList};
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Internal helper that adapts Python callables for Rust integrations.
///
/// This utility ensures we can pass Python callables into Rust functions that
/// expect `Fn(f64) -> f64 + Copy` while preserving Python exceptions.
pub(super) struct CallableAdapter {
    callable: Py<PyAny>,
    error: RefCell<Option<PyErr>>,
}

#[derive(Clone, Copy)]
struct CallableProxy<'a> {
    callable: &'a Py<PyAny>,
    error: &'a RefCell<Option<PyErr>>,
}

struct CallbackPanic;

impl CallableAdapter {
    pub(super) fn new(callable: Bound<'_, PyAny>) -> PyResult<Self> {
        if !callable.is_callable() {
            return Err(PyTypeError::new_err("Expected a callable object"));
        }
        let owned: Py<PyAny> = callable.unbind();
        Ok(Self {
            callable: owned,
            error: RefCell::new(None),
        })
    }

    pub(super) fn closure(&self) -> impl Fn(f64) -> f64 + Copy + '_ {
        let proxy = CallableProxy {
            callable: &self.callable,
            error: &self.error,
        };
        move |x| proxy.invoke(x)
    }

    pub(super) fn run_value<R>(&self, eval: impl FnOnce() -> R) -> PyResult<R> {
        match catch_unwind(AssertUnwindSafe(eval)) {
            Ok(value) => self.take_or_return(value),
            Err(payload) => {
                if payload.downcast_ref::<CallbackPanic>().is_none() {
                    std::panic::resume_unwind(payload)
                }

                if let Some(err) = self.error.borrow_mut().take() {
                    return Err(err);
                }
                Err(PyRuntimeError::new_err(
                    "Python callable failed but no error was captured",
                ))
            }
        }
    }

    pub(super) fn run_core<R, E>(
        &self,
        eval: impl FnOnce() -> Result<R, E>,
        map_err: impl Fn(E) -> PyErr,
    ) -> PyResult<R> {
        let raw = self.run_value(eval)?;
        raw.map_err(map_err)
    }

    fn take_or_return<R>(&self, value: R) -> PyResult<R> {
        if let Some(err) = self.error.borrow_mut().take() {
            Err(err)
        } else {
            Ok(value)
        }
    }
}

#[allow(clippy::panic)]
impl<'a> CallableProxy<'a> {
    #[cold]
    fn capture_and_panic(&self, err: PyErr) -> ! {
        *self.error.borrow_mut() = Some(err);
        std::panic::panic_any(CallbackPanic);
    }

    fn invoke(&self, x: f64) -> f64 {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            let result = callable
                .call1((x,))
                .unwrap_or_else(|err| self.capture_and_panic(err));
            result
                .extract::<f64>()
                .unwrap_or_else(|err| self.capture_and_panic(err))
        })
    }
}

pub(super) struct VectorCallableAdapter {
    callable: Py<PyAny>,
    error: RefCell<Option<PyErr>>,
}

#[derive(Clone, Copy)]
struct VectorCallableProxy<'a> {
    callable: &'a Py<PyAny>,
    error: &'a RefCell<Option<PyErr>>,
}

impl VectorCallableAdapter {
    pub(super) fn new(callable: Bound<'_, PyAny>) -> PyResult<Self> {
        if !callable.is_callable() {
            return Err(PyTypeError::new_err("Expected a callable object"));
        }
        let owned: Py<PyAny> = callable.unbind();
        Ok(Self {
            callable: owned,
            error: RefCell::new(None),
        })
    }

    pub(super) fn objective_closure(&self) -> impl Fn(&[f64]) -> f64 + Copy + '_ {
        let proxy = VectorCallableProxy {
            callable: &self.callable,
            error: &self.error,
        };
        move |params| proxy.invoke_objective(params)
    }

    pub(super) fn residual_closure(&self) -> impl Fn(&[f64], &mut [f64]) + Copy + '_ {
        let proxy = VectorCallableProxy {
            callable: &self.callable,
            error: &self.error,
        };
        move |params, output| proxy.invoke_residual(params, output)
    }

    pub(super) fn run_core<R, E>(
        &self,
        eval: impl FnOnce() -> Result<R, E>,
        map_err: impl Fn(E) -> PyErr,
    ) -> PyResult<R> {
        let raw = self.run_value(eval)?;
        raw.map_err(map_err)
    }

    fn run_value<R>(&self, eval: impl FnOnce() -> R) -> PyResult<R> {
        match catch_unwind(AssertUnwindSafe(eval)) {
            Ok(value) => self.take_or_return(value),
            Err(payload) => {
                if payload.downcast_ref::<CallbackPanic>().is_none() {
                    std::panic::resume_unwind(payload)
                }

                if let Some(err) = self.error.borrow_mut().take() {
                    return Err(err);
                }
                Err(PyRuntimeError::new_err(
                    "Python callable failed but no error was captured",
                ))
            }
        }
    }

    fn take_or_return<R>(&self, value: R) -> PyResult<R> {
        if let Some(err) = self.error.borrow_mut().take() {
            Err(err)
        } else {
            Ok(value)
        }
    }
}

#[allow(clippy::panic)]
impl<'a> VectorCallableProxy<'a> {
    #[cold]
    fn capture_and_panic(&self, err: PyErr) -> ! {
        *self.error.borrow_mut() = Some(err);
        std::panic::panic_any(CallbackPanic);
    }

    fn list_from_slice<'py>(&self, py: Python<'py>, params: &[f64]) -> Bound<'py, PyList> {
        PyList::new(py, params).unwrap_or_else(|err| self.capture_and_panic(err))
    }

    fn invoke_objective(&self, params: &[f64]) -> f64 {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            let list = self.list_from_slice(py, params);
            let result = callable
                .call1((list,))
                .unwrap_or_else(|err| self.capture_and_panic(err));
            result
                .extract::<f64>()
                .unwrap_or_else(|err| self.capture_and_panic(err))
        })
    }

    fn invoke_residual(&self, params: &[f64], output: &mut [f64]) {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            let list = self.list_from_slice(py, params);
            let result = callable
                .call1((list,))
                .unwrap_or_else(|err| self.capture_and_panic(err));
            let values = result
                .extract::<Vec<f64>>()
                .unwrap_or_else(|err| self.capture_and_panic(err));

            if values.len() != output.len() {
                self.capture_and_panic(PyValueError::new_err(format!(
                    "Residual function returned {} values, expected {}",
                    values.len(),
                    output.len()
                )));
            }

            for (dest, value) in output.iter_mut().zip(values.into_iter()) {
                *dest = value;
            }
        })
    }
}
