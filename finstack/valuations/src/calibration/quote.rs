//! Market quote data structures.

use finstack_core::dates::Date;
use finstack_core::F;

/// Market quote with bid/ask spread and metadata.
#[derive(Clone, Debug)]
pub struct MarketQuote {
    /// Instrument identifier
    pub instrument_id: String,
    /// Quote value (rate, spread, volatility, etc.)
    pub value: F,
    /// Bid-ask spread (optional)
    pub bid_ask_spread: Option<F>,
    /// Quote timestamp
    pub as_of: Date,
    /// Market convention/source
    pub source: String,
    /// Quality indicator (0-100, 100 = best)
    pub quality: Option<u8>,
}

impl MarketQuote {
    /// Create a new market quote.
    pub fn new(
        instrument_id: impl Into<String>,
        value: F,
        as_of: Date,
        source: impl Into<String>,
    ) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            value,
            bid_ask_spread: None,
            as_of,
            source: source.into(),
            quality: None,
        }
    }

    /// Set bid-ask spread.
    pub fn with_bid_ask_spread(mut self, spread: F) -> Self {
        self.bid_ask_spread = Some(spread);
        self
    }

    /// Set quality indicator.
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = Some(quality);
        self
    }
}
