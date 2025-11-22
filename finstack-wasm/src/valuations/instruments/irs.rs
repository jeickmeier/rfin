use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsFrequency};
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::irs::InterestRateSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InterestRateSwap)]
#[derive(Clone, Debug)]
pub struct JsInterestRateSwap(InterestRateSwap);

impl InstrumentWrapper for JsInterestRateSwap {
    type Inner = InterestRateSwap;
    fn from_inner(inner: InterestRateSwap) -> Self {
        JsInterestRateSwap(inner)
    }
    fn inner(&self) -> InterestRateSwap {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateSwap)]
impl JsInterestRateSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        start: &JsDate,
        end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        side: &str,
        fixed_frequency: Option<JsFrequency>,
        fixed_day_count: Option<JsDayCount>,
        float_frequency: Option<JsFrequency>,
        float_day_count: Option<JsDayCount>,
        business_day_convention: Option<JsBusinessDayConvention>,
        calendar_id: Option<String>,
        stub_kind: Option<crate::core::dates::schedule::JsStubKind>,
        reset_lag_days: Option<i32>,
    ) -> Result<JsInterestRateSwap, JsValue> {
        use finstack_core::dates::BusinessDayConvention;
        use finstack_core::dates::StubKind;
        use finstack_valuations::instruments::common::parameters::legs::{
            FixedLegSpec, FloatLegSpec, PayReceive,
        };

        let side_parsed: PayReceive = side.parse().map_err(js_error)?;
        let bdc = business_day_convention
            .map(Into::<BusinessDayConvention>::into)
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
        let fixed_freq = fixed_frequency
            .map(|f| f.inner())
            .unwrap_or(finstack_core::dates::Frequency::semi_annual());
        let float_freq = float_frequency
            .map(|f| f.inner())
            .unwrap_or(finstack_core::dates::Frequency::quarterly());
        let fixed_dc = fixed_day_count
            .map(|d| d.inner())
            .unwrap_or(finstack_core::dates::DayCount::Thirty360);
        let float_dc = float_day_count
            .map(|d| d.inner())
            .unwrap_or(finstack_core::dates::DayCount::Act360);
        let stub = stub_kind.map(|s| s.inner()).unwrap_or(StubKind::None);
        let fixed = FixedLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            rate: fixed_rate,
            freq: fixed_freq,
            dc: fixed_dc,
            bdc,
            calendar_id: calendar_id.clone(),
            stub,
            start: start.inner(),
            end: end.inner(),
            par_method: None,
            compounding_simple: true,
        };
        let float = FloatLegSpec {
            discount_curve_id: curve_id_from_str(discount_curve),
            forward_curve_id: curve_id_from_str(forward_curve),
            spread_bp: 0.0,
            freq: float_freq,
            dc: float_dc,
            bdc,
            calendar_id: calendar_id.clone(),
            fixing_calendar_id: calendar_id,
            stub,
            reset_lag_days: reset_lag_days.unwrap_or(2),
            start: start.inner(),
            end: end.inner(),
            compounding: Default::default(),
        };
        let swap = InterestRateSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .side(side_parsed)
            .fixed(fixed)
            .float(float)
            .build()
            .map_err(|e| js_error(e.to_string()))?;
        Ok(JsInterestRateSwap::from_inner(swap))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        self.0.fixed.rate
    }

    #[wasm_bindgen(getter, js_name = floatSpreadBp)]
    pub fn float_spread_bp(&self) -> f64 {
        self.0.float.spread_bp
    }

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.0.fixed.start)
    }

    #[wasm_bindgen(getter)]
    pub fn end(&self) -> JsDate {
        JsDate::from_core(self.0.fixed.end)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.fixed.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.0.float.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::IRS as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateSwap(id='{}', notional={}, fixed_rate={:.4})",
            self.0.id, self.0.notional, self.0.fixed.rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateSwap {
        JsInterestRateSwap::from_inner(self.0.clone())
    }
}
