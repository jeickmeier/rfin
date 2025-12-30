//! Stable identifiers for convention lookups.
//!
//! This module provides type-safe identifiers for all convention types. These identifiers
//! prevent accidental mismatches between different ID types and ensure conventions are looked
//! up correctly.

use finstack_core::currency::Currency;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical rate index identifier (shared with finstack-core).
pub type IndexId = finstack_core::types::IndexId;

/// Stable identifier for an Interest Rate Future contract (e.g., "CME:SR3").
///
/// Used to look up [`IrFutureConventions`](crate::market::conventions::defs::IrFutureConventions)
/// from the convention registry. Contract IDs typically follow exchange:contract format.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::IrFutureContractId;
///
/// let id = IrFutureContractId::new("CME:SR3");
/// assert_eq!(id.as_str(), "CME:SR3");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct IrFutureContractId(String);

impl IrFutureContractId {
    /// Create a new `IrFutureContractId` from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The contract identifier string
    ///
    /// # Returns
    ///
    /// A new `IrFutureContractId` instance.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// View the inner string representation.
    ///
    /// # Returns
    ///
    /// A string slice containing the identifier.
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
///
/// Represents the ISDA documentation clause used for CDS contracts. Different clauses
/// define different restructuring events and settlement procedures. Used as part of
/// [`CdsConventionKey`] to look up CDS conventions.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::CdsDocClause;
///
/// let clause = CdsDocClause::Cr14; // Cum-Restructuring 2014
/// ```
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
///
/// CDS conventions are identified by both currency and documentation clause, as different
/// clauses have different market conventions. Used to look up [`CdsConventions`](crate::market::conventions::defs::CdsConventions)
/// from the convention registry.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::{CdsConventionKey, CdsDocClause};
/// use finstack_core::currency::Currency;
///
/// let key = CdsConventionKey {
///     currency: Currency::USD,
///     doc_clause: CdsDocClause::Cr14,
/// };
/// ```
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
///
/// Used to look up [`OptionConventions`](crate::market::conventions::defs::OptionConventions)
/// from the convention registry. Convention IDs typically follow "{currency}-{asset-class}" format.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::OptionConventionId;
///
/// let id = OptionConventionId::new("USD-EQUITY");
/// assert_eq!(id.as_str(), "USD-EQUITY");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct OptionConventionId(pub String);

impl OptionConventionId {
    /// Create a new `OptionConventionId` from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The convention identifier string
    ///
    /// # Returns
    ///
    /// A new `OptionConventionId` instance.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string representation.
    ///
    /// # Returns
    ///
    /// A string slice containing the identifier.
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
///
/// Used to look up [`SwaptionConventions`](crate::market::conventions::defs::SwaptionConventions)
/// from the convention registry. Convention IDs are typically currency codes.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::SwaptionConventionId;
///
/// let id = SwaptionConventionId::new("USD");
/// assert_eq!(id.as_str(), "USD");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SwaptionConventionId(pub String);

impl SwaptionConventionId {
    /// Create a new `SwaptionConventionId` from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The convention identifier string
    ///
    /// # Returns
    ///
    /// A new `SwaptionConventionId` instance.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string representation.
    ///
    /// # Returns
    ///
    /// A string slice containing the identifier.
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
///
/// Used to look up [`InflationSwapConventions`](crate::market::conventions::defs::InflationSwapConventions)
/// from the convention registry. Convention IDs typically follow "{currency}-{index}" format.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::market::conventions::ids::InflationSwapConventionId;
///
/// let id = InflationSwapConventionId::new("USD-CPI");
/// assert_eq!(id.as_str(), "USD-CPI");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct InflationSwapConventionId(pub String);

impl InflationSwapConventionId {
    /// Create a new `InflationSwapConventionId` from a string.
    ///
    /// # Arguments
    ///
    /// * `s` - The convention identifier string
    ///
    /// # Returns
    ///
    /// A new `InflationSwapConventionId` instance.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// View the inner string representation.
    ///
    /// # Returns
    ///
    /// A string slice containing the identifier.
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
