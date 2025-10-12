//! Instrument-level shock adapters.

use crate::error::Result;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::pricer::InstrumentType;

/// Apply percent shock to instruments matching given types.
///
/// This operates on a vector of instruments and applies a price shock
/// to all instruments whose type matches one of the target types.
///
/// Returns the count of affected instruments.
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

/// Apply spread shock (in bp) to instruments matching given types.
///
/// This is primarily applicable to fixed income instruments (bonds, CDS, etc.).
///
/// Returns the count of affected instruments.
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
