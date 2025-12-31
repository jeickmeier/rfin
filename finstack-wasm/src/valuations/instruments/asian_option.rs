use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::exotics::asian_option::{AsianOption, AveragingMethod};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Averaging method for Asian options.
#[wasm_bindgen(js_name = AveragingMethod)]
#[derive(Clone, Copy, Debug)]
pub enum JsAveragingMethod {
    Arithmetic,
    Geometric,
}

impl From<AveragingMethod> for JsAveragingMethod {
    fn from(method: AveragingMethod) -> Self {
        match method {
            AveragingMethod::Arithmetic => JsAveragingMethod::Arithmetic,
            AveragingMethod::Geometric => JsAveragingMethod::Geometric,
        }
    }
}

impl From<JsAveragingMethod> for AveragingMethod {
    fn from(method: JsAveragingMethod) -> Self {
        match method {
            JsAveragingMethod::Arithmetic => AveragingMethod::Arithmetic,
            JsAveragingMethod::Geometric => AveragingMethod::Geometric,
        }
    }
}

#[wasm_bindgen(js_name = AsianOption)]
#[derive(Clone, Debug)]
pub struct JsAsianOption {
    pub(crate) inner: AsianOption,
}

impl InstrumentWrapper for JsAsianOption {
    type Inner = AsianOption;
    fn from_inner(inner: AsianOption) -> Self {
        JsAsianOption { inner }
    }
    fn inner(&self) -> AsianOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = AsianOption)]
impl JsAsianOption {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn builder(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        expiry: &JsDate,
        fixing_dates: Array,
        notional: &JsMoney,
        discount_curve: &str,
        spot_id: &str,
        vol_surface: &str,
        averaging_method: Option<String>,
        option_type: Option<String>,
        div_yield_id: Option<String>,
    ) -> Result<JsAsianOption, JsValue> {
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            let date_str = item
                .as_string()
                .ok_or_else(|| js_error("Fixing dates must be ISO date strings (YYYY-MM-DD)"))?;
            fixing_dates_vec.push(parse_iso_date(&date_str)?);
        }

        let avg_method = match averaging_method.as_deref() {
            None | Some("arithmetic") => AveragingMethod::Arithmetic,
            Some("geometric") => AveragingMethod::Geometric,
            Some(other) => {
                return Err(js_error(format!("Unknown averaging method: {other}")));
            }
        };

        let opt_type = match option_type.as_deref() {
            None | Some("call") => OptionType::Call,
            Some("put") => OptionType::Put,
            Some(other) => {
                return Err(js_error(format!("Unknown option type: {other}")));
            }
        };

        let strike_money = finstack_core::money::Money::new(strike, notional.inner().currency());

        let mut builder = AsianOption::builder();
        builder = builder.id(instrument_id_from_str(instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike_money);
        builder = builder.option_type(opt_type);
        builder = builder.averaging_method(avg_method);
        builder = builder.expiry(expiry.inner());
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.notional(notional.inner());
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.discount_curve_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_surface_id(curve_id_from_str(vol_surface));
        builder = builder.pricing_overrides(Default::default());
        builder = builder.attributes(Default::default());
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(curve_id_from_str(&div));
        }

        builder
            .build()
            .map(JsAsianOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.underlying_ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.strike)
    }

    #[wasm_bindgen(getter, js_name = optionType)]
    pub fn option_type(&self) -> String {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
        .to_string()
    }

    #[wasm_bindgen(getter, js_name = averagingMethod)]
    pub fn averaging_method(&self) -> JsAveragingMethod {
        self.inner.averaging_method.into()
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = fixingDates)]
    pub fn fixing_dates(&self) -> Array {
        let arr = Array::new();
        for date in &self.inner.fixing_dates {
            arr.push(&JsDate::from_core(*date).into());
        }
        arr
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = spotId)]
    pub fn spot_id(&self) -> String {
        self.inner.spot_id.clone()
    }

    #[wasm_bindgen(getter, js_name = volSurface)]
    pub fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dividendYieldId)]
    pub fn div_yield_id(&self) -> Option<String> {
        self.inner
            .div_yield_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::AsianOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "AsianOption(id='{}', ticker='{}', strike={}, expiry={}, averaging_method='{:?}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike.amount(),
            self.inner.expiry,
            self.inner.averaging_method
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsAsianOption {
        JsAsianOption::from_inner(self.inner.clone())
    }
}

fn parse_iso_date(value: &str) -> Result<finstack_core::dates::Date, JsValue> {
    use crate::core::error::js_error;
    use time::Month;

    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(js_error(format!(
            "Date '{value}' must be in ISO format YYYY-MM-DD"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| js_error(format!("Invalid year component in date '{value}'")))?;
    let month: u8 = parts[1]
        .parse()
        .map_err(|_| js_error(format!("Invalid month component in date '{value}'")))?;
    let day: u8 = parts[2]
        .parse()
        .map_err(|_| js_error(format!("Invalid day component in date '{value}'")))?;
    let month_enum = Month::try_from(month)
        .map_err(|_| js_error(format!("Month component must be 1-12 in date '{value}'")))?;
    finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
        .map_err(|e| js_error(format!("Invalid date '{value}': {e}")))
}
