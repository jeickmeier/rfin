use crate::dates::DayCount;

/// Convention defaults inferred from a forward-curve identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ForwardConventionDefaults {
    pub day_count: DayCount,
    pub reset_lag_business_days: i32,
}

#[inline]
fn normalize_curve_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

#[inline]
fn leading_currency_code(normalized_id: &str) -> Option<&str> {
    match normalized_id.split(['-', '_']).next() {
        Some(
            code @ ("USD" | "EUR" | "GBP" | "JPY" | "CHF" | "CAD" | "AUD" | "NZD" | "SEK" | "NOK"),
        ) => Some(code),
        _ => None,
    }
}

#[inline]
fn contains_any(normalized_id: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| normalized_id.contains(needle))
}

#[inline]
fn has_explicit_term_marker(normalized_id: &str) -> bool {
    contains_any(
        normalized_id,
        &[
            "1D", "1W", "2W", "1M", "2M", "3M", "6M", "9M", "12M", "18M", "1Y",
        ],
    )
}

#[inline]
fn inferred_currency_day_count(currency: &str) -> DayCount {
    match currency {
        "USD" | "EUR" | "CHF" | "SEK" | "NOK" => DayCount::Act360,
        "GBP" | "JPY" | "CAD" | "AUD" | "NZD" => DayCount::Act365F,
        _ => DayCount::Act365F,
    }
}

/// Infer a market-standard day-count basis from a curve identifier.
///
/// The fallback remains `Act365F` for synthetic IDs that carry no market hint.
#[inline]
pub(crate) fn infer_discount_curve_day_count(id: &str) -> DayCount {
    let normalized_id = normalize_curve_id(id);

    if contains_any(
        &normalized_id,
        &["SOFR", "FEDFUNDS", "EFFR", "ESTR", "EURIBOR", "SARON"],
    ) {
        return DayCount::Act360;
    }

    if contains_any(
        &normalized_id,
        &[
            "SONIA", "TONAR", "TONA", "TIBOR", "CORRA", "CDOR", "AONIA", "BBSW", "BKBM",
        ],
    ) {
        return DayCount::Act365F;
    }

    if let Some(currency) = leading_currency_code(&normalized_id) {
        return inferred_currency_day_count(currency);
    }

    DayCount::Act365F
}

/// Infer forward-curve day-count and reset-lag defaults from an index identifier.
///
/// Reset lag is interpreted in business days using positive T-minus semantics.
#[inline]
pub(crate) fn infer_forward_curve_defaults(id: &str) -> ForwardConventionDefaults {
    let normalized_id = normalize_curve_id(id);
    let day_count = infer_discount_curve_day_count(id);

    let is_overnight = normalized_id.contains("OIS")
        || contains_any(
            &normalized_id,
            &[
                "SONIA", "TONAR", "TONA", "SARON", "ESTR", "FEDFUNDS", "EFFR", "CORRA", "AONIA",
            ],
        )
        || (normalized_id.contains("SOFR") && !has_explicit_term_marker(&normalized_id));

    let reset_lag_business_days = if is_overnight {
        0
    } else if contains_any(
        &normalized_id,
        &["SOFR", "EURIBOR", "LIBOR", "TIBOR", "BBSW", "CDOR", "BKBM"],
    ) {
        2
    } else {
        0
    };

    ForwardConventionDefaults {
        day_count,
        reset_lag_business_days,
    }
}
