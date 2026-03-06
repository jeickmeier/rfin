//! Quote types for calibration in WASM.

use crate::core::dates::FsDate;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, IndexId, InflationSwapConventionId, SwaptionConventionId,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::ids::{Pillar, QuoteId};
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use wasm_bindgen::prelude::*;

/// Rates market quote.
#[wasm_bindgen(js_name = RatesQuote)]
#[derive(Clone, Debug)]
pub struct JsRatesQuote {
    inner: RateQuote,
}

impl JsRatesQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: RateQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> RateQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RatesQuote)]
impl JsRatesQuote {
    /// Create a deposit quote.
    #[wasm_bindgen(js_name = deposit)]
    pub fn deposit(id: &str, index: &str, maturity: &FsDate, rate: f64) -> JsRatesQuote {
        Self {
            inner: RateQuote::Deposit {
                id: QuoteId::new(id),
                index: IndexId::new(index),
                pillar: Pillar::Date(maturity.inner()),
                rate,
            },
        }
    }

    /// Create an FRA quote.
    #[wasm_bindgen(js_name = fra)]
    pub fn fra(id: &str, index: &str, start: &FsDate, end: &FsDate, rate: f64) -> JsRatesQuote {
        Self {
            inner: RateQuote::Fra {
                id: QuoteId::new(id),
                index: IndexId::new(index),
                start: Pillar::Date(start.inner()),
                end: Pillar::Date(end.inner()),
                rate,
            },
        }
    }

    /// Create a swap quote without spread.
    ///
    /// @param {string} id - Quote identifier
    /// @param {string} index - Rate index (e.g., "USD-SOFR-3M")
    /// @param {FsDate} maturity - Swap maturity date
    /// @param {number} rate - Par swap rate in decimal (e.g., 0.05 for 5%)
    /// @returns {JsRatesQuote} Swap quote
    #[wasm_bindgen(js_name = swap)]
    pub fn swap(id: &str, index: &str, maturity: &FsDate, rate: f64) -> JsRatesQuote {
        Self {
            inner: RateQuote::Swap {
                id: QuoteId::new(id),
                index: IndexId::new(index),
                pillar: Pillar::Date(maturity.inner()),
                rate,
                spread_decimal: None,
            },
        }
    }

    /// Create a swap quote with spread.
    ///
    /// @param {string} id - Quote identifier
    /// @param {string} index - Rate index (e.g., "USD-SOFR-3M")
    /// @param {FsDate} maturity - Swap maturity date
    /// @param {number} rate - Par swap rate in decimal (e.g., 0.05 for 5%)
    /// @param {number} spreadDecimal - Spread in decimal (e.g., 0.0010 for 10bp)
    /// @returns {JsRatesQuote} Swap quote with spread
    ///
    /// @example
    /// ```typescript
    /// // 5Y swap at 5% with 10bp spread:
    /// const quote = JsRatesQuote.swapWithSpread(
    ///   "swap_5y",
    ///   "USD-SOFR-3M",
    ///   new FsDate(2030, 1, 15),
    ///   0.05,
    ///   0.0010  // 10bp in decimal
    /// );
    /// ```
    #[wasm_bindgen(js_name = swapWithSpread)]
    pub fn swap_with_spread(
        id: &str,
        index: &str,
        maturity: &FsDate,
        rate: f64,
        spread_decimal: f64,
    ) -> JsRatesQuote {
        Self {
            inner: RateQuote::Swap {
                id: QuoteId::new(id),
                index: IndexId::new(index),
                pillar: Pillar::Date(maturity.inner()),
                rate,
                spread_decimal: Some(spread_decimal),
            },
        }
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
    inner: CdsQuote,
}

impl JsCreditQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CdsQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CdsQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CreditQuote)]
impl JsCreditQuote {
    /// Create a CDS par spread quote.
    #[wasm_bindgen(js_name = cdsParSpread)]
    pub fn cds_par_spread(
        id: &str,
        entity: &str,
        maturity: &FsDate,
        spread_bp: f64,
        recovery_rate: f64,
        currency: &str,
        doc_clause: &str,
    ) -> Result<JsCreditQuote, JsValue> {
        use finstack_core::currency::Currency;
        use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};

        let ccy: Currency = currency
            .parse::<Currency>()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;
        let doc: CdsDocClause = doc_clause
            .parse::<CdsDocClause>()
            .map_err(|e: String| JsValue::from_str(&e))?;

        Ok(Self {
            inner: CdsQuote::CdsParSpread {
                id: QuoteId::new(id),
                entity: entity.to_string(),
                pillar: Pillar::Date(maturity.inner()),
                spread_bp,
                recovery_rate,
                convention: CdsConventionKey {
                    currency: ccy,
                    doc_clause: doc,
                },
            },
        })
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::Cds(self.inner.clone()))
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

