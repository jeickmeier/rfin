use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::decimal::decimal_to_f64_or_warn;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::InflationLinkedBondParams;
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationLinkedBond)]
#[derive(Clone, Debug)]
pub struct JsInflationLinkedBond {
    pub(crate) inner: InflationLinkedBond,
}

impl JsInflationLinkedBond {
    pub(crate) fn from_inner(inner: InflationLinkedBond) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InflationLinkedBond {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = InflationLinkedBondBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInflationLinkedBondBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    real_coupon: Option<f64>,
    issue: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    base_index: Option<f64>,
    discount_curve: Option<String>,
    inflation_curve: Option<String>,
    indexation: Option<String>,
    frequency: Option<String>,
    day_count: Option<DayCount>,
    deflation_protection: Option<String>,
}

#[wasm_bindgen(js_class = InflationLinkedBondBuilder)]
impl JsInflationLinkedBondBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInflationLinkedBondBuilder {
        JsInflationLinkedBondBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInflationLinkedBondBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = realCoupon)]
    pub fn real_coupon(mut self, real_coupon: f64) -> JsInflationLinkedBondBuilder {
        self.real_coupon = Some(real_coupon);
        self
    }

    #[wasm_bindgen(js_name = issue)]
    pub fn issue(mut self, issue: &JsDate) -> JsInflationLinkedBondBuilder {
        self.issue = Some(issue.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsInflationLinkedBondBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = baseIndex)]
    pub fn base_index(mut self, base_index: f64) -> JsInflationLinkedBondBuilder {
        self.base_index = Some(base_index);
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsInflationLinkedBondBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = inflationCurve)]
    pub fn inflation_curve(mut self, inflation_curve: &str) -> JsInflationLinkedBondBuilder {
        self.inflation_curve = Some(inflation_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = indexation)]
    pub fn indexation(mut self, indexation: String) -> JsInflationLinkedBondBuilder {
        self.indexation = Some(indexation);
        self
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(mut self, frequency: String) -> JsInflationLinkedBondBuilder {
        self.frequency = Some(frequency);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsInflationLinkedBondBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = deflationProtection)]
    pub fn deflation_protection(
        mut self,
        deflation_protection: String,
    ) -> JsInflationLinkedBondBuilder {
        self.deflation_protection = Some(deflation_protection);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInflationLinkedBond, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: notional (money) is required".to_string())
        })?;
        let real_coupon = self.real_coupon.ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: realCoupon is required".to_string())
        })?;
        let issue = self
            .issue
            .ok_or_else(|| js_error("InflationLinkedBondBuilder: issue is required".to_string()))?;
        let maturity = self.maturity.ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: maturity is required".to_string())
        })?;
        let base_index = self.base_index.ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: baseIndex is required".to_string())
        })?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: discountCurve is required".to_string())
        })?;
        let inflation_curve = self.inflation_curve.as_deref().ok_or_else(|| {
            js_error("InflationLinkedBondBuilder: inflationCurve is required".to_string())
        })?;

        let indexation_method =
            parse_optional_with_default(self.indexation, IndexationMethod::TIPS)?;
        let freq = parse_optional_with_default(self.frequency, Tenor::semi_annual())?;
        let dc = self.day_count.unwrap_or(DayCount::ActAct);
        let deflation = parse_optional_with_default(
            self.deflation_protection,
            DeflationProtection::MaturityOnly,
        )?;

        let params = InflationLinkedBondParams::new(
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            freq,
            dc,
        )
        .map_err(|e| js_error(e.to_string()))?;

        InflationLinkedBond::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(params.notional)
            .real_coupon(params.real_coupon)
            .frequency(params.frequency)
            .day_count(params.day_count)
            .issue_date(params.issue)
            .maturity(params.maturity)
            .base_index(params.base_index)
            .base_date(params.issue)
            .indexation_method(indexation_method)
            .lag(indexation_method.standard_lag())
            .deflation_protection(deflation)
            .bdc(BusinessDayConvention::Following)
            .stub(StubKind::None)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(curve_id_from_str(inflation_curve))
            .attributes(Default::default())
            .build()
            .map(JsInflationLinkedBond::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = InflationLinkedBond)]
impl JsInflationLinkedBond {
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
        decimal_to_f64_or_warn(&self.inner.real_coupon, "realCoupon")
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInflationLinkedBond, JsValue> {
        from_js_value(value).map(JsInflationLinkedBond::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this inflation-linked bond.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = self
            .inner
            .build_full_schedule(market.inner(), as_of)
            .map_err(|e| js_error(e.to_string()))?;
        let outstanding_path = sched
            .outstanding_path_per_flow()
            .map_err(|e| js_error(e.to_string()))?;

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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::InflationLinkedBond.to_string()
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
