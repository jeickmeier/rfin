use crate::core::common::parse::ParseFromString;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::core::utils::js_array_from_iter;
use finstack_core::cashflow::primitives::{AmortizationSpec, CFKind, CashFlow};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

fn kind_label(kind: CFKind) -> &'static str {
    match kind {
        CFKind::Fixed => "fixed",
        CFKind::FloatReset => "float_reset",
        CFKind::Notional => "notional",
        CFKind::PIK => "pik",
        CFKind::Amortization => "amortization",
        CFKind::Fee => "fee",
        CFKind::Stub => "stub",
        _ => "unknown",
    }
}

#[wasm_bindgen(js_name = CFKind)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct JsCFKind {
    inner: CFKind,
}

impl JsCFKind {
    pub(crate) fn from_inner(inner: CFKind) -> Self {
        Self { inner }
    }
}

impl From<CFKind> for JsCFKind {
    fn from(value: CFKind) -> Self {
        Self::from_inner(value)
    }
}

#[wasm_bindgen(js_class = CFKind)]
impl JsCFKind {
    /// Fixed coupon payment (predetermined rate).
    ///
    /// @returns {CFKind} Fixed coupon cashflow kind
    #[wasm_bindgen(js_name = Fixed)]
    pub fn fixed() -> JsCFKind {
        CFKind::Fixed.into()
    }

    /// Floating rate payment (index + margin, with reset date).
    ///
    /// @returns {CFKind} Floating rate cashflow kind
    #[wasm_bindgen(js_name = FloatReset)]
    pub fn float_reset() -> JsCFKind {
        CFKind::FloatReset.into()
    }

    /// Principal notional exchange (initial/final principal).
    ///
    /// @returns {CFKind} Notional exchange cashflow kind
    #[wasm_bindgen(js_name = Notional)]
    pub fn notional() -> JsCFKind {
        CFKind::Notional.into()
    }

    /// Payment-in-kind (capitalized interest).
    ///
    /// @returns {CFKind} PIK cashflow kind
    #[wasm_bindgen(js_name = PIK)]
    pub fn pik() -> JsCFKind {
        CFKind::PIK.into()
    }

    /// Principal amortization (scheduled principal repayment).
    ///
    /// @returns {CFKind} Amortization cashflow kind
    #[wasm_bindgen(js_name = Amortization)]
    pub fn amortization() -> JsCFKind {
        CFKind::Amortization.into()
    }

    /// Upfront fee or periodic fee payment.
    ///
    /// @returns {CFKind} Fee cashflow kind
    #[wasm_bindgen(js_name = Fee)]
    pub fn fee() -> JsCFKind {
        CFKind::Fee.into()
    }

    /// Stub payment (irregular period adjustment).
    ///
    /// @returns {CFKind} Stub cashflow kind
    #[wasm_bindgen(js_name = Stub)]
    pub fn stub() -> JsCFKind {
        CFKind::Stub.into()
    }

    /// Create cashflow kind from string name.
    ///
    /// @param {string} name - Cashflow kind name ("fixed", "pik", "amortization", etc.)
    /// @returns {CFKind} Corresponding cashflow kind
    /// @throws {Error} If name is not recognized
    ///
    /// @example
    /// ```javascript
    /// const fixedKind = CFKind.fromName("fixed");
    /// const pikKind = CFKind.fromName("PIK");
    /// const amortKind = CFKind.fromName("amortization");
    /// ```
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsCFKind, JsValue> {
        CFKind::parse_from_string(name).map(Into::into)
    }

    /// String name of this cashflow kind.
    ///
    /// @type {string}
    /// @readonly
    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        kind_label(self.inner).to_string()
    }

    /// String representation of the cashflow kind.
    ///
    /// @returns {string} Human-readable description
    #[wasm_bindgen(js_name = toString)]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        format!("CFKind({})", kind_label(self.inner))
    }
}

#[wasm_bindgen(js_name = CashFlow)]
#[derive(Clone, Copy, Debug)]
pub struct JsCashFlow {
    inner: CashFlow,
}

impl JsCashFlow {
    pub(crate) fn from_inner(inner: CashFlow) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = CashFlow)]
impl JsCashFlow {
    /// Validate cashflow amount and fields.
    ///
    /// @throws {Error} If the cashflow amount is zero
    ///
    /// @example
    /// ```javascript
    /// const cf = new CashFlow(
    ///     new Date(2025, 2, 15),
    ///     new Money(50000, new Currency("USD")),
    ///     CFKind.Fixed(),
    ///     0.25
    /// );
    /// cf.validate(); // Should pass
    /// ```
    #[wasm_bindgen]
    pub fn validate(&self) -> Result<(), JsValue> {
        self.inner.validate().map_err(|e| js_error(e.to_string()))
    }

    /// Cashflow kind (Fixed, FloatReset, PIK, etc.).
    ///
    /// @type {CFKind}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> JsCFKind {
        self.inner.kind.into()
    }

    /// Payment date for this cashflow.
    ///
    /// @type {Date}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn date(&self) -> JsDate {
        JsDate::from_core(self.inner.date)
    }

    /// Reset date for floating rate cashflows.
    ///
    /// @type {Date|null}
    /// @readonly
    #[wasm_bindgen(getter, js_name = resetDate)]
    pub fn reset_date(&self) -> Option<JsDate> {
        self.inner.reset_date.map(JsDate::from_core)
    }

    /// Payment amount for this cashflow.
    ///
    /// @type {Money}
    /// @readonly
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.amount)
    }

    /// Accrual factor for this cashflow period.
    ///
    /// @type {number}
    #[wasm_bindgen(getter, js_name = accrualFactor)]
    pub fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }

    /// Set the accrual factor for this cashflow.
    ///
    /// @param {number} value - New accrual factor
    #[wasm_bindgen(setter, js_name = accrualFactor)]
    pub fn set_accrual_factor(&mut self, value: f64) {
        self.inner.accrual_factor = value;
    }

    /// Convert cashflow to tuple representation for serialization.
    ///
    /// @returns {Array} Tuple [Date, Money, CFKind, accrualFactor, resetDate|null]
    ///
    /// @example
    /// ```javascript
    /// const cf = CashFlow.fixed(date, amount);
    /// const tuple = cf.toTuple();
    /// console.log(tuple[0]); // Date object
    /// console.log(tuple[1]); // Money object
    /// console.log(tuple[2]); // CFKind object
    /// console.log(tuple[3]); // accrual factor number
    /// console.log(tuple[4]); // reset date or null
    /// ```
    #[wasm_bindgen(js_name = toTuple)]
    pub fn to_tuple(&self) -> js_sys::Array {
        let reset = self
            .inner
            .reset_date
            .map(|d| JsValue::from(JsDate::from_core(d)))
            .unwrap_or(JsValue::NULL);
        let tuple = js_sys::Array::new();
        tuple.push(&JsValue::from(JsDate::from_core(self.inner.date)));
        tuple.push(&JsValue::from(JsMoney::from_inner(self.inner.amount)));
        tuple.push(&JsValue::from(JsCFKind::from(self.inner.kind)));
        tuple.push(&JsValue::from_f64(self.inner.accrual_factor));
        tuple.push(&reset);
        tuple
    }
}

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
