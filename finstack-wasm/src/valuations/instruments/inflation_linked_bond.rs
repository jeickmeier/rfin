use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::instruments::inflation_linked_bond::parameters::InflationLinkedBondParams;
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationLinkedBond)]
#[derive(Clone, Debug)]
pub struct JsInflationLinkedBond {
    inner: InflationLinkedBond,
}

impl JsInflationLinkedBond {
    pub(crate) fn from_inner(inner: InflationLinkedBond) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InflationLinkedBond {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InflationLinkedBond)]
impl JsInflationLinkedBond {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        real_coupon: f64,
        issue: &JsDate,
        maturity: &JsDate,
        base_index: f64,
        discount_curve: &str,
        inflation_curve: &str,
        indexation: Option<String>,
        frequency: Option<String>,
        day_count: Option<JsDayCount>,
        deflation_protection: Option<String>,
    ) -> Result<JsInflationLinkedBond, JsValue> {
        let indexation_method = parse_optional_with_default(indexation, IndexationMethod::TIPS)?;
        let freq = parse_optional_with_default(frequency, Frequency::semi_annual())?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::ActAct);
        let deflation = parse_optional_with_default(deflation_protection, DeflationProtection::MaturityOnly)?;

        let params = InflationLinkedBondParams::new(
            notional.inner(),
            real_coupon,
            issue.inner(),
            maturity.inner(),
            base_index,
            freq,
            dc,
        );

        let builder = InflationLinkedBond::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(params.notional)
            .real_coupon(params.real_coupon)
            .freq(params.frequency)
            .dc(params.day_count)
            .issue(params.issue)
            .maturity(params.maturity)
            .base_index(params.base_index)
            .base_date(params.issue)
            .indexation_method(indexation_method)
            .lag(indexation_method.standard_lag())
            .deflation_protection(deflation)
            .bdc(BusinessDayConvention::Following)
            .stub(StubKind::None)
            .disc_id(curve_id_from_str(discount_curve))
            .inflation_id(curve_id_from_str(inflation_curve))
            .attributes(Default::default());

        builder
            .build()
            .map(JsInflationLinkedBond::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = realCoupon)]
    pub fn real_coupon(&self) -> f64 {
        self.inner.real_coupon
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InflationLinkedBond as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationLinkedBond(id='{}', coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationLinkedBond {
        JsInflationLinkedBond::from_inner(self.inner.clone())
    }
}
