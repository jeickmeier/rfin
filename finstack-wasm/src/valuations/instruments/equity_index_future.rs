//! Equity Index Future WASM bindings.

use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::bond_future::JsFuturePosition;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_index_future::{
    EquityFutureSpecs, EquityIndexFuture,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::prelude::Instrument;
use wasm_bindgen::prelude::*;

/// Contract specifications for equity index futures.
#[wasm_bindgen(js_name = EquityFutureSpecs)]
#[derive(Clone)]
pub struct JsEquityFutureSpecs {
    inner: EquityFutureSpecs,
}

#[wasm_bindgen(js_class = EquityFutureSpecs)]
impl JsEquityFutureSpecs {
    /// E-mini S&P 500 specifications (ES).
    /// - Multiplier: $50 per index point
    /// - Tick size: 0.25 index points
    /// - Tick value: $12.50 per tick
    #[wasm_bindgen(js_name = sp500Emini)]
    pub fn sp500_emini() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::sp500_emini(),
        }
    }

    /// E-mini Nasdaq-100 specifications (NQ).
    /// - Multiplier: $20 per index point
    /// - Tick size: 0.25 index points
    /// - Tick value: $5.00 per tick
    #[wasm_bindgen(js_name = nasdaq100Emini)]
    pub fn nasdaq100_emini() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::nasdaq100_emini(),
        }
    }

    /// Micro E-mini S&P 500 specifications (MES).
    /// - Multiplier: $5 per index point
    /// - Tick size: 0.25 index points
    /// - Tick value: $1.25 per tick
    #[wasm_bindgen(js_name = sp500MicroEmini)]
    pub fn sp500_micro_emini() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::sp500_micro_emini(),
        }
    }

    /// Euro Stoxx 50 specifications (FESX).
    #[wasm_bindgen(js_name = euroStoxx50)]
    pub fn euro_stoxx_50() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::euro_stoxx_50(),
        }
    }

    /// DAX specifications (FDAX).
    pub fn dax() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::dax(),
        }
    }

    /// FTSE 100 specifications.
    #[wasm_bindgen(js_name = ftse100)]
    pub fn ftse_100() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::ftse_100(),
        }
    }

    /// Nikkei 225 specifications.
    #[wasm_bindgen(js_name = nikkei225)]
    pub fn nikkei_225() -> JsEquityFutureSpecs {
        JsEquityFutureSpecs {
            inner: EquityFutureSpecs::nikkei_225(),
        }
    }

    /// Get the contract multiplier.
    #[wasm_bindgen(getter)]
    pub fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    /// Get the tick size.
    #[wasm_bindgen(getter, js_name = tickSize)]
    pub fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    /// Get the tick value.
    #[wasm_bindgen(getter, js_name = tickValue)]
    pub fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }
}

impl JsEquityFutureSpecs {
    pub(crate) fn inner(&self) -> EquityFutureSpecs {
        self.inner.clone()
    }
}

/// Equity index future instrument.
///
/// Represents a futures contract on an equity index such as S&P 500, Nasdaq-100,
/// Euro Stoxx 50, DAX, FTSE 100, or Nikkei 225.
///
/// @example
/// ```javascript
/// const future = new EquityIndexFuture(
///   "ESH5",
///   "SPX",
///   Money.fromCode(2_250_000, "USD"), // Position notional
///   new FsDate(2025, 3, 21),    // Expiry date
///   new FsDate(2025, 3, 20),    // Last trading date
///   FuturePosition.Long(),
///   EquityFutureSpecs.sp500Emini(),
///   "USD-OIS",
///   "SPX-SPOT"
/// );
/// ```
#[wasm_bindgen(js_name = EquityIndexFuture)]
#[derive(Clone)]
pub struct JsEquityIndexFuture {
    inner: EquityIndexFuture,
}

