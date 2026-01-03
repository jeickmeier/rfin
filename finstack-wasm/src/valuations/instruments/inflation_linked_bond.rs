use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::InflationLinkedBondParams;
use finstack_valuations::instruments::fixed_income::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = InflationLinkedBond)]
#[derive(Clone, Debug)]
pub struct JsInflationLinkedBond {
    pub(crate) inner: InflationLinkedBond,
}

impl JsInflationLinkedBond {
    pub(crate) fn from_inner(inner: InflationLinkedBond) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> InflationLinkedBond {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = InflationLinkedBond)]
impl JsInflationLinkedBond {
    /// Create an inflation-linked bond (TIPS-style by default).
    ///
    /// Conventions:
    /// - `real_coupon` is a **decimal rate** (e.g. `0.015` for 1.5% real coupon).
    /// - `base_index` is the CPI/index level at `issue` used for index ratio normalization.
    /// - Discounting uses `discount_curve` and indexation uses `inflation_curve`.
    /// - `indexation` / `deflation_protection` are parsed from strings; unsupported values will throw.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param real_coupon - Real coupon rate (decimal)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param base_index - CPI/index level at base date (typically issue date)
    /// @param discount_curve - Discount curve ID
    /// @param inflation_curve - Inflation index/curve ID
    /// @param indexation - Optional indexation method (e.g. `"tips"`)
    /// @param frequency - Optional coupon frequency (e.g. `"6M"`, `"1Y"`)
    /// @param day_count - Optional day count (defaults to Act/Act)
    /// @param deflation_protection - Optional deflation protection mode
    /// @returns A new `InflationLinkedBond`
    /// @throws {Error} If parsing fails or inputs are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { InflationLinkedBond, Money, FsDate, DayCount } from "finstack-wasm";
    ///
    /// await init();
    /// const ilb = new InflationLinkedBond(
    ///   "tips_1",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   0.015,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2034, 1, 2),
    ///   310.25,
    ///   "USD-OIS",
    ///   "US-CPI",
    ///   "tips",
    ///   "6M",
    ///   DayCount.actAct(),
    ///   null
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        real_coupon: f64,
        issue: &JsDate,
        maturity: &JsDate,
        base_index: f64,
        discount_curve: &str,
        inflation_curve: &str,
        indexation: Option<String>,
        frequency: Option<String>,
        day_count: Option<JsDayCount>,
        deflation_protection: Option<String>,
    ) -> Result<JsInflationLinkedBond, JsValue> {
        let indexation_method = parse_optional_with_default(indexation, IndexationMethod::TIPS)?;
        let freq = parse_optional_with_default(frequency, Tenor::semi_annual())?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::ActAct);
        let deflation =
            parse_optional_with_default(deflation_protection, DeflationProtection::MaturityOnly)?;

        let params = InflationLinkedBondParams::new(
            notional.inner(),
            real_coupon,
            issue.inner(),
            maturity.inner(),
            base_index,
            freq,
            dc,
        );

        let builder = InflationLinkedBond::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(params.notional)
            .real_coupon(params.real_coupon)
            .freq(params.frequency)
            .dc(params.day_count)
            .issue(params.issue)
            .maturity(params.maturity)
            .base_index(params.base_index)
            .base_date(params.issue)
            .indexation_method(indexation_method)
            .lag(indexation_method.standard_lag())
            .deflation_protection(deflation)
            .bdc(BusinessDayConvention::Following)
            .stub(StubKind::None)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .inflation_index_id(curve_id_from_str(inflation_curve))
            .attributes(Default::default());

        builder
            .build()
            .map(JsInflationLinkedBond::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = realCoupon)]
    pub fn real_coupon(&self) -> f64 {
        self.inner.real_coupon
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsInflationLinkedBond, JsValue> {
        from_js_value(value).map(JsInflationLinkedBond::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this inflation-linked bond.
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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::InflationLinkedBond as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "InflationLinkedBond(id='{}', coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsInflationLinkedBond {
        JsInflationLinkedBond::from_inner(self.inner.clone())
    }
}
