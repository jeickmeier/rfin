use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsFrequency;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_core::math::stats::RealizedVarMethod;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::variance_swap::{PayReceive as VarSwapPayReceive, VarianceSwap};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = VarianceSwap)]
#[derive(Clone, Debug)]
pub struct JsVarianceSwap(VarianceSwap);

impl InstrumentWrapper for JsVarianceSwap {
    type Inner = VarianceSwap;
    fn from_inner(inner: VarianceSwap) -> Self {
        JsVarianceSwap(inner)
    }
    fn inner(&self) -> VarianceSwap {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = VarianceSwap)]
impl JsVarianceSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        underlying_id: &str,
        notional: &JsMoney,
        strike_variance: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        observation_frequency: &JsFrequency,
        realized_method: Option<String>,
        side: Option<String>,
    ) -> Result<JsVarianceSwap, JsValue> {
        if strike_variance < 0.0 {
            return Err(js_error("Strike variance must be non-negative".to_string()));
        }

        if maturity.inner() <= start_date.inner() {
            return Err(js_error(
                "Maturity must be after observation start".to_string(),
            ));
        }

        let method = parse_optional_with_default(realized_method, RealizedVarMethod::CloseToClose)?;
        let direction = parse_optional_with_default(side, VarSwapPayReceive::Receive)?;

        let swap = VarianceSwap {
            id: instrument_id_from_str(instrument_id),
            underlying_id: underlying_id.to_string(),
            notional: notional.inner(),
            strike_variance,
            start_date: start_date.inner(),
            maturity: maturity.inner(),
            observation_freq: observation_frequency.inner(),
            realized_var_method: method,
            side: direction,
            disc_id: curve_id_from_str(discount_curve),
            day_count: DayCount::Act365F,
            attributes: Attributes::new(),
        };

        Ok(JsVarianceSwap::from_inner(swap))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = strikeVariance)]
    pub fn strike_variance(&self) -> f64 {
        self.0.strike_variance
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::VarianceSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "VarianceSwap(id='{}', strike_var={})",
            self.0.id, self.0.strike_variance
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsVarianceSwap {
        JsVarianceSwap::from_inner(self.0.clone())
    }
}
