use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyAnyMethods;
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

impl<'a> CallableProxy<'a> {
    fn invoke(&self, x: f64) -> f64 {
        Python::with_gil(|py| {
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
