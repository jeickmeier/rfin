use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::rates::repo::{
    CollateralSpec, CollateralType, Repo, RepoType,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

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
