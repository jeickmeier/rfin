//! Fee specification types for fixed and periodic fees.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::Bps;
use rust_decimal::Decimal;

/// Fee specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
        /// Calendar id (use "weekends_only" for weekends-only adjustments).
        calendar_id: String,
        /// Stub.
        stub: StubKind,
        /// How the outstanding balance is sampled for fee calculation.
        #[serde(default)]
        accrual_basis: FeeAccrualBasis,
    },
}

/// Controls how the outstanding balance is sampled during fee accrual.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FeeAccrualBasis {
    /// Use outstanding at a single point in time (period start). Current behavior.
    #[default]
    PointInTime,
    /// Time-weighted average outstanding over the accrual period.
    TimeWeightedAverage,
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FeeBase {
    /// Base on drawn outstanding (post-amortization, post-PIK).
    Drawn,
    /// Base on undrawn = max(limit - outstanding, 0).
    Undrawn {
        /// Facility limit.
        facility_limit: Money,
    },
}

/// Fee tier for utilization-based fee structures.
///
/// Tiers are evaluated in order: the first tier where utilization >= threshold applies.
/// Tiers should be sorted by threshold (ascending).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
