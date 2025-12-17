//! Quote types for calibration in WASM.

use crate::core::common::parse::ParseFromString;
use crate::core::dates::{FsDate, Tenor};
use crate::utils::json::{from_js_value, to_js_value};
use finstack_valuations::calibration::domain::quotes::{
    CreditQuote, FutureSpecs, InflationQuote, InstrumentConventions, MarketQuote, RatesQuote,
    VolQuote,
};
use wasm_bindgen::prelude::*;

/// Future contract specifications.
#[wasm_bindgen(js_name = FutureSpecs)]
#[derive(Clone, Debug)]
pub struct JsFutureSpecs {
    inner: FutureSpecs,
}

impl JsFutureSpecs {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: FutureSpecs) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> FutureSpecs {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FutureSpecs)]
impl JsFutureSpecs {
    /// Create future specifications.
    #[wasm_bindgen(constructor)]
    pub fn new(
        multiplier: f64,
        face_value: f64,
        delivery_months: u8,
        day_count: &str,
    ) -> Result<JsFutureSpecs, JsValue> {
        use finstack_core::dates::DayCount;
        let dc = DayCount::parse_from_string(day_count)?;
        let specs = FutureSpecs {
            multiplier,
            face_value,
            delivery_months,
            day_count: dc,
            convexity_adjustment: None,
            ..Default::default()
        };
        Ok(Self { inner: specs })
    }

