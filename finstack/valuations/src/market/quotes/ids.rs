use finstack_core::dates::Tenor;
use serde::{Deserialize, Serialize};
use std::fmt;
use time::Date;

/// A stable identifier for a market quote (e.g., "USD-OIS-SWAP-5Y").
///
/// This ID is used for human readability, logging, and potentially matching against external data sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct QuoteId(String);

impl QuoteId {
    /// Create a new QuoteId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// view the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for QuoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for QuoteId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for QuoteId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// The maturity pillar of a quote.
///
/// OTC instruments (Swaps, Deposits) typically use `Tenor` (e.g. "5Y") to allow rolling headers.
/// Futures or bespoke runs may use `Date` to pin a specific maturity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pillar {
    /// A relative tenor (e.g., 5Y, 3M).
    Tenor(Tenor),
    /// An absolute date.
    Date(Date),
}

impl fmt::Display for Pillar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pillar::Tenor(t) => write!(f, "{}", t),
            Pillar::Date(d) => write!(f, "{}", d),
        }
    }
}
