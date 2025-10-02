use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::convertible::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = ConversionPolicy)]
#[derive(Clone, Debug)]
pub struct JsConversionPolicy {
    inner: ConversionPolicy,
}

#[wasm_bindgen(js_class = ConversionPolicy)]
impl JsConversionPolicy {
    #[wasm_bindgen(js_name = voluntary)]
    pub fn voluntary() -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::Voluntary,
        }
    }

    #[wasm_bindgen(js_name = mandatoryOn)]
    pub fn mandatory_on(conversion_date: &JsDate) -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::MandatoryOn(conversion_date.inner()),
        }
    }

    #[wasm_bindgen(js_name = uponEvent)]
    pub fn upon_event(price_threshold: f64, lookback_days: u32) -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::UponEvent(ConversionEvent::PriceTrigger {
                threshold: price_threshold,
                lookback_days,
            }),
        }
    }
}

#[wasm_bindgen(js_name = ConversionSpec)]
#[derive(Clone, Debug)]
pub struct JsConversionSpec {
    inner: ConversionSpec,
}

#[wasm_bindgen(js_class = ConversionSpec)]
impl JsConversionSpec {
    #[wasm_bindgen(constructor)]
    pub fn new(
        policy: &JsConversionPolicy,
        ratio: Option<f64>,
        price: Option<f64>,
    ) -> Result<JsConversionSpec, JsValue> {
        if ratio.is_none() && price.is_none() {
            return Err(js_error(
                "Provide either conversion ratio or conversion price".to_string(),
            ));
        }

        Ok(JsConversionSpec {
            inner: ConversionSpec {
                ratio,
                price,
                policy: policy.inner.clone(),
                anti_dilution: AntiDilutionPolicy::None,
                dividend_adjustment: DividendAdjustment::None,
            },
        })
    }
}

#[wasm_bindgen(js_name = ConvertibleBond)]
#[derive(Clone, Debug)]
pub struct JsConvertibleBond(ConvertibleBond);

impl InstrumentWrapper for JsConvertibleBond {
    type Inner = ConvertibleBond;
    fn from_inner(inner: ConvertibleBond) -> Self {
        JsConvertibleBond(inner)
    }
    fn inner(&self) -> ConvertibleBond {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = ConvertibleBond)]
impl JsConvertibleBond {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        conversion: &JsConversionSpec,
        underlying_equity_id: Option<String>,
    ) -> Result<JsConvertibleBond, JsValue> {
        let bond = ConvertibleBond {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            issue: issue.inner(),
            maturity: maturity.inner(),
            disc_id: curve_id_from_str(discount_curve),
            conversion: conversion.inner.clone(),
            underlying_equity_id,
            call_put: None,
            fixed_coupon: None,
            floating_coupon: None,
            attributes: Default::default(),
        };

        Ok(JsConvertibleBond::from_inner(bond))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter, js_name = conversionRatio)]
    pub fn conversion_ratio(&self) -> Option<f64> {
        self.0.conversion.ratio
    }

    #[wasm_bindgen(getter, js_name = conversionPrice)]
    pub fn conversion_price(&self) -> Option<f64> {
        self.0.conversion.price
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Convertible as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ConvertibleBond(id='{}', notional={})",
            self.0.id, self.0.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsConvertibleBond {
        JsConvertibleBond::from_inner(self.0.clone())
    }
}
