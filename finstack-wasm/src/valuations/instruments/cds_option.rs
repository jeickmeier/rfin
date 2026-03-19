use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOption;
use finstack_valuations::instruments::credit_derivatives::cds_option::CDSOptionParams;
use finstack_valuations::instruments::{CreditParams, OptionType};
use finstack_valuations::pricer::InstrumentType;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CdsOption)]
#[derive(Clone, Debug)]
pub struct JsCDSOption {
    pub(crate) inner: CDSOption,
}

impl InstrumentWrapper for JsCDSOption {
    type Inner = CDSOption;
    fn from_inner(inner: CDSOption) -> Self {
        JsCDSOption { inner }
    }
    fn inner(&self) -> CDSOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = CdsOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCDSOptionBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    strike: Option<f64>,
    expiry: Option<finstack_core::dates::Date>,
    cds_maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    vol_surface: Option<String>,
    option_type: Option<String>,
    recovery_rate: Option<f64>,
    underlying_is_index: Option<bool>,
    index_factor: Option<f64>,
}

#[wasm_bindgen(js_class = CDSOptionBuilder)]
impl JsCDSOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCDSOptionBuilder {
        JsCDSOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsCDSOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    /// Set strike spread as decimal rate (e.g., 0.015 for 150bp).
    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsCDSOptionBuilder {
        self.strike = Some(strike);
        self
    }

    /// Set strike spread in basis points (deprecated, use `strike` with decimal).
    #[wasm_bindgen(js_name = strikeSpreadBp)]
    pub fn strike_spread_bp(mut self, strike_spread_bp: f64) -> JsCDSOptionBuilder {
        self.strike = Some(strike_spread_bp / 10000.0);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsCDSOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = cdsMaturity)]
    pub fn cds_maturity(mut self, cds_maturity: &JsDate) -> JsCDSOptionBuilder {
        self.cds_maturity = Some(cds_maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsCDSOptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = creditCurve)]
    pub fn credit_curve(mut self, credit_curve: &str) -> JsCDSOptionBuilder {
        self.credit_curve = Some(credit_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsCDSOptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsCDSOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = recoveryRate)]
    pub fn recovery_rate(mut self, recovery_rate: f64) -> JsCDSOptionBuilder {
        self.recovery_rate = Some(recovery_rate);
        self
    }

    #[wasm_bindgen(js_name = underlyingIsIndex)]
    pub fn underlying_is_index(mut self, underlying_is_index: bool) -> JsCDSOptionBuilder {
        self.underlying_is_index = Some(underlying_is_index);
        self
    }

    #[wasm_bindgen(js_name = indexFactor)]
    pub fn index_factor(mut self, index_factor: f64) -> JsCDSOptionBuilder {
        self.index_factor = Some(index_factor);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCDSOption, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("CDSOptionBuilder: notional (money) is required".to_string())
        })?;
        let strike_f64 = self
            .strike
            .ok_or_else(|| js_error("CDSOptionBuilder: strike is required".to_string()))?;
        let strike = Decimal::try_from(strike_f64)
            .map_err(|e| js_error(format!("Invalid strike value: {}", e)))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("CDSOptionBuilder: expiry is required".to_string()))?;
        let cds_maturity = self
            .cds_maturity
            .ok_or_else(|| js_error("CDSOptionBuilder: cdsMaturity is required".to_string()))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("CDSOptionBuilder: discountCurve is required".to_string()))?;
        let credit_curve = self
            .credit_curve
            .as_deref()
            .ok_or_else(|| js_error("CDSOptionBuilder: creditCurve is required".to_string()))?;
        let vol_surface = self
            .vol_surface
            .as_deref()
            .ok_or_else(|| js_error("CDSOptionBuilder: volSurface is required".to_string()))?;

        let option_type_value = parse_optional_with_default(self.option_type, OptionType::Call)?;
        let recovery = self.recovery_rate.unwrap_or(0.40);
        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let mut option_params =
            CDSOptionParams::new(strike, expiry, cds_maturity, notional, option_type_value)
                .map_err(|e| js_error(e.to_string()))?;

        if self.underlying_is_index.unwrap_or(false) {
            let factor = self.index_factor.unwrap_or(1.0);
            option_params = option_params
                .as_index(factor)
                .map_err(|e| js_error(e.to_string()))?;
        }

        let credit = curve_id_from_str(credit_curve);
        let credit_params = CreditParams::new("CDS_OPTION", recovery, credit.clone());

        let disc_str = optional_static_str(Some(discount_curve.to_string()))
            .ok_or_else(|| js_error("discount curve required".to_string()))?;
        let vol_str = optional_static_str(Some(vol_surface.to_string()))
            .ok_or_else(|| js_error("vol surface required".to_string()))?;

        let option = CDSOption::new(
            instrument_id_from_str(&self.instrument_id),
            &option_params,
            &credit_params,
            disc_str,
            vol_str,
        )
        .map_err(|e| js_error(e.to_string()))?;

        Ok(JsCDSOption::from_inner(option))
    }
}

