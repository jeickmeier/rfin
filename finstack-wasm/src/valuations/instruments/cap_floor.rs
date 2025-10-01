use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use finstack_core::dates::{DayCount, Frequency};
use finstack_valuations::instruments::cap_floor::InterestRateOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn frequency_from_payments(payments_per_year: Option<u32>) -> Result<Frequency, JsValue> {
    let payments = payments_per_year.unwrap_or(4);
    Frequency::from_payments_per_year(payments)
        .map_err(|e| js_error(format!("Invalid payments per year: {e}")))
}

fn extract_day_count(dc: Option<JsDayCount>) -> DayCount {
    dc.map(|d| d.inner()).unwrap_or(DayCount::Act360)
}

#[wasm_bindgen(js_name = InterestRateOption)]
#[derive(Clone, Debug)]
pub struct JsInterestRateOption {
    inner: InterestRateOption,
}

impl JsInterestRateOption {
    pub(crate) fn from_inner(inner: InterestRateOption) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InterestRateOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InterestRateOption)]
impl JsInterestRateOption {
    #[wasm_bindgen(js_name = cap)]
    #[allow(clippy::too_many_arguments)]
    pub fn cap(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        start_date: &JsDate,
        end_date: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: Option<String>,
        payments_per_year: Option<u32>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateOption, JsValue> {
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count);
        let vol_id = optional_static_str(vol_surface).unwrap_or("IR-CAP-VOL");

        let option = InterestRateOption::new_cap(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            strike,
            start_date.inner(),
            end_date.inner(),
            freq,
            dc,
            curve_id_from_str(discount_curve),
            curve_id_from_str(forward_curve),
            vol_id,
        );

        Ok(JsInterestRateOption::from_inner(option))
    }

    #[wasm_bindgen(js_name = floor)]
    #[allow(clippy::too_many_arguments)]
    pub fn floor(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        start_date: &JsDate,
        end_date: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        vol_surface: Option<String>,
        payments_per_year: Option<u32>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateOption, JsValue> {
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count);
        let vol_id = optional_static_str(vol_surface).unwrap_or("IR-CAP-VOL");

        let option = InterestRateOption::new_floor(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            strike,
            start_date.inner(),
            end_date.inner(),
            freq,
            dc,
            curve_id_from_str(discount_curve),
            curve_id_from_str(forward_curve),
            vol_id,
        );

        Ok(JsInterestRateOption::from_inner(option))
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

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.inner.end_date)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CapFloor as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateOption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateOption {
        JsInterestRateOption::from_inner(self.inner.clone())
    }
}