    /// With convexity adjustment.
    #[wasm_bindgen(js_name = withConvexityAdjustment)]
    pub fn with_convexity_adjustment(&self, adjustment: f64) -> JsFutureSpecs {
        let mut next = self.inner.clone();
        next.convexity_adjustment = Some(adjustment);
        Self::from_inner(next)
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFutureSpecs, JsValue> {
        from_js_value(value).map(JsFutureSpecs::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Rates market quote.
#[wasm_bindgen(js_name = RatesQuote)]
#[derive(Clone, Debug)]
pub struct JsRatesQuote {
    inner: RatesQuote,
}

impl JsRatesQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: RatesQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> RatesQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RatesQuote)]
impl JsRatesQuote {
    /// Create a deposit quote.
    #[wasm_bindgen(js_name = deposit)]
    pub fn deposit(maturity: &FsDate, rate: f64, day_count: &str) -> Result<JsRatesQuote, JsValue> {
        use finstack_core::dates::DayCount;
        let dc = DayCount::parse_from_string(day_count)?;
        Ok(Self {
            inner: RatesQuote::Deposit {
                maturity: maturity.inner(),
                rate,
                conventions: InstrumentConventions::default().with_day_count(dc),
            },
        })
    }

    /// Create an FRA quote.
    #[wasm_bindgen(js_name = fra)]
    pub fn fra(
        start: &FsDate,
        end: &FsDate,
        rate: f64,
        day_count: &str,
    ) -> Result<JsRatesQuote, JsValue> {
        use finstack_core::dates::DayCount;
        let dc = DayCount::parse_from_string(day_count)?;
        Ok(Self {
            inner: RatesQuote::FRA {
                start: start.inner(),
                end: end.inner(),
                rate,
                conventions: InstrumentConventions::default().with_day_count(dc),
            },
        })
    }

    /// Create a swap quote.
    #[wasm_bindgen(js_name = swap)]
    pub fn swap(
        maturity: &FsDate,
        rate: f64,
        fixed_freq: &Tenor,
        float_freq: &Tenor,
        fixed_dc: &str,
        float_dc: &str,
        index: &str,
    ) -> Result<JsRatesQuote, JsValue> {
        use finstack_core::dates::DayCount;
        let fixed_day_count = DayCount::parse_from_string(fixed_dc)?;
        let float_day_count = DayCount::parse_from_string(float_dc)?;

        Ok(Self {
            inner: RatesQuote::Swap {
                maturity: maturity.inner(),
                rate,
                is_ois: false,
                conventions: Default::default(),
                fixed_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(fixed_freq.inner())
                    .with_day_count(fixed_day_count),
                float_leg_conventions: InstrumentConventions::default()
                    .with_payment_frequency(float_freq.inner())
                    .with_day_count(float_day_count)
                    .with_index(index),
            },
        })
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::Rates(self.inner.clone()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsRatesQuote, JsValue> {
        from_js_value(value).map(JsRatesQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Credit market quote.
#[wasm_bindgen(js_name = CreditQuote)]
#[derive(Clone, Debug)]
pub struct JsCreditQuote {
    inner: CreditQuote,
}

impl JsCreditQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CreditQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CreditQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CreditQuote)]
impl JsCreditQuote {
    /// Create a CDS quote.
    #[wasm_bindgen(js_name = cds)]
    pub fn cds(
        entity: &str,
        maturity: &FsDate,
        spread_bp: f64,
        recovery_rate: f64,
        currency: &str,
    ) -> Result<JsCreditQuote, JsValue> {
        let ccy: finstack_core::currency::Currency = currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", currency)))?;
        Ok(Self {
            inner: CreditQuote::CDS {
                entity: entity.to_string(),
                maturity: maturity.inner(),
                spread_bp,
                recovery_rate,
                currency: ccy,
                conventions: Default::default(),
            },
        })
    }

    /// Create a CDS tranche quote for base correlation calibration.
    ///
    /// @param {string} index - Index identifier (e.g., "CDX.NA.IG.42", "iTraxx.Europe.40")
    /// @param {number} attachment - Attachment point (% of notional, e.g., 0.0 for equity tranche)
    /// @param {number} detachment - Detachment point (% of notional, e.g., 3.0 for 0-3% tranche)
    /// @param {FsDate} maturity - Tranche maturity date
    /// @param {number} upfront_pct - Upfront payment (% of notional)
    /// @param {number} running_spread_bp - Running spread in basis points
    /// @returns {CreditQuote} CDS tranche quote for calibration
    ///
    /// @example
    /// ```javascript
    /// // 0-3% equity tranche
    /// const equityTranche = CreditQuote.cdsTranche(
    ///   "CDX.NA.IG.42", 0.0, 3.0, new FsDate(2029, 6, 20), 35.0, 500.0
    /// );
    /// // 3-7% mezzanine tranche
    /// const mezzTranche = CreditQuote.cdsTranche(
    ///   "CDX.NA.IG.42", 3.0, 7.0, new FsDate(2029, 6, 20), 8.0, 500.0
    /// );
    /// ```
    #[wasm_bindgen(js_name = cdsTranche)]
    pub fn cds_tranche(
        index: &str,
        attachment: f64,
        detachment: f64,
        maturity: &FsDate,
        upfront_pct: f64,
        running_spread_bp: f64,
    ) -> JsCreditQuote {
        Self {
            inner: CreditQuote::CDSTranche {
                index: index.to_string(),
                attachment,
                detachment,
                maturity: maturity.inner(),
                upfront_pct,
                running_spread_bp,
                conventions: Default::default(),
            },
        }
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::Credit(self.inner.clone()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCreditQuote, JsValue> {
        from_js_value(value).map(JsCreditQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Volatility quote.
#[wasm_bindgen(js_name = VolQuote)]
#[derive(Clone, Debug)]
pub struct JsVolQuote {
    inner: VolQuote,
}

impl JsVolQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: VolQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> VolQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = VolQuote)]
impl JsVolQuote {
    /// Create an option vol quote.
    #[wasm_bindgen(js_name = optionVol)]
    pub fn option_vol(
        underlying: &str,
        expiry: &FsDate,
        strike: f64,
        vol: f64,
        option_type: &str,
    ) -> JsVolQuote {
        Self {
            inner: VolQuote::OptionVol {
                underlying: underlying.to_string().into(),
                expiry: expiry.inner(),
                strike,
                vol,
                option_type: option_type.to_string(),
                conventions: Default::default(),
            },
        }
    }

    /// Create a swaption vol quote.
    #[wasm_bindgen(js_name = swaptionVol)]
    pub fn swaption_vol(
        expiry: &FsDate,
        tenor: &FsDate,
        strike: f64,
        vol: f64,
        quote_type: &str,
    ) -> JsVolQuote {
        Self {
            inner: VolQuote::SwaptionVol {
                expiry: expiry.inner(),
                tenor: tenor.inner(),
                strike,
                vol,
                quote_type: quote_type.to_string(),
                conventions: Default::default(),
                fixed_leg_conventions: Default::default(),
                float_leg_conventions: Default::default(),
            },
        }
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::Vol(self.inner.clone()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsVolQuote, JsValue> {
        from_js_value(value).map(JsVolQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Inflation quote.
#[wasm_bindgen(js_name = InflationQuote)]
#[derive(Clone, Debug)]
pub struct JsInflationQuote {
    inner: InflationQuote,
}

impl JsInflationQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: InflationQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> InflationQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InflationQuote)]
impl JsInflationQuote {
    /// Create an inflation swap quote.
    #[wasm_bindgen(js_name = inflationSwap)]
    pub fn inflation_swap(maturity: &FsDate, rate: f64, index: &str) -> JsInflationQuote {
        Self {
            inner: InflationQuote::InflationSwap {
                maturity: maturity.inner(),
                rate,
                index: index.to_string(),
                conventions: Default::default(),
            },
        }
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::Inflation(self.inner.clone()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsInflationQuote, JsValue> {
        from_js_value(value).map(JsInflationQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Polymorphic market quote.
#[wasm_bindgen(js_name = MarketQuote)]
#[derive(Clone, Debug)]
pub struct JsMarketQuote {
    inner: MarketQuote,
}

impl JsMarketQuote {
    pub(crate) fn from_inner(inner: MarketQuote) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = MarketQuote)]
impl JsMarketQuote {
    /// Quote kind (rates, credit, vol, inflation).
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        match &self.inner {
            MarketQuote::Rates(_) => "rates".to_string(),
            MarketQuote::Credit(_) => "credit".to_string(),
            MarketQuote::Vol(_) => "vol".to_string(),
            MarketQuote::Inflation(_) => "inflation".to_string(),
        }
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMarketQuote, JsValue> {
        from_js_value(value).map(JsMarketQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
