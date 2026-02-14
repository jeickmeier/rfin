//! Inflation-linked bond specific parameters.

use finstack_core::{
    dates::{Date, DayCount, Tenor},
    money::Money,
};
use rust_decimal::Decimal;

/// Inflation-linked bond specific parameters.
///
/// Groups parameters specific to inflation-linked bonds.
#[derive(Debug, Clone)]
pub struct InflationLinkedBondParams {
    /// Notional amount
    pub notional: Money,
    /// Real coupon rate
    pub real_coupon: Decimal,
    /// Issue date
    pub issue: Date,
    /// Maturity date
    pub maturity: Date,
    /// Base index value at issue
    pub base_index: f64,
    /// Payment frequency
    pub frequency: Tenor,
    /// Day count convention
    pub day_count: DayCount,
}

impl InflationLinkedBondParams {
    /// Create new inflation-linked bond parameters
    pub fn new(
        notional: Money,
        real_coupon: f64,
        issue: Date,
        maturity: Date,
        base_index: f64,
        frequency: Tenor,
        day_count: DayCount,
    ) -> Self {
        Self {
            notional,
            real_coupon: Decimal::try_from(real_coupon).unwrap_or_default(),
            issue,
            maturity,
            base_index,
            frequency,
            day_count,
        }
    }

    /// Create US TIPS parameters (semi-annual, Act/Act)
    pub fn tips(
        notional: Money,
        real_coupon: f64,
        issue: Date,
        maturity: Date,
        base_index: f64,
    ) -> Self {
        Self::new(
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            Tenor::semi_annual(),
            DayCount::ActAct,
        )
    }

    /// Create UK linker parameters (semi-annual, Act/Act)
    pub fn uk_linker(
        notional: Money,
        real_coupon: f64,
        issue: Date,
        maturity: Date,
        base_index: f64,
    ) -> Self {
        Self::new(
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            Tenor::semi_annual(),
            DayCount::ActAct,
        )
    }
}
