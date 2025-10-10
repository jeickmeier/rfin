//! Value type bindings for statements.

use crate::core::currency::JsCurrency;
use finstack_statements::types::AmountOrScalar;
use wasm_bindgen::prelude::*;

/// Amount or scalar value.
///
/// Represents either a plain scalar number or a currency amount.
/// This enables currency-safe arithmetic in statement models.
#[wasm_bindgen]
pub struct JsAmountOrScalar {
    pub(crate) inner: AmountOrScalar,
}

#[wasm_bindgen]
impl JsAmountOrScalar {
    /// Create a scalar value (no currency).
    ///
    /// # Arguments
    /// * `value` - Numeric value
    ///
    /// # Returns
    /// Amount or scalar instance
    #[wasm_bindgen(js_name = scalar)]
    pub fn scalar(value: f64) -> JsAmountOrScalar {
        JsAmountOrScalar {
            inner: AmountOrScalar::scalar(value),
        }
    }

    /// Create a currency amount.
    ///
    /// # Arguments
    /// * `value` - Numeric value
    /// * `currency` - Currency code
    ///
    /// # Returns
    /// Amount or scalar instance
    #[wasm_bindgen(js_name = amount)]
    pub fn amount(value: f64, currency: &JsCurrency) -> JsAmountOrScalar {
        let currency_inner = currency.inner();
        
        JsAmountOrScalar {
            inner: AmountOrScalar::amount(value, currency_inner),
        }
    }

    /// Check if this is a scalar value.
    ///
    /// # Returns
    /// True if scalar, false if currency amount
    #[wasm_bindgen(js_name = isScalar)]
    pub fn is_scalar(&self) -> bool {
        matches!(self.inner, AmountOrScalar::Scalar(_))
    }

    /// Check if this is a currency amount.
    ///
    /// # Returns
    /// True if currency amount, false if scalar
    #[wasm_bindgen(js_name = isAmount)]
    pub fn is_amount(&self) -> bool {
        matches!(self.inner, AmountOrScalar::Amount(_))
    }

    /// Get the numeric value.
    ///
    /// # Returns
    /// Numeric value (works for both scalar and amount)
    #[wasm_bindgen(js_name = getValue)]
    pub fn get_value(&self) -> f64 {
        match &self.inner {
            AmountOrScalar::Scalar(v) => *v,
            AmountOrScalar::Amount(m) => m.amount(),
        }
    }

    /// Get the currency if this is an amount.
    ///
    /// # Returns
    /// Currency or null if scalar
    #[wasm_bindgen(js_name = getCurrency)]
    pub fn get_currency(&self) -> Option<JsCurrency> {
        match &self.inner {
            AmountOrScalar::Amount(m) => {
                let currency_str = m.currency().to_string();
                JsCurrency::new(&currency_str).ok()
            },
            AmountOrScalar::Scalar(_) => None,
        }
    }

    /// Create from JSON representation.
    ///
    /// # Arguments
    /// * `value` - JavaScript object
    ///
    /// # Returns
    /// Amount or scalar instance
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsAmountOrScalar, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsAmountOrScalar { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize AmountOrScalar: {}", e)))
    }

    /// Convert to JSON representation.
    ///
    /// # Returns
    /// JavaScript object
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize AmountOrScalar: {}", e)))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        match &self.inner {
            AmountOrScalar::Scalar(v) => format!("{}", v),
            AmountOrScalar::Amount(m) => format!("{} {}", m.amount(), m.currency()),
        }
    }
}

impl JsAmountOrScalar {
    #[allow(dead_code)]
    pub(crate) fn new(inner: AmountOrScalar) -> Self {
        Self { inner }
    }
}