// =============================================================================
// CDSTrancheQuote
// =============================================================================

/// CDS Index Tranche market quote.
#[wasm_bindgen(js_name = CdsTrancheQuote)]
#[derive(Clone, Debug)]
pub struct JsCDSTrancheQuote {
    inner: CDSTrancheQuote,
}

impl JsCDSTrancheQuote {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: CDSTrancheQuote) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CDSTrancheQuote {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CDSTrancheQuote)]
impl JsCDSTrancheQuote {
    /// Create a CDS tranche quote.
    ///
    /// @param {string} id - Quote identifier
    /// @param {string} index - Index identifier (e.g. "CDX.NA.IG")
    /// @param {number} attachment - Attachment point (decimal, e.g. 0.03 for 3%)
    /// @param {number} detachment - Detachment point (decimal, e.g. 0.07 for 7%)
    /// @param {FsDate} maturity - Maturity date
    /// @param {number} upfrontPct - Upfront payment percentage
    /// @param {number} runningSpreadBp - Running spread in basis points
    /// @param {string} currency - Currency code
    /// @param {string} docClause - CDS doc clause
    /// @returns {JsCDSTrancheQuote} CDS tranche quote
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        index: &str,
        attachment: f64,
        detachment: f64,
        maturity: &FsDate,
        upfront_pct: f64,
        running_spread_bp: f64,
        currency: &str,
        doc_clause: &str,
    ) -> Result<JsCDSTrancheQuote, JsValue> {
        use finstack_core::currency::Currency;

        let ccy: Currency = currency
            .parse::<Currency>()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;
        let doc: CdsDocClause = doc_clause
            .parse::<CdsDocClause>()
            .map_err(|e: String| JsValue::from_str(&e))?;

        Ok(Self {
            inner: CDSTrancheQuote::CDSTranche {
                id: QuoteId::new(id),
                index: index.to_string(),
                attachment,
                detachment,
                maturity: maturity.inner(),
                upfront_pct,
                running_spread_bp,
                convention: CdsConventionKey {
                    currency: ccy,
                    doc_clause: doc,
                },
            },
        })
    }

    /// Get the quote identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().as_str().to_string()
    }

    /// Bump the running spread by a decimal amount.
    #[wasm_bindgen(js_name = bumpSpreadDecimal)]
    pub fn bump_spread_decimal(&self, bump_decimal: f64) -> JsCDSTrancheQuote {
        Self {
            inner: self.inner.bump_spread_decimal(bump_decimal),
        }
    }

    /// Bump the running spread by basis points.
    #[wasm_bindgen(js_name = bumpSpreadBp)]
    pub fn bump_spread_bp(&self, bump_bp: f64) -> JsCDSTrancheQuote {
        Self {
            inner: self.inner.bump_spread_bp(bump_bp),
        }
    }

    /// Convert to MarketQuote.
    #[wasm_bindgen(js_name = toMarketQuote)]
    pub fn to_market_quote(&self) -> JsMarketQuote {
        JsMarketQuote::from_inner(MarketQuote::CDSTranche(self.inner.clone()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCDSTrancheQuote, JsValue> {
        from_js_value(value).map(JsCDSTrancheQuote::from_inner)
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// VolQuote
// =============================================================================

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
    /// Create a swaption vol quote.
    #[wasm_bindgen(js_name = swaptionVol)]
    pub fn swaption_vol(
        expiry: &FsDate,
        maturity: &FsDate,
        strike: f64,
        vol: f64,
        quote_type: &str,
        convention: &str,
    ) -> JsVolQuote {
        Self {
            inner: VolQuote::SwaptionVol {
                expiry: expiry.inner(),
                maturity: maturity.inner(),
                strike,
                vol,
                quote_type: quote_type.to_string(),
                convention: SwaptionConventionId::new(convention),
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
    pub fn inflation_swap(
        maturity: &FsDate,
        rate: f64,
        index: &str,
        convention: &str,
    ) -> JsInflationQuote {
        Self {
            inner: InflationQuote::InflationSwap {
                maturity: maturity.inner(),
                rate,
                index: index.to_string(),
                convention: InflationSwapConventionId::new(convention),
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
            MarketQuote::Bond(_) => "bond".to_string(),
            MarketQuote::Rates(_) => "rates".to_string(),
            MarketQuote::Cds(_) => "cds".to_string(),
            MarketQuote::CDSTranche(_) => "cds_tranche".to_string(),
            MarketQuote::Fx(_) => "fx".to_string(),
            MarketQuote::Inflation(_) => "inflation".to_string(),
            MarketQuote::Vol(_) => "vol".to_string(),
            MarketQuote::Xccy(_) => "xccy".to_string(),
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
