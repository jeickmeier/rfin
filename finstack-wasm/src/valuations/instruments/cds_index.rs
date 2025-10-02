use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::cds::{CDSConvention, PayReceive as CdsPayReceive};
use finstack_valuations::instruments::cds_index::parameters::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::cds_index::CDSIndex;
use finstack_valuations::instruments::common::parameters::CreditParams;
use finstack_valuations::pricer::InstrumentType;
use std::collections::HashSet;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

// String interning cache to avoid memory leaks while still satisfying 'static lifetime requirements.
// This is a workaround for the core API requiring 'static str.
// TODO: Ideally, the core API should be updated to accept non-static strings.
static CURVE_ID_CACHE: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);

fn intern_curve_id(curve: &finstack_core::types::CurveId) -> &'static str {
    let curve_str = curve.as_str();

    let mut cache = CURVE_ID_CACHE.lock().unwrap();
    let cache = cache.get_or_insert_with(HashSet::new);

    // Check if already interned
    if let Some(&existing) = cache.get(curve_str) {
        return existing;
    }

    // Intern the new string
    let leaked: &'static str = Box::leak(curve_str.to_string().into_boxed_str());
    cache.insert(leaked);
    leaked
}

#[wasm_bindgen(js_name = CDSIndex)]
#[derive(Clone, Debug)]
pub struct JsCDSIndex(CDSIndex);

impl InstrumentWrapper for JsCDSIndex {
    type Inner = CDSIndex;
    fn from_inner(inner: CDSIndex) -> Self {
        JsCDSIndex(inner)
    }
    fn inner(&self) -> CDSIndex {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = CDSIndex)]
impl JsCDSIndex {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        index_name: &str,
        series: u16,
        version: u16,
        notional: &JsMoney,
        fixed_coupon_bp: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        side: Option<String>,
        recovery_rate: Option<f64>,
        index_factor: Option<f64>,
    ) -> Result<JsCDSIndex, JsValue> {
        let side_value = parse_optional_with_default(side, CdsPayReceive::PayProtection)?;
        let recovery = recovery_rate.unwrap_or(0.40);

        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let mut index_params = CDSIndexParams::new(index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = index_factor {
            index_params = index_params.with_index_factor(factor);
        }

        let construction =
            CDSIndexConstructionParams::new(notional.inner(), side_value, CDSConvention::IsdaNa);

        let disc_curve = curve_id_from_str(discount_curve);
        let credit_curve_id = curve_id_from_str(credit_curve);

        let credit_params =
            CreditParams::new(index_name.to_string(), recovery, credit_curve_id.clone());

        let index = CDSIndex::new_standard(
            instrument_id_from_str(instrument_id),
            &index_params,
            &construction,
            start_date.inner(),
            maturity.inner(),
            &credit_params,
            intern_curve_id(&disc_curve),
            intern_curve_id(&credit_curve_id),
        );

        Ok(JsCDSIndex::from_inner(index))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = indexName)]
    pub fn index_name(&self) -> String {
        self.0.index_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedCouponBp)]
    pub fn fixed_coupon_bp(&self) -> f64 {
        self.0.premium.spread_bp
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.0.premium.end)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDSIndex as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CDSIndex(id='{}', name='{}', series={})",
            self.0.id, self.0.index_name, self.0.series
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCDSIndex {
        JsCDSIndex::from_inner(self.0.clone())
    }
}
