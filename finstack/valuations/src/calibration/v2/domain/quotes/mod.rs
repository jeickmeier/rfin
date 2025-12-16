pub mod conventions;
pub mod rates;
pub mod credit;
pub mod vol;
pub mod inflation;
pub mod market_quote;

pub use conventions::InstrumentConventions;
pub use rates::{RatesQuote, FutureSpecs};
pub use credit::CreditQuote;
pub use vol::VolQuote;
pub use inflation::InflationQuote;
pub use market_quote::MarketQuote;

/// Trait for filtering quote collections into specific types.
pub trait ExtractQuotes<T> {
    /// Extract quotes of type `T` from a collection of market quotes.
    fn extract_quotes(&self) -> Vec<T>;
}

impl ExtractQuotes<RatesQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<RatesQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Rates(rq) => Some(rq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<CreditQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<CreditQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Credit(cq) => Some(cq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<VolQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<VolQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Vol(vq) => Some(vq.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ExtractQuotes<InflationQuote> for [MarketQuote] {
    fn extract_quotes(&self) -> Vec<InflationQuote> {
        self.iter()
            .filter_map(|q| match q {
                MarketQuote::Inflation(iq) => Some(iq.clone()),
                _ => None,
            })
            .collect()
    }
}

