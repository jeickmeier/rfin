use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::DayCount;
use finstack_core::Result;

/// Resolve the day count convention for a discount or forward curve from market conventions.
pub(crate) fn curve_day_count_from_quotes(quotes: &[RateQuote]) -> Result<DayCount> {
    let registry = ConventionRegistry::try_global()?;
    let mut curve_dc: Option<DayCount> = None;

    for q in quotes {
        let index_id = match q {
            RateQuote::Deposit { index, .. } => index.clone(),
            RateQuote::Fra { index, .. } => index.clone(),
            RateQuote::Swap { index, .. } => index.clone(),
            RateQuote::Futures { contract, .. } => {
                registry.require_ir_future(contract)?.index_id.clone()
            }
        };

        let idx_conv = registry.require_rate_index(&index_id)?;
        match curve_dc {
            Some(dc) if dc != idx_conv.day_count => {
                return Err(finstack_core::Error::Validation(format!(
                    "Mixed rate index day counts for curve construction: got {:?} and {:?}",
                    dc, idx_conv.day_count
                )));
            }
            Some(_) => {}
            None => curve_dc = Some(idx_conv.day_count),
        }
    }

    curve_dc.ok_or_else(|| {
        finstack_core::Error::Validation(
            "Unable to resolve curve day count: no rate quotes provided".to_string(),
        )
    })
}
