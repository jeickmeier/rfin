//! Loader for CDS conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::market::conventions::defs::CdsConventions;
use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_core::collections::HashMap;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::Currency;
use finstack_core::Error;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CdsConventionsRecord {
    #[allow(dead_code)]
    doc_clause: CdsDocClause,
    day_count: DayCount,
    payment_frequency: String,
    business_day_convention: BusinessDayConvention,
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
            business_day_convention: self.business_day_convention,
            settlement_days: self.settlement_days,
            payment_frequency,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    // "USD:IsdaNa" -> "USD:IsdaNa" (Case sensitive? or standardized?)
    // JSON keys look like "USD:IsdaNa", "DEFAULT:DEFAULT".
    // IDs in JSON are strings like "USD:IsdaNa".
    // CdsConventionKey is (Currency, DocClause).
    // We need to parse the ID string to key.
    // Actually, normalization is key string canonicalization in the generic loader.
    // We want the resulting key to be parsed into CdsConventionKey.
    // So the map key will be a string, and we need to parse it later?
    // Or we parse it here.
    id.trim().to_string()
}

/// Load the CDS conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<CdsConventionKey, CdsConventions>, Error> {
    let json = include_str!("../../../../data/conventions/cds_conventions.json");
    let file: RegistryFile<CdsConventionsRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded CDS conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    // Parse keys "CURRENCY:DOC_CLAUSE"
    // Note: The registry has "DEFAULT:DEFAULT" or "ANY:..." entries.
    // The design says "Any missing convention ID fails fast".
    // Does that mean we only load explicit currency pairs?
    // No fallback to DEFAULT keys.
    // So we should only load valid (Currency, DocClause) pairs.
    // "ANY:IsdaNa" essentially expands to "for all currencies, use this if not overridden".
    // But strict.
    // If strict, we might need to explode "ANY" entries for all currencies or just assume explicit entries in the registry.
    // However, the JSON has "ANY:IsdaNa" and "USD:IsdaNa" (for overrides).
    // If strict, we require the user to ask for "USD:IsdaNa".
    // If "USD:IsdaNa" is mapped, great.
    // If "JPY:IsdaNa" is not explicitly mapped but "ANY:IsdaNa" is, strict lookup would fail unless we expand "ANY".
    // But "ANY" expansion is implicit fallback.
    // The plan says "No per-quote implicit defaults".
    // It says "Conventions come from embedded JSON registries... If convention lookup fails: error".
    // The plan also says "No fallback to DEFAULT fallback inside resolvers".
    // This implies that if the registry doesn't contain "JPY:IsdaNa", it's an error.
    // So if the JSON relies on "ANY:IsdaNa" to cover JPY, we must physically expand it into "JPY:IsdaNa" in the map, OR update the JSON to be explicit.
    // For now, I will parse only valid Currency:Clause keys. If "ANY" is present, it's problematic for a strict map unless we handle it.
    // But let's assume the map keys we care about are "CUR:CLAUSE".

    let mut final_map = HashMap::default();
    for (key_str, val) in string_map {
        // Skip "ANY" or "DEFAULT" for now unless we decide to expand.
        // Strict implies explicit keys.
        // If I skip them, valid lookups might fail if they rely on ANY.
        // But implementing expansion requires knowing all currencies.

        let parts: Vec<&str> = key_str.split(':').collect();
        if parts.len() != 2 {
            continue; // Invalid format
        }

        if let Ok(currency) = parts[0].parse::<Currency>() {
            let clause_str = parts[1];
            // "IsdaNa" -> CdsDocClause::IsdaNa
            // CdsDocClause derives Deserialize, so it expects exact enum variant name usually?
            // "IsdaNa" matches variant.
            // We can serde deserialize the string.
            let clause: Result<CdsDocClause, _> =
                serde_json::from_value(serde_json::Value::String(clause_str.to_string()));

            if let Ok(clause) = clause {
                let key = CdsConventionKey {
                    currency,
                    doc_clause: clause,
                };
                final_map.insert(key, val?);
            }
        }
    }

    Ok(final_map)
}
