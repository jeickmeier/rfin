//! Levered real estate equity WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::real_estate::{
    LeveredRealEstateEquity, RealEstateAsset,
};
use finstack_valuations::instruments::InstrumentJson;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Levered real estate equity instrument (asset minus debt).
#[wasm_bindgen(js_name = LeveredRealEstateEquity)]
#[derive(Clone, Debug)]
pub struct JsLeveredRealEstateEquity {
    pub(crate) inner: LeveredRealEstateEquity,
}

impl InstrumentWrapper for JsLeveredRealEstateEquity {
    type Inner = LeveredRealEstateEquity;
    fn from_inner(inner: LeveredRealEstateEquity) -> Self {
        JsLeveredRealEstateEquity { inner }
    }
    fn inner(&self) -> LeveredRealEstateEquity {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = LeveredRealEstateEquityBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsLeveredRealEstateEquityBuilder {
    instrument_id: String,
    currency: Option<finstack_core::currency::Currency>,
    asset: Option<RealEstateAsset>,
    financing: Option<Vec<JsValue>>,
    exit_date: Option<finstack_core::dates::Date>,
    discount_curve_id: Option<String>,
}

#[wasm_bindgen(js_class = LeveredRealEstateEquityBuilder)]
impl JsLeveredRealEstateEquityBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsLeveredRealEstateEquityBuilder {
        JsLeveredRealEstateEquityBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &JsCurrency) -> JsLeveredRealEstateEquityBuilder {
        self.currency = Some(currency.inner());
        self
    }

    #[wasm_bindgen(js_name = asset)]
    pub fn asset(mut self, asset: &crate::valuations::instruments::RealEstateAsset) -> Self {
        self.asset = Some(asset.inner.clone());
        self
    }

    /// Financing stack as an array of tagged `{ type, spec }` objects.
    ///
    /// Each entry must match `InstrumentJson` (snake_case type tag + spec payload).
    /// For example:
    /// `{ type: "term_loan", spec: termLoan.toJson() }`
    #[wasm_bindgen(js_name = financing)]
    pub fn financing(mut self, financing: Vec<JsValue>) -> Self {
        self.financing = Some(financing);
        self
    }

    #[wasm_bindgen(js_name = exitDate)]
    pub fn exit_date(mut self, exit_date: &JsDate) -> JsLeveredRealEstateEquityBuilder {
        self.exit_date = Some(exit_date.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(
        mut self,
        discount_curve_id: &str,
    ) -> JsLeveredRealEstateEquityBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsLeveredRealEstateEquity, JsValue> {
        let id = instrument_id_from_str(&self.instrument_id);
        let currency = self.currency.ok_or_else(|| js_error("Missing currency"))?;
        let asset = self.asset.ok_or_else(|| js_error("Missing asset"))?;
        let financing = self
            .financing
            .ok_or_else(|| js_error("Missing financing"))?
            .into_iter()
            .map(from_js_value::<InstrumentJson>)
            .collect::<Result<Vec<_>, _>>()?;
        let dcid = self
            .discount_curve_id
            .ok_or_else(|| js_error("Missing discountCurveId"))?;

        let inst = LeveredRealEstateEquity::builder()
            .id(id)
            .currency(currency)
            .asset(asset)
            .financing(financing)
            .exit_date_opt(self.exit_date)
            .discount_curve_id(curve_id_from_str(&dcid))
            .attributes(Default::default())
            .build()
            .map_err(|e| js_error(e.to_string()))?;

        Ok(JsLeveredRealEstateEquity { inner: inst })
    }
}

#[wasm_bindgen(js_class = LeveredRealEstateEquity)]
impl JsLeveredRealEstateEquity {
    #[wasm_bindgen(js_name = instrumentId, getter)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = currency, getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
    }

    #[wasm_bindgen(js_name = discountCurveId, getter)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = exitDate, getter)]
    pub fn exit_date(&self) -> Option<JsDate> {
        self.inner.exit_date.map(JsDate::from_core)
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::LeveredRealEstateEquity.to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsLeveredRealEstateEquity, JsValue> {
        from_js_value(value).map(|inner| JsLeveredRealEstateEquity { inner })
    }
}
