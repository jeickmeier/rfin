use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::swaption::parameters::SwaptionParams;
use finstack_valuations::instruments::swaption::{Swaption, SwaptionExercise, SwaptionSettlement};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

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
        vol_surface: &str,
        exercise: Option<String>,
        settlement: Option<String>,
        fixed_frequency: Option<crate::core::dates::daycount::JsTenor>,
        float_frequency: Option<crate::core::dates::daycount::JsTenor>,
        day_count: Option<crate::core::dates::daycount::JsDayCount>,
        business_day_convention: Option<crate::core::dates::calendar::JsBusinessDayConvention>,
        calendar_id: Option<String>,
    ) -> Result<JsSwaption, JsValue> {
        let vol_surface_id = curve_id_from_str(vol_surface);
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
            vol_surface_id,
        );
        swaption.exercise = exercise_style;
        swaption.settlement = settlement_type;
        if let Some(f) = fixed_frequency {
            swaption.fixed_freq = f.inner();
        }
        if let Some(f) = float_frequency {
            swaption.float_freq = f.inner();
        }
        if let Some(dc) = day_count {
            swaption.day_count = dc.inner();
        }
        if let Some(b) = business_day_convention {
            let bdc: finstack_core::dates::BusinessDayConvention = b.into(); /* used in schedule calculations elsewhere */
            let _ = bdc;
        }
        if let Some(cal) = calendar_id {
            let _ = cal;
        }

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
        vol_surface: &str,
        exercise: Option<String>,
        settlement: Option<String>,
        fixed_frequency: Option<crate::core::dates::daycount::JsTenor>,
        float_frequency: Option<crate::core::dates::daycount::JsTenor>,
        day_count: Option<crate::core::dates::daycount::JsDayCount>,
        business_day_convention: Option<crate::core::dates::calendar::JsBusinessDayConvention>,
        calendar_id: Option<String>,
    ) -> Result<JsSwaption, JsValue> {
        let vol_surface_id = curve_id_from_str(vol_surface);
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
            vol_surface_id,
        );
        swaption.exercise = exercise_style;
        swaption.settlement = settlement_type;
        if let Some(f) = fixed_frequency {
            swaption.fixed_freq = f.inner();
        }
        if let Some(f) = float_frequency {
            swaption.float_freq = f.inner();
        }
        if let Some(dc) = day_count {
            swaption.day_count = dc.inner();
        }
        if let Some(b) = business_day_convention {
            let bdc: finstack_core::dates::BusinessDayConvention = b.into();
            let _ = bdc;
        }
        if let Some(cal) = calendar_id {
            let _ = cal;
        }

        Ok(JsSwaption::from_inner(swaption))
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
        self.inner.strike_rate
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
        self.inner.forward_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Swaption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Swaption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsSwaption {
        JsSwaption::from_inner(self.inner.clone())
    }
}
