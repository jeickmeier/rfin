use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::{
    Attributes, EquityUnderlyingParams, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = EquityOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsEquityOptionBuilder {
    instrument_id: String,
    ticker: Option<String>,
    strike: Option<f64>,
    option_type: Option<String>,
    expiry: Option<finstack_core::dates::Date>,
    notional: Option<Money>,
    exercise_style: Option<String>,
    day_count: Option<String>,
    settlement: Option<String>,
    discount_curve_id: Option<String>,
    spot_id: Option<String>,
    vol_surface_id: Option<String>,
    div_yield_id: Option<Option<String>>,
}

#[wasm_bindgen(js_class = EquityOptionBuilder)]
impl JsEquityOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsEquityOptionBuilder {
        JsEquityOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsEquityOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsEquityOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsEquityOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsEquityOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = notionalAmount)]
    pub fn notional_amount(mut self, notional_amount: f64) -> JsEquityOptionBuilder {
        self.notional = Some(Money::new(notional_amount, Currency::USD));
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsEquityOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = exerciseStyle)]
    pub fn exercise_style(mut self, exercise_style: String) -> JsEquityOptionBuilder {
        self.exercise_style = Some(exercise_style);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: String) -> JsEquityOptionBuilder {
        self.day_count = Some(day_count);
        self
    }

    #[wasm_bindgen(js_name = settlement)]
    pub fn settlement(mut self, settlement: String) -> JsEquityOptionBuilder {
        self.settlement = Some(settlement);
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve_id: String) -> JsEquityOptionBuilder {
        self.discount_curve_id = Some(discount_curve_id);
        self
    }

    #[wasm_bindgen(js_name = spotId)]
    pub fn spot_id(mut self, spot_id: String) -> JsEquityOptionBuilder {
        self.spot_id = Some(spot_id);
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface_id: String) -> JsEquityOptionBuilder {
        self.vol_surface_id = Some(vol_surface_id);
        self
    }

    #[wasm_bindgen(js_name = divYieldId)]
    pub fn div_yield_id(mut self, div_yield_id: Option<String>) -> JsEquityOptionBuilder {
        self.div_yield_id = Some(div_yield_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsEquityOption, JsValue> {
        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| js_error("EquityOptionBuilder: ticker is required".to_string()))?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("EquityOptionBuilder: strike is required".to_string()))?;
        let option_type = self
            .option_type
            .as_deref()
            .ok_or_else(|| js_error("EquityOptionBuilder: optionType is required".to_string()))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("EquityOptionBuilder: expiry is required".to_string()))?;
        let notional = self
            .notional
            .unwrap_or_else(|| Money::new(1.0, Currency::USD));
        let exercise_style = self
            .exercise_style
            .unwrap_or_else(|| "european".to_string())
            .parse::<ExerciseStyle>()
            .map_err(js_error)?;
        let settlement = self
            .settlement
            .unwrap_or_else(|| "cash".to_string())
            .parse::<SettlementType>()
            .map_err(js_error)?;
        let day_count = parse_day_count_name(self.day_count.as_deref().unwrap_or("act_365f"))?;
        let discount_curve_id = self
            .discount_curve_id
            .unwrap_or_else(|| format!("{}-OIS", notional.currency()));
        let spot_id = self.spot_id.unwrap_or_else(|| "EQUITY-SPOT".to_string());
        let vol_surface_id = self
            .vol_surface_id
            .unwrap_or_else(|| "EQUITY-VOL".to_string());
        let div_yield_id = self
            .div_yield_id
            .unwrap_or_else(|| Some("EQUITY-DIVYIELD".to_string()));

        let option_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(js_error(format!(
                    "Invalid optionType '{other}'; expected 'call' or 'put'"
                )));
            }
        };

        let underlying = EquityUnderlyingParams::new(ticker, spot_id.as_str(), notional.currency());
        let underlying = match div_yield_id.as_deref() {
            Some(div_yield_id) => underlying.with_dividend_yield(div_yield_id),
            None => underlying,
        };

        let opt = EquityOption::builder()
            .id(InstrumentId::new(self.instrument_id))
            .underlying_ticker(underlying.ticker)
            .strike(strike)
            .option_type(option_type)
            .exercise_style(exercise_style)
            .expiry(expiry)
            .notional(notional)
            .day_count(day_count)
            .settlement(settlement)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .spot_id(underlying.spot_id)
            .vol_surface_id(CurveId::new(vol_surface_id))
            .div_yield_id_opt(underlying.div_yield_id)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map_err(|e| js_error(e.to_string()))?;
        opt.validate().map_err(|e| js_error(e.to_string()))?;
        Ok(JsEquityOption::from_inner(opt))
    }
}

