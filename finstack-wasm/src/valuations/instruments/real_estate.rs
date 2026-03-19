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
    RealEstateAsset, RealEstatePropertyType, RealEstateValuationMethod,
};
use finstack_valuations::prelude::Instrument;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_property_type(s: &str) -> Result<RealEstatePropertyType, JsValue> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "office" => Ok(RealEstatePropertyType::Office),
        "multifamily" | "multi_family" | "multi-family" | "residential" => {
            Ok(RealEstatePropertyType::Multifamily)
        }
        "retail" => Ok(RealEstatePropertyType::Retail),
        "industrial" => Ok(RealEstatePropertyType::Industrial),
        "hospitality" | "hotel" => Ok(RealEstatePropertyType::Hospitality),
        "mixeduse" | "mixed_use" | "mixed-use" => Ok(RealEstatePropertyType::MixedUse),
        "other" => Ok(RealEstatePropertyType::Other),
        other => Err(js_error(format!("Unsupported propertyType '{other}'"))),
    }
}

fn parse_date_amount_schedule(
    label: &str,
    entries: Vec<JsValue>,
) -> Result<Vec<(finstack_core::dates::Date, f64)>, JsValue> {
    let mut schedule = Vec::with_capacity(entries.len());
    for entry in entries {
        let arr: js_sys::Array = entry.into();
        if arr.length() != 4 {
            return Err(js_error(format!(
                "{label} entries must be [year, month, day, amount]"
            )));
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
    Ok(schedule)
}

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

#[wasm_bindgen(js_name = RealEstateAssetBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsRealEstateAssetBuilder {
    instrument_id: String,
    currency: Option<finstack_core::currency::Currency>,
    valuation_date: Option<finstack_core::dates::Date>,
    valuation_method: Option<RealEstateValuationMethod>,
    property_type: Option<String>,
    noi_schedule: Option<Vec<JsValue>>,
    capex_schedule: Option<Vec<JsValue>>,
    discount_curve_id: Option<String>,
    day_count: Option<String>,
    discount_rate: Option<f64>,
    cap_rate: Option<f64>,
    stabilized_noi: Option<f64>,
    terminal_cap_rate: Option<f64>,
    terminal_growth_rate: Option<f64>,
    sale_date: Option<finstack_core::dates::Date>,
    sale_price: Option<finstack_core::money::Money>,
    acquisition_cost: Option<f64>,
    acquisition_costs: Option<Vec<JsValue>>,
    disposition_cost_pct: Option<f64>,
    disposition_costs: Option<Vec<JsValue>>,
    purchase_price: Option<finstack_core::money::Money>,
}

#[wasm_bindgen(js_class = RealEstateAssetBuilder)]
impl JsRealEstateAssetBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsRealEstateAssetBuilder {
        JsRealEstateAssetBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &JsCurrency) -> JsRealEstateAssetBuilder {
        self.currency = Some(currency.inner());
        self
    }

    #[wasm_bindgen(js_name = valuationDate)]
    pub fn valuation_date(mut self, valuation_date: &JsDate) -> JsRealEstateAssetBuilder {
        self.valuation_date = Some(valuation_date.inner());
        self
    }

    #[wasm_bindgen(js_name = valuationMethod)]
    pub fn valuation_method(
        mut self,
        valuation_method: &JsRealEstateValuationMethod,
    ) -> JsRealEstateAssetBuilder {
        self.valuation_method = Some(valuation_method.inner());
        self
    }

    #[wasm_bindgen(js_name = propertyType)]
    pub fn property_type(mut self, property_type: &str) -> JsRealEstateAssetBuilder {
        self.property_type = Some(property_type.to_string());
        self
    }

    #[wasm_bindgen(js_name = noiSchedule)]
    pub fn noi_schedule(mut self, noi_schedule: Vec<JsValue>) -> JsRealEstateAssetBuilder {
        self.noi_schedule = Some(noi_schedule);
        self
    }

    #[wasm_bindgen(js_name = capexSchedule)]
    pub fn capex_schedule(mut self, capex_schedule: Vec<JsValue>) -> JsRealEstateAssetBuilder {
        self.capex_schedule = Some(capex_schedule);
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: &str) -> JsRealEstateAssetBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsRealEstateAssetBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = discountRate)]
    pub fn discount_rate(mut self, discount_rate: f64) -> JsRealEstateAssetBuilder {
        self.discount_rate = Some(discount_rate);
        self
    }

    #[wasm_bindgen(js_name = capRate)]
    pub fn cap_rate(mut self, cap_rate: f64) -> JsRealEstateAssetBuilder {
        self.cap_rate = Some(cap_rate);
        self
    }

    #[wasm_bindgen(js_name = stabilizedNoi)]
    pub fn stabilized_noi(mut self, stabilized_noi: f64) -> JsRealEstateAssetBuilder {
        self.stabilized_noi = Some(stabilized_noi);
        self
    }

    #[wasm_bindgen(js_name = terminalCapRate)]
    pub fn terminal_cap_rate(mut self, terminal_cap_rate: f64) -> JsRealEstateAssetBuilder {
        self.terminal_cap_rate = Some(terminal_cap_rate);
        self
    }

    #[wasm_bindgen(js_name = terminalGrowthRate)]
    pub fn terminal_growth_rate(mut self, terminal_growth_rate: f64) -> JsRealEstateAssetBuilder {
        self.terminal_growth_rate = Some(terminal_growth_rate);
        self
    }

    #[wasm_bindgen(js_name = saleDate)]
    pub fn sale_date(mut self, sale_date: &JsDate) -> JsRealEstateAssetBuilder {
        self.sale_date = Some(sale_date.inner());
        self
    }

    #[wasm_bindgen(js_name = salePrice)]
    pub fn sale_price(mut self, sale_price: &JsMoney) -> JsRealEstateAssetBuilder {
        self.sale_price = Some(sale_price.inner());
        self
    }

    #[wasm_bindgen(js_name = acquisitionCost)]
    pub fn acquisition_cost(mut self, acquisition_cost: f64) -> JsRealEstateAssetBuilder {
        self.acquisition_cost = Some(acquisition_cost);
        self
    }

    #[wasm_bindgen(js_name = acquisitionCosts)]
    pub fn acquisition_costs(
        mut self,
        acquisition_costs: Vec<JsValue>,
    ) -> JsRealEstateAssetBuilder {
        self.acquisition_costs = Some(acquisition_costs);
        self
    }

    #[wasm_bindgen(js_name = dispositionCostPct)]
    pub fn disposition_cost_pct(mut self, disposition_cost_pct: f64) -> JsRealEstateAssetBuilder {
        self.disposition_cost_pct = Some(disposition_cost_pct);
        self
    }

    #[wasm_bindgen(js_name = dispositionCosts)]
    pub fn disposition_costs(
        mut self,
        disposition_costs: Vec<JsValue>,
    ) -> JsRealEstateAssetBuilder {
        self.disposition_costs = Some(disposition_costs);
        self
    }

    #[wasm_bindgen(js_name = purchasePrice)]
    pub fn purchase_price(mut self, purchase_price: &JsMoney) -> JsRealEstateAssetBuilder {
        self.purchase_price = Some(purchase_price.inner());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsRealEstateAsset, JsValue> {
        let currency = self
            .currency
            .ok_or_else(|| js_error("RealEstateAssetBuilder: currency is required".to_string()))?;
        let valuation_date = self.valuation_date.ok_or_else(|| {
            js_error("RealEstateAssetBuilder: valuationDate is required".to_string())
        })?;
        let valuation_method = self.valuation_method.ok_or_else(|| {
            js_error("RealEstateAssetBuilder: valuationMethod is required".to_string())
        })?;
        let noi_schedule = self.noi_schedule.ok_or_else(|| {
            js_error("RealEstateAssetBuilder: noiSchedule is required".to_string())
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("RealEstateAssetBuilder: discountCurveId is required".to_string())
        })?;

        let dc = parse_optional_with_default(self.day_count, DayCount::Act365F)?;

        let schedule = parse_date_amount_schedule("noiSchedule", noi_schedule)?;
        let capex_schedule = if let Some(entries) = self.capex_schedule {
            Some(parse_date_amount_schedule("capexSchedule", entries)?)
        } else {
            None
        };

        let mut builder = RealEstateAsset::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .currency(currency)
            .valuation_date(valuation_date)
            .valuation_method(valuation_method)
            .property_type_opt(match self.property_type {
                Some(s) => Some(parse_property_type(&s)?),
                None => None,
            })
            .noi_schedule(schedule)
            .capex_schedule_opt(capex_schedule)
            .discount_curve_id(curve_id_from_str(discount_curve_id))
            .day_count(dc)
            .attributes(Default::default());

        if let Some(rate) = self.discount_rate {
            builder = builder.discount_rate(rate);
        }
        if let Some(rate) = self.cap_rate {
            builder = builder.cap_rate(rate);
        }
        if let Some(noi) = self.stabilized_noi {
            builder = builder.stabilized_noi(noi);
        }
        if let Some(rate) = self.terminal_cap_rate {
            builder = builder.terminal_cap_rate(rate);
        }
        if let Some(g) = self.terminal_growth_rate {
            builder = builder.terminal_growth_rate(g);
        }
        if let Some(d) = self.sale_date {
            builder = builder.sale_date(d);
        }
        if let Some(px) = self.sale_price {
            builder = builder.sale_price(px);
        }
        if let Some(cost) = self.acquisition_cost {
            builder = builder.acquisition_cost(cost);
        }
        if let Some(costs) = self.acquisition_costs {
            let costs = costs
                .into_iter()
                .map(from_js_value::<finstack_core::money::Money>)
                .collect::<Result<Vec<_>, _>>()?;
            builder = builder.acquisition_costs(costs);
        }
        if let Some(pct) = self.disposition_cost_pct {
            builder = builder.disposition_cost_pct(pct);
        }
        if let Some(costs) = self.disposition_costs {
            let costs = costs
                .into_iter()
                .map(from_js_value::<finstack_core::money::Money>)
                .collect::<Result<Vec<_>, _>>()?;
            builder = builder.disposition_costs(costs);
        }
        if let Some(px) = self.purchase_price {
            builder = builder.purchase_price(px);
        }

        builder
            .build()
            .map(JsRealEstateAsset::from_inner)
            .map_err(|e| js_error(e.to_string()))
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
        web_sys::console::warn_1(&JsValue::from_str(
            "RealEstateAsset constructor is deprecated; use RealEstateAssetBuilder instead.",
        ));
        let dc = parse_optional_with_default(day_count, DayCount::Act365F)?;

        // Parse NOI schedule from JS arrays [year, month, day, amount]
        let schedule = parse_date_amount_schedule("noiSchedule", noi_schedule)?;

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

    /// Calculate present value.
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, market: &JsMarketContext, as_of: &JsDate) -> Result<JsMoney, JsValue> {
        self.inner
            .value(market.inner(), as_of.inner())
            .map(JsMoney::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::RealEstateAsset.to_string()
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
