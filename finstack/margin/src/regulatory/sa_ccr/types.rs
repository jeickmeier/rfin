//! SA-CCR types and data structures.
//!
//! Defines the asset class taxonomy, trade representation, netting set
//! configuration, and EAD result per BCBS 279.

use crate::types::NettingSetId;
use finstack_core::dates::Date;
use finstack_core::HashMap;

/// SA-CCR asset class for add-on computation.
///
/// Each derivative trade is assigned to exactly one asset class.
/// The add-on formula and supervisory parameters differ by class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum SaCcrAssetClass {
    /// Interest rate derivatives.
    InterestRate,
    /// Foreign exchange derivatives.
    ForeignExchange,
    /// Credit derivatives.
    Credit,
    /// Equity derivatives.
    Equity,
    /// Commodity derivatives.
    Commodity,
}

impl SaCcrAssetClass {
    /// All asset classes in canonical order.
    pub const ALL: &'static [SaCcrAssetClass] = &[
        SaCcrAssetClass::InterestRate,
        SaCcrAssetClass::ForeignExchange,
        SaCcrAssetClass::Credit,
        SaCcrAssetClass::Equity,
        SaCcrAssetClass::Commodity,
    ];
}

impl std::fmt::Display for SaCcrAssetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InterestRate => write!(f, "Interest Rate"),
            Self::ForeignExchange => write!(f, "Foreign Exchange"),
            Self::Credit => write!(f, "Credit"),
            Self::Equity => write!(f, "Equity"),
            Self::Commodity => write!(f, "Commodity"),
        }
    }
}

/// SA-CCR option type for delta computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SaCcrOptionType {
    /// Long call option.
    CallLong,
    /// Short call option.
    CallShort,
    /// Long put option.
    PutLong,
    /// Short put option.
    PutShort,
}

/// A single derivative trade for SA-CCR EAD computation.
///
/// Captures the trade-level attributes required by the SA-CCR formula:
/// notional, maturity dates, direction, underlier, and option characteristics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaCcrTrade {
    /// Unique trade identifier.
    pub trade_id: String,
    /// Asset class assignment.
    pub asset_class: SaCcrAssetClass,
    /// Adjusted notional in reporting currency.
    pub notional: f64,
    /// Trade start date (for maturity factor computation).
    pub start_date: Date,
    /// Trade end date / maturity.
    pub end_date: Date,
    /// Underlier reference (e.g., currency pair, issuer, equity name, commodity).
    pub underlier: String,
    /// Hedging set identifier within the asset class.
    /// Trades with the same hedging set can partially offset.
    pub hedging_set: String,
    /// Long (+1.0) or short (-1.0) direction.
    pub direction: f64,
    /// Supervisory delta adjustment.
    /// For linear trades: +1 (long) or -1 (short).
    /// For options: delta from Black-Scholes or equivalent.
    pub supervisory_delta: f64,
    /// Current mark-to-market value.
    pub mtm: f64,
    /// Whether this trade is an option.
    pub is_option: bool,
    /// Option exercise type if applicable.
    pub option_type: Option<SaCcrOptionType>,
}

/// Netting set configuration for SA-CCR.
///
/// Captures the collateral terms that determine whether the margined
/// or unmargined RC/PFE formulas apply.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaCcrNettingSetConfig {
    /// Netting set identifier.
    pub netting_set_id: NettingSetId,
    /// Whether the netting set is subject to a margin agreement.
    pub is_margined: bool,
    /// Net current collateral held (positive = bank holds collateral).
    pub collateral: f64,
    /// Threshold amount (TH) under the margin agreement.
    pub threshold: f64,
    /// Minimum transfer amount (MTA).
    pub mta: f64,
    /// Net independent collateral amount (NICA).
    pub nica: f64,
    /// Margin period of risk in business days (default: 10 for bilateral).
    pub mpor_days: u32,
}

impl SaCcrNettingSetConfig {
    /// Create an unmargined netting set configuration.
    #[must_use]
    pub fn unmargined(netting_set_id: NettingSetId, collateral: f64) -> Self {
        Self {
            netting_set_id,
            is_margined: false,
            collateral,
            threshold: 0.0,
            mta: 0.0,
            nica: 0.0,
            mpor_days: 10,
        }
    }

    /// Create a margined netting set configuration.
    #[must_use]
    pub fn margined(
        netting_set_id: NettingSetId,
        collateral: f64,
        threshold: f64,
        mta: f64,
        nica: f64,
        mpor_days: u32,
    ) -> Self {
        Self {
            netting_set_id,
            is_margined: true,
            collateral,
            threshold,
            mta,
            nica,
            mpor_days,
        }
    }
}

/// SA-CCR Exposure at Default result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EadResult {
    /// Exposure at Default: `alpha * (RC + PFE)`.
    pub ead: f64,
    /// Replacement cost component.
    pub rc: f64,
    /// Potential future exposure component.
    pub pfe: f64,
    /// PFE multiplier (accounts for over-collateralization).
    pub multiplier: f64,
    /// Aggregate add-on before multiplier.
    pub add_on_aggregate: f64,
    /// Add-on by asset class.
    pub add_on_by_asset_class: HashMap<SaCcrAssetClass, f64>,
    /// Alpha multiplier (1.4 per regulation).
    pub alpha: f64,
    /// Maturity factor applied.
    pub maturity_factor: f64,
}
