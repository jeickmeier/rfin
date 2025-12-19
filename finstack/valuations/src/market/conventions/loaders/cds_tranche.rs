//! Loader for CDS tranche conventions embedded in JSON registries.

use crate::market::conventions::defs::CdsConventions;
use crate::market::conventions::ids::{CdsConventionKey, CdsDocClause};
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::Currency;
use finstack_core::Error;
use std::collections::HashMap;
use serde::{Deserialize, Deserializer};
use serde::de::Error as DeError;
use serde_json::Value;

fn deserialize_business_day_convention<'de, D>(
    deserializer: D,
) -> Result<BusinessDayConvention, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let normalized = raw.to_lowercase();
    BusinessDayConvention::deserialize(Value::String(normalized))
        .map_err(D::Error::custom)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CdsTrancheConventionsRecord {
    id: String,
    #[allow(dead_code)]
    doc_clause: CdsDocClause,
    day_count: DayCount,
    payment_frequency: String,
    #[serde(deserialize_with = "deserialize_business_day_convention")]
    business_day_convention: BusinessDayConvention,
    #[allow(dead_code)]
    stub_convention: String,
    settlement_days: i32,
    calendar_id: String,
}

impl CdsTrancheConventionsRecord {
    fn into_conventions(self) -> Result<CdsConventions, Error> {
        let payment_frequency = Tenor::parse(&self.payment_frequency).map_err(|e| {
            Error::Validation(format!(
                "Invalid `payment_frequency` in CDS Tranche conventions registry: '{}': {}",
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
    id.trim().to_string()
}

/// Load the CDS Tranche conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<CdsConventionKey, CdsConventions>, Error> {
    let json = include_str!("../../../../data/conventions/cds_tranche_conventions.json");
    let records: Vec<CdsTrancheConventionsRecord> = serde_json::from_str(json)
        .map_err(|e| {
            Error::Validation(format!(
                "Failed to parse embedded CDS Tranche conventions registry JSON: {e}"
            ))
        })?;

    let mut final_map = HashMap::new();
    for rec in records {
        let id = normalize_registry_id(&rec.id);
        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() != 2 {
            return Err(Error::Validation(format!(
                "Invalid CDS Tranche convention id '{}', expected format <Ccy>:<DocClause>",
                id
            )));
        }

        let currency = parts[0].parse::<Currency>().map_err(|e| {
            Error::Validation(format!(
                "Invalid currency in CDS Tranche convention id '{}': {}",
                id, e
            ))
        })?;

        let clause: CdsDocClause =
            serde_json::from_value(Value::String(parts[1].to_string())).map_err(|e| {
                Error::Validation(format!(
                    "Invalid doc clause in CDS Tranche convention id '{}': {}",
                    id, e
                ))
            })?;

        let key = CdsConventionKey {
            currency,
            doc_clause: clause,
        };

        let conventions = rec.clone().into_conventions()?;

        if final_map.insert(key, conventions).is_some() {
            return Err(Error::Validation(format!(
                "Duplicate CDS Tranche convention key '{}'",
                id
            )));
        }
    }

    Ok(final_map)
}
