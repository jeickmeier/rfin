#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::F;
use hashbrown::HashMap;

#[derive(Clone, Debug)]
pub struct ValuationResult {
    pub instrument_id: String,
    pub as_of: Date,
    pub value: Money,
    pub measures: HashMap<String, F>,
    pub meta: ResultsMeta,
}

impl ValuationResult {
    pub fn stamped<S: Into<String>>(instrument_id: S, as_of: Date, value: Money) -> Self {
        Self {
            instrument_id: instrument_id.into(),
            as_of,
            value,
            measures: HashMap::new(),
            meta: finstack_core::config::results_meta(),
        }
    }
}


