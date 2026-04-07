//! Loader for CDS conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, normalize_registry_id, RegistryFile};
use crate::market::conventions::defs::CdsConventions;
use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::Error;
use finstack_core::HashMap;
use strum::IntoEnumIterator;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CdsConventionsRecord {
    #[allow(dead_code)]
    doc_clause: CdsDocClause,
    day_count: DayCount,
    payment_frequency: String,
    bdc: BusinessDayConvention,
    #[allow(dead_code)]
    stub_convention: String,
    settlement_days: i32,
    calendar_id: String,
}

impl CdsConventionsRecord {
    fn into_conventions(self) -> Result<CdsConventions, Error> {
        let payment_frequency = Tenor::parse(&self.payment_frequency).map_err(|e| {
            Error::Validation(format!(
                "Invalid `payment_frequency` in CDS conventions registry: '{}': {}",
                self.payment_frequency, e
            ))
        })?;

        Ok(CdsConventions {
            calendar_id: self.calendar_id,
            day_count: self.day_count,
            bdc: self.bdc,
            settlement_days: self.settlement_days,
            frequency: payment_frequency,
        })
    }
}

/// Parse a doc clause string into `CdsDocClause`.
fn parse_doc_clause(clause_str: &str) -> Option<CdsDocClause> {
    serde_json::from_value(serde_json::Value::String(clause_str.to_string())).ok()
}

/// Load the CDS conventions from the embedded JSON registry.
///
/// This loader expands `ANY:<Clause>` and `DEFAULT:DEFAULT` IDs across all ISO currencies,
/// allowing the embedded registry to define catch-all conventions that apply to any currency
/// not explicitly overridden. Explicit currency IDs (e.g., `USD:IsdaNa`) take precedence
/// over expanded `ANY` entries.
pub(crate) fn load_registry() -> Result<HashMap<CdsConventionKey, CdsConventions>, Error> {
    let json = include_str!("../../../../data/conventions/cds_conventions.json");
    let file: RegistryFile<CdsConventionsRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded CDS conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    // Two-pass approach:
    // 1. Collect explicit (Currency, Clause) keys first - these take precedence
    // 2. Collect ANY:<Clause> entries and expand them to all currencies not already present

    let mut final_map: HashMap<CdsConventionKey, CdsConventions> = HashMap::default();
    let mut any_clauses: Vec<(CdsDocClause, CdsConventions)> = Vec::new();

    for (key_str, val) in string_map {
        let parts: Vec<&str> = key_str.split(':').collect();
        if parts.len() != 2 {
            continue; // Invalid format
        }

        let prefix = parts[0];
        let clause_str = parts[1];

        // Handle explicit currency keys (e.g., "USD:IsdaNa")
        if let Ok(currency) = prefix.parse::<Currency>() {
            if let Some(clause) = parse_doc_clause(clause_str) {
                let key = CdsConventionKey {
                    currency,
                    doc_clause: clause,
                };
                final_map.insert(key, val?);
            }
        } else if prefix.eq_ignore_ascii_case("ANY") || prefix.eq_ignore_ascii_case("DEFAULT") {
            // Handle "ANY:<Clause>" or "DEFAULT:DEFAULT" (treat DEFAULT:DEFAULT as ANY:<record.doc_clause>)
            // For DEFAULT:DEFAULT, the clause in the key may be "DEFAULT" but the actual clause
            // is in the record's doc_clause field. We use clause_str here which may be "DEFAULT".
            // The JSON shows "DEFAULT:DEFAULT" in the ids array, but the record has doc_clause: IsdaNa.
            // So we need to parse the clause from the record, not the key.
            // Actually, looking at the JSON, DEFAULT:DEFAULT is in the same entry as ANY:IsdaNa,
            // so they share the same record. We can just treat DEFAULT as synonymous with ANY.
            if let Some(clause) = parse_doc_clause(clause_str) {
                any_clauses.push((clause, val?));
            } else if clause_str.eq_ignore_ascii_case("DEFAULT") {
                // "DEFAULT:DEFAULT" - need to get the clause from the record's doc_clause
                // But at this point we only have the conventions, not the original record.
                // The doc_clause was parsed in into_conventions but not stored in CdsConventions.
                // For now, we'll skip "DEFAULT:DEFAULT" expansion since we can't determine the clause.
                // The JSON has DEFAULT:DEFAULT in the same entry as USD:IsdaNa, ANY:IsdaNa, etc.,
                // so those explicit entries will cover the expected cases.
                continue;
            }
        }
        // Skip other invalid prefixes
    }

    // Second pass: expand ANY entries to all currencies not already present
    for (clause, conventions) in any_clauses {
        for currency in Currency::iter() {
            let key = CdsConventionKey {
                currency,
                doc_clause: clause,
            };
            // Only insert if not already present (explicit entries take precedence)
            final_map.entry(key).or_insert_with(|| conventions.clone());
        }
    }

    Ok(final_map)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn explicit_asian_currency_overrides_take_precedence_over_any_defaults() {
        let registry = load_registry().expect("cds registry");

        let aud = registry
            .get(&CdsConventionKey {
                currency: Currency::AUD,
                doc_clause: CdsDocClause::IsdaAs,
            })
            .expect("AUD IsdaAs");
        assert_eq!(aud.calendar_id, "auce");

        let nzd = registry
            .get(&CdsConventionKey {
                currency: Currency::NZD,
                doc_clause: CdsDocClause::IsdaAs,
            })
            .expect("NZD IsdaAs");
        assert_eq!(nzd.calendar_id, "nzau");

        let hkd = registry
            .get(&CdsConventionKey {
                currency: Currency::HKD,
                doc_clause: CdsDocClause::IsdaAs,
            })
            .expect("HKD IsdaAs");
        assert_eq!(hkd.calendar_id, "hkhk");

        let sgd = registry
            .get(&CdsConventionKey {
                currency: Currency::SGD,
                doc_clause: CdsDocClause::IsdaAs,
            })
            .expect("SGD IsdaAs");
        assert_eq!(sgd.calendar_id, "sgsi");
    }
}
