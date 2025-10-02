use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::pricer::InstrumentType;
use crate::valuations::instruments::InstrumentWrapper;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = BasisSwapLeg)]
#[derive(Clone, Debug)]
pub struct JsBasisSwapLeg {
    inner: BasisSwapLeg,
}

#[wasm_bindgen(js_class = BasisSwapLeg)]
impl JsBasisSwapLeg {
    #[wasm_bindgen(constructor)]
    pub fn new(
        forward_curve: &str,
        frequency: Option<String>,
        day_count: Option<String>,
        spread: Option<f64>,
    ) -> Result<JsBasisSwapLeg, JsValue> {
        let freq = parse_optional_with_default(frequency, Frequency::quarterly())?;
        let dc = parse_optional_with_default(day_count, DayCount::Act360)?;

        Ok(JsBasisSwapLeg {
            inner: BasisSwapLeg {
                forward_curve_id: curve_id_from_str(forward_curve),
                frequency: freq,
                day_count: dc,
                bdc: BusinessDayConvention::ModifiedFollowing,
                spread: spread.unwrap_or(0.0),
            },
        })
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn spread(&self) -> f64 {
        self.inner.spread
    }
}

#[wasm_bindgen(js_name = BasisSwap)]
#[derive(Clone, Debug)]
pub struct JsBasisSwap(BasisSwap);

impl InstrumentWrapper for JsBasisSwap {
    type Inner = BasisSwap;
    fn from_inner(inner: BasisSwap) -> Self {
        JsBasisSwap(inner)
    }
    fn inner(&self) -> BasisSwap {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = BasisSwap)]
impl JsBasisSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        start_date: &JsDate,
        maturity: &JsDate,
        primary_leg: &JsBasisSwapLeg,
        reference_leg: &JsBasisSwapLeg,
        discount_curve: &str,
        calendar: Option<String>,
        stub: Option<String>,
    ) -> Result<JsBasisSwap, JsValue> {
        let stub_kind = parse_optional_with_default(stub, StubKind::None)?;

        let builder = BasisSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .start_date(start_date.inner())
            .maturity_date(maturity.inner())
            .primary_leg(primary_leg.inner.clone())
            .reference_leg(reference_leg.inner.clone())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .stub_kind(stub_kind)
            .calendar_id_opt(optional_static_str(calendar))
            .attributes(Default::default());

        builder
            .build()
            .map(JsBasisSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::BasisSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("BasisSwap(id='{}')", self.0.id)
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasisSwap {
        JsBasisSwap::from_inner(self.0.clone())
    }
}
