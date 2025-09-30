use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::utils::{js_array_from_iter, js_error};
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

fn parse_kind(name: &str) -> Option<CFKind> {
    let normalized = name.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    match normalized.as_str() {
        "fixed" => Some(CFKind::Fixed),
        "float_reset" => Some(CFKind::FloatReset),
        "notional" => Some(CFKind::Notional),
        "pik" => Some(CFKind::PIK),
        "amortization" | "amort" => Some(CFKind::Amortization),
        "fee" => Some(CFKind::Fee),
        "stub" => Some(CFKind::Stub),
        _ => None,
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

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CFKind {
        self.inner
    }
}

impl From<CFKind> for JsCFKind {
    fn from(value: CFKind) -> Self {
        Self::from_inner(value)
    }
}

#[wasm_bindgen(js_class = CFKind)]
impl JsCFKind {
    #[wasm_bindgen(js_name = Fixed)]
    pub fn fixed() -> JsCFKind {
        CFKind::Fixed.into()
    }

    #[wasm_bindgen(js_name = FloatReset)]
    pub fn float_reset() -> JsCFKind {
        CFKind::FloatReset.into()
    }

    #[wasm_bindgen(js_name = Notional)]
    pub fn notional() -> JsCFKind {
        CFKind::Notional.into()
    }

    #[wasm_bindgen(js_name = PIK)]
    pub fn pik() -> JsCFKind {
        CFKind::PIK.into()
    }

    #[wasm_bindgen(js_name = Amortization)]
    pub fn amortization() -> JsCFKind {
        CFKind::Amortization.into()
    }

    #[wasm_bindgen(js_name = Fee)]
    pub fn fee() -> JsCFKind {
        CFKind::Fee.into()
    }

    #[wasm_bindgen(js_name = Stub)]
    pub fn stub() -> JsCFKind {
        CFKind::Stub.into()
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsCFKind, JsValue> {
        parse_kind(name)
            .map(Into::into)
            .ok_or_else(|| js_error(format!("Unknown cashflow kind: {name}")))
    }

    #[wasm_bindgen(getter, js_name = name)]
    pub fn name(&self) -> String {
        kind_label(self.inner).to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
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

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CashFlow {
        self.inner
    }
}

#[wasm_bindgen(js_class = CashFlow)]
impl JsCashFlow {
    #[wasm_bindgen(js_name = fixed)]
    pub fn fixed(
        date: &JsDate,
        amount: &JsMoney,
        accrual_factor: Option<f64>,
    ) -> Result<JsCashFlow, JsValue> {
        let mut cf = CashFlow::fixed_cf(date.inner(), amount.inner())
            .map_err(|e| js_error(e.to_string()))?;
        if let Some(value) = accrual_factor {
            cf.accrual_factor = value;
        }
        Ok(Self::from_inner(cf))
    }

    #[wasm_bindgen(js_name = floating)]
    pub fn floating(
        date: &JsDate,
        amount: &JsMoney,
        reset_date: Option<JsDate>,
        accrual_factor: Option<f64>,
    ) -> Result<JsCashFlow, JsValue> {
        let reset = reset_date.map(|d| d.inner());
        let mut cf = CashFlow::floating_cf(date.inner(), amount.inner(), reset)
            .map_err(|e| js_error(e.to_string()))?;
        if let Some(value) = accrual_factor {
            cf.accrual_factor = value;
        }
        Ok(Self::from_inner(cf))
    }

    #[wasm_bindgen(js_name = pik)]
    pub fn pik(date: &JsDate, amount: &JsMoney) -> Result<JsCashFlow, JsValue> {
        CashFlow::pik_cf(date.inner(), amount.inner())
            .map(Self::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = amortization)]
    pub fn amortization(date: &JsDate, amount: &JsMoney) -> Result<JsCashFlow, JsValue> {
        CashFlow::amort_cf(date.inner(), amount.inner())
            .map(Self::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = principalExchange)]
    pub fn principal_exchange(date: &JsDate, amount: &JsMoney) -> Result<JsCashFlow, JsValue> {
        CashFlow::principal_exchange(date.inner(), amount.inner())
            .map(Self::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fee)]
    pub fn fee(date: &JsDate, amount: &JsMoney) -> Result<JsCashFlow, JsValue> {
        CashFlow::fee(date.inner(), amount.inner())
            .map(Self::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> JsCFKind {
        self.inner.kind.into()
    }

    #[wasm_bindgen(getter)]
    pub fn date(&self) -> JsDate {
        JsDate::from_core(self.inner.date)
    }

    #[wasm_bindgen(getter, js_name = resetDate)]
    pub fn reset_date(&self) -> Option<JsDate> {
        self.inner.reset_date.map(|d| JsDate::from_core(d))
    }

    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.amount)
    }

    #[wasm_bindgen(getter, js_name = accrualFactor)]
    pub fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }

    #[wasm_bindgen(setter, js_name = accrualFactor)]
    pub fn set_accrual_factor(&mut self, value: f64) {
        self.inner.accrual_factor = value;
    }

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

    #[allow(dead_code)]
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
    #[wasm_bindgen(js_name = none)]
    pub fn none() -> JsAmortizationSpec {
        AmortizationSpec::None.into()
    }

    #[wasm_bindgen(js_name = linearTo)]
    pub fn linear_to(final_notional: &JsMoney) -> JsAmortizationSpec {
        AmortizationSpec::LinearTo {
            final_notional: final_notional.inner(),
        }
        .into()
    }

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
            .zip(remaining.into_iter())
            .map(|(d, m)| (d.inner(), m.inner()))
            .collect();
        Ok(AmortizationSpec::StepRemaining { schedule }.into())
    }

    #[wasm_bindgen(js_name = percentPerPeriod)]
    pub fn percent_per_period(pct: f64) -> JsAmortizationSpec {
        AmortizationSpec::PercentPerPeriod { pct }.into()
    }

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
            .zip(amounts.into_iter())
            .map(|(d, m)| (d.inner(), m.inner()))
            .collect();
        Ok(AmortizationSpec::CustomPrincipal { items }.into())
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
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