impl JsEquityIndexFuture {
    pub(crate) fn inner(&self) -> EquityIndexFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = EquityIndexFutureBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsEquityIndexFutureBuilder {
    instrument_id: String,
    index_ticker: Option<String>,
    notional: Option<Money>,
    expiry_date: Option<finstack_core::dates::Date>,
    last_trading_date: Option<finstack_core::dates::Date>,
    position: Option<finstack_valuations::instruments::fixed_income::bond_future::Position>,
    specs: Option<EquityFutureSpecs>,
    discount_curve_id: Option<String>,
    spot_id: Option<String>,
}

#[wasm_bindgen(js_class = EquityIndexFutureBuilder)]
impl JsEquityIndexFutureBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsEquityIndexFutureBuilder {
        JsEquityIndexFutureBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = indexTicker)]
    pub fn index_ticker(mut self, index_ticker: String) -> JsEquityIndexFutureBuilder {
        self.index_ticker = Some(index_ticker);
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(
        mut self,
        notional: &crate::core::money::JsMoney,
    ) -> JsEquityIndexFutureBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = expiryDate)]
    pub fn expiry_date(mut self, expiry_date: &FsDate) -> JsEquityIndexFutureBuilder {
        self.expiry_date = Some(expiry_date.inner());
        self
    }

    #[wasm_bindgen(js_name = lastTradingDate)]
    pub fn last_trading_date(mut self, last_trading_date: &FsDate) -> JsEquityIndexFutureBuilder {
        self.last_trading_date = Some(last_trading_date.inner());
        self
    }

    #[wasm_bindgen(js_name = position)]
    pub fn position(mut self, position: &JsFuturePosition) -> JsEquityIndexFutureBuilder {
        self.position = Some(position.inner());
        self
    }

    #[wasm_bindgen(js_name = specs)]
    pub fn specs(mut self, specs: &JsEquityFutureSpecs) -> JsEquityIndexFutureBuilder {
        self.specs = Some(specs.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: String) -> JsEquityIndexFutureBuilder {
        self.discount_curve_id = Some(discount_curve_id);
        self
    }

    #[wasm_bindgen(js_name = spotId)]
    pub fn spot_id(mut self, spot_id: String) -> JsEquityIndexFutureBuilder {
        self.spot_id = Some(spot_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsEquityIndexFuture, JsValue> {
        let index_ticker = self.index_ticker.as_deref().ok_or_else(|| {
            JsValue::from_str("EquityIndexFutureBuilder: indexTicker is required")
        })?;
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("EquityIndexFutureBuilder: notional is required"))?;
        let expiry_date = self
            .expiry_date
            .ok_or_else(|| JsValue::from_str("EquityIndexFutureBuilder: expiryDate is required"))?;
        let last_trading_date = self.last_trading_date.ok_or_else(|| {
            JsValue::from_str("EquityIndexFutureBuilder: lastTradingDate is required")
        })?;
        let position = self
            .position
            .ok_or_else(|| JsValue::from_str("EquityIndexFutureBuilder: position is required"))?;
        let specs = self
            .specs
            .ok_or_else(|| JsValue::from_str("EquityIndexFutureBuilder: specs is required"))?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("EquityIndexFutureBuilder: discountCurveId is required")
        })?;
        let spot_id = self
            .spot_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("EquityIndexFutureBuilder: spotId is required"))?;

        let future = EquityIndexFuture::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .underlying_ticker(index_ticker.to_string())
            .notional(notional)
            .expiry_date(expiry_date)
            .last_trading_date(last_trading_date)
            .position(position)
            .contract_specs(specs)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .spot_id(spot_id.to_string())
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsEquityIndexFuture { inner: future })
    }
}

#[wasm_bindgen(js_class = EquityIndexFuture)]
impl JsEquityIndexFuture {
    /// Create a new equity index future.
    ///
    /// @param {string} id - Instrument identifier (e.g., "ESH5")
    /// @param {string} indexTicker - Index ticker symbol (e.g., "SPX")
    /// @param {Money} notional - Position notional
    /// @param {FsDate} expiryDate - Contract expiry date
    /// @param {FsDate} lastTradingDate - Last trading date
    /// @param {FuturePosition} position - Long or Short
    /// @param {EquityFutureSpecs} specs - Contract specifications
    /// @param {string} discountCurveId - Discount curve ID
    /// @param {string} spotId - Index spot price identifier
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        index_ticker: &str,
        notional: &crate::core::money::JsMoney,
        expiry_date: &FsDate,
        last_trading_date: &FsDate,
        position: &JsFuturePosition,
        specs: &JsEquityFutureSpecs,
        discount_curve_id: &str,
        spot_id: &str,
    ) -> Result<JsEquityIndexFuture, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "EquityIndexFuture constructor is deprecated; use EquityIndexFutureBuilder instead.",
        ));
        let future = EquityIndexFuture::builder()
            .id(InstrumentId::new(id))
            .underlying_ticker(index_ticker.to_string())
            .notional(notional.inner())
            .expiry_date(expiry_date.inner())
            .last_trading_date(last_trading_date.inner())
            .position(position.inner())
            .contract_specs(specs.inner())
            .discount_curve_id(CurveId::new(discount_curve_id))
            .spot_id(spot_id.to_string())
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsEquityIndexFuture { inner: future })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the index ticker.
    #[wasm_bindgen(getter, js_name = indexTicker)]
    pub fn index_ticker(&self) -> String {
        self.inner.underlying_ticker.clone()
    }

    /// Get the configured position notional.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> crate::core::money::JsMoney {
        crate::core::money::JsMoney::from_inner(self.inner.notional)
    }

    /// Get the entry price (if set).
    #[wasm_bindgen(getter, js_name = entryPrice)]
    pub fn entry_price(&self) -> Option<f64> {
        self.inner.entry_price
    }

    /// Get the quoted price (if set).
    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> Option<f64> {
        self.inner.quoted_price
    }

    /// Set the entry price.
    #[wasm_bindgen(js_name = setEntryPrice)]
    pub fn set_entry_price(&mut self, price: f64) {
        self.inner.entry_price = Some(price);
    }

    /// Set the quoted market price.
    #[wasm_bindgen(js_name = setQuotedPrice)]
    pub fn set_quoted_price(&mut self, price: f64) {
        self.inner.quoted_price = Some(price);
    }

    /// Calculate delta exposure (index point sensitivity).
    /// Returns USD P&L change for a 1-point move in the index.
    pub fn delta(&self) -> f64 {
        self.inner.delta()
    }

    /// Calculate notional value at a given price.
    #[wasm_bindgen(js_name = notionalValue)]
    pub fn notional_value(&self, price: f64) -> f64 {
        self.inner.num_contracts(price.max(1e-12)) * price * self.inner.contract_specs.multiplier
    }

    /// Get the fair forward price using cost-of-carry model.
    #[wasm_bindgen(js_name = fairForward)]
    pub fn fair_forward(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .fair_forward(market.inner(), as_of.inner())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate present value.
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .value(market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsEquityIndexFuture, JsValue> {
        from_js_value(value).map(|inner| JsEquityIndexFuture { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
