//! Inflation cap/floor WASM bindings.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::rates::inflation_cap_floor::{
    InflationCapFloor, InflationCapFloorType,
};
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::prelude::Instrument;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Inflation option type.
#[wasm_bindgen(js_name = InflationCapFloorType)]
#[derive(Clone, Copy)]
pub struct JsInflationCapFloorType {
    inner: InflationCapFloorType,
}

#[wasm_bindgen(js_class = InflationCapFloorType)]
impl JsInflationCapFloorType {
    /// Cap (portfolio of caplets).
    #[wasm_bindgen(js_name = Cap)]
    pub fn cap() -> JsInflationCapFloorType {
        JsInflationCapFloorType {
            inner: InflationCapFloorType::Cap,
        }
    }

    /// Floor (portfolio of floorlets).
    #[wasm_bindgen(js_name = Floor)]
    pub fn floor() -> JsInflationCapFloorType {
        JsInflationCapFloorType {
            inner: InflationCapFloorType::Floor,
        }
    }

    /// Single-period caplet.
    #[wasm_bindgen(js_name = Caplet)]
    pub fn caplet() -> JsInflationCapFloorType {
        JsInflationCapFloorType {
            inner: InflationCapFloorType::Caplet,
        }
    }

    /// Single-period floorlet.
    #[wasm_bindgen(js_name = Floorlet)]
    pub fn floorlet() -> JsInflationCapFloorType {
        JsInflationCapFloorType {
            inner: InflationCapFloorType::Floorlet,
        }
    }

    /// Check if this is a cap type.
    #[wasm_bindgen(js_name = isCap)]
    pub fn is_cap(&self) -> bool {
        matches!(
            self.inner,
            InflationCapFloorType::Cap | InflationCapFloorType::Caplet
        )
    }

    /// Get string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

impl JsInflationCapFloorType {
    pub(crate) fn inner(&self) -> InflationCapFloorType {
        self.inner
    }
}

/// YoY inflation cap/floor instrument.
#[wasm_bindgen(js_name = InflationCapFloor)]
#[derive(Clone, Debug)]
pub struct JsInflationCapFloor {
    pub(crate) inner: InflationCapFloor,
}

impl InstrumentWrapper for JsInflationCapFloor {
    type Inner = InflationCapFloor;
    fn from_inner(inner: InflationCapFloor) -> Self {
        JsInflationCapFloor { inner }
    }
    fn inner(&self) -> InflationCapFloor {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = InflationCapFloorBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInflationCapFloorBuilder {
    instrument_id: String,
    option_type: Option<InflationCapFloorType>,
    notional: Option<finstack_core::money::Money>,
    strike_rate: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    end_date: Option<finstack_core::dates::Date>,
    inflation_index_id: Option<String>,
    discount_curve_id: Option<String>,
    vol_surface_id: Option<String>,
    frequency: Option<String>,
    day_count: Option<String>,
    stub_kind: Option<String>,
    bdc: Option<String>,
    calendar_id: Option<String>,
}

#[wasm_bindgen(js_class = InflationCapFloorBuilder)]
impl JsInflationCapFloorBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInflationCapFloorBuilder {
        JsInflationCapFloorBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(
        mut self,
        option_type: &JsInflationCapFloorType,
    ) -> JsInflationCapFloorBuilder {
        self.option_type = Some(option_type.inner());
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInflationCapFloorBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = strikeRate)]
    pub fn strike_rate(mut self, strike_rate: f64) -> JsInflationCapFloorBuilder {
        self.strike_rate = Some(strike_rate);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsInflationCapFloorBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = endDate)]
    pub fn end_date(mut self, end_date: &JsDate) -> JsInflationCapFloorBuilder {
        self.end_date = Some(end_date.inner());
        self
    }

