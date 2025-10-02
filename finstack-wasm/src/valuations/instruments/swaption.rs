use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::swaption::parameters::SwaptionParams;
use finstack_valuations::instruments::swaption::{Swaption, SwaptionExercise, SwaptionSettlement};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Swaption)]
#[derive(Clone, Debug)]
pub struct JsSwaption(Swaption);

impl InstrumentWrapper for JsSwaption {
    type Inner = Swaption;
    fn from_inner(inner: Swaption) -> Self {
        JsSwaption(inner)
    }
    fn inner(&self) -> Swaption {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = Swaption)]
impl JsSwaption {
    #[wasm_bindgen(js_name = payer)]
    #[allow(clippy::too_many_arguments)]
    pub fn payer(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        expiry: &JsDate,
        swap_start: &JsDate,
        swap_end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: Option<String>,
        exercise: Option<String>,
        settlement: Option<String>,
    ) -> Result<JsSwaption, JsValue> {
        let vol_id = optional_static_str(vol_surface).unwrap_or("SWAPTION-VOL");
        let exercise_style = parse_optional_with_default(exercise, SwaptionExercise::European)?;
        let settlement_type =
            parse_optional_with_default(settlement, SwaptionSettlement::Physical)?;

        let params = SwaptionParams::payer(
            notional.inner(),
            strike,
            expiry.inner(),
            swap_start.inner(),
            swap_end.inner(),
        );

        let mut swaption = Swaption::new_payer(
            instrument_id_from_str(instrument_id),
            &params,
            curve_id_from_str(discount_curve),
            curve_id_from_str(forward_curve),
            vol_id,
        );
        swaption.exercise = exercise_style;
        swaption.settlement = settlement_type;

        Ok(JsSwaption::from_inner(swaption))
    }

    #[wasm_bindgen(js_name = receiver)]
    #[allow(clippy::too_many_arguments)]
    pub fn receiver(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        expiry: &JsDate,
        swap_start: &JsDate,
        swap_end: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: Option<String>,
        exercise: Option<String>,
        settlement: Option<String>,
    ) -> Result<JsSwaption, JsValue> {
        let vol_id = optional_static_str(vol_surface).unwrap_or("SWAPTION-VOL");
        let exercise_style = parse_optional_with_default(exercise, SwaptionExercise::European)?;
        let settlement_type =
            parse_optional_with_default(settlement, SwaptionSettlement::Physical)?;

        let params = SwaptionParams::receiver(
            notional.inner(),
            strike,
            expiry.inner(),
            swap_start.inner(),
            swap_end.inner(),
        );

        let mut swaption = Swaption::new_receiver(
            instrument_id_from_str(instrument_id),
            &params,
            curve_id_from_str(discount_curve),
            curve_id_from_str(forward_curve),
            vol_id,
        );
        swaption.exercise = exercise_style;
        swaption.settlement = settlement_type;

        Ok(JsSwaption::from_inner(swaption))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.0.notional)
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.0.strike_rate
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.0.expiry)
    }

    #[wasm_bindgen(getter, js_name = swapStart)]
    pub fn swap_start(&self) -> JsDate {
        JsDate::from_core(self.0.swap_start)
    }

    #[wasm_bindgen(getter, js_name = swapEnd)]
    pub fn swap_end(&self) -> JsDate {
        JsDate::from_core(self.0.swap_end)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.0.forward_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Swaption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Swaption(id='{}', strike={:.4})",
            self.0.id, self.0.strike_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsSwaption {
        JsSwaption::from_inner(self.0.clone())
    }
}
