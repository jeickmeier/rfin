use crate::calibration::solver::BootstrapTarget;
use crate::market::conventions::registry::ConventionRegistry;
use crate::market::quotes::rates::RateQuote;
use finstack_core::dates::DayCount;
use finstack_core::Result;

/// Sort bootstrap quotes by strictly increasing knot time.
///
/// Market-standard behavior: quote ordering should not affect calibration outcomes.
/// The core bootstrapper assumes quotes are already sorted, so we enforce that here.
pub fn sort_bootstrap_quotes<T: BootstrapTarget>(
    target: &T,
    quotes: &mut Vec<T::Quote>,
) -> Result<()> {
    if quotes.len() <= 1 {
        return Ok(());
    }

    // Drain + compute times once, then stable-sort by time with deterministic tie-breaker.
    let mut items: Vec<(f64, usize, T::Quote)> = Vec::with_capacity(quotes.len());
    for (idx, q) in quotes.drain(..).enumerate() {
        let t = target.quote_time(&q)?;
        if !t.is_finite() || t <= 0.0 {
            return Err(finstack_core::Error::Calibration {
                message: format!("Bootstrap quote_time must be finite and > 0; got t={}", t),
                category: "bootstrapping".to_string(),
            });
        }
        items.push((t, idx, q));
    }

    items.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    // Enforce strict monotonicity (matches `SequentialBootstrapper` requirements).
    let mut last_t = 0.0_f64;
    for (t, _, _) in &items {
        if *t <= last_t {
            return Err(finstack_core::Error::Calibration {
                message: format!(
                    "Bootstrap requires strictly increasing quote times; got t={:.12} after last_time={:.12}",
                    t, last_t
                ),
                category: "bootstrapping".to_string(),
            });
        }
        last_t = *t;
    }

    quotes.extend(items.into_iter().map(|(_, _, q)| q));
    Ok(())
}

/// Resolve the day count convention for a discount or forward curve from market conventions.
pub fn curve_day_count_from_quotes(quotes: &[RateQuote]) -> Result<DayCount> {
    let registry = ConventionRegistry::global();
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
