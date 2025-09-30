use crate::core::utils::js_error;
use js_sys::Function;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use wasm_bindgen::{JsCast, JsValue};

struct CallbackPanic;

#[derive(Clone)]
pub(crate) struct JsCallable {
    func: Function,
    error: RefCell<Option<JsValue>>,
}

impl JsCallable {
    pub(crate) fn new(value: &JsValue) -> Result<Self, JsValue> {
        if let Some(function) = value.dyn_ref::<Function>() {
            Ok(Self {
                func: function.clone(),
                error: RefCell::new(None),
            })
        } else {
            Err(js_error("Expected a JavaScript function"))
        }
    }

    pub(crate) fn closure(&self) -> impl Fn(f64) -> f64 + Copy + '_ {
        let proxy = JsCallableProxy {
            func: &self.func,
            error: &self.error,
        };
        move |x| proxy.invoke(x)
    }

    pub(crate) fn run_value<R>(&self, eval: impl FnOnce() -> R) -> Result<R, JsValue> {
        match catch_unwind(AssertUnwindSafe(eval)) {
            Ok(value) => self.take_or_return(value),
            Err(payload) => {
                if payload.downcast_ref::<CallbackPanic>().is_some() {
                    if let Some(err) = self.error.borrow_mut().take() {
                        Err(err)
                    } else {
                        Err(js_error(
                            "JavaScript callable failed but no error was captured",
                        ))
                    }
                } else {
                    std::panic::resume_unwind(payload)
                }
            }
        }
    }

    pub(crate) fn run_core<R, E>(
        &self,
        eval: impl FnOnce() -> Result<R, E>,
        map_err: impl Fn(E) -> JsValue,
    ) -> Result<R, JsValue> {
        let raw = self.run_value(eval)?;
        raw.map_err(map_err)
    }

    fn take_or_return<R>(&self, value: R) -> Result<R, JsValue> {
        if let Some(err) = self.error.borrow_mut().take() {
            Err(err)
        } else {
            Ok(value)
        }
    }
}

#[derive(Clone, Copy)]
struct JsCallableProxy<'a> {
    func: &'a Function,
    error: &'a RefCell<Option<JsValue>>,
}

impl JsCallableProxy<'_> {
    fn invoke(&self, x: f64) -> f64 {
        match self.func.call1(&JsValue::UNDEFINED, &JsValue::from_f64(x)) {
            Ok(value) => value.as_f64().unwrap_or_else(|| {
                *self.error.borrow_mut() = Some(js_error("Function must return a finite number"));
                std::panic::panic_any(CallbackPanic);
            }),
            Err(err) => {
                *self.error.borrow_mut() = Some(err);
                std::panic::panic_any(CallbackPanic);
            }
        }
    }
}
