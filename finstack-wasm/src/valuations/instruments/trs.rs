use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::instrument_id_from_str;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::common::parameters::legs::FinancingLegSpec;
use finstack_valuations::instruments::common::parameters::underlying::{
    EquityUnderlyingParams, IndexUnderlyingParams,
};
use finstack_valuations::instruments::equity_trs::EquityTotalReturnSwap;
use finstack_valuations::instruments::fi_trs::FIIndexTotalReturnSwap;
use finstack_valuations::instruments::{TrsScheduleSpec, TrsSide};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

// Simplified TRS schedule spec for WASM
#[wasm_bindgen(js_name = TrsScheduleSpec)]
#[derive(Clone, Debug)]
pub struct JsTrsScheduleSpec {
    pub(crate) inner: TrsScheduleSpec,
}

#[wasm_bindgen(js_class = TrsScheduleSpec)]
impl JsTrsScheduleSpec {
    #[wasm_bindgen(constructor)]
    pub fn new(
        start: &JsDate,
        end: &JsDate,
        schedule_params: &crate::valuations::cashflow::builder::JsScheduleParams,
    ) -> Result<JsTrsScheduleSpec, JsValue> {
        if end.inner() <= start.inner() {
            return Err(js_error("Schedule end must be after start".to_string()));
        }

        let spec =
            TrsScheduleSpec::from_params(start.inner(), end.inner(), schedule_params.inner());
        Ok(JsTrsScheduleSpec { inner: spec })
    }
}

// Financing leg specification
#[wasm_bindgen(js_name = TrsFinancingLegSpec)]
#[derive(Clone, Debug)]
pub struct JsFinancingLegSpec {
    pub(crate) inner: FinancingLegSpec,
}

#[wasm_bindgen(js_class = TrsFinancingLegSpec)]
impl JsFinancingLegSpec {
    #[wasm_bindgen(constructor)]
    pub fn new(
        discount_curve: &str,
        forward_curve: &str,
        day_count: &crate::core::dates::daycount::JsDayCount,
        spread_bp: Option<f64>,
    ) -> JsFinancingLegSpec {
        JsFinancingLegSpec {
            inner: FinancingLegSpec::new(
                discount_curve.to_string(),
                forward_curve.to_string(),
                spread_bp.unwrap_or(0.0),
                day_count.inner(),
            ),
        }
    }
}

// Equity underlying parameters
#[wasm_bindgen(js_name = EquityUnderlying)]
#[derive(Clone, Debug)]
pub struct JsEquityUnderlying {
    pub(crate) inner: EquityUnderlyingParams,
}

#[wasm_bindgen(js_class = EquityUnderlying)]
impl JsEquityUnderlying {
    #[wasm_bindgen(constructor)]
    pub fn new(
        ticker: &str,
        spot_id: &str,
        currency: &JsCurrency,
        div_yield_id: Option<String>,
    ) -> JsEquityUnderlying {
        let mut params = EquityUnderlyingParams::new(ticker, spot_id, currency.inner());
        if let Some(div) = div_yield_id {
            params = params.with_dividend_yield(&div);
        }
        JsEquityUnderlying { inner: params }
    }
}

// Index underlying parameters
#[wasm_bindgen(js_name = IndexUnderlying)]
#[derive(Clone, Debug)]
pub struct JsIndexUnderlying {
    pub(crate) inner: IndexUnderlyingParams,
}

#[wasm_bindgen(js_class = IndexUnderlying)]
impl JsIndexUnderlying {
    #[wasm_bindgen(constructor)]
    pub fn new(
        index_id: &str,
        base_currency: &JsCurrency,
        yield_id: Option<String>,
    ) -> JsIndexUnderlying {
        let mut params = IndexUnderlyingParams::new(index_id, base_currency.inner());
        if let Some(y) = yield_id {
            params = params.with_yield(&y);
        }
        JsIndexUnderlying { inner: params }
    }
}

// Equity TRS
#[wasm_bindgen(js_name = EquityTotalReturnSwap)]
#[derive(Clone, Debug)]
pub struct JsEquityTotalReturnSwap {
    pub(crate) inner: EquityTotalReturnSwap,
}

impl InstrumentWrapper for JsEquityTotalReturnSwap {
    type Inner = EquityTotalReturnSwap;
    fn from_inner(inner: EquityTotalReturnSwap) -> Self {
        JsEquityTotalReturnSwap { inner }
    }
    fn inner(&self) -> EquityTotalReturnSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = EquityTotalReturnSwap)]
impl JsEquityTotalReturnSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        underlying: &JsEquityUnderlying,
        financing: &JsFinancingLegSpec,
        schedule: &JsTrsScheduleSpec,
        receive_total_return: bool,
        initial_level: Option<f64>,
    ) -> JsEquityTotalReturnSwap {
        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = EquityTotalReturnSwap {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side,
            initial_level,
            attributes: Default::default(),
            margin_spec: None,
        };

        JsEquityTotalReturnSwap::from_inner(trs)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::EquityTotalReturnSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "EquityTotalReturnSwap(id='{}', notional={})",
            self.inner.id, self.inner.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquityTotalReturnSwap {
        JsEquityTotalReturnSwap::from_inner(self.inner.clone())
    }
}

// FI Index TRS
#[wasm_bindgen(js_name = FiIndexTotalReturnSwap)]
#[derive(Clone, Debug)]
pub struct JsFiIndexTotalReturnSwap {
    pub(crate) inner: FIIndexTotalReturnSwap,
}

impl InstrumentWrapper for JsFiIndexTotalReturnSwap {
    type Inner = FIIndexTotalReturnSwap;
    fn from_inner(inner: FIIndexTotalReturnSwap) -> Self {
        JsFiIndexTotalReturnSwap { inner }
    }
    fn inner(&self) -> FIIndexTotalReturnSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FiIndexTotalReturnSwap)]
impl JsFiIndexTotalReturnSwap {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        underlying: &JsIndexUnderlying,
        financing: &JsFinancingLegSpec,
        schedule: &JsTrsScheduleSpec,
        receive_total_return: bool,
        initial_level: Option<f64>,
    ) -> JsFiIndexTotalReturnSwap {
        let side = if receive_total_return {
            TrsSide::ReceiveTotalReturn
        } else {
            TrsSide::PayTotalReturn
        };

        let trs = FIIndexTotalReturnSwap {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            underlying: underlying.inner.clone(),
            financing: financing.inner.clone(),
            schedule: schedule.inner.clone(),
            side,
            initial_level,
            attributes: Default::default(),
            margin_spec: None,
        };

        JsFiIndexTotalReturnSwap::from_inner(trs)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FIIndexTotalReturnSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FiIndexTotalReturnSwap(id='{}', notional={})",
            self.inner.id, self.inner.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFiIndexTotalReturnSwap {
        JsFiIndexTotalReturnSwap::from_inner(self.inner.clone())
    }
}
