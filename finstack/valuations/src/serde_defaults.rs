//! Shared serde default helpers for instrument structs.

use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind};

use crate::instruments::SettlementType;

/// Default stub convention for optional schedule stub fields.
pub(crate) fn stub_short_front() -> StubKind {
    StubKind::ShortFront
}

/// Default business day convention for optional BDC fields.
pub(crate) fn bdc_modified_following() -> BusinessDayConvention {
    BusinessDayConvention::ModifiedFollowing
}

/// Default day count convention for option instruments (ACT/365F).
pub(crate) fn day_count_act365f() -> DayCount {
    DayCount::Act365F
}

/// Default settlement type for option instruments (cash).
pub(crate) fn settlement_cash() -> SettlementType {
    SettlementType::Cash
}

/// Default contract multiplier (1.0).
pub(crate) fn multiplier_one() -> f64 {
    1.0
}
