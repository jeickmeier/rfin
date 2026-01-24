use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::repo::{
    CollateralSpec, CollateralType, Repo, RepoType,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = RepoBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsRepoBuilder {
    instrument_id: String,
    cash_amount: Option<finstack_core::money::Money>,
    collateral: Option<CollateralSpec>,
    repo_rate: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    repo_type: Option<String>,
    haircut: Option<f64>,
    day_count: Option<finstack_core::dates::DayCount>,
    business_day_convention: Option<finstack_core::dates::BusinessDayConvention>,
}

#[wasm_bindgen(js_class = RepoBuilder)]
impl JsRepoBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsRepoBuilder {
        JsRepoBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, cash_amount: &JsMoney) -> JsRepoBuilder {
        self.cash_amount = Some(cash_amount.inner());
        self
    }

    #[wasm_bindgen(js_name = collateral)]
    pub fn collateral(mut self, collateral: &JsRepoCollateral) -> JsRepoBuilder {
        self.collateral = Some(collateral.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = repoRate)]
    pub fn repo_rate(mut self, repo_rate: f64) -> JsRepoBuilder {
        self.repo_rate = Some(repo_rate);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &JsDate) -> JsRepoBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsRepoBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsRepoBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = repoType)]
    pub fn repo_type(mut self, repo_type: String) -> JsRepoBuilder {
        self.repo_type = Some(repo_type);
        self
    }

    #[wasm_bindgen(js_name = haircut)]
    pub fn haircut(mut self, haircut: f64) -> JsRepoBuilder {
        self.haircut = Some(haircut);
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsRepoBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn business_day_convention(
        mut self,
        business_day_convention: JsBusinessDayConvention,
    ) -> JsRepoBuilder {
        self.business_day_convention = Some(business_day_convention.into());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsRepo, JsValue> {
        let cash_amount = self
            .cash_amount
            .ok_or_else(|| js_error("RepoBuilder: cash amount (money) is required".to_string()))?;
        let collateral = self
            .collateral
            .ok_or_else(|| js_error("RepoBuilder: collateral is required".to_string()))?;
        let repo_rate = self
            .repo_rate
            .ok_or_else(|| js_error("RepoBuilder: repoRate is required".to_string()))?;
        let start_date = self
            .start_date
            .ok_or_else(|| js_error("RepoBuilder: startDate is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("RepoBuilder: maturity is required".to_string()))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("RepoBuilder: discountCurve is required".to_string()))?;

        let repo_type_value = parse_optional_with_default(self.repo_type, RepoType::Term)?;
        let dc = self.day_count.unwrap_or(DayCount::Act360);
        let bdc = self
            .business_day_convention
            .unwrap_or(finstack_core::dates::BusinessDayConvention::Following);

        Repo::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .cash_amount(cash_amount)
            .collateral(collateral)
            .repo_rate(repo_rate)
            .start_date(start_date)
            .maturity(maturity)
            .haircut(self.haircut.unwrap_or(0.0))
            .repo_type(repo_type_value)
            .triparty(false)
            .day_count(dc)
            .bdc(bdc)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .build()
            .map(JsRepo::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = RepoCollateral)]
#[derive(Clone, Debug)]
pub struct JsRepoCollateral {
    pub(crate) inner: CollateralSpec,
}

#[wasm_bindgen(js_class = RepoCollateral)]
impl JsRepoCollateral {
    /// Create a repo collateral specification.
    ///
    /// @param instrument_id - Identifier of the collateral instrument (e.g. CUSIP)
    /// @param quantity - Quantity of collateral units
    /// @param market_value_id - Market scalar/price id used to value the collateral
    /// @returns A `RepoCollateral`
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str, quantity: f64, market_value_id: &str) -> JsRepoCollateral {
        JsRepoCollateral {
            inner: CollateralSpec {
                collateral_type: CollateralType::General,
                instrument_id: instrument_id.to_string(),
                quantity,
                market_value_id: market_value_id.to_string(),
            },
        }
    }
}

#[wasm_bindgen(js_name = Repo)]
#[derive(Clone, Debug)]
pub struct JsRepo {
    pub(crate) inner: Repo,
}

impl InstrumentWrapper for JsRepo {
    type Inner = Repo;
    fn from_inner(inner: Repo) -> Self {
        JsRepo { inner }
    }
    fn inner(&self) -> Repo {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Repo)]
impl JsRepo {
    /// Create a repurchase agreement (repo).
    ///
    /// Conventions:
    /// - `repo_rate` is a **decimal rate** (e.g. `0.05` for 5%).
    /// - `haircut` is a fraction in **decimal** (e.g. `0.02` for 2%).
    ///
    /// @param instrument_id - Unique identifier
    /// @param cash_amount - Cash amount exchanged (currency-tagged)
    /// @param collateral - Collateral specification
    /// @param repo_rate - Repo rate (decimal)
    /// @param start_date - Start date
    /// @param maturity - End/maturity date
    /// @param discount_curve - Discount curve ID
    /// @param repo_type - Optional repo type string (e.g. `"term"`)
    /// @param haircut - Optional haircut (decimal)
    /// @param day_count - Optional day count (defaults Act/360)
    /// @param business_day_convention - Optional business day convention
    /// @returns A new `Repo`
    /// @throws {Error} If inputs are invalid or parsing fails
    ///
    /// @example
    /// ```javascript
    /// import init, { Repo, RepoCollateral, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const collateral = new RepoCollateral("UST-10Y", 100.0, "UST-10Y-PRICE");
    /// const repo = new Repo(
    ///   "repo_1",
    ///   Money.fromCode(10_000_000, "USD"),
    ///   collateral,
    ///   0.05,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2024, 2, 2),
    ///   "USD-OIS",
    ///   "term",
    ///   0.02
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        cash_amount: &JsMoney,
        collateral: &JsRepoCollateral,
        repo_rate: f64,
        start_date: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        repo_type: Option<String>,
        haircut: Option<f64>,
        day_count: Option<JsDayCount>,
        business_day_convention: Option<JsBusinessDayConvention>,
    ) -> Result<JsRepo, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "Repo constructor is deprecated; use RepoBuilder instead.",
        ));
        let repo_type_value = parse_optional_with_default(repo_type, RepoType::Term)?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);
        let bdc = business_day_convention
            .map(Into::<finstack_core::dates::BusinessDayConvention>::into)
            .unwrap_or(finstack_core::dates::BusinessDayConvention::Following);

        let builder = Repo::builder()
            .id(instrument_id_from_str(instrument_id))
            .cash_amount(cash_amount.inner())
            .collateral(collateral.inner.clone())
            .repo_rate(repo_rate)
            .start_date(start_date.inner())
            .maturity(maturity.inner())
            .haircut(haircut.unwrap_or(0.0))
            .repo_type(repo_type_value)
            .triparty(false)
            .day_count(dc)
            .bdc(bdc)
            .discount_curve_id(curve_id_from_str(discount_curve));

        builder
            .build()
            .map(JsRepo::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsRepo, JsValue> {
        from_js_value(value).map(JsRepo::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get cashflows for this repo.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::money::JsMoney;
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

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = cashAmount)]
    pub fn cash_amount(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.cash_amount)
    }

    #[wasm_bindgen(getter, js_name = repoRate)]
    pub fn repo_rate(&self) -> f64 {
        self.inner.repo_rate
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Repo as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Repo(id='{}', rate={:.4})",
            self.inner.id, self.inner.repo_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRepo {
        JsRepo::from_inner(self.inner.clone())
    }
}
