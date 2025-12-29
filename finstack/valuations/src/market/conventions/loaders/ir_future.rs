//! Loader for interest rate future conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, RegistryFile};
use crate::market::conventions::defs::IrFutureConventions;
use crate::market::conventions::ids::{IndexId, IrFutureContractId};
use finstack_core::HashMap;
use finstack_core::Error;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct IrFutureConventionsRecord {
    index_id: String,
    calendar_id: String,
    settlement_days: i32,
    delivery_months: u8,
    face_value: f64,
    tick_size: f64,
    tick_value: f64,
    #[serde(default)]
    convexity_adjustment: Option<f64>,
}

impl IrFutureConventionsRecord {
    fn into_conventions(self) -> Result<IrFutureConventions, Error> {
        if self.delivery_months == 0 {
            return Err(Error::Validation(
                "IR future conventions delivery_months must be > 0".to_string(),
            ));
        }
        if !self.face_value.is_finite() || self.face_value <= 0.0 {
            return Err(Error::Validation(
                "IR future conventions face_value must be positive".to_string(),
            ));
        }
        if !self.tick_size.is_finite() || self.tick_size <= 0.0 {
            return Err(Error::Validation(
                "IR future conventions tick_size must be positive".to_string(),
            ));
        }
        if !self.tick_value.is_finite() || self.tick_value <= 0.0 {
            return Err(Error::Validation(
                "IR future conventions tick_value must be positive".to_string(),
            ));
        }
        if self.settlement_days < 0 {
            return Err(Error::Validation(
                "IR future conventions settlement_days must be non-negative".to_string(),
            ));
        }

        Ok(IrFutureConventions {
            index_id: IndexId::new(self.index_id),
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            delivery_months: self.delivery_months,
            face_value: self.face_value,
            tick_size: self.tick_size,
            tick_value: self.tick_value,
            convexity_adjustment: self.convexity_adjustment,
        })
    }
}

fn normalize_registry_id(id: &str) -> String {
    id.trim().to_string()
}

/// Load the IR futures conventions from the embedded JSON registry.
pub fn load_registry() -> Result<HashMap<IrFutureContractId, IrFutureConventions>, Error> {
    let json = include_str!("../../../../data/conventions/ir_future_conventions.json");
    let file: RegistryFile<IrFutureConventionsRecord> =
        serde_json::from_str(json).map_err(|e| {
            Error::Validation(format!(
                "Failed to parse embedded IR future conventions registry JSON: {e}"
            ))
        })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(IrFutureContractId::new(k), v?);
    }
    Ok(final_map)
}
