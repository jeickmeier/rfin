//! Instrument-level shock adapters.
//!
//! Applies metadata-driven price and spread shocks to instrument collections,
//! enabling `OperationSpec` variants to affect subsets of a portfolio by type.

use crate::error::Result;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::pricer::InstrumentType;

/// Apply a percentage price shock to instruments matching the provided types.
///
/// # Arguments
/// - `instruments`: Slice of instrument trait objects to mutate.
/// - `instrument_types`: Instrument types that should receive the shock.
/// - `pct`: Percentage change to attach as metadata.
///
/// # Returns
/// [`Result`](crate::error::Result) containing the number of instruments that
/// were updated.
///
/// # Errors
/// Currently always returns `Ok`; the [`Result`](crate::error::Result) wrapper
/// is reserved for future validation failures.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::adapters::instruments::apply_instrument_type_price_shock;
/// use finstack_valuations::instruments::common::traits::Instrument;
/// use finstack_valuations::pricer::InstrumentType;
///
/// fn run(mut instruments: Vec<Box<dyn Instrument>>) -> finstack_scenarios::Result<usize> {
///     apply_instrument_type_price_shock(&mut instruments, &[InstrumentType::Bond], -3.0)
/// }
/// ```
pub fn apply_instrument_type_price_shock(
    instruments: &mut [Box<dyn Instrument>],
    instrument_types: &[InstrumentType],
    pct: f64,
) -> Result<usize> {
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            // Apply price shock via attributes or pricing overrides
            // For now, store the shock as a metadata attribute
            let shock_str = format!("{:.4}", pct);
            instrument
                .attributes_mut()
                .meta
                .insert("scenario_price_shock_pct".to_string(), shock_str);
            count += 1;
        }
    }

    Ok(count)
}

/// Apply a spread shock (basis points) to instruments matching the provided types.
///
/// # Arguments
/// - `instruments`: Slice of instrument trait objects to mutate.
/// - `instrument_types`: Instrument types that should receive the spread shock.
/// - `bp`: Basis-point change to attach as metadata.
///
/// # Returns
/// [`Result`](crate::error::Result) containing the number of instruments that
/// were updated.
///
/// # Errors
/// Currently always returns `Ok`; errors may be introduced in the future to
/// surface validation failures.
///
/// # Examples
/// ```rust
/// use finstack_scenarios::adapters::instruments::apply_instrument_type_spread_shock;
/// use finstack_valuations::instruments::common::traits::Instrument;
/// use finstack_valuations::pricer::InstrumentType;
///
/// fn run(mut instruments: Vec<Box<dyn Instrument>>) -> finstack_scenarios::Result<usize> {
///     apply_instrument_type_spread_shock(&mut instruments, &[InstrumentType::CDS], 25.0)
/// }
/// ```
pub fn apply_instrument_type_spread_shock(
    instruments: &mut [Box<dyn Instrument>],
    instrument_types: &[InstrumentType],
    bp: f64,
) -> Result<usize> {
    let mut count = 0;

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            // Apply spread shock via attributes
            let shock_str = format!("{:.2}", bp);
            instrument
                .attributes_mut()
                .meta
                .insert("scenario_spread_shock_bp".to_string(), shock_str);
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    // Note: Full testing requires concrete instrument implementations
    // These tests are placeholders for when integration testing is added
}
