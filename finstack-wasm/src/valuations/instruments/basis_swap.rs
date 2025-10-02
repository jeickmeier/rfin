use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::error::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_frequency(label: Option<String>) -> Result<Frequency, JsValue> {
    match label.as_deref() {
        None | Some("quarterly") => Ok(Frequency::quarterly()),
        Some("monthly") => Ok(Frequency::monthly()),
        Some("semi_annual") | Some("semiannual") => Ok(Frequency::semi_annual()),
        Some("annual") => Ok(Frequency::annual()),
        Some(other) => Err(js_error(format!("Unsupported frequency: {other}"))),
    }
}

fn parse_day_count(label: Option<String>) -> Result<DayCount, JsValue> {
    match label.as_deref() {
        None | Some("act_360") => Ok(DayCount::Act360),
        Some("act_365f") => Ok(DayCount::Act365F),
        Some("thirty_360") | Some("30_360") => Ok(DayCount::Thirty360),
        Some(other) => Err(js_error(format!("Unsupported day count: {other}"))),
    }
}

fn parse_stub(label: Option<String>) -> Result<StubKind, JsValue> {
    match label.as_deref() {
        None | Some("none") => Ok(StubKind::None),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid stub kind: {e}"))),
    }
}

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
        let freq = parse_frequency(frequency)?;
        let dc = parse_day_count(day_count)?;

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
pub struct JsBasisSwap {
    inner: BasisSwap,
}

impl JsBasisSwap {
    pub(crate) fn from_inner(inner: BasisSwap) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> BasisSwap {
        self.inner.clone()
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
        let stub_kind = parse_stub(stub)?;

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
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::BasisSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("BasisSwap(id='{}')", self.inner.id)
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasisSwap {
        JsBasisSwap::from_inner(self.inner.clone())
    }
}

