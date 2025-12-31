//! Frequency type for WASM bindings.
//!
//! `Frequency` is semantically equivalent to `Tenor` and is provided for API clarity
//! when representing payment frequencies (e.g., annual, quarterly, monthly).

use finstack_core::dates::{Tenor, TenorUnit};
use wasm_bindgen::prelude::*;

use super::daycount::JsTenor;
use crate::core::error::js_error;

/// Frequency token representing a payment interval.
///
/// Frequency is interchangeable with Tenor but is named to better convey
/// its usage in schedule generation and day-count contexts.
#[wasm_bindgen(js_name = Frequency)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsFrequency {
    inner: Tenor,
}

impl JsFrequency {
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Tenor {
        self.inner
    }

    pub(crate) fn from_inner(inner: Tenor) -> Self {
        Self { inner }
    }
}

impl From<Tenor> for JsFrequency {
    fn from(value: Tenor) -> Self {
        Self::from_inner(value)
    }
}

impl From<JsFrequency> for JsTenor {
    fn from(value: JsFrequency) -> Self {
        JsTenor::from_inner(value.inner)
    }
}

impl From<JsTenor> for JsFrequency {
    fn from(value: JsTenor) -> Self {
        Self::from_inner(value.inner())
    }
}

#[wasm_bindgen(js_class = Frequency)]
impl JsFrequency {
    #[wasm_bindgen(constructor)]
    pub fn new(months: u8) -> Result<JsFrequency, JsValue> {
        JsFrequency::from_months(months)
    }

    #[wasm_bindgen(js_name = fromMonths)]
    pub fn from_months(months: u8) -> Result<JsFrequency, JsValue> {
        if months == 0 {
            return Err(js_error("Months must be positive"));
        }
        Ok(Self::from_inner(Tenor::new(
            months as u32,
            TenorUnit::Months,
        )))
    }

    #[wasm_bindgen(js_name = fromDays)]
    pub fn from_days(days: u16) -> Result<JsFrequency, JsValue> {
        if days == 0 {
            return Err(js_error("Days must be greater than zero"));
        }
        Ok(Self::from_inner(Tenor::new(days as u32, TenorUnit::Days)))
    }

    #[wasm_bindgen(js_name = fromPaymentsPerYear)]
    pub fn from_payments_per_year(payments: u32) -> Result<JsFrequency, JsValue> {
        Tenor::from_payments_per_year(payments)
            .map(Self::from_inner)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = annual)]
    pub fn annual() -> JsFrequency {
        Self::from_inner(Tenor::annual())
    }

    #[wasm_bindgen(js_name = semiAnnual)]
    pub fn semi_annual() -> JsFrequency {
        Self::from_inner(Tenor::semi_annual())
    }

    #[wasm_bindgen(js_name = quarterly)]
    pub fn quarterly() -> JsFrequency {
        Self::from_inner(Tenor::quarterly())
    }

    #[wasm_bindgen(js_name = monthly)]
    pub fn monthly() -> JsFrequency {
        Self::from_inner(Tenor::monthly())
    }

    #[wasm_bindgen(js_name = biMonthly)]
    pub fn bi_monthly() -> JsFrequency {
        Self::from_inner(Tenor::bimonthly())
    }

    #[wasm_bindgen(js_name = biWeekly)]
    pub fn bi_weekly() -> JsFrequency {
        Self::from_inner(Tenor::biweekly())
    }

    #[wasm_bindgen(js_name = weekly)]
    pub fn weekly() -> JsFrequency {
        Self::from_inner(Tenor::weekly())
    }

    #[wasm_bindgen(js_name = daily)]
    pub fn daily() -> JsFrequency {
        Self::from_inner(Tenor::daily())
    }

    #[wasm_bindgen(getter)]
    pub fn months(&self) -> Option<u32> {
        self.inner.months()
    }

    #[wasm_bindgen(getter)]
    pub fn days(&self) -> Option<u32> {
        self.inner.days()
    }

    /// Convert to a Tenor
    #[wasm_bindgen(js_name = toTenor)]
    pub fn to_tenor(&self) -> JsTenor {
        JsTenor::from_inner(self.inner)
    }

    /// Create from a Tenor
    #[wasm_bindgen(js_name = fromTenor)]
    pub fn from_tenor(tenor: &JsTenor) -> JsFrequency {
        Self::from_inner(tenor.inner())
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        if let Some(m) = self.inner.months() {
            format!("Frequency.months({m})")
        } else if let Some(d) = self.inner.days() {
            format!("Frequency.days({d})")
        } else {
            "Frequency(?)".to_string()
        }
    }
}
