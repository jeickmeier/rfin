use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_valuations::instruments::cds::{CDSConvention, PayReceive};
use finstack_valuations::instruments::cds_index::parameters::{
    CDSIndexConstructionParams, CDSIndexParams,
};
use finstack_valuations::instruments::cds_index::CDSIndex;
use finstack_valuations::instruments::common::parameters::CreditParams;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_cds_side(label: Option<String>) -> Result<PayReceive, JsValue> {
    match label.as_deref() {
        None | Some("pay_protection") => Ok(PayReceive::PayProtection),
        Some("receive_protection") => Ok(PayReceive::ReceiveProtection),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid CDS side: {e}"))),
    }
}

fn leak_curve_id(curve: &finstack_core::types::CurveId) -> &'static str {
    Box::leak(curve.as_str().to_string().into_boxed_str())
}

#[wasm_bindgen(js_name = CDSIndex)]
#[derive(Clone, Debug)]
pub struct JsCDSIndex {
    inner: CDSIndex,
}

impl JsCDSIndex {
    pub(crate) fn from_inner(inner: CDSIndex) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> CDSIndex {
        self.inner.clone()
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
        let side_value = parse_cds_side(side)?;
        let recovery = recovery_rate.unwrap_or(0.40);
        
        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error("recovery_rate must be between 0 and 1".to_string()));
        }

        let mut index_params = CDSIndexParams::new(index_name, series, version, fixed_coupon_bp);
        if let Some(factor) = index_factor {
            index_params = index_params.with_index_factor(factor);
        }

        let construction = CDSIndexConstructionParams::new(
            notional.inner(),
            side_value,
            CDSConvention::IsdaNa,
        );
        
        let disc_curve = curve_id_from_str(discount_curve);
        let credit_curve_id = curve_id_from_str(credit_curve);
        
        let credit_params = CreditParams::new(
            index_name.to_string(),
            recovery,
            credit_curve_id.clone(),
        );

        let index = CDSIndex::new_standard(
            instrument_id_from_str(instrument_id),
            &index_params,
            &construction,
            start_date.inner(),
            maturity.inner(),
            &credit_params,
            leak_curve_id(&disc_curve),
            leak_curve_id(&credit_curve_id),
        );

        Ok(JsCDSIndex::from_inner(index))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = indexName)]
    pub fn index_name(&self) -> String {
        self.inner.index_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = fixedCouponBp)]
    pub fn fixed_coupon_bp(&self) -> f64 {
        self.inner.premium.spread_bp
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.premium.end)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDSIndex as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CDSIndex(id='{}', name='{}', series={})",
            self.inner.id, self.inner.index_name, self.inner.series
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCDSIndex {
        JsCDSIndex::from_inner(self.inner.clone())
    }
}

