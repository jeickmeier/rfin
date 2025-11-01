use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str, parameters::JsBarrierType as JsMcBarrierType};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::barrier_option::{BarrierOption, BarrierType as BarrierOptionType};
use finstack_valuations::instruments::common::mc::payoff::barrier::BarrierType as McBarrierType;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = BarrierOption)]
#[derive(Clone, Debug)]
pub struct JsBarrierOption(BarrierOption);

impl InstrumentWrapper for JsBarrierOption {
    type Inner = BarrierOption;
    fn from_inner(inner: BarrierOption) -> Self {
        JsBarrierOption(inner)
    }
    fn inner(&self) -> BarrierOption {
        self.0.clone()
    }
}

#[wasm_bindgen(js_class = BarrierOption)]
impl JsBarrierOption {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn builder(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        barrier: f64,
        option_type: &str,
        barrier_type: &str,
        expiry: &JsDate,
        notional: &JsMoney,
        discount_curve: &str,
        spot_id: &str,
        vol_surface: &str,
        dividend_yield_id: Option<String>,
        use_gobet_miri: Option<bool>,
    ) -> Result<JsBarrierOption, JsValue> {
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let opt_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(js_error(format!("Unknown option type: {other}")));
            }
        };

        let barrier_type_enum = match barrier_type.to_lowercase().replace('_', "").as_str() {
            "upandout" => BarrierOptionType::UpAndOut,
            "upandin" => BarrierOptionType::UpAndIn,
            "downandout" => BarrierOptionType::DownAndOut,
            "downandin" => BarrierOptionType::DownAndIn,
            other => {
                return Err(js_error(format!("Unknown barrier type: {other}")));
            }
        };

        let strike_money = finstack_core::money::Money::new(strike, notional.inner().currency());
        let barrier_money = finstack_core::money::Money::new(barrier, notional.inner().currency());
        let notional_amount = notional.inner().amount();

        let mut builder = BarrierOption::builder();
        builder = builder.id(instrument_id_from_str(instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike_money);
        builder = builder.barrier(barrier_money);
        builder = builder.option_type(opt_type);
        builder = builder.barrier_type(barrier_type_enum);
        builder = builder.expiry(expiry.inner());
        builder = builder.notional(notional_amount);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.use_gobet_miri(use_gobet_miri.unwrap_or(false));
        builder = builder.disc_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_id(curve_id_from_str(vol_surface));
        if let Some(div) = dividend_yield_id {
            builder = builder.div_yield_id(div);
        }
        builder = builder.pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.attributes(finstack_valuations::instruments::common::traits::Attributes::new());

        builder
            .build()
            .map(JsBarrierOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.0.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.0.underlying_ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> JsMoney {
        JsMoney::from_inner(self.0.strike)
    }

    #[wasm_bindgen(getter)]
    pub fn barrier(&self) -> JsMoney {
        JsMoney::from_inner(self.0.barrier)
    }

    #[wasm_bindgen(getter, js_name = optionType)]
    pub fn option_type(&self) -> String {
        match self.0.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
        .to_string()
    }

    #[wasm_bindgen(getter, js_name = barrierType)]
    pub fn barrier_type(&self) -> JsMcBarrierType {
        // Convert BarrierOptionType to McBarrierType
        let mc_type = match self.0.barrier_type {
            BarrierOptionType::UpAndOut => McBarrierType::UpAndOut,
            BarrierOptionType::UpAndIn => McBarrierType::UpAndIn,
            BarrierOptionType::DownAndOut => McBarrierType::DownAndOut,
            BarrierOptionType::DownAndIn => McBarrierType::DownAndIn,
        };
        JsMcBarrierType::from_inner(mc_type)
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.0.expiry)
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.0.notional
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.0.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = spotId)]
    pub fn spot_id(&self) -> String {
        self.0.spot_id.clone()
    }

    #[wasm_bindgen(getter, js_name = volSurface)]
    pub fn vol_surface(&self) -> String {
        self.0.vol_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dividendYieldId)]
    pub fn dividend_yield_id(&self) -> Option<String> {
        self.0.div_yield_id.clone()
    }

    #[wasm_bindgen(getter, js_name = useGobetMiri)]
    pub fn use_gobet_miri(&self) -> bool {
        self.0.use_gobet_miri
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::BarrierOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "BarrierOption(id='{}', ticker='{}', strike={}, barrier={}, barrier_type='{:?}')",
            self.0.id.as_str(),
            self.0.underlying_ticker,
            self.0.strike.amount(),
            self.0.barrier.amount(),
            self.0.barrier_type
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBarrierOption {
        JsBarrierOption::from_inner(self.0.clone())
    }
}

