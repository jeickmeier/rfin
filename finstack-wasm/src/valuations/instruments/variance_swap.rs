use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsTenor;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_core::math::stats::RealizedVarMethod;
use finstack_valuations::instruments::equity::variance_swap::{
    PayReceive as VarSwapPayReceive, VarianceSwap,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Realized variance calculation method for variance swaps.
#[wasm_bindgen(js_name = RealizedVarMethod)]
#[derive(Clone, Copy, Debug)]
pub enum JsRealizedVarMethod {
    /// Close-to-close method (default, simplest)
    CloseToClose,
    /// Parkinson's method (uses high-low range)
    Parkinson,
    /// Garman-Klass method (uses OHLC)
    GarmanKlass,
    /// Rogers-Satchell method (uses OHLC)
    RogersSatchell,
    /// Yang-Zhang method (most accurate, uses OHLC)
    YangZhang,
}

impl From<RealizedVarMethod> for JsRealizedVarMethod {
    fn from(method: RealizedVarMethod) -> Self {
        match method {
            RealizedVarMethod::CloseToClose => JsRealizedVarMethod::CloseToClose,
            RealizedVarMethod::Parkinson => JsRealizedVarMethod::Parkinson,
            RealizedVarMethod::GarmanKlass => JsRealizedVarMethod::GarmanKlass,
            RealizedVarMethod::RogersSatchell => JsRealizedVarMethod::RogersSatchell,
            RealizedVarMethod::YangZhang => JsRealizedVarMethod::YangZhang,
        }
    }
}

impl From<JsRealizedVarMethod> for RealizedVarMethod {
    fn from(method: JsRealizedVarMethod) -> Self {
        match method {
            JsRealizedVarMethod::CloseToClose => RealizedVarMethod::CloseToClose,
            JsRealizedVarMethod::Parkinson => RealizedVarMethod::Parkinson,
            JsRealizedVarMethod::GarmanKlass => RealizedVarMethod::GarmanKlass,
            JsRealizedVarMethod::RogersSatchell => RealizedVarMethod::RogersSatchell,
            JsRealizedVarMethod::YangZhang => RealizedVarMethod::YangZhang,
        }
    }
}

#[wasm_bindgen(js_name = VarianceSwap)]
#[derive(Clone, Debug)]
pub struct JsVarianceSwap {
    pub(crate) inner: VarianceSwap,
}

impl InstrumentWrapper for JsVarianceSwap {
    type Inner = VarianceSwap;
    fn from_inner(inner: VarianceSwap) -> Self {
        JsVarianceSwap { inner }
    }
    fn inner(&self) -> VarianceSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = VarianceSwap)]
impl JsVarianceSwap {
    /// Create a variance swap.
    ///
    /// Conventions:
    /// - `strike_variance` is the variance strike (non-negative).
    /// - `observation_frequency` controls sampling frequency for realized variance.
    /// - `side` defaults to receiving variance if omitted.
    ///
    /// @param instrument_id - Unique identifier
    /// @param underlying_id - Underlying identifier (used to look up spot/returns)
    /// @param notional - Notional (currency-tagged)
    /// @param strike_variance - Strike variance (non-negative)
    /// @param start_date - Observation start date
    /// @param maturity - Observation end/maturity date
    /// @param discount_curve - Discount curve ID
    /// @param observation_frequency - Sampling frequency (Tenor)
    /// @param realized_method - Optional realized variance method string
    /// @param side - Optional pay/receive side string
    /// @returns A new `VarianceSwap`
    /// @throws {Error} If inputs are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { VarianceSwap, Money, FsDate, Tenor } from "finstack-wasm";
    ///
    /// await init();
    /// const vs = new VarianceSwap(
    ///   "vs_1",
    ///   "AAPL",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   0.04,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2025, 1, 2),
    ///   "USD-OIS",
    ///   Tenor.daily(),
    ///   "close_to_close",
    ///   "receive"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        underlying_id: &str,
        notional: &JsMoney,
        strike_variance: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        observation_frequency: &JsTenor,
        realized_method: Option<String>,
        side: Option<String>,
    ) -> Result<JsVarianceSwap, JsValue> {
        if strike_variance < 0.0 {
            return Err(js_error("Strike variance must be non-negative".to_string()));
        }

        if maturity.inner() <= start_date.inner() {
            return Err(js_error(
                "Maturity must be after observation start".to_string(),
            ));
        }

        let method = parse_optional_with_default(realized_method, RealizedVarMethod::CloseToClose)?;
        let direction = parse_optional_with_default(side, VarSwapPayReceive::Receive)?;

        let swap = VarianceSwap {
            id: instrument_id_from_str(instrument_id),
            underlying_id: underlying_id.to_string(),
            notional: notional.inner(),
            strike_variance,
            start_date: start_date.inner(),
            maturity: maturity.inner(),
            observation_freq: observation_frequency.inner(),
            realized_var_method: method,
            side: direction,
            discount_curve_id: curve_id_from_str(discount_curve),
            day_count: DayCount::Act365F,
            attributes: Attributes::new(),
        };

        Ok(JsVarianceSwap::from_inner(swap))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = strikeVariance)]
    pub fn strike_variance(&self) -> f64 {
        self.inner.strike_variance
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::VarianceSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "VarianceSwap(id='{}', strike_var={})",
            self.inner.id, self.inner.strike_variance
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsVarianceSwap {
        JsVarianceSwap::from_inner(self.inner.clone())
    }
}