#[wasm_bindgen(js_name = EquityOption)]
#[derive(Clone, Debug)]
pub struct JsEquityOption {
    pub(crate) inner: EquityOption,
}

impl InstrumentWrapper for JsEquityOption {
    type Inner = EquityOption;
    fn from_inner(inner: EquityOption) -> Self {
        JsEquityOption { inner }
    }
    fn inner(&self) -> EquityOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = EquityOption)]
impl JsEquityOption {
    /// Parse an equity option from a JSON value (as produced by `toJson`).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsEquityOption, JsValue> {
        from_js_value(value).map(JsEquityOption::from_inner)
    }

    /// Serialize this equity option to a JSON value.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
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
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = notionalAmount)]
    pub fn notional_amount(&self) -> f64 {
        self.inner.notional.amount()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::EquityOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "EquityOption(id='{}', ticker='{}')",
            self.inner.id, self.inner.underlying_ticker
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquityOption {
        JsEquityOption::from_inner(self.inner.clone())
    }
}

fn parse_day_count_name(value: &str) -> Result<DayCount, JsValue> {
    match value.to_ascii_lowercase().as_str() {
        "act_360" | "act/360" | "act360" => Ok(DayCount::Act360),
        "act_365f" | "act/365f" | "act365f" => Ok(DayCount::Act365F),
        "act_act" | "act/act" | "actact" => Ok(DayCount::ActAct),
        "thirty_360" | "30/360" | "30e/360" | "30_360" => Ok(DayCount::Thirty360),
        other => Err(js_error(format!("Unsupported day count '{other}'"))),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn builder_supports_market_identifier_overrides() {
        let option = JsEquityOptionBuilder::new("EQOPT-1")
            .ticker("AAPL".to_string())
            .strike(180.0)
            .option_type("call".to_string())
            .expiry(&JsDate::new(2026, 6, 19).expect("valid date"))
            .money(&JsMoney::from_code(100.0, "EUR").expect("valid money"))
            .exercise_style("american".to_string())
            .day_count("act_360".to_string())
            .settlement("physical".to_string())
            .discount_curve("EUR-OIS".to_string())
            .spot_id("AAPL-SPOT".to_string())
            .vol_surface("AAPL-VOL".to_string())
            .div_yield_id(Some("AAPL-DIV".to_string()))
            .build()
            .expect("builder should accept explicit overrides");

        assert_eq!(option.instrument_type(), "equity_option");
        assert_eq!(option.notional_amount(), 100.0);
    }

    #[wasm_bindgen_test]
    fn builder_rejects_unknown_day_count() {
        let err = JsEquityOptionBuilder::new("EQOPT-2")
            .ticker("AAPL".to_string())
            .strike(180.0)
            .option_type("call".to_string())
            .expiry(&JsDate::new(2026, 6, 19).expect("valid date"))
            .day_count("bad_dc".to_string())
            .build()
            .expect_err("invalid day count should error");

        let message = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
            .ok()
            .and_then(|value| value.as_string());
        assert_eq!(message.as_deref(), Some("Unsupported day count 'bad_dc'"));
    }
}
