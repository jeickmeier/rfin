//! Helper functions to extract volatility ID and expiry from instruments.
//!
//! Used by generic FD calculators to get market data needed for adaptive bump sizes.
//! These helpers use Any downcasting to access instrument-specific fields.

use finstack_core::dates::Date;
use finstack_core::types::CurveId;

/// Extract volatility surface ID from an instrument if available.
///
/// Uses Any downcasting to check for common vol_id field patterns.
/// Returns None if the instrument doesn't have a volatility surface.
pub fn get_instrument_vol_id(instrument: &dyn std::any::Any) -> Option<CurveId> {
    use crate::instruments::*;

    // Try downcasting to each instrument type with vol_id
    if let Some(eq_opt) = instrument.downcast_ref::<equity_option::EquityOption>() {
        return Some(eq_opt.vol_id.clone());
    }
    if let Some(fx_opt) = instrument.downcast_ref::<fx_option::FxOption>() {
        return Some(fx_opt.vol_id.clone());
    }
    if let Some(asian) = instrument.downcast_ref::<asian_option::AsianOption>() {
        return Some(asian.vol_id.clone());
    }
    if let Some(autocall) = instrument.downcast_ref::<autocallable::Autocallable>() {
        return Some(autocall.vol_id.clone());
    }
    if let Some(barrier) = instrument.downcast_ref::<barrier_option::BarrierOption>() {
        return Some(barrier.vol_id.clone());
    }
    if let Some(lookback) = instrument.downcast_ref::<lookback_option::LookbackOption>() {
        return Some(lookback.vol_id.clone());
    }
    if let Some(cliquet) = instrument.downcast_ref::<cliquet_option::CliquetOption>() {
        return Some(cliquet.vol_id.clone());
    }
    if let Some(range_accrual) = instrument.downcast_ref::<range_accrual::RangeAccrual>() {
        return Some(range_accrual.vol_id.clone());
    }
    if let Some(quanto) = instrument.downcast_ref::<quanto_option::QuantoOption>() {
        return Some(quanto.vol_id.clone());
    }
    if let Some(swaption) = instrument.downcast_ref::<swaption::Swaption>() {
        return Some(swaption.vol_id.clone());
    }
    if let Some(fx_barrier) = instrument.downcast_ref::<fx_barrier_option::FxBarrierOption>() {
        return Some(fx_barrier.fx_vol_id.clone());
    }

    None
}

/// Extract expiry date from an instrument if available.
///
/// Uses Any downcasting to check for common expiry field patterns.
/// Returns None if the instrument doesn't have a clear expiry concept.
pub fn get_instrument_expiry_for_adaptive(instrument: &dyn std::any::Any) -> Option<Date> {
    use crate::instruments::*;

    // Try downcasting to each instrument type with expiry
    if let Some(eq_opt) = instrument.downcast_ref::<equity_option::EquityOption>() {
        return Some(eq_opt.expiry);
    }
    if let Some(fx_opt) = instrument.downcast_ref::<fx_option::FxOption>() {
        return Some(fx_opt.expiry);
    }
    if let Some(asian) = instrument.downcast_ref::<asian_option::AsianOption>() {
        return Some(asian.expiry);
    }
    if let Some(barrier) = instrument.downcast_ref::<barrier_option::BarrierOption>() {
        return Some(barrier.expiry);
    }
    if let Some(lookback) = instrument.downcast_ref::<lookback_option::LookbackOption>() {
        return Some(lookback.expiry);
    }
    if let Some(cliquet) = instrument.downcast_ref::<cliquet_option::CliquetOption>() {
        // Cliquet doesn't have a single expiry - use last reset date
        return cliquet.reset_dates.last().copied();
    }
    if let Some(range_accrual) = instrument.downcast_ref::<range_accrual::RangeAccrual>() {
        // RangeAccrual doesn't have a single expiry - use last observation date
        return range_accrual.observation_dates.last().copied();
    }
    if let Some(quanto) = instrument.downcast_ref::<quanto_option::QuantoOption>() {
        return Some(quanto.expiry);
    }
    if let Some(swaption) = instrument.downcast_ref::<swaption::Swaption>() {
        return Some(swaption.expiry);
    }
    if let Some(fx_barrier) = instrument.downcast_ref::<fx_barrier_option::FxBarrierOption>() {
        return Some(fx_barrier.expiry);
    }

    None
}

/// Extract day count convention from an instrument if available.
///
/// Uses Any downcasting to check for common day_count field patterns.
/// Returns None if the instrument doesn't have a day_count field.
pub fn get_instrument_day_count(
    instrument: &dyn std::any::Any,
) -> Option<finstack_core::dates::DayCount> {
    use crate::instruments::*;

    // Try downcasting to each instrument type with day_count
    if let Some(eq_opt) = instrument.downcast_ref::<equity_option::EquityOption>() {
        return Some(eq_opt.day_count);
    }
    if let Some(fx_opt) = instrument.downcast_ref::<fx_option::FxOption>() {
        return Some(fx_opt.day_count);
    }
    if let Some(asian) = instrument.downcast_ref::<asian_option::AsianOption>() {
        return Some(asian.day_count);
    }
    if let Some(autocall) = instrument.downcast_ref::<autocallable::Autocallable>() {
        return Some(autocall.day_count);
    }
    if let Some(barrier) = instrument.downcast_ref::<barrier_option::BarrierOption>() {
        return Some(barrier.day_count);
    }
    if let Some(lookback) = instrument.downcast_ref::<lookback_option::LookbackOption>() {
        return Some(lookback.day_count);
    }
    if let Some(cliquet) = instrument.downcast_ref::<cliquet_option::CliquetOption>() {
        return Some(cliquet.day_count);
    }
    if let Some(range_accrual) = instrument.downcast_ref::<range_accrual::RangeAccrual>() {
        return Some(range_accrual.day_count);
    }
    if let Some(quanto) = instrument.downcast_ref::<quanto_option::QuantoOption>() {
        return Some(quanto.day_count);
    }
    if let Some(swaption) = instrument.downcast_ref::<swaption::Swaption>() {
        return Some(swaption.day_count);
    }
    if let Some(fx_barrier) = instrument.downcast_ref::<fx_barrier_option::FxBarrierOption>() {
        return Some(fx_barrier.day_count);
    }

    None
}
