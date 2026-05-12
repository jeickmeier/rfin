//! WASM-exposed value-object classes that `Performance` accepts as inputs.

use finstack_analytics as fa;
use wasm_bindgen::prelude::*;

use super::support::{parse_cagr_convention, parse_iso_date};

/// Annualization basis for CAGR.
#[wasm_bindgen(js_name = CagrBasis)]
pub struct WasmCagrBasis {
    #[allow(dead_code)]
    pub(super) inner: fa::risk_metrics::CagrBasis,
}

#[wasm_bindgen(js_class = CagrBasis)]
impl WasmCagrBasis {
    /// Create a factor-based basis from periods per year.
    #[wasm_bindgen(js_name = factor)]
    pub fn factor(ann_factor: f64) -> Self {
        Self {
            inner: fa::risk_metrics::CagrBasis::factor(ann_factor),
        }
    }

    /// Create a date-based basis from ISO dates and an optional convention string.
    #[wasm_bindgen(js_name = dates)]
    pub fn dates(start: &str, end: &str, convention: Option<String>) -> Result<Self, JsValue> {
        Ok(Self {
            inner: fa::risk_metrics::CagrBasis::dates_with_convention(
                parse_iso_date(start)?,
                parse_iso_date(end)?,
                parse_cagr_convention(convention.as_deref())?,
            ),
        })
    }
}

/// Policy for handling missing dates during benchmark alignment.
#[wasm_bindgen(js_name = BenchmarkAlignmentPolicy)]
pub struct WasmBenchmarkAlignmentPolicy {
    #[allow(dead_code)]
    pub(super) inner: fa::benchmark::BenchmarkAlignmentPolicy,
}

#[wasm_bindgen(js_class = BenchmarkAlignmentPolicy)]
impl WasmBenchmarkAlignmentPolicy {
    /// Fill missing benchmark dates with zero returns.
    #[wasm_bindgen(js_name = zeroOnMissing)]
    pub fn zero_on_missing() -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ZeroReturnOnMissingDates,
        }
    }

    /// Raise an error if benchmark dates don't cover all target dates.
    #[wasm_bindgen(js_name = errorOnMissing)]
    pub fn error_on_missing() -> Self {
        Self {
            inner: fa::benchmark::BenchmarkAlignmentPolicy::ErrorOnMissingDates,
        }
    }
}