#[wasm_bindgen(js_class = CDSOption)]
impl JsCDSOption {
    /// Create an option on a CDS spread (CDS option).
    ///
    /// Conventions:
    /// - `strike` is a **decimal rate** (e.g., 0.01 = 100bp).
    /// - `recovery_rate` is in **decimal** (0..1).
    /// - `option_type` defaults to `"call"` if omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Option notional (currency-tagged)
    /// @param strike - Strike spread as decimal rate
    /// @param expiry - Option expiry date
    /// @param cds_maturity - Underlying CDS maturity date
    /// @param discount_curve - Discount curve ID
    /// @param credit_curve - Credit/hazard curve ID
    /// @param vol_surface - Vol surface ID
    /// @param option_type - Optional `"call"`/`"put"`
    /// @param recovery_rate - Optional recovery override (0..1)
    /// @param underlying_is_index - Optional: treat underlying as index
    /// @param index_factor - Optional index factor (if underlying is index)
    /// @returns A new `CDSOption`
    /// @throws {Error} If recovery is outside [0,1] or inputs invalid
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        strike: f64,
        expiry: &JsDate,
        cds_maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        vol_surface: &str,
        option_type: Option<String>,
        recovery_rate: Option<f64>,
        underlying_is_index: Option<bool>,
        index_factor: Option<f64>,
    ) -> Result<JsCDSOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "CDSOption constructor is deprecated; use CDSOptionBuilder instead.",
        ));
        let option_type_value = parse_optional_with_default(option_type, OptionType::Call)?;
        let recovery = recovery_rate.unwrap_or(0.40);

        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let strike_decimal = Decimal::try_from(strike)
            .map_err(|e| js_error(format!("Invalid strike value: {}", e)))?;

        let mut option_params = CDSOptionParams::new(
            strike_decimal,
            expiry.inner(),
            cds_maturity.inner(),
            notional.inner(),
            option_type_value,
        )
        .map_err(|e| js_error(e.to_string()))?;

        if underlying_is_index.unwrap_or(false) {
            let factor = index_factor.unwrap_or(1.0);
            option_params = option_params
                .as_index(factor)
                .map_err(|e| js_error(e.to_string()))?;
        }

        let credit = curve_id_from_str(credit_curve);
        let credit_params = CreditParams::new("CDS_OPTION", recovery, credit.clone());

        let disc_str = optional_static_str(Some(discount_curve.to_string()))
            .ok_or_else(|| js_error("discount curve required".to_string()))?;
        let vol_str = optional_static_str(Some(vol_surface.to_string()))
            .ok_or_else(|| js_error("vol surface required".to_string()))?;

        let option = CDSOption::new(
            instrument_id_from_str(instrument_id),
            &option_params,
            &credit_params,
            disc_str,
            vol_str,
        )
        .map_err(|e| js_error(e.to_string()))?;

        Ok(JsCDSOption::from_inner(option))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCDSOption, JsValue> {
        from_js_value(value).map(JsCDSOption::from_inner)
    }

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

    /// Strike spread as decimal rate.
    #[wasm_bindgen(getter, js_name = strike)]
    pub fn strike(&self) -> f64 {
        self.inner.strike.to_f64().unwrap_or(0.0)
    }

    /// Strike spread in basis points (backward-compatible alias).
    #[wasm_bindgen(getter, js_name = strikeSpreadBp)]
    pub fn strike_spread_bp(&self) -> f64 {
        self.inner.strike.to_f64().unwrap_or(0.0) * 10000.0
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = cdsMaturity)]
    pub fn cds_maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.cds_maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CDSOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CDSOption(id='{}', strike={})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCDSOption {
        JsCDSOption::from_inner(self.inner.clone())
    }
}
