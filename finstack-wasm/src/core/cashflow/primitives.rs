use crate::core::common::parse::ParseFromString;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use finstack_core::cashflow::primitives::{CFKind, CashFlow};
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
    /// Create a fixed coupon cashflow.
    ///
    /// @param {Date} date - Payment date
    /// @param {Money} amount - Payment amount
    /// @param {number} accrualFactor - Accrual factor for the period
    /// @returns {CashFlow} Fixed coupon cashflow
    ///
    /// @example
    /// ```javascript
    /// const fixed = CashFlow.fixed(
    ///     new FsDate(2025, 3, 15),
    ///     Money.fromCode(12500.0, 'USD'),
    ///     0.25
    /// );
    /// ```
    #[wasm_bindgen(js_name = fixed)]
    pub fn fixed(date: &JsDate, amount: &JsMoney, accrual_factor: f64) -> JsCashFlow {
        JsCashFlow::from_inner(CashFlow {
            date: date.inner(),
            reset_date: None,
            amount: amount.inner(),
            kind: CFKind::Fixed,
            accrual_factor,
            rate: None,
        })
    }

    /// Create a floating rate cashflow with reset date.
    ///
    /// @param {Date} paymentDate - Payment date
    /// @param {Money} amount - Payment amount
    /// @param {Date} resetDate - Index reset date
    /// @param {number} accrualFactor - Accrual factor for the period
    /// @returns {CashFlow} Floating rate cashflow
    ///
    /// @example
    /// ```javascript
    /// const floating = CashFlow.floating(
    ///     new FsDate(2025, 6, 15),
    ///     Money.fromCode(13750.0, 'USD'),
    ///     new FsDate(2025, 3, 15),
    ///     0.25
    /// );
    /// ```
    #[wasm_bindgen(js_name = floating)]
    pub fn floating(
        payment_date: &JsDate,
        amount: &JsMoney,
        reset_date: &JsDate,
        accrual_factor: f64,
    ) -> JsCashFlow {
        JsCashFlow::from_inner(CashFlow {
            date: payment_date.inner(),
            reset_date: Some(reset_date.inner()),
            amount: amount.inner(),
            kind: CFKind::FloatReset,
            accrual_factor,
            rate: None,
        })
    }

    /// Create a fee cashflow.
    ///
    /// @param {Date} date - Payment date
    /// @param {Money} amount - Fee amount
    /// @returns {CashFlow} Fee cashflow
    ///
    /// @example
    /// ```javascript
    /// const fee = CashFlow.fee(
    ///     new FsDate(2025, 1, 15),
    ///     Money.fromCode(150000.0, 'USD')
    /// );
    /// ```
    #[wasm_bindgen(js_name = fee)]
    pub fn fee(date: &JsDate, amount: &JsMoney) -> JsCashFlow {
        JsCashFlow::from_inner(CashFlow {
            date: date.inner(),
            reset_date: None,
            amount: amount.inner(),
            kind: CFKind::Fee,
            accrual_factor: 0.0,
            rate: None,
        })
    }

    /// Create a principal exchange (notional) cashflow.
    ///
    /// @param {Date} date - Payment date
    /// @param {Money} amount - Principal amount (negative for repayment)
    /// @returns {CashFlow} Principal exchange cashflow
    ///
    /// @example
    /// ```javascript
    /// const principal = CashFlow.principalExchange(
    ///     new FsDate(2030, 3, 15),
    ///     Money.fromCode(-5000000.0, 'USD')
    /// );
    /// ```
    #[wasm_bindgen(js_name = principalExchange)]
    pub fn principal_exchange(date: &JsDate, amount: &JsMoney) -> JsCashFlow {
        JsCashFlow::from_inner(CashFlow {
            date: date.inner(),
            reset_date: None,
            amount: amount.inner(),
            kind: CFKind::Notional,
            accrual_factor: 0.0,
            rate: None,
        })
    }

    /// Validate cashflow amount and fields.
    /// Validate cashflow amount and fields.
    ///
    /// @throws {Error} If the cashflow amount is zero
    ///
    /// @example
    /// ```javascript
    /// const cf = CashFlow.fixed(
    ///     new FsDate(2025, 2, 15),
    ///     Money.fromCode(50000, 'USD'),
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
