//! Instrument-level shock adapters.
//!
//! Applies price and spread shocks to instrument collections via pricing overrides,
//! enabling `OperationSpec` variants to affect subsets of a portfolio by type.
//! When instruments support pricing_overrides_mut(), shocks are applied functionally.
//! Otherwise, shocks are stored as metadata attributes for downstream processing.

use crate::error::Result;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::pricer::InstrumentType;

/// Apply a percentage price shock to instruments matching the provided types.
///
/// # Arguments
/// - `instruments`: Slice of instrument trait objects to mutate.
/// - `instrument_types`: Instrument types that should receive the shock.
/// - `pct`: Percentage change (e.g., -3.0 for -3% price shock).
///
/// # Returns
/// [`Result`](crate::error::Result) containing the number of instruments that
/// were updated.
///
/// # Pricing Effect
///
/// When the instrument supports `pricing_overrides_mut()`, the shock is applied
/// to the `scenario_price_shock_pct` field, which affects actual pricing:
/// - New price = Base price × (1 + shock_pct/100)
///
/// For instruments without pricing override support, the shock is stored as
/// metadata for downstream processing.
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
    let shock_decimal = pct / 100.0; // Convert percentage to decimal

    for instrument in instruments.iter_mut() {
        let inst_type = instrument.key();

        if instrument_types.contains(&inst_type) {
            // Try to apply via scenario_overrides for functional pricing effect
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_price_shock_pct = Some(shock_decimal);
            } else {
                // Fallback: store as metadata for downstream processing
                let shock_str = format!("{:.6}", shock_decimal);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_price_shock_pct".to_string(), shock_str);
            }
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
/// - `bp`: Basis-point change (e.g., 25.0 for +25bp spread widening).
///
/// # Returns
/// [`Result`](crate::error::Result) containing the number of instruments that
/// were updated.
///
/// # Pricing Effect
///
/// When the instrument supports `pricing_overrides_mut()`, the shock is applied
/// to the `scenario_spread_shock_bp` field, which affects actual pricing:
/// - New spread = Base spread + shock_bp
///
/// For instruments without pricing override support, the shock is stored as
/// metadata for downstream processing.
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
            // Try to apply via scenario_overrides for functional pricing effect
            if let Some(overrides) = instrument.scenario_overrides_mut() {
                overrides.scenario_spread_shock_bp = Some(bp);
            } else {
                // Fallback: store as metadata for downstream processing
                let shock_str = format!("{:.2}", bp);
                instrument
                    .attributes_mut()
                    .meta
                    .insert("scenario_spread_shock_bp".to_string(), shock_str);
            }
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
