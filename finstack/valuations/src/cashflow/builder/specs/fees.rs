//! Fee specification types for fixed and periodic fees.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::Bps;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

/// Fee specification.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeSpec {
    /// Fixed variant.
    Fixed {
        /// Date.
        date: Date,
        /// Amount.
        amount: Money,
    },
    /// Periodic Bps variant.
    PeriodicBps {
        /// Base.
        base: FeeBase,
        /// Fee rate in basis points. Uses Decimal for exact representation.
        bps: Decimal,
        /// Freq.
        freq: Tenor,
        /// Dc.
        dc: DayCount,
        /// Bdc.
        bdc: BusinessDayConvention,
        /// Calendar id.
        calendar_id: Option<String>,
        /// Stub.
        stub: StubKind,
    },
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit − outstanding, 0).
    Undrawn {
        /// Facility limit.
        facility_limit: Money,
    },
}

/// Fee tier for utilization-based fee structures.
///
/// Tiers are evaluated in order: the first tier where utilization >= threshold applies.
/// Tiers should be sorted by threshold (ascending).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FeeTier {
    /// Utilization threshold (0.0 to 1.0). Fee applies when utilization >= this threshold.
    pub threshold: Decimal,
    /// Fee rate in basis points for this tier.
    pub bps: Decimal,
}

impl FeeTier {
    /// Create a fee tier using typed basis points.
    pub fn from_bps(threshold: f64, bps: Bps) -> Self {
        Self {
            threshold: Decimal::try_from(threshold).unwrap_or(Decimal::ZERO),
            bps: Decimal::from(bps.as_bps()),
        }
    }
}

/// Evaluate fee tiers to find the applicable rate for a given utilization.
///
/// Returns the fee rate from the highest tier where utilization >= threshold.
/// If no tiers match or tiers are empty, returns 0.0.
///
/// # Arguments
///
/// * `tiers` - Slice of fee tiers, should be sorted by threshold ascending
/// * `utilization` - Current utilization rate (0.0 to 1.0) as Decimal
///
/// # Returns
///
/// The fee rate in basis points for the applicable tier, or 0.0 if no tier matches
pub fn evaluate_fee_tiers(tiers: &[FeeTier], utilization: Decimal) -> Decimal {
    tiers
        .iter()
        .rev()
        .find(|tier| utilization >= tier.threshold)
        .map(|tier| tier.bps)
        .unwrap_or(Decimal::ZERO)
}

/// Evaluate fee tiers with f64 utilization for convenience.
///
/// Converts the utilization to Decimal, evaluates, and returns f64.
pub fn evaluate_fee_tiers_f64(tiers: &[FeeTier], utilization: f64) -> f64 {
    let util_dec = Decimal::try_from(utilization).unwrap_or(Decimal::ZERO);
    evaluate_fee_tiers(tiers, util_dec).to_f64().unwrap_or(0.0)
}
