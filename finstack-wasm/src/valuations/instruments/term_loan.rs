use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fixed_income::term_loan::TermLoan;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = TermLoanBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsTermLoanBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = TermLoanBuilder)]
impl JsTermLoanBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsTermLoanBuilder {
        JsTermLoanBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsTermLoanBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsTermLoan, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("TermLoanBuilder: jsonString is required"))?;
        serde_json::from_str(json_str)
            .map(JsTermLoan::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

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
            .cashflow_schedule(market.inner(), as_of)
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
        JsDate::from_core(self.inner.issue_date)
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
    pub fn instrument_type(&self) -> String {
        InstrumentType::TermLoan.to_string()
    }

    /// Convert to a string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "TermLoan(id='{}', issue='{}', maturity='{}')",
            self.inner.id, self.inner.issue_date, self.inner.maturity
        )
    }

    /// Clone the term loan.
    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsTermLoan {
        JsTermLoan::from_inner(self.inner.clone())
    }
}
