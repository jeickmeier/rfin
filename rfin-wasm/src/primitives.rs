//! WASM bindings for primitives module.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Currency code representation
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Currency {
    code: String,
}

#[wasm_bindgen]
impl Currency {
    /// Create a new currency
    #[wasm_bindgen(constructor)]
    pub fn new(code: String) -> Self {
        Currency { code }
    }

    /// Get the currency code
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }
}

/// Money representation
#[wasm_bindgen]
pub struct Money {
    // TODO: Wrap rfin_core::primitives::money::Money
    amount: f64,
    currency: Currency,
}

#[wasm_bindgen]
impl Money {
    /// Create new money instance
    #[wasm_bindgen(constructor)]
    pub fn new(amount: f64, currency: Currency) -> Self {
        Money { amount, currency }
    }

    /// Get the amount
    #[wasm_bindgen(getter)]
    pub fn amount(&self) -> f64 {
        self.amount
    }

    /// Get the currency
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Currency {
        self.currency.clone()
    }
}
