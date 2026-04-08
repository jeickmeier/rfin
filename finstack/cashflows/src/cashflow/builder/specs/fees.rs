//! Fee specification types for fixed and periodic fees.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::Bps;
use rust_decimal::Decimal;

/// Fee specification for fixed-fee and periodic-basis-point programs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum FeeSpec {
    /// Fixed fee paid once on a specified date.
    Fixed {
        /// Payment date of the fixed fee.
        #[schemars(with = "String")]
        date: Date,
        /// Fee amount in currency units.
        amount: Money,
    },
    /// Periodic fee quoted in basis points and accrued over generated periods.
    PeriodicBps {
        /// Economic balance used as the fee base.
        base: FeeBase,
        /// Fee quote in basis points per annum, stored as `Decimal` to preserve
        /// the quoted value exactly.
        bps: Decimal,
        /// Accrual and payment frequency for the fee schedule.
        freq: Tenor,
        /// Day-count convention used to annualize the fee accrual.
        dc: DayCount,
        /// Business-day convention applied to generated fee dates.
        bdc: BusinessDayConvention,
        /// Holiday calendar identifier used with `bdc`.
        ///
        /// Use `"weekends_only"` when only weekend adjustment is required.
        calendar_id: String,
        /// Stub-handling rule for irregular first or last fee periods.
        stub: StubKind,
        /// How the outstanding balance is sampled for fee calculation.
        #[serde(default, skip_serializing_if = "FeeAccrualBasis::is_default")]
        accrual_basis: FeeAccrualBasis,
    },
}

/// Controls how the outstanding balance is sampled during fee accrual.
#[derive(
    Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub enum FeeAccrualBasis {
    /// Use outstanding at a single point in time (period start). Current behavior.
    #[default]
    PointInTime,
    /// Time-weighted average outstanding over the accrual period.
    TimeWeightedAverage,
}

impl FeeAccrualBasis {
    /// Returns true if this is the default variant (for serde skip_serializing_if).
    pub fn is_default(&self) -> bool {
        matches!(self, Self::PointInTime)
    }
}

/// Fee base for periodic bps fees.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum FeeBase {
    /// Fee base is the drawn outstanding after amortization and PIK updates.
    Drawn,
    /// Base on undrawn = max(limit - outstanding, 0).
    Undrawn {
        /// Total facility commitment used to compute the undrawn amount.
        facility_limit: Money,
    },
}

/// Fee tier for utilization-based fee structures.
///
/// Tiers are evaluated in order: the first tier where utilization >= threshold applies.
/// Tiers should be sorted by threshold (ascending).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct FeeTier {
    /// Utilization threshold (0.0 to 1.0). Fee applies when utilization >= this threshold.
    pub threshold: Decimal,
    /// Fee rate in basis points for this tier.
    pub bps: Decimal,
}

impl FeeTier {
    /// Create a fee tier using typed basis points.
    ///
    /// # Panics (debug builds only)
    ///
    /// Asserts that `threshold` is finite.
    pub fn from_bps(threshold: f64, bps: Bps) -> Self {
        debug_assert!(
            threshold.is_finite(),
            "FeeTier::from_bps: threshold is not finite ({threshold})"
        );
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
