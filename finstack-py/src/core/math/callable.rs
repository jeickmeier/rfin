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
                if payload.downcast_ref::<CallbackPanic>().is_some() {
                    if let Some(err) = self.error.borrow_mut().take() {
                        Err(err)
                    } else {
                        Err(PyRuntimeError::new_err(
                            "Python callable failed but no error was captured",
                        ))
                    }
                } else {
                    std::panic::resume_unwind(payload)
                }
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
    fn invoke(&self, x: f64) -> f64 {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            match callable.call1((x,)) {
                Ok(result) => match result.extract::<f64>() {
                    Ok(value) => value,
                    Err(err) => {
                        *self.error.borrow_mut() = Some(err);
                        std::panic::panic_any(CallbackPanic);
                    }
                },
                Err(err) => {
                    *self.error.borrow_mut() = Some(err);
                    std::panic::panic_any(CallbackPanic);
                }
            }
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
                if payload.downcast_ref::<CallbackPanic>().is_some() {
                    if let Some(err) = self.error.borrow_mut().take() {
                        Err(err)
                    } else {
                        Err(PyRuntimeError::new_err(
                            "Python callable failed but no error was captured",
                        ))
                    }
                } else {
                    std::panic::resume_unwind(payload)
                }
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
    fn invoke_objective(&self, params: &[f64]) -> f64 {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            let list = match PyList::new(py, params) {
                Ok(value) => value,
                Err(err) => {
                    *self.error.borrow_mut() = Some(err);
                    std::panic::panic_any(CallbackPanic);
                }
            };
            match callable.call1((list,)) {
                Ok(result) => match result.extract::<f64>() {
                    Ok(value) => value,
                    Err(err) => {
                        *self.error.borrow_mut() = Some(err);
                        std::panic::panic_any(CallbackPanic);
                    }
                },
                Err(err) => {
                    *self.error.borrow_mut() = Some(err);
                    std::panic::panic_any(CallbackPanic);
                }
            }
        })
    }

    fn invoke_residual(&self, params: &[f64], output: &mut [f64]) {
        Python::attach(|py| {
            let callable = self.callable.bind(py);
            let list = match PyList::new(py, params) {
                Ok(value) => value,
                Err(err) => {
                    *self.error.borrow_mut() = Some(err);
                    std::panic::panic_any(CallbackPanic);
                }
            };
            match callable.call1((list,)) {
                Ok(result) => match result.extract::<Vec<f64>>() {
                    Ok(values) => {
                        if values.len() != output.len() {
                            *self.error.borrow_mut() = Some(PyValueError::new_err(format!(
                                "Residual function returned {} values, expected {}",
                                values.len(),
                                output.len()
                            )));
                            std::panic::panic_any(CallbackPanic);
                        }
                        for (dest, value) in output.iter_mut().zip(values.into_iter()) {
                            *dest = value;
                        }
                    }
                    Err(err) => {
                        *self.error.borrow_mut() = Some(err);
                        std::panic::panic_any(CallbackPanic);
                    }
                },
                Err(err) => {
                    *self.error.borrow_mut() = Some(err);
                    std::panic::panic_any(CallbackPanic);
                }
            }
        })
    }
}