    #[wasm_bindgen(js_name = inflationIndexId)]
    pub fn inflation_index_id(mut self, inflation_index_id: &str) -> JsInflationCapFloorBuilder {
        self.inflation_index_id = Some(inflation_index_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: &str) -> JsInflationCapFloorBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurfaceId)]
    pub fn vol_surface_id(mut self, vol_surface_id: &str) -> JsInflationCapFloorBuilder {
        self.vol_surface_id = Some(vol_surface_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(mut self, frequency: String) -> JsInflationCapFloorBuilder {
        self.frequency = Some(frequency);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsInflationCapFloorBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = stubKind)]
    pub fn stub_kind(mut self, stub_kind: String) -> JsInflationCapFloorBuilder {
        self.stub_kind = Some(stub_kind);
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn bdc(mut self, bdc: String) -> JsInflationCapFloorBuilder {
        self.bdc = Some(bdc);
        self
    }

    #[wasm_bindgen(js_name = calendarId)]
    pub fn calendar_id(mut self, calendar_id: String) -> JsInflationCapFloorBuilder {
        self.calendar_id = Some(calendar_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInflationCapFloor, JsValue> {
        let option_type = self.option_type.ok_or_else(|| {
            js_error("InflationCapFloorBuilder: optionType is required".to_string())
        })?;
        let notional = self.notional.ok_or_else(|| {
            js_error("InflationCapFloorBuilder: notional (money) is required".to_string())
        })?;
        let strike_rate = self.strike_rate.ok_or_else(|| {
            js_error("InflationCapFloorBuilder: strikeRate is required".to_string())
        })?;
        let start_date = self.start_date.ok_or_else(|| {
            js_error("InflationCapFloorBuilder: startDate is required".to_string())
        })?;
        let end_date = self
            .end_date
            .ok_or_else(|| js_error("InflationCapFloorBuilder: endDate is required".to_string()))?;
        let inflation_index_id = self.inflation_index_id.as_deref().ok_or_else(|| {
            js_error("InflationCapFloorBuilder: inflationIndexId is required".to_string())
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("InflationCapFloorBuilder: discountCurveId is required".to_string())
        })?;
        let vol_surface_id = self.vol_surface_id.as_deref().ok_or_else(|| {
            js_error("InflationCapFloorBuilder: volSurfaceId is required".to_string())
        })?;

        let freq = parse_optional_with_default(self.frequency, Tenor::annual())?;
        let dc = parse_optional_with_default(self.day_count, DayCount::Act365F)?;
        let stub = parse_optional_with_default(self.stub_kind, StubKind::None)?;
        let bdc_value =
            parse_optional_with_default(self.bdc, BusinessDayConvention::ModifiedFollowing)?;

        let mut builder = InflationCapFloor::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .option_type(option_type)
            .notional(notional)
            .strike_rate(strike_rate)
            .start_date(start_date)
            .maturity(end_date)
            .frequency(freq)
            .day_count(dc)
            .stub(stub)
            .bdc(bdc_value)
            .inflation_index_id(curve_id_from_str(inflation_index_id))
            .discount_curve_id(curve_id_from_str(discount_curve_id))
            .vol_surface_id(curve_id_from_str(vol_surface_id))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Default::default());

        if let Some(cal_id) = self.calendar_id {
            builder = builder.calendar_id(cal_id);
        }

        let inner = builder.build().map_err(|e| js_error(e.to_string()))?;
        Ok(JsInflationCapFloor { inner })
    }
}

#[wasm_bindgen(js_class = InflationCapFloor)]
impl JsInflationCapFloor {
    /// Create a new inflation cap/floor.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        option_type: &JsInflationCapFloorType,
        notional: &JsMoney,
        strike_rate: f64,
        start_date: &JsDate,
        end_date: &JsDate,
        inflation_index_id: &str,
        discount_curve_id: &str,
        vol_surface_id: &str,
        frequency: Option<String>,
        day_count: Option<String>,
        stub_kind: Option<String>,
        bdc: Option<String>,
        calendar_id: Option<String>,
    ) -> Result<JsInflationCapFloor, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "InflationCapFloor constructor is deprecated; use InflationCapFloorBuilder instead.",
        ));
        let freq = parse_optional_with_default(frequency, Tenor::annual())?;
        let dc = parse_optional_with_default(day_count, DayCount::Act365F)?;
        let stub = parse_optional_with_default(stub_kind, StubKind::None)?;
        let bdc_value = parse_optional_with_default(bdc, BusinessDayConvention::ModifiedFollowing)?;

        let mut builder = InflationCapFloor::builder()
            .id(instrument_id_from_str(instrument_id))
            .option_type(option_type.inner())
            .notional(notional.inner())
            .strike_rate(strike_rate)
            .start_date(start_date.inner())
            .maturity(end_date.inner())
            .frequency(freq)
            .day_count(dc)
            .stub(stub)
            .bdc(bdc_value)
            .inflation_index_id(curve_id_from_str(inflation_index_id))
            .discount_curve_id(curve_id_from_str(discount_curve_id))
            .vol_surface_id(curve_id_from_str(vol_surface_id))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Default::default());

        if let Some(cal_id) = calendar_id {
            builder = builder.calendar_id(cal_id);
        }

        let inner = builder.build().map_err(|e| js_error(e.to_string()))?;

        Ok(JsInflationCapFloor { inner })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the notional.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    /// Get the strike rate.
    #[wasm_bindgen(getter, js_name = strikeRate)]
    pub fn strike_rate(&self) -> f64 {
        self.inner.strike_rate
    }

    /// Get the start date.
    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    /// Get the end date.
    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    /// Check if this is a cap.
    #[wasm_bindgen(js_name = isCap)]
    pub fn is_cap(&self) -> bool {
        matches!(
            self.inner.option_type,
            InflationCapFloorType::Cap | InflationCapFloorType::Caplet
        )
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
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InflationCapFloor as u16
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInflationCapFloor, JsValue> {
        from_js_value(value).map(|inner| JsInflationCapFloor { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get a cashflow view for this inflation cap/floor.
    ///
    /// Option payoffs depend on realized inflation; this returns an empty schedule placeholder
    /// for API consistency.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationCapFloor(id='{}', type={}, strike={:.4})",
            self.inner.id, self.inner.option_type, self.inner.strike_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationCapFloor {
        JsInflationCapFloor::from_inner(self.inner.clone())
    }
}
