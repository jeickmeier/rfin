//! Core types for comparable company analysis.
//!
//! Defines the building blocks: company identifiers, valuation multiples,
//! period conventions, and per-company metric containers.

use finstack_core::types::Attributes;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Opaque company identifier within a peer set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompanyId(pub String);

impl CompanyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CompanyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Time basis for computing a valuation multiple.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeriodBasis {
    /// Last twelve months (trailing).
    Ltm,
    /// Next twelve months (forward consensus or forecast).
    Ntm,
    /// Custom period identified by a label (e.g., "FY2025E").
    Custom(String),
}

/// Valuation multiple.
///
/// Enterprise value multiples use EV as the numerator. Equity multiples
/// use market capitalization or share price. Credit multiples use spread
/// or yield as the numerator and a fundamental metric as denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Multiple {
    // ---- Enterprise value multiples ----
    /// EV / EBITDA
    EvEbitda,
    /// EV / Revenue
    EvRevenue,
    /// EV / EBIT
    EvEbit,
    /// EV / Free Cash Flow (unlevered)
    EvFcf,

    // ---- Equity multiples ----
    /// Price / Earnings
    Pe,
    /// Price / Book Value
    Pb,
    /// Price / Tangible Book Value
    Ptbv,
    /// Price / Free Cash Flow (levered)
    PFcf,
    /// Dividend Yield (dividend / price, expressed as a ratio)
    DividendYield,

    // ---- Credit multiples ----
    /// Spread per turn of leverage (OAS / (Debt / EBITDA))
    SpreadPerTurn,
    /// Yield / Interest Coverage
    YieldPerCoverage,
}

impl Multiple {
    /// Returns true if this is an enterprise-value-based multiple.
    pub fn is_ev_multiple(&self) -> bool {
        matches!(
            self,
            Self::EvEbitda | Self::EvRevenue | Self::EvEbit | Self::EvFcf
        )
    }

    /// Returns true if this is an equity-based multiple.
    pub fn is_equity_multiple(&self) -> bool {
        matches!(
            self,
            Self::Pe | Self::Pb | Self::Ptbv | Self::PFcf | Self::DividendYield
        )
    }

    /// Returns true if this is a credit-specific multiple.
    pub fn is_credit_multiple(&self) -> bool {
        matches!(self, Self::SpreadPerTurn | Self::YieldPerCoverage)
    }

    /// Human-readable short label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::EvEbitda => "EV/EBITDA",
            Self::EvRevenue => "EV/Revenue",
            Self::EvEbit => "EV/EBIT",
            Self::EvFcf => "EV/FCF",
            Self::Pe => "P/E",
            Self::Pb => "P/B",
            Self::Ptbv => "P/TBV",
            Self::PFcf => "P/FCF",
            Self::DividendYield => "Div Yield",
            Self::SpreadPerTurn => "Spread/Turn",
            Self::YieldPerCoverage => "Yield/Coverage",
        }
    }
}

impl FromStr for Multiple {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "evebitda" | "ev_ebitda" | "ev/ebitda" => Ok(Self::EvEbitda),
            "evrevenue" | "ev_revenue" | "ev/revenue" => Ok(Self::EvRevenue),
            "evebit" | "ev_ebit" | "ev/ebit" => Ok(Self::EvEbit),
            "evfcf" | "ev_fcf" | "ev/fcf" => Ok(Self::EvFcf),
            "pe" | "p_e" | "p/e" => Ok(Self::Pe),
            "pb" | "p_b" | "p/b" => Ok(Self::Pb),
            "ptbv" | "p_tbv" | "p/tbv" => Ok(Self::Ptbv),
            "pfcf" | "p_fcf" | "p/fcf" => Ok(Self::PFcf),
            "dividendyield" | "dividend_yield" | "dividend/yield" | "div_yield" => {
                Ok(Self::DividendYield)
            }
            "spreadperturn" | "spread_per_turn" | "spread/turn" => Ok(Self::SpreadPerTurn),
            "yieldpercoverage" | "yield_per_coverage" | "yield/coverage" => {
                Ok(Self::YieldPerCoverage)
            }
            other => Err(format!(
                "unknown multiple {other:?}; expected one of EvEbitda, EvRevenue, EvEbit, EvFcf, Pe, Pb, Ptbv, PFcf, DividendYield, SpreadPerTurn, YieldPerCoverage"
            )),
        }
    }
}

/// Metrics for a single company in a peer set.
///
/// All monetary values should be in the same currency before constructing
/// a `PeerSet`. Currency normalization is the caller's responsibility.
/// Ratios are plain scalars (e.g., `6.5` means 6.5x leverage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMetrics {
    /// Company identifier.
    pub id: CompanyId,

    /// Optional instrument-level attributes (sector, geography, rating).
    /// Used by `PeerFilter` for inclusion/exclusion decisions.
    pub attributes: Attributes,

    // ---- Pricing / market data ----
    /// Enterprise value.
    pub enterprise_value: Option<f64>,
    /// Equity market capitalization.
    pub market_cap: Option<f64>,
    /// Share price.
    pub share_price: Option<f64>,
    /// Option-adjusted spread in basis points.
    pub oas_bps: Option<f64>,
    /// Yield to worst / yield to maturity.
    pub yield_pct: Option<f64>,

    // ---- Fundamental metrics ----
    /// EBITDA (period basis determined by the PeerSet context).
    pub ebitda: Option<f64>,
    /// Revenue.
    pub revenue: Option<f64>,
    /// EBIT.
    pub ebit: Option<f64>,
    /// Unlevered free cash flow.
    pub ufcf: Option<f64>,
    /// Levered free cash flow.
    pub lfcf: Option<f64>,
    /// Net income / earnings.
    pub net_income: Option<f64>,
    /// Book value of equity.
    pub book_value: Option<f64>,
    /// Tangible book value.
    pub tangible_book_value: Option<f64>,
    /// Dividends per share (annualized).
    pub dividends_per_share: Option<f64>,

    // ---- Credit metrics ----
    /// Total debt / EBITDA.
    pub leverage: Option<f64>,
    /// EBITDA / Interest Expense.
    pub interest_coverage: Option<f64>,
    /// Revenue growth rate (decimal, e.g., 0.05 = 5%).
    pub revenue_growth: Option<f64>,
    /// EBITDA margin (decimal, e.g., 0.25 = 25%).
    pub ebitda_margin: Option<f64>,

    /// Arbitrary additional metrics keyed by name.
    /// Used for custom multiples or regression factors.
    pub custom: IndexMap<String, f64>,
}

impl CompanyMetrics {
    /// Create a new `CompanyMetrics` with only the company ID set.
    /// All other fields default to `None` / empty.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: CompanyId::new(id),
            attributes: Attributes::default(),
            enterprise_value: None,
            market_cap: None,
            share_price: None,
            oas_bps: None,
            yield_pct: None,
            ebitda: None,
            revenue: None,
            ebit: None,
            ufcf: None,
            lfcf: None,
            net_income: None,
            book_value: None,
            tangible_book_value: None,
            dividends_per_share: None,
            leverage: None,
            interest_coverage: None,
            revenue_growth: None,
            ebitda_margin: None,
            custom: IndexMap::new(),
        }
    }
}
