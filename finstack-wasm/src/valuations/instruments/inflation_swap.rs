use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::decimal::decimal_to_f64_or_warn;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, f64_to_decimal, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::inflation_swap::InflationSwap;
use finstack_valuations::instruments::PayReceive;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationSwap)]
#[derive(Clone, Debug)]
pub struct JsInflationSwap {
    pub(crate) inner: InflationSwap,
}

impl InstrumentWrapper for JsInflationSwap {
    type Inner = InflationSwap;
    fn from_inner(inner: InflationSwap) -> Self {
        JsInflationSwap { inner }
    }
    fn inner(&self) -> InflationSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = InflationSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInflationSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    fixed_rate: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    inflation_curve: Option<String>,
    side: Option<String>,
    day_count: Option<String>,
}

#[wasm_bindgen(js_class = InflationSwapBuilder)]
impl JsInflationSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInflationSwapBuilder {
        JsInflationSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInflationSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = fixedRate)]
    pub fn fixed_rate(mut self, fixed_rate: f64) -> JsInflationSwapBuilder {
        self.fixed_rate = Some(fixed_rate);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsInflationSwapBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsInflationSwapBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsInflationSwapBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = inflationCurve)]
    pub fn inflation_curve(mut self, inflation_curve: &str) -> JsInflationSwapBuilder {
        self.inflation_curve = Some(inflation_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: String) -> JsInflationSwapBuilder {
        self.side = Some(side);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsInflationSwapBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInflationSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("InflationSwapBuilder: notional (money) is required".to_string())
        })?;
        let fixed_rate = self
            .fixed_rate
            .ok_or_else(|| js_error("InflationSwapBuilder: fixedRate is required".to_string()))?;
        let start_date = self
            .start_date
            .ok_or_else(|| js_error("InflationSwapBuilder: startDate is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("InflationSwapBuilder: maturity is required".to_string()))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("InflationSwapBuilder: discountCurve is required".to_string())
        })?;
        let inflation_curve = self.inflation_curve.as_deref().ok_or_else(|| {
            js_error("InflationSwapBuilder: inflationCurve is required".to_string())
        })?;

        let side_value = parse_optional_with_default(self.side, PayReceive::PayFixed)?;
        let dc = parse_optional_with_default(self.day_count, DayCount::ActAct)?;

        InflationSwap::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .fixed_rate(f64_to_decimal(fixed_rate, "fixed_rate")?)
            .start_date(start_date)
            .maturity(maturity)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(inflation_curve.into())
            .day_count(dc)
            .side(side_value)
            .attributes(Default::default())
            .build()
            .map(JsInflationSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = InflationSwap)]
impl JsInflationSwap {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInflationSwap, JsValue> {
        from_js_value(value).map(JsInflationSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this zero-coupon inflation swap (fixed + inflation legs at maturity).
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = self
            .inner
            .cashflow_schedule(market.inner(), as_of)
            .map_err(|e| js_error(e.to_string()))?;
        let outstanding_path = sched.outstanding_path_per_flow().unwrap_or_default();

        let result = Array::new();
        for (idx, cf) in sched.flows.iter().enumerate() {
            let entry = Array::new();
            entry.push(&JsDate::from_core(cf.date).into());
            entry.push(&JsMoney::from_inner(cf.amount).into());
            entry.push(&JsValue::from_str(&format!("{:?}", cf.kind)));
            let outstanding = outstanding_path
                .get(idx)
                .map(|(_, m)| m.amount())
                .unwrap_or(0.0);
            entry.push(&JsValue::from_f64(outstanding));
            result.push(&entry);
        }
        Ok(result)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedRate)]
    pub fn fixed_rate(&self) -> f64 {
        decimal_to_f64_or_warn(&self.inner.fixed_rate, "fixedRate")
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::InflationSwap.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationSwap(id='{}', fixed_rate={:.4})",
            self.inner.id, self.inner.fixed_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationSwap {
        JsInflationSwap::from_inner(self.inner.clone())
    }
}
