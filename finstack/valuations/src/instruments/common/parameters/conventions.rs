//! Market conventions for standard instrument types.
//!
//! Provides enums and associated methods for common market conventions,
//! eliminating the need for multiple instrument-specific constructors.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Standard bond market conventions by region/issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BondConvention {
    /// US Treasury: Semi-annual, ActActISMA
    USTreasury,
    /// German Bund: Annual, ActAct
    GermanBund,
    /// UK Gilt: Semi-annual, ActAct
    UKGilt,
    /// French OAT: Annual, ActAct
    FrenchOAT,
    /// Standard corporate: Semi-annual, 30/360
    Corporate,
}

impl BondConvention {
    /// Day count convention for this market
    pub fn day_count(&self) -> DayCount {
        match self {
            BondConvention::USTreasury
            | BondConvention::GermanBund
            | BondConvention::UKGilt
            | BondConvention::FrenchOAT => DayCount::ActActIsma,
            BondConvention::Corporate => DayCount::Thirty360,
        }
    }

    /// Payment frequency for this market
    pub fn frequency(&self) -> Tenor {
        match self {
            BondConvention::USTreasury | BondConvention::UKGilt | BondConvention::Corporate => {
                Tenor::semi_annual()
            }
            BondConvention::GermanBund | BondConvention::FrenchOAT => Tenor::annual(),
        }
    }

    /// Business day convention for this market
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        BusinessDayConvention::Following
    }

    /// Stub convention for this market
    pub fn stub_convention(&self) -> StubKind {
        StubKind::None
    }

    /// Default discount curve ID for this market
    pub fn default_disc_curve(&self) -> &'static str {
        match self {
            BondConvention::USTreasury | BondConvention::Corporate => "USD-TREASURY",
            BondConvention::GermanBund | BondConvention::FrenchOAT => "EUR-BUND",
            BondConvention::UKGilt => "GBP-GILT",
        }
    }
}

impl std::fmt::Display for BondConvention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BondConvention::USTreasury => write!(f, "us_treasury"),
            BondConvention::GermanBund => write!(f, "german_bund"),
            BondConvention::UKGilt => write!(f, "uk_gilt"),
            BondConvention::FrenchOAT => write!(f, "french_oat"),
            BondConvention::Corporate => write!(f, "corporate"),
        }
    }
}

impl std::str::FromStr for BondConvention {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "us_treasury" | "ust" | "treasury" => Ok(BondConvention::USTreasury),
            "german_bund" | "bund" => Ok(BondConvention::GermanBund),
            "uk_gilt" | "gilt" => Ok(BondConvention::UKGilt),
            "french_oat" | "oat" => Ok(BondConvention::FrenchOAT),
            "corporate" | "corp" => Ok(BondConvention::Corporate),
            other => Err(format!("Unknown bond convention: {}", other)),
        }
    }
}

/// Standard interest rate swap conventions by region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum IRSConvention {
    /// USD: Semi-annual, Act/360, SOFR
    USDStandard,
    /// EUR: Annual (fixed), Semi-annual (float), Act/360, ESTR
    EURStandard,
    /// GBP: Semi-annual, Act/365, SONIA
    GBPStandard,
    /// JPY: Semi-annual, Act/365, TONA
    JPYStandard,
}

impl IRSConvention {
    /// Fixed leg day count for this convention
    pub fn fixed_day_count(&self) -> DayCount {
        match self {
            IRSConvention::USDStandard => DayCount::Thirty360,
            IRSConvention::EURStandard => DayCount::Thirty360,
            IRSConvention::GBPStandard => DayCount::Act365F,
            IRSConvention::JPYStandard => DayCount::Act365F,
        }
    }

    /// Float leg day count for this convention
    pub fn float_day_count(&self) -> DayCount {
        match self {
            IRSConvention::USDStandard | IRSConvention::EURStandard => DayCount::Act360,
            IRSConvention::GBPStandard | IRSConvention::JPYStandard => DayCount::Act365F,
        }
    }

    /// Fixed leg frequency for this convention
    pub fn fixed_frequency(&self) -> Tenor {
        match self {
            IRSConvention::USDStandard
            | IRSConvention::GBPStandard
            | IRSConvention::JPYStandard => Tenor::semi_annual(),
            IRSConvention::EURStandard => Tenor::annual(),
        }
    }

    /// Float leg frequency for this convention
    pub fn float_frequency(&self) -> Tenor {
        Tenor::semi_annual()
    }

    /// Business day convention for this convention
    pub fn business_day_convention(&self) -> BusinessDayConvention {
        BusinessDayConvention::ModifiedFollowing
    }

    /// Calendar identifier for this convention
    pub fn calendar_id(&self) -> Option<String> {
        match self {
            IRSConvention::USDStandard => Some("us".to_string()),
            IRSConvention::EURStandard => Some("target2".to_string()),
            IRSConvention::GBPStandard => Some("gblo".to_string()),
            IRSConvention::JPYStandard => Some("jpto".to_string()),
        }
    }

    /// Discount curve ID for this convention
    pub fn disc_curve_id(&self) -> &'static str {
        match self {
            IRSConvention::USDStandard => "USD-OIS",
            IRSConvention::EURStandard => "EUR-ESTR",
            IRSConvention::GBPStandard => "GBP-SONIA",
            IRSConvention::JPYStandard => "JPY-TONA",
        }
    }

    /// Forward curve ID for this convention
    pub fn forward_curve_id(&self) -> &'static str {
        match self {
            IRSConvention::USDStandard => "USD-SOFR-3M",
            IRSConvention::EURStandard => "EUR-EURIBOR-6M",
            IRSConvention::GBPStandard => "GBP-SONIA",
            IRSConvention::JPYStandard => "JPY-TONA",
        }
    }

    /// Reset lag in business days for this convention
    pub fn reset_lag_days(&self) -> i32 {
        match self {
            IRSConvention::USDStandard | IRSConvention::EURStandard => 2,
            IRSConvention::GBPStandard | IRSConvention::JPYStandard => 0, // Same-day fixing
        }
    }
}

impl std::fmt::Display for IRSConvention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRSConvention::USDStandard => write!(f, "usd_standard"),
            IRSConvention::EURStandard => write!(f, "eur_standard"),
            IRSConvention::GBPStandard => write!(f, "gbp_standard"),
            IRSConvention::JPYStandard => write!(f, "jpy_standard"),
        }
    }
}

impl std::str::FromStr for IRSConvention {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "usd_standard" | "usd" => Ok(IRSConvention::USDStandard),
            "eur_standard" | "eur" => Ok(IRSConvention::EURStandard),
            "gbp_standard" | "gbp" => Ok(IRSConvention::GBPStandard),
            "jpy_standard" | "jpy" => Ok(IRSConvention::JPYStandard),
            other => Err(format!("Unknown IRS convention: {}", other)),
        }
    }
}
