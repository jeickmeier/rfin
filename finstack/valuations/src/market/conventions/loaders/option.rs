//! Loader for option conventions embedded in JSON registries.

use crate::market::conventions::defs::OptionConventions;
use crate::market::conventions::ids::OptionConventionId;
use finstack_core::dates::BusinessDayConvention;
use finstack_core::Error;
use finstack_core::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OptionConventionRecord {
    calendar_id: String,
    settlement_days: i32,
    business_day_convention: BusinessDayConvention,
}

impl OptionConventionRecord {
    fn into_conventions(self) -> Result<OptionConventions, Error> {
        Ok(OptionConventions {
            calendar_id: self.calendar_id,
            settlement_days: self.settlement_days,
            business_day_convention: self.business_day_convention,
        })
    }
}

/// Load the Option conventions from the embedded JSON registry.
pub(crate) fn load_registry() -> Result<HashMap<OptionConventionId, OptionConventions>, Error> {
    let json = include_str!("../../../../data/conventions/option_conventions.json");
    super::json::parse_and_rekey(
        json,
        "Option",
        OptionConventionId::new,
        |rec: &OptionConventionRecord| rec.clone().into_conventions(),
    )
}
