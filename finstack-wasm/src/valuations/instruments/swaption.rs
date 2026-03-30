use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::{JsDayCount, JsTenor};
use crate::core::money::JsMoney;
use crate::utils::decimal::decimal_to_f64_or_warn;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::swaption::SwaptionParams;
use finstack_valuations::instruments::rates::swaption::{
    Swaption, SwaptionExercise, SwaptionSettlement,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = SwaptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsSwaptionBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    strike: Option<f64>,
    swaption_type: Option<String>,
    expiry: Option<finstack_core::dates::Date>,
    swap_start: Option<finstack_core::dates::Date>,
    swap_end: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    forward_curve: Option<String>,
    vol_surface: Option<String>,
    exercise: Option<String>,
    settlement: Option<String>,
    fixed_frequency: Option<finstack_core::dates::Tenor>,
    float_frequency: Option<finstack_core::dates::Tenor>,
    day_count: Option<finstack_core::dates::DayCount>,
    business_day_convention: Option<finstack_core::dates::BusinessDayConvention>,
    calendar_id: Option<String>,
}

#[wasm_bindgen(js_class = SwaptionBuilder)]
impl JsSwaptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsSwaptionBuilder {
        JsSwaptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsSwaptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsSwaptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = swaptionType)]
    pub fn swaption_type(mut self, swaption_type: String) -> JsSwaptionBuilder {
        self.swaption_type = Some(swaption_type);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsSwaptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = swapStart)]
    pub fn swap_start(mut self, swap_start: &JsDate) -> JsSwaptionBuilder {
        self.swap_start = Some(swap_start.inner());
        self
    }

    #[wasm_bindgen(js_name = swapEnd)]
    pub fn swap_end(mut self, swap_end: &JsDate) -> JsSwaptionBuilder {
        self.swap_end = Some(swap_end.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsSwaptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: &str) -> JsSwaptionBuilder {
        self.forward_curve = Some(forward_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsSwaptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = exercise)]
    pub fn exercise(mut self, exercise: String) -> JsSwaptionBuilder {
        self.exercise = Some(exercise);
        self
    }

    #[wasm_bindgen(js_name = settlement)]
    pub fn settlement(mut self, settlement: String) -> JsSwaptionBuilder {
        self.settlement = Some(settlement);
        self
    }

    #[wasm_bindgen(js_name = fixedFrequency)]
    pub fn fixed_frequency(mut self, fixed_frequency: JsTenor) -> JsSwaptionBuilder {
        self.fixed_frequency = Some(fixed_frequency.inner());
        self
    }

    #[wasm_bindgen(js_name = floatFrequency)]
    pub fn float_frequency(mut self, float_frequency: JsTenor) -> JsSwaptionBuilder {
        self.float_frequency = Some(float_frequency.inner());
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsSwaptionBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn business_day_convention(
        mut self,
        business_day_convention: JsBusinessDayConvention,
    ) -> JsSwaptionBuilder {
        self.business_day_convention = Some(business_day_convention.into());
        self
    }

    #[wasm_bindgen(js_name = calendarId)]
    pub fn calendar_id(mut self, calendar_id: String) -> JsSwaptionBuilder {
        self.calendar_id = Some(calendar_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsSwaption, JsValue> {
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: notional (money) is required"))?;
        let strike = self
            .strike
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: strike is required"))?;
        let swaption_type = self
            .swaption_type
            .as_deref()
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: swaptionType is required"))?;
        let expiry = self
            .expiry
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: expiry is required"))?;
        let swap_start = self
            .swap_start
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: swapStart is required"))?;
        let swap_end = self
            .swap_end
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: swapEnd is required"))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: discountCurve is required"))?;
        let forward_curve = self
            .forward_curve
            .as_deref()
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: forwardCurve is required"))?;
        let vol_surface = self
            .vol_surface
            .as_deref()
            .ok_or_else(|| JsValue::from_str("SwaptionBuilder: volSurface is required"))?;

        let vol_surface_id = curve_id_from_str(vol_surface);
        let exercise_style =
            parse_optional_with_default(self.exercise, SwaptionExercise::European)?;
        let settlement_type =
            parse_optional_with_default(self.settlement, SwaptionSettlement::Physical)?;

        let params = match swaption_type.to_lowercase().as_str() {
            "payer" => SwaptionParams::payer(notional, strike, expiry, swap_start, swap_end),
            "receiver" => SwaptionParams::receiver(notional, strike, expiry, swap_start, swap_end),
            other => {
                return Err(JsValue::from_str(&format!(
                    "Invalid swaption_type '{other}'; expected 'payer' or 'receiver'"
                )));
            }
        };

        let mut swaption = match swaption_type.to_lowercase().as_str() {
            "payer" => Swaption::new_payer(
                instrument_id_from_str(&self.instrument_id),
                &params,
                curve_id_from_str(discount_curve),
                curve_id_from_str(forward_curve),
                vol_surface_id,
            ),
            "receiver" => Swaption::new_receiver(
                instrument_id_from_str(&self.instrument_id),
                &params,
                curve_id_from_str(discount_curve),
                curve_id_from_str(forward_curve),
                vol_surface_id,
            ),
            other => {
                return Err(JsValue::from_str(&format!(
                    "Invalid swaption_type '{other}'; expected 'payer' or 'receiver'"
                )));
            }
        };

        swaption.exercise_style = exercise_style;
        swaption.settlement = settlement_type;
        if let Some(f) = self.fixed_frequency {
            swaption.fixed_freq = f;
        }
        if let Some(f) = self.float_frequency {
            swaption.float_freq = f;
        }
        if let Some(dc) = self.day_count {
            swaption.day_count = dc;
        }
        if let Some(_bdc) = self.business_day_convention {
            // kept for API symmetry; schedule application handled inside pricing layer
        }
        if let Some(_cal) = self.calendar_id {
            // kept for API symmetry; schedule application handled inside pricing layer
        }

        Ok(JsSwaption::from_inner(swaption))
    }
}

#[wasm_bindgen(js_name = Swaption)]
#[derive(Clone, Debug)]
pub struct JsSwaption {
    pub(crate) inner: Swaption,
}

impl InstrumentWrapper for JsSwaption {
    type Inner = Swaption;
    fn from_inner(inner: Swaption) -> Self {
        JsSwaption { inner }
    }
    fn inner(&self) -> Swaption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Swaption)]
impl JsSwaption {
    /// Parse a swaption from a JSON value (as produced by `toJson`).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsSwaption, JsValue> {
        from_js_value(value).map(JsSwaption::from_inner)
    }

    /// Serialize this swaption to a JSON value.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        decimal_to_f64_or_warn(&self.inner.strike, "strike")
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = swapStart)]
    pub fn swap_start(&self) -> JsDate {
        JsDate::from_core(self.inner.swap_start)
    }

    #[wasm_bindgen(getter, js_name = swapEnd)]
    pub fn swap_end(&self) -> JsDate {
        JsDate::from_core(self.inner.swap_end)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::Swaption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Swaption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsSwaption {
        JsSwaption::from_inner(self.inner.clone())
    }
}
