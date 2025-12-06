use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, optional_static_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::cds_option::parameters::CdsOptionParams;
use finstack_valuations::instruments::cds_option::CdsOption;
use finstack_valuations::instruments::common::parameters::{CreditParams, OptionType};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CdsOption)]
#[derive(Clone, Debug)]
pub struct JsCdsOption {
    pub(crate) inner: CdsOption,
}

impl InstrumentWrapper for JsCdsOption {
    type Inner = CdsOption;
    fn from_inner(inner: CdsOption) -> Self {
        JsCdsOption { inner }
    }
    fn inner(&self) -> CdsOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CdsOption)]
impl JsCdsOption {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        strike_spread_bp: f64,
        expiry: &JsDate,
        cds_maturity: &JsDate,
        discount_curve: &str,
        credit_curve: &str,
        vol_surface: &str,
        option_type: Option<String>,
        recovery_rate: Option<f64>,
        underlying_is_index: Option<bool>,
        index_factor: Option<f64>,
    ) -> Result<JsCdsOption, JsValue> {
        let option_type_value = parse_optional_with_default(option_type, OptionType::Call)?;
        let recovery = recovery_rate.unwrap_or(0.40);

        if !(0.0..=1.0).contains(&recovery) {
            return Err(js_error(
                "recovery_rate must be between 0 and 1".to_string(),
            ));
        }

        let mut option_params = CdsOptionParams::new(
            strike_spread_bp,
            expiry.inner(),
            cds_maturity.inner(),
            notional.inner(),
            option_type_value,
        );

        if underlying_is_index.unwrap_or(false) {
            let factor = index_factor.unwrap_or(1.0);
            option_params = option_params.as_index(factor);
        }

        let credit = curve_id_from_str(credit_curve);
        let credit_params = CreditParams::new("CDS_OPTION", recovery, credit.clone());

        let disc_str = optional_static_str(Some(discount_curve.to_string()))
            .ok_or_else(|| js_error("discount curve required".to_string()))?;
        let vol_str = optional_static_str(Some(vol_surface.to_string()))
            .ok_or_else(|| js_error("vol surface required".to_string()))?;

        let option = CdsOption::new(
            instrument_id_from_str(instrument_id),
            &option_params,
            &credit_params,
            disc_str,
            vol_str,
        );

        Ok(JsCdsOption::from_inner(option))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = strikeSpreadBp)]
    pub fn strike_spread_bp(&self) -> f64 {
        self.inner.strike_spread_bp
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
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CDSOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CdsOption(id='{}', strike_bp={:.1})",
            self.inner.id, self.inner.strike_spread_bp
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCdsOption {
        JsCdsOption::from_inner(self.inner.clone())
    }
}
