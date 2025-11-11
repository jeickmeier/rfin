//! WASM bindings for cashflow builder specification types.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use finstack_valuations::cashflow::builder::AmortizationSpec;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = AmortizationSpec)]
#[derive(Clone, Debug)]
pub struct JsAmortizationSpec {
    inner: AmortizationSpec,
}

impl JsAmortizationSpec {
    pub(crate) fn from_inner(inner: AmortizationSpec) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> AmortizationSpec {
        self.inner.clone()
    }
}

impl From<AmortizationSpec> for JsAmortizationSpec {
    fn from(value: AmortizationSpec) -> Self {
        Self::from_inner(value)
    }
}

#[wasm_bindgen(js_class = AmortizationSpec)]
impl JsAmortizationSpec {
    /// No amortization (bullet maturity).
    ///
    /// @returns {AmortizationSpec} No amortization specification
    ///
    /// @example
    /// ```javascript
    /// const noAmort = AmortizationSpec.none();
    /// // All principal repaid at maturity
    /// ```
    #[wasm_bindgen(js_name = none)]
    pub fn none() -> JsAmortizationSpec {
        AmortizationSpec::None.into()
    }

    /// Linear amortization to final notional amount.
    ///
    /// @param {Money} final_notional - Final remaining balance at maturity
    /// @returns {AmortizationSpec} Linear amortization specification
    ///
    /// @example
    /// ```javascript
    /// const finalBalance = new Money(200000, new Currency("USD"));
    /// const linearAmort = AmortizationSpec.linearTo(finalBalance);
    /// // Amortizes from initial notional to $200K linearly over term
    /// ```
    #[wasm_bindgen(js_name = linearTo)]
    pub fn linear_to(final_notional: &JsMoney) -> JsAmortizationSpec {
        AmortizationSpec::LinearTo {
            final_notional: final_notional.inner(),
        }
        .into()
    }

    /// Step-down amortization with specified remaining balances.
    ///
    /// @param {Array<Date>} dates - Amortization dates
    /// @param {Array<Money>} remaining - Remaining balances after each date
    /// @returns {AmortizationSpec} Step remaining specification
    /// @throws {Error} If dates and remaining arrays have different lengths
    ///
    /// @example
    /// ```javascript
    /// const dates = [
    ///   new Date(2026, 5, 15),  // June 15, 2026
    ///   new Date(2027, 5, 15),  // June 15, 2027
    ///   new Date(2028, 5, 15)   // June 15, 2028
    /// ];
    /// const remaining = [
    ///   new Money(800000, usd),  // $800K remaining
    ///   new Money(600000, usd),  // $600K remaining
    ///   new Money(400000, usd)   // $400K remaining
    /// ];
    /// const stepAmort = AmortizationSpec.stepRemaining(dates, remaining);
    /// ```
    #[wasm_bindgen(js_name = stepRemaining)]
    pub fn step_remaining(
        dates: Vec<JsDate>,
        remaining: Vec<JsMoney>,
    ) -> Result<JsAmortizationSpec, JsValue> {
        if dates.len() != remaining.len() {
            return Err(js_error(
                "Step remaining schedule requires matching date and remaining arrays",
            ));
        }
        let schedule = dates
            .into_iter()
            .zip(remaining)
            .map(|(d, m)| (d.inner(), m.inner()))
            .collect();
        Ok(AmortizationSpec::StepRemaining { schedule }.into())
    }

    /// Percentage amortization per period.
    ///
    /// @param {number} pct - Percentage to amortize each period (0.0 to 1.0)
    /// @returns {AmortizationSpec} Percentage per period specification
    ///
    /// @example
    /// ```javascript
    /// const pctAmort = AmortizationSpec.percentPerPeriod(0.05); // 5% per period
    /// // Amortizes 5% of remaining balance each period
    /// ```
    #[wasm_bindgen(js_name = percentPerPeriod)]
    pub fn percent_per_period(pct: f64) -> JsAmortizationSpec {
        AmortizationSpec::PercentPerPeriod { pct }.into()
    }

    /// Custom principal payment schedule.
    ///
    /// @param {Array<Date>} dates - Principal payment dates
    /// @param {Array<Money>} amounts - Principal payment amounts
    /// @returns {AmortizationSpec} Custom principal specification
    /// @throws {Error} If dates and amounts arrays have different lengths
    ///
    /// @example
    /// ```javascript
    /// const paymentDates = [
    ///   new Date(2026, 5, 15),
    ///   new Date(2027, 5, 15),
    ///   new Date(2028, 5, 15)
    /// ];
    /// const paymentAmounts = [
    ///   new Money(200000, usd),  // $200K payment
    ///   new Money(300000, usd),  // $300K payment
    ///   new Money(500000, usd)   // $500K payment
    /// ];
    /// const customAmort = AmortizationSpec.customPrincipal(paymentDates, paymentAmounts);
    /// ```
    #[wasm_bindgen(js_name = customPrincipal)]
    pub fn custom_principal(
        dates: Vec<JsDate>,
        amounts: Vec<JsMoney>,
    ) -> Result<JsAmortizationSpec, JsValue> {
        if dates.len() != amounts.len() {
            return Err(js_error(
                "Custom principal schedule requires matching date and amount arrays",
            ));
        }
        let items = dates
            .into_iter()
            .zip(amounts)
            .map(|(d, m)| (d.inner(), m.inner()))
            .collect();
        Ok(AmortizationSpec::CustomPrincipal { items }.into())
    }

    /// String representation of the amortization specification.
    ///
    /// @returns {string} Human-readable description
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match &self.inner {
            AmortizationSpec::None => "AmortizationSpec.none()".to_string(),
            AmortizationSpec::LinearTo { .. } => "AmortizationSpec.linearTo(...)".to_string(),
            AmortizationSpec::StepRemaining { .. } => {
                "AmortizationSpec.stepRemaining(...)".to_string()
            }
            AmortizationSpec::PercentPerPeriod { pct } => {
                format!("AmortizationSpec.percentPerPeriod({pct})")
            }
            AmortizationSpec::CustomPrincipal { .. } => {
                "AmortizationSpec.customPrincipal(...)".to_string()
            }
        }
    }

    /// Convert amortization specification to schedule array.
    ///
    /// @returns {Array<Array>} Array of [Date, Money] tuples for scheduled payments
    ///
    /// @example
    /// ```javascript
    /// const amort = AmortizationSpec.stepRemaining(dates, remaining);
    /// const schedule = amort.toSchedule();
    /// console.log(schedule[0]); // [Date, Money] tuple for first payment
    /// ```
    #[wasm_bindgen(js_name = toSchedule)]
    pub fn to_schedule(&self) -> js_sys::Array {
        use crate::core::utils::js_array_from_iter;
        use wasm_bindgen::JsValue;

        match &self.inner {
            AmortizationSpec::StepRemaining { schedule }
            | AmortizationSpec::CustomPrincipal { items: schedule } => {
                js_array_from_iter(schedule.iter().map(|(d, m)| {
                    let tuple = js_sys::Array::new();
                    tuple.push(&JsValue::from(JsDate::from_core(*d)));
                    tuple.push(&JsValue::from(JsMoney::from_inner(*m)));
                    JsValue::from(tuple)
                }))
            }
            _ => js_sys::Array::new(),
        }
    }
}
