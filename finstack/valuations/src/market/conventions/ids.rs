use finstack_core::currency::Currency;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable identifier for a rate index (e.g., "USD-SOFR-OIS", "EUR-EURIBOR-6M").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct IndexId(String);

impl IndexId {
    /// Create a new IndexId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IndexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IndexId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Stable identifier for an Interest Rate Future contract (e.g., "CME:SR3").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct IrFutureContractId(String);

impl IrFutureContractId {
    /// Create a new IrFutureContractId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// View the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IrFutureContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IrFutureContractId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// CDS market standard documentation clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CdsDocClause {
    /// Cum-Restructuring 2014 (CR14)
    Cr14,
    /// Modified-Restructuring 2014 (MR14)
    Mr14,
    /// Modified-Modified-Restructuring 2014 (MM14)
    Mm14,
    /// No-Restructuring 2014 (XR14)
    Xr14,
    /// ISDA North American Corporate (IsdaNa)
    IsdaNa,
    /// ISDA European Corporate (IsdaEu)
    IsdaEu,
    /// ISDA Asia Corporate (IsdaAs)
    IsdaAs,
    /// ISDA Australia Corporate (IsdaAu)
    IsdaAu,
    /// ISDA New Zealand Corporate (IsdaNz)
    IsdaNz,
    /// Custom / Other
    Custom,
}

impl std::str::FromStr for CdsDocClause {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cr14" | "cr" => Ok(CdsDocClause::Cr14),
            "mr14" | "mr" => Ok(CdsDocClause::Mr14),
            "mm14" | "mm" => Ok(CdsDocClause::Mm14),
            "xr14" | "xr" => Ok(CdsDocClause::Xr14),
            "isdana" | "isda_na" | "na" => Ok(CdsDocClause::IsdaNa),
            "isdaeu" | "isda_eu" | "eu" => Ok(CdsDocClause::IsdaEu),
            "isdaas" | "isda_as" | "as" => Ok(CdsDocClause::IsdaAs),
            "isdaau" | "isda_au" | "au" => Ok(CdsDocClause::IsdaAu),
            "isdanz" | "isda_nz" | "nz" => Ok(CdsDocClause::IsdaNz),
            "custom" => Ok(CdsDocClause::Custom),
            _ => Err(format!("Unknown CDS doc clause: {}", s)),
        }
    }
}

/// Key to look up CDS conventions (Currency + DocClause).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CdsConventionKey {
    /// The currency of the CDS.
    pub currency: Currency,
    /// The documentation clause (e.g. CR14, MM14).
    pub doc_clause: CdsDocClause,
}

impl fmt::Display for CdsConventionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:?}", self.currency, self.doc_clause)
    }
}

/// Identifier for Option market conventions (Equity/FX/Commodity).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct OptionConventionId(pub String);

impl OptionConventionId {
    /// Create a new OptionConventionId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OptionConventionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for OptionConventionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Identifier for Swaption market conventions (e.g., "USD", "EUR").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SwaptionConventionId(pub String);

impl SwaptionConventionId {
    /// Create a new SwaptionConventionId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SwaptionConventionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SwaptionConventionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Identifier for Inflation Swap market conventions (e.g., "USD-CPI", "UK-RPI").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct InflationSwapConventionId(pub String);

impl InflationSwapConventionId {
    /// Create a new InflationSwapConventionId from a string.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InflationSwapConventionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for InflationSwapConventionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}
