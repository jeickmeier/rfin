//! Loader for bond conventions embedded in JSON registries.

use super::json::{build_lookup_map_mapped, normalize_registry_id, RegistryFile};
use crate::instruments::BondConvention;
use crate::market::conventions::defs::BondConventions;
use crate::market::conventions::ids::BondConventionId;
use finstack_core::currency::Currency;
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BondConventionRecord {
    currency: Currency,
    market_convention: BondConvention,
    default_discount_curve_id: String,
}

impl BondConventionRecord {
    fn into_conventions(self) -> Result<BondConventions, Error> {
        Ok(BondConventions {
            currency: self.currency,
            market_convention: self.market_convention,
            default_discount_curve_id: self.default_discount_curve_id,
        })
    }
}

pub fn load_registry() -> Result<HashMap<BondConventionId, BondConventions>, Error> {
    let json = include_str!("../../../../data/conventions/bond_conventions.json");
    let file: RegistryFile<BondConventionRecord> = serde_json::from_str(json).map_err(|e| {
        Error::Validation(format!(
            "Failed to parse embedded bond conventions registry JSON: {e}"
        ))
    })?;

    let string_map = build_lookup_map_mapped(file, normalize_registry_id, |rec| {
        rec.clone().into_conventions()
    })?;

    let mut final_map = HashMap::default();
    for (k, v) in string_map {
        final_map.insert(BondConventionId::new(k), v?);
    }
    Ok(final_map)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn usd_ust_bond_conventions_are_available() {
        let registry = load_registry().expect("bond registry");
        let conv = registry
            .get(&BondConventionId::new("USD-UST"))
            .expect("USD-UST conventions");

        assert_eq!(conv.currency, Currency::USD);
        assert_eq!(conv.market_convention, BondConvention::USTreasury);
        assert_eq!(conv.default_discount_curve_id, "USD-TREASURY");
    }

    #[test]
    fn all_standard_bond_conventions_are_available() {
        let registry = load_registry().expect("bond registry");

        let agency = registry
            .get(&BondConventionId::new("USD-AGENCY"))
            .expect("USD-AGENCY");
        assert_eq!(agency.currency, Currency::USD);
        assert_eq!(agency.market_convention, BondConvention::USAgency);

        let bund = registry
            .get(&BondConventionId::new("EUR-BUND"))
            .expect("EUR-BUND");
        assert_eq!(bund.currency, Currency::EUR);
        assert_eq!(bund.market_convention, BondConvention::GermanBund);

        let gilt = registry
            .get(&BondConventionId::new("GBP-GILT"))
            .expect("GBP-GILT");
        assert_eq!(gilt.currency, Currency::GBP);
        assert_eq!(gilt.market_convention, BondConvention::UKGilt);

        let oat = registry
            .get(&BondConventionId::new("EUR-OAT"))
            .expect("EUR-OAT");
        assert_eq!(oat.currency, Currency::EUR);
        assert_eq!(oat.market_convention, BondConvention::FrenchOAT);

        let jgb = registry
            .get(&BondConventionId::new("JPY-JGB"))
            .expect("JPY-JGB");
        assert_eq!(jgb.currency, Currency::JPY);
        assert_eq!(jgb.market_convention, BondConvention::JGB);
    }
}
