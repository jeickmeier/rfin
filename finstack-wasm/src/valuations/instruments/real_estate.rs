//! Real estate asset WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::equity::real_estate::{
    RealEstateAsset, RealEstateValuationMethod,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Valuation method for a real estate asset.
#[wasm_bindgen(js_name = RealEstateValuationMethod)]
#[derive(Clone, Copy)]
pub struct JsRealEstateValuationMethod {
    inner: RealEstateValuationMethod,
}

#[wasm_bindgen(js_class = RealEstateValuationMethod)]
impl JsRealEstateValuationMethod {
    /// Discounted cashflow using an explicit NOI schedule and discount rate.
    #[wasm_bindgen(js_name = Dcf)]
    pub fn dcf() -> JsRealEstateValuationMethod {
        JsRealEstateValuationMethod {
            inner: RealEstateValuationMethod::Dcf,
        }
    }

    /// Direct capitalization using a stabilized NOI and cap rate.
    #[wasm_bindgen(js_name = DirectCap)]
    pub fn direct_cap() -> JsRealEstateValuationMethod {
        JsRealEstateValuationMethod {
            inner: RealEstateValuationMethod::DirectCap,
        }
    }

    /// Check if this is DCF method.
    #[wasm_bindgen(js_name = isDcf)]
    pub fn is_dcf(&self) -> bool {
        matches!(self.inner, RealEstateValuationMethod::Dcf)
    }
}

impl JsRealEstateValuationMethod {
    pub(crate) fn inner(&self) -> RealEstateValuationMethod {
        self.inner
    }
}

/// Real estate asset valuation instrument.
///
/// Supports DCF (explicit NOI schedule) and direct capitalization valuation.
#[wasm_bindgen(js_name = RealEstateAsset)]
#[derive(Clone, Debug)]
pub struct JsRealEstateAsset {
    pub(crate) inner: RealEstateAsset,
}

impl InstrumentWrapper for JsRealEstateAsset {
    type Inner = RealEstateAsset;
    fn from_inner(inner: RealEstateAsset) -> Self {
        JsRealEstateAsset { inner }
    }
    fn inner(&self) -> RealEstateAsset {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RealEstateAsset)]
impl JsRealEstateAsset {
    /// Create a new real estate asset.
    ///
    /// @param {string} instrumentId - Unique identifier
    /// @param {Currency} currency - Valuation currency
    /// @param {FsDate} valuationDate - Base date for discounting
    /// @param {RealEstateValuationMethod} valuationMethod - DCF or DirectCap
    /// @param {Array<[number, number, number, number]>} noiSchedule - Array of [year, month, day, amount] tuples
    /// @param {string} discountCurveId - Discount curve identifier
    /// @param {string} [dayCount] - Day count convention (default: Act365F)
    /// @param {number} [discountRate] - Discount rate for DCF (annualized)
    /// @param {number} [capRate] - Cap rate for direct cap
    /// @param {number} [stabilizedNoi] - Stabilized NOI override for direct cap
    /// @param {number} [terminalCapRate] - Terminal cap rate for DCF
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        currency: &JsCurrency,
        valuation_date: &JsDate,
        valuation_method: &JsRealEstateValuationMethod,
        noi_schedule: Vec<JsValue>,
        discount_curve_id: &str,
        day_count: Option<String>,
        discount_rate: Option<f64>,
        cap_rate: Option<f64>,
        stabilized_noi: Option<f64>,
        terminal_cap_rate: Option<f64>,
    ) -> Result<JsRealEstateAsset, JsValue> {
        let dc = parse_optional_with_default(day_count, DayCount::Act365F)?;

        // Parse NOI schedule from JS arrays [year, month, day, amount]
        let mut schedule = Vec::with_capacity(noi_schedule.len());
        for entry in noi_schedule {
            let arr: js_sys::Array = entry.into();
            if arr.length() != 4 {
                return Err(js_error(
                    "NOI schedule entries must be [year, month, day, amount]".to_string(),
                ));
            }
            let year = arr
                .get(0)
                .as_f64()
                .ok_or_else(|| js_error("Invalid year"))? as i32;
            let month = arr
                .get(1)
                .as_f64()
                .ok_or_else(|| js_error("Invalid month"))? as u8;
            let day = arr.get(2).as_f64().ok_or_else(|| js_error("Invalid day"))? as u8;
            let amount = arr
                .get(3)
                .as_f64()
                .ok_or_else(|| js_error("Invalid amount"))?;

            let date = finstack_core::dates::Date::from_calendar_date(
                year,
                time::Month::try_from(month).map_err(|e| js_error(e.to_string()))?,
                day,
            )
            .map_err(|e| js_error(e.to_string()))?;
            schedule.push((date, amount));
        }

        let mut builder = RealEstateAsset::builder()
            .id(instrument_id_from_str(instrument_id))
            .currency(currency.inner())
            .valuation_date(valuation_date.inner())
            .valuation_method(valuation_method.inner())
            .noi_schedule(schedule)
            .discount_curve_id(curve_id_from_str(discount_curve_id))
            .day_count(dc)
            .attributes(Default::default());

        if let Some(rate) = discount_rate {
            builder = builder.discount_rate(rate);
        }
        if let Some(rate) = cap_rate {
            builder = builder.cap_rate(rate);
        }
        if let Some(noi) = stabilized_noi {
            builder = builder.stabilized_noi(noi);
        }
        if let Some(rate) = terminal_cap_rate {
            builder = builder.terminal_cap_rate(rate);
        }

        builder
            .build()
            .map(JsRealEstateAsset::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the currency.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
    }

    /// Get the valuation date.
    #[wasm_bindgen(getter, js_name = valuationDate)]
    pub fn valuation_date(&self) -> JsDate {
        JsDate::from_core(self.inner.valuation_date)
    }

    /// Get the discount rate (if set).
    #[wasm_bindgen(getter, js_name = discountRate)]
    pub fn discount_rate(&self) -> Option<f64> {
        self.inner.discount_rate
    }

    /// Get the cap rate (if set).
    #[wasm_bindgen(getter, js_name = capRate)]
    pub fn cap_rate(&self) -> Option<f64> {
        self.inner.cap_rate
    }

    /// Get the stabilized NOI (if set).
    #[wasm_bindgen(getter, js_name = stabilizedNoi)]
    pub fn stabilized_noi(&self) -> Option<f64> {
        self.inner.stabilized_noi
    }

    /// Get the appraisal value override (if set).
    #[wasm_bindgen(getter, js_name = appraisalValue)]
    pub fn appraisal_value(&self) -> Option<JsMoney> {
        self.inner.appraisal_value.map(JsMoney::from_inner)
    }

    /// Set an appraisal value override.
    #[wasm_bindgen(js_name = setAppraisalValue)]
    pub fn set_appraisal_value(&mut self, value: &JsMoney) {
        self.inner.appraisal_value = Some(value.inner());
    }

    /// Calculate the NPV.
    pub fn npv(&self, market: &JsMarketContext, as_of: &JsDate) -> Result<JsMoney, JsValue> {
        self.inner
            .npv(market.inner(), as_of.inner())
            .map(JsMoney::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::RealEstateAsset as u16
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsRealEstateAsset, JsValue> {
        from_js_value(value).map(|inner| JsRealEstateAsset { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "RealEstateAsset(id='{}', currency={})",
            self.inner.id, self.inner.currency
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRealEstateAsset {
        JsRealEstateAsset::from_inner(self.inner.clone())
    }
}
