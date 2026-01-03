use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fixed_income::term_loan::TermLoan;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Term loan instrument with DDTL (Delayed Draw Term Loan) support.
///
/// A term loan is a debt instrument with a defined maturity, optional amortization,
/// and support for both fixed and floating rates. The DDTL variant allows for
/// delayed draws during an availability period with commitment fees and usage fees.
#[wasm_bindgen(js_name = TermLoan)]
#[derive(Clone, Debug)]
pub struct JsTermLoan {
    pub(crate) inner: TermLoan,
}

impl InstrumentWrapper for JsTermLoan {
    type Inner = TermLoan;
    fn from_inner(inner: TermLoan) -> Self {
        JsTermLoan { inner }
    }
    fn inner(&self) -> TermLoan {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = TermLoan)]
impl JsTermLoan {
    /// Create a term loan from a JSON string specification.
    ///
    /// The JSON should match the TermLoanSpec schema from finstack-valuations.
    /// This is the recommended way to create complex term loans with DDTL features,
    /// covenants, and custom amortization schedules.
    ///
    /// # Arguments
    /// * `json_str` - JSON string matching the TermLoanSpec schema
    ///
    /// # Returns
    /// A new TermLoan instance
    ///
    /// # Errors
    /// Returns an error if JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsTermLoan, JsValue> {
        serde_json::from_str(json_str)
            .map(JsTermLoan::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Serialize the term loan to a JavaScript object.
    ///
    /// # Returns
    /// JavaScript object representation of the term loan
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this term loan.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use finstack_valuations::cashflow::CashflowProvider;

        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let sched = self
            .inner
            .build_full_schedule(market.inner(), as_of)
            .map_err(|e| js_error(e.to_string()))?;
        let outstanding_path = sched
            .outstanding_path_per_flow()
            .map_err(|e| js_error(e.to_string()))?;

        let result = Array::new();
        for (idx, cf) in sched.flows.iter().enumerate() {
            let entry = Array::new();
            entry.push(&JsDate::from_core(cf.date).into());
            entry.push(&JsMoney::from_inner(cf.amount).into());
            entry.push(&JsValue::from_str(&format!("{:?}", cf.kind)));
            let outstanding = outstanding_path
                .get(idx)
                .map(|(_, m)| m.amount())
                .unwrap_or(0.0);
            entry.push(&JsValue::from_f64(outstanding));
            result.push(&entry);
        }
        Ok(result)
    }

    /// Serialize the term loan to a JSON string.
    ///
    /// # Returns
    /// JSON string representation of the term loan
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Get the instrument identifier.
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the currency code.
    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> String {
        self.inner.currency.to_string()
    }

    /// Get the notional limit.
    #[wasm_bindgen(getter, js_name = notionalLimit)]
    pub fn notional_limit(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional_limit)
    }

    /// Get the issue date.
    #[wasm_bindgen(getter)]
    pub fn issue(&self) -> JsDate {
        JsDate::from_core(self.inner.issue)
    }

    /// Get the maturity date.
    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    /// Get the discount curve identifier.
    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Get the instrument type.
    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::TermLoan as u16
    }

    /// Convert to a string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "TermLoan(id='{}', issue='{}', maturity='{}')",
            self.inner.id, self.inner.issue, self.inner.maturity
        )
    }

    /// Clone the term loan.
    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsTermLoan {
        JsTermLoan::from_inner(self.inner.clone())
    }
}
