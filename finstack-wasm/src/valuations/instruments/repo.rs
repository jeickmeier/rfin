use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::error::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use finstack_valuations::instruments::repo::{CollateralSpec, CollateralType, Repo, RepoType};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn parse_repo_type(label: Option<String>) -> Result<RepoType, JsValue> {
    match label.as_deref() {
        None | Some("term") => Ok(RepoType::Term),
        Some("open") => Ok(RepoType::Open),
        Some(s) => s
            .parse()
            .map_err(|e: String| js_error(format!("Invalid repo type: {e}"))),
    }
}

#[wasm_bindgen(js_name = RepoCollateral)]
#[derive(Clone, Debug)]
pub struct JsRepoCollateral {
    inner: CollateralSpec,
}

#[wasm_bindgen(js_class = RepoCollateral)]
impl JsRepoCollateral {
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
    inner: Repo,
}

impl JsRepo {
    pub(crate) fn from_inner(inner: Repo) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Repo {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Repo)]
impl JsRepo {
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
    ) -> Result<JsRepo, JsValue> {
        let repo_type_value = parse_repo_type(repo_type)?;
        let dc = day_count.map(|d| d.inner()).unwrap_or(DayCount::Act360);

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
            .bdc(BusinessDayConvention::Following)
            .disc_id(curve_id_from_str(discount_curve));

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
        format!("Repo(id='{}', rate={:.4})", self.inner.id, self.inner.repo_rate)
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRepo {
        JsRepo::from_inner(self.inner.clone())
    }
}

