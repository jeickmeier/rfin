//! WASM bindings for VolatilityIndexFuture.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::ir_future::Position;
use finstack_valuations::instruments::vol_index_future::{
    VolIndexContractSpecs, VolatilityIndexFuture,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = VolatilityIndexFuture)]
#[derive(Clone, Debug)]
pub struct JsVolatilityIndexFuture {
    pub(crate) inner: VolatilityIndexFuture,
}

impl InstrumentWrapper for JsVolatilityIndexFuture {
    type Inner = VolatilityIndexFuture;
    fn from_inner(inner: VolatilityIndexFuture) -> Self {
        JsVolatilityIndexFuture { inner }
    }
    fn inner(&self) -> VolatilityIndexFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = VolatilityIndexFuture)]
impl JsVolatilityIndexFuture {
    /// Create a volatility index future (e.g., VIX future).
    ///
    /// @param {string} instrumentId - Unique identifier for the instrument
    /// @param {Money} notional - Notional amount (e.g., $100,000 USD)
    /// @param {number} quotedPrice - Quoted future price (e.g., 18.50 for VIX at 18.50)
    /// @param {Date} expiry - Expiry date of the future
    /// @param {string} discountCurve - ID of the discount curve for NPV calculations
    /// @param {string} volIndexCurve - ID of the volatility index curve for forward levels
    /// @param {string} position - Position type: "long" (default) or "short"
    /// @param {number} multiplier - Contract multiplier (default: 1000 for VIX)
    /// @param {number} tickSize - Minimum price movement (default: 0.05)
    /// @param {number} tickValue - Dollar value per tick (default: 50)
    /// @param {string} indexId - Index identifier (default: "VIX")
    /// @returns {VolatilityIndexFuture} The constructed future instrument
    ///
    /// @example
    /// ```javascript
    /// const future = new VolatilityIndexFuture(
    ///   "VIX-F-MAR25",
    ///   new Money("USD", 100000),
    ///   18.50,
    ///   new Date(2025, 2, 15),
    ///   "USD-OIS",
    ///   "VIX-Forward",
    ///   "long",
    ///   1000,     // multiplier
    ///   0.05,     // tick size
    ///   50,       // tick value
    ///   "VIX"     // index id
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        quoted_price: f64,
        expiry: &JsDate,
        discount_curve: &str,
        vol_index_curve: &str,
        position: Option<String>,
        multiplier: Option<f64>,
        tick_size: Option<f64>,
        tick_value: Option<f64>,
        index_id: Option<String>,
    ) -> Result<JsVolatilityIndexFuture, JsValue> {
        let position_value = parse_optional_with_default(position, Position::Long)?;

        let specs = VolIndexContractSpecs {
            multiplier: multiplier.unwrap_or(1000.0),
            tick_size: tick_size.unwrap_or(0.05),
            tick_value: tick_value.unwrap_or(50.0),
            index_id: index_id.unwrap_or_else(|| "VIX".to_string()),
        };

        let builder = VolatilityIndexFuture::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .quoted_price(quoted_price)
            .expiry_date(expiry.inner())
            .discount_curve_id(curve_id_from_str(discount_curve))
            .vol_index_curve_id(curve_id_from_str(vol_index_curve))
            .position(position_value)
            .contract_specs(specs)
            .attributes(Default::default());

        builder
            .build()
            .map(JsVolatilityIndexFuture::from_inner)
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

    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::VolatilityIndexFuture as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "VolatilityIndexFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsVolatilityIndexFuture {
        JsVolatilityIndexFuture::from_inner(self.inner.clone())
    }
}

