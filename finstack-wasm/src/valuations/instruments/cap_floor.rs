use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{DayCount, Tenor};
use finstack_valuations::instruments::rates::cap_floor::InterestRateOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn frequency_from_payments(payments_per_year: Option<u32>) -> Result<Tenor, JsValue> {
    let payments = payments_per_year.unwrap_or(4);
    Tenor::from_payments_per_year(payments)
        .map_err(|e| js_error(format!("Invalid payments per year: {e}")))
}

fn extract_day_count(dc: Option<JsDayCount>) -> DayCount {
    dc.map(|d| d.inner()).unwrap_or(DayCount::Act360)
}

/// Interest rate option (cap or floor) on a floating index.
///
/// Use `InterestRateOption.cap(...)` or `InterestRateOption.floor(...)` to construct.
#[wasm_bindgen(js_name = InterestRateOption)]
#[derive(Clone, Debug)]
pub struct JsInterestRateOption(InterestRateOption);

impl InstrumentWrapper for JsInterestRateOption {
    type Inner = InterestRateOption;
    fn from_inner(inner: InterestRateOption) -> Self {
        JsInterestRateOption(inner)
    }
    fn inner(&self) -> InterestRateOption {
        self.0.clone()
    }
}

#[wasm_bindgen(js_name = InterestRateOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsInterestRateOptionBuilder {
    instrument_id: String,
    option_kind: Option<String>,
    notional: Option<finstack_core::money::Money>,
    strike: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    end_date: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    forward_curve: Option<String>,
    vol_surface: Option<String>,
    payments_per_year: Option<u32>,
    day_count: Option<DayCount>,
}

#[wasm_bindgen(js_class = InterestRateOptionBuilder)]
impl JsInterestRateOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsInterestRateOptionBuilder {
        JsInterestRateOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    /// Set option kind: `"cap"` or `"floor"`.
    #[wasm_bindgen(js_name = kind)]
    pub fn kind(mut self, kind: String) -> JsInterestRateOptionBuilder {
        self.option_kind = Some(kind);
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsInterestRateOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsInterestRateOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsInterestRateOptionBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = endDate)]
    pub fn end_date(mut self, end_date: &JsDate) -> JsInterestRateOptionBuilder {
        self.end_date = Some(end_date.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsInterestRateOptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: &str) -> JsInterestRateOptionBuilder {
        self.forward_curve = Some(forward_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsInterestRateOptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = paymentsPerYear)]
    pub fn payments_per_year(mut self, payments_per_year: u32) -> JsInterestRateOptionBuilder {
        self.payments_per_year = Some(payments_per_year);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsInterestRateOptionBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsInterestRateOption, JsValue> {
        let kind = self.option_kind.as_deref().ok_or_else(|| {
            js_error("InterestRateOptionBuilder: kind is required ('cap' or 'floor')".to_string())
        })?;
        let notional = self.notional.ok_or_else(|| {
            js_error("InterestRateOptionBuilder: notional (money) is required".to_string())
        })?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("InterestRateOptionBuilder: strike is required".to_string()))?;
        let start_date = self.start_date.ok_or_else(|| {
            js_error("InterestRateOptionBuilder: startDate is required".to_string())
        })?;
        let end_date = self.end_date.ok_or_else(|| {
            js_error("InterestRateOptionBuilder: endDate is required".to_string())
        })?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateOptionBuilder: discountCurve is required".to_string())
        })?;
        let forward_curve = self.forward_curve.as_deref().ok_or_else(|| {
            js_error("InterestRateOptionBuilder: forwardCurve is required".to_string())
        })?;
        let vol_surface = self.vol_surface.as_deref().ok_or_else(|| {
            js_error("InterestRateOptionBuilder: volSurface is required".to_string())
        })?;

        let freq = frequency_from_payments(self.payments_per_year)?;
        let dc = self.day_count.unwrap_or(DayCount::Act360);
        let vol_surface_id = curve_id_from_str(vol_surface);

        let option = match kind.to_lowercase().as_str() {
            "cap" => InterestRateOption::new_cap(
                instrument_id_from_str(&self.instrument_id),
                notional,
                strike,
                start_date,
                end_date,
                freq,
                dc,
                curve_id_from_str(discount_curve),
                curve_id_from_str(forward_curve),
                vol_surface_id,
            ),
            "floor" => InterestRateOption::new_floor(
                instrument_id_from_str(&self.instrument_id),
                notional,
                strike,
                start_date,
                end_date,
                freq,
                dc,
                curve_id_from_str(discount_curve),
                curve_id_from_str(forward_curve),
                vol_surface_id,
            ),
            other => {
                return Err(js_error(format!(
                    "Invalid kind '{other}'; expected 'cap' or 'floor'"
                )));
            }
        };

        Ok(JsInterestRateOption::from_inner(option))
    }
}

#[wasm_bindgen(js_class = InterestRateOption)]
impl JsInterestRateOption {
    /// Create an interest rate cap.
    ///
    /// Conventions:
    /// - `strike` is a **decimal rate** (e.g. `0.04` for 4%).
    /// - `payments_per_year` defaults to 4 (quarterly) if omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Notional (currency-tagged)
    /// @param strike - Cap strike rate (decimal)
    /// @param start_date - Accrual start date
    /// @param end_date - Accrual end date
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID
    /// @param vol_surface - Vol surface ID
    /// @param payments_per_year - Optional payments per year (frequency)
    /// @param day_count - Optional day count (defaults Act/360)
    /// @returns A new `InterestRateOption` (cap)
    /// @throws {Error} If frequency is invalid
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
        vol_surface: &str,
        payments_per_year: Option<u32>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "InterestRateOption.cap is deprecated; use InterestRateOptionBuilder instead.",
        ));
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count);
        let vol_surface_id = curve_id_from_str(vol_surface);

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
            vol_surface_id,
        );

        Ok(JsInterestRateOption::from_inner(option))
    }

    /// Create an interest rate floor.
    ///
    /// Conventions:
    /// - `strike` is a **decimal rate**.
    /// - `payments_per_year` defaults to 4 (quarterly) if omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Notional (currency-tagged)
    /// @param strike - Floor strike rate (decimal)
    /// @param start_date - Accrual start date
    /// @param end_date - Accrual end date
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID
    /// @param vol_surface - Vol surface ID
    /// @param payments_per_year - Optional payments per year (frequency)
    /// @param day_count - Optional day count (defaults Act/360)
    /// @returns A new `InterestRateOption` (floor)
    /// @throws {Error} If frequency is invalid
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
        vol_surface: &str,
        payments_per_year: Option<u32>,
        day_count: Option<JsDayCount>,
    ) -> Result<JsInterestRateOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "InterestRateOption.floor is deprecated; use InterestRateOptionBuilder instead.",
        ));
        let freq = frequency_from_payments(payments_per_year)?;
        let dc = extract_day_count(day_count);
        let vol_surface_id = curve_id_from_str(vol_surface);

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
            vol_surface_id,
        );

        Ok(JsInterestRateOption::from_inner(option))
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

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.0.start_date)
    }

    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.0.end_date)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.0.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInterestRateOption, JsValue> {
        from_js_value(value).map(JsInterestRateOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.0)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CapFloor as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InterestRateOption(id='{}', strike={:.4})",
            self.0.id, self.0.strike_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInterestRateOption {
        JsInterestRateOption::from_inner(self.0.clone())
    }
}
