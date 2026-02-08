//! WASM bindings for Agency MBS instruments.
//!
//! This module provides JavaScript bindings for:
//! - `AgencyMbsPassthrough` - Agency mortgage-backed security passthrough
//! - `AgencyTba` - To-Be-Announced forward contract
//! - `DollarRoll` - Dollar roll between TBA months
//! - `AgencyCmo` - Collateralized mortgage obligation

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::cashflow::builder::specs::PrepaymentModelSpec;
use finstack_valuations::instruments::fixed_income::cmo::{
    AgencyCmo, CmoTranche, CmoTrancheType, CmoWaterfall, PacCollar,
};
use finstack_valuations::instruments::fixed_income::dollar_roll::DollarRoll;
use finstack_valuations::instruments::fixed_income::mbs_passthrough::{
    AgencyMbsPassthrough, AgencyProgram, PoolType,
};
use finstack_valuations::instruments::fixed_income::tba::{AgencyTba, TbaTerm};
use finstack_valuations::instruments::Attributes;
use wasm_bindgen::prelude::*;

// =============================================================================
// Agency MBS Passthrough
// =============================================================================

/// JavaScript representation of an agency MBS passthrough.
#[wasm_bindgen(js_name = AgencyMbsPassthrough)]
#[derive(Clone, Debug)]
pub struct JsAgencyMbsPassthrough {
    pub(crate) inner: AgencyMbsPassthrough,
}

impl InstrumentWrapper for JsAgencyMbsPassthrough {
    type Inner = AgencyMbsPassthrough;
    fn from_inner(inner: AgencyMbsPassthrough) -> Self {
        JsAgencyMbsPassthrough { inner }
    }
    fn inner(&self) -> AgencyMbsPassthrough {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = AgencyMbsPassthrough)]
impl JsAgencyMbsPassthrough {
    /// Create a new agency MBS passthrough.
    ///
    /// @param instrumentId - Unique identifier
    /// @param poolId - Pool identifier (CUSIP or internal ID)
    /// @param agency - Agency program ("fnma", "fhlmc", "gnma")
    /// @param originalFace - Original face value
    /// @param currentFace - Current face value
    /// @param currency - Currency
    /// @param wac - Weighted average coupon
    /// @param passThroughRate - Net coupon paid to investors
    /// @param wam - Weighted average maturity in months
    /// @param issueDate - Pool issue date
    /// @param maturityDate - Pool maturity date
    /// @param discountCurveId - Discount curve ID
    /// @param currentFactor - Current pool factor (optional)
    /// @param servicingFeeRate - Servicing fee rate (default 0.0025)
    /// @param guaranteeFeeRate - Guarantee fee rate (default 0.0025)
    /// @param psaSpeed - PSA prepayment speed (default 1.0)
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        pool_id: &str,
        agency: &str,
        original_face: f64,
        current_face: f64,
        currency: &JsCurrency,
        wac: f64,
        pass_through_rate: f64,
        wam: u32,
        issue_date: &JsDate,
        maturity_date: &JsDate,
        discount_curve_id: &str,
        current_factor: Option<f64>,
        servicing_fee_rate: Option<f64>,
        guarantee_fee_rate: Option<f64>,
        psa_speed: Option<f64>,
    ) -> Result<JsAgencyMbsPassthrough, JsValue> {
        let agency_enum = match agency.to_lowercase().as_str() {
            "fnma" | "fannie" => AgencyProgram::Fnma,
            "fhlmc" | "freddie" => AgencyProgram::Fhlmc,
            "gnma" | "ginnie" => AgencyProgram::Gnma,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid agency: '{}'. Must be 'fnma', 'fhlmc', or 'gnma'",
                    agency
                )));
            }
        };

        let ccy = currency.inner();
        let factor = current_factor.unwrap_or(current_face / original_face);
        let serv_fee = servicing_fee_rate.unwrap_or(0.0025);
        let guar_fee = guarantee_fee_rate.unwrap_or(0.0025);
        let psa = psa_speed.unwrap_or(1.0);

        let mbs = AgencyMbsPassthrough::builder()
            .id(InstrumentId::new(instrument_id))
            .pool_id(pool_id.to_string())
            .agency(agency_enum)
            .pool_type(PoolType::Generic)
            .original_face(Money::new(original_face, ccy))
            .current_face(Money::new(current_face, ccy))
            .current_factor(factor)
            .wac(wac)
            .pass_through_rate(pass_through_rate)
            .servicing_fee_rate(serv_fee)
            .guarantee_fee_rate(guar_fee)
            .wam(wam)
            .issue_date(issue_date.inner())
            .maturity_date(maturity_date.inner())
            .prepayment_model(PrepaymentModelSpec::psa(psa))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .day_count(DayCount::Thirty360)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsAgencyMbsPassthrough::from_inner(mbs))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = poolId)]
    pub fn pool_id(&self) -> String {
        self.inner.pool_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn agency(&self) -> String {
        match self.inner.agency {
            AgencyProgram::Fnma => "fnma".to_string(),
            AgencyProgram::Fhlmc => "fhlmc".to_string(),
            AgencyProgram::Gnma => "gnma".to_string(),
        }
    }

    #[wasm_bindgen(getter, js_name = originalFace)]
    pub fn original_face(&self) -> f64 {
        self.inner.original_face.amount()
    }

    #[wasm_bindgen(getter, js_name = currentFace)]
    pub fn current_face(&self) -> f64 {
        self.inner.current_face.amount()
    }

    #[wasm_bindgen(getter, js_name = currentFactor)]
    pub fn current_factor(&self) -> f64 {
        self.inner.current_factor
    }

    #[wasm_bindgen(getter)]
    pub fn wac(&self) -> f64 {
        self.inner.wac
    }

    #[wasm_bindgen(getter, js_name = passThroughRate)]
    pub fn pass_through_rate(&self) -> f64 {
        self.inner.pass_through_rate
    }

    #[wasm_bindgen(getter)]
    pub fn wam(&self) -> u32 {
        self.inner.wam
    }

    #[wasm_bindgen(getter, js_name = issueDate)]
    pub fn issue_date(&self) -> JsDate {
        JsDate::from_core(self.inner.issue_date)
    }

    #[wasm_bindgen(getter, js_name = maturityDate)]
    pub fn maturity_date(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity_date)
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsAgencyMbsPassthrough, JsValue> {
        from_js_value(value).map(JsAgencyMbsPassthrough::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// Agency TBA
// =============================================================================

/// JavaScript representation of an agency TBA forward.
#[wasm_bindgen(js_name = AgencyTba)]
#[derive(Clone, Debug)]
pub struct JsAgencyTba {
    pub(crate) inner: AgencyTba,
}

impl InstrumentWrapper for JsAgencyTba {
    type Inner = AgencyTba;
    fn from_inner(inner: AgencyTba) -> Self {
        JsAgencyTba { inner }
    }
    fn inner(&self) -> AgencyTba {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = AgencyTba)]
impl JsAgencyTba {
    /// Create a new TBA forward contract.
    ///
    /// @param instrumentId - Unique identifier
    /// @param agency - Agency program ("fnma", "fhlmc", "gnma")
    /// @param coupon - Pass-through coupon rate
    /// @param term - Original loan term ("15y", "20y", "30y")
    /// @param settlementYear - Settlement year
    /// @param settlementMonth - Settlement month (1-12)
    /// @param notional - Trade notional
    /// @param currency - Currency
    /// @param tradePrice - Trade price (percentage of par)
    /// @param discountCurveId - Discount curve ID
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        agency: &str,
        coupon: f64,
        term: &str,
        settlement_year: i32,
        settlement_month: u8,
        notional: f64,
        currency: &JsCurrency,
        trade_price: f64,
        discount_curve_id: &str,
    ) -> Result<JsAgencyTba, JsValue> {
        let agency_enum = match agency.to_lowercase().as_str() {
            "fnma" | "fannie" => AgencyProgram::Fnma,
            "fhlmc" | "freddie" => AgencyProgram::Fhlmc,
            "gnma" | "ginnie" => AgencyProgram::Gnma,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid agency: '{}'. Must be 'fnma', 'fhlmc', or 'gnma'",
                    agency
                )));
            }
        };

        let term_enum = match term.to_lowercase().as_str() {
            "15y" | "15" | "fifteen" => TbaTerm::FifteenYear,
            "20y" | "20" | "twenty" => TbaTerm::TwentyYear,
            "30y" | "30" | "thirty" => TbaTerm::ThirtyYear,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid term: '{}'. Must be '15y', '20y', or '30y'",
                    term
                )));
            }
        };

        let ccy = currency.inner();

        let tba = AgencyTba::builder()
            .id(InstrumentId::new(instrument_id))
            .agency(agency_enum)
            .coupon(coupon)
            .term(term_enum)
            .settlement_year(settlement_year)
            .settlement_month(settlement_month)
            .notional(Money::new(notional, ccy))
            .trade_price(trade_price)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsAgencyTba::from_inner(tba))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn agency(&self) -> String {
        match self.inner.agency {
            AgencyProgram::Fnma => "fnma".to_string(),
            AgencyProgram::Fhlmc => "fhlmc".to_string(),
            AgencyProgram::Gnma => "gnma".to_string(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    #[wasm_bindgen(getter)]
    pub fn term(&self) -> String {
        match self.inner.term {
            TbaTerm::FifteenYear => "15y".to_string(),
            TbaTerm::TwentyYear => "20y".to_string(),
            TbaTerm::ThirtyYear => "30y".to_string(),
            _ => unreachable!("unknown TbaTerm variant"),
        }
    }

    #[wasm_bindgen(getter, js_name = settlementYear)]
    pub fn settlement_year(&self) -> i32 {
        self.inner.settlement_year
    }

    #[wasm_bindgen(getter, js_name = settlementMonth)]
    pub fn settlement_month(&self) -> u8 {
        self.inner.settlement_month
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    #[wasm_bindgen(getter, js_name = tradePrice)]
    pub fn trade_price(&self) -> f64 {
        self.inner.trade_price
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsAgencyTba, JsValue> {
        from_js_value(value).map(JsAgencyTba::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// Dollar Roll
// =============================================================================

/// JavaScript representation of a dollar roll.
#[wasm_bindgen(js_name = DollarRoll)]
#[derive(Clone, Debug)]
pub struct JsDollarRoll {
    pub(crate) inner: DollarRoll,
}

impl InstrumentWrapper for JsDollarRoll {
    type Inner = DollarRoll;
    fn from_inner(inner: DollarRoll) -> Self {
        JsDollarRoll { inner }
    }
    fn inner(&self) -> DollarRoll {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = DollarRoll)]
impl JsDollarRoll {
    /// Create a new dollar roll position.
    ///
    /// @param instrumentId - Unique identifier
    /// @param agency - Agency program ("fnma", "fhlmc", "gnma")
    /// @param coupon - Pass-through coupon rate
    /// @param term - Original loan term ("15y", "20y", "30y")
    /// @param notional - Trade notional
    /// @param currency - Currency
    /// @param frontSettlementYear - Front-month settlement year
    /// @param frontSettlementMonth - Front-month settlement month (1-12)
    /// @param backSettlementYear - Back-month settlement year
    /// @param backSettlementMonth - Back-month settlement month (1-12)
    /// @param frontPrice - Front-month price (sell price)
    /// @param backPrice - Back-month price (buy price)
    /// @param discountCurveId - Discount curve ID
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        agency: &str,
        coupon: f64,
        term: &str,
        notional: f64,
        currency: &JsCurrency,
        front_settlement_year: i32,
        front_settlement_month: u8,
        back_settlement_year: i32,
        back_settlement_month: u8,
        front_price: f64,
        back_price: f64,
        discount_curve_id: &str,
    ) -> Result<JsDollarRoll, JsValue> {
        let agency_enum = match agency.to_lowercase().as_str() {
            "fnma" | "fannie" => AgencyProgram::Fnma,
            "fhlmc" | "freddie" => AgencyProgram::Fhlmc,
            "gnma" | "ginnie" => AgencyProgram::Gnma,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid agency: '{}'. Must be 'fnma', 'fhlmc', or 'gnma'",
                    agency
                )));
            }
        };

        let term_enum = match term.to_lowercase().as_str() {
            "15y" | "15" | "fifteen" => TbaTerm::FifteenYear,
            "20y" | "20" | "twenty" => TbaTerm::TwentyYear,
            "30y" | "30" | "thirty" => TbaTerm::ThirtyYear,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid term: '{}'. Must be '15y', '20y', or '30y'",
                    term
                )));
            }
        };

        let ccy = currency.inner();

        let roll = DollarRoll::builder()
            .id(InstrumentId::new(instrument_id))
            .agency(agency_enum)
            .coupon(coupon)
            .term(term_enum)
            .notional(Money::new(notional, ccy))
            .front_settlement_year(front_settlement_year)
            .front_settlement_month(front_settlement_month)
            .back_settlement_year(back_settlement_year)
            .back_settlement_month(back_settlement_month)
            .front_price(front_price)
            .back_price(back_price)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsDollarRoll::from_inner(roll))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn agency(&self) -> String {
        match self.inner.agency {
            AgencyProgram::Fnma => "fnma".to_string(),
            AgencyProgram::Fhlmc => "fhlmc".to_string(),
            AgencyProgram::Gnma => "gnma".to_string(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    #[wasm_bindgen(getter, js_name = frontPrice)]
    pub fn front_price(&self) -> f64 {
        self.inner.front_price
    }

    #[wasm_bindgen(getter, js_name = backPrice)]
    pub fn back_price(&self) -> f64 {
        self.inner.back_price
    }

    /// Get the drop (price difference between front and back month).
    #[wasm_bindgen(js_name = dropValue)]
    pub fn drop_value(&self) -> f64 {
        self.inner.drop()
    }

    /// Get the drop in 32nds.
    #[wasm_bindgen(js_name = drop32nds)]
    pub fn drop_32nds(&self) -> f64 {
        self.inner.drop_32nds()
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsDollarRoll, JsValue> {
        from_js_value(value).map(JsDollarRoll::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// CMO Tranche
// =============================================================================

/// JavaScript representation of a CMO tranche.
#[wasm_bindgen(js_name = CmoTranche)]
#[derive(Clone, Debug)]
pub struct JsCmoTranche {
    pub(crate) inner: CmoTranche,
}

#[wasm_bindgen(js_class = CmoTranche)]
impl JsCmoTranche {
    /// Create a sequential tranche.
    #[wasm_bindgen(js_name = sequential)]
    pub fn sequential(
        tranche_id: &str,
        face: f64,
        currency: &JsCurrency,
        coupon: f64,
        priority: u32,
    ) -> JsCmoTranche {
        JsCmoTranche {
            inner: CmoTranche::sequential(
                tranche_id,
                Money::new(face, currency.inner()),
                coupon,
                priority,
            ),
        }
    }

    /// Create a PAC tranche.
    #[wasm_bindgen(js_name = pac)]
    pub fn pac(
        tranche_id: &str,
        face: f64,
        currency: &JsCurrency,
        coupon: f64,
        priority: u32,
        lower_psa: f64,
        upper_psa: f64,
    ) -> JsCmoTranche {
        JsCmoTranche {
            inner: CmoTranche::pac(
                tranche_id,
                Money::new(face, currency.inner()),
                coupon,
                priority,
                PacCollar::new(lower_psa, upper_psa),
            ),
        }
    }

    /// Create a support tranche.
    #[wasm_bindgen(js_name = support)]
    pub fn support(
        tranche_id: &str,
        face: f64,
        currency: &JsCurrency,
        coupon: f64,
        priority: u32,
    ) -> JsCmoTranche {
        JsCmoTranche {
            inner: CmoTranche::support(
                tranche_id,
                Money::new(face, currency.inner()),
                coupon,
                priority,
            ),
        }
    }

    /// Create an IO strip.
    #[wasm_bindgen(js_name = ioStrip)]
    pub fn io_strip(
        tranche_id: &str,
        notional: f64,
        currency: &JsCurrency,
        coupon: f64,
    ) -> JsCmoTranche {
        JsCmoTranche {
            inner: CmoTranche::io_strip(tranche_id, Money::new(notional, currency.inner()), coupon),
        }
    }

    /// Create a PO strip.
    #[wasm_bindgen(js_name = poStrip)]
    pub fn po_strip(tranche_id: &str, face: f64, currency: &JsCurrency) -> JsCmoTranche {
        JsCmoTranche {
            inner: CmoTranche::po_strip(tranche_id, Money::new(face, currency.inner())),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[wasm_bindgen(getter, js_name = trancheType)]
    pub fn tranche_type(&self) -> String {
        match self.inner.tranche_type {
            CmoTrancheType::Sequential => "sequential".to_string(),
            CmoTrancheType::Pac => "pac".to_string(),
            CmoTrancheType::Support => "support".to_string(),
            CmoTrancheType::InterestOnly => "io".to_string(),
            CmoTrancheType::PrincipalOnly => "po".to_string(),
        }
    }

    #[wasm_bindgen(getter, js_name = originalFace)]
    pub fn original_face(&self) -> f64 {
        self.inner.original_face.amount()
    }

    #[wasm_bindgen(getter, js_name = currentFace)]
    pub fn current_face(&self) -> f64 {
        self.inner.current_face.amount()
    }

    #[wasm_bindgen(getter)]
    pub fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    #[wasm_bindgen(getter)]
    pub fn priority(&self) -> u32 {
        self.inner.priority
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCmoTranche, JsValue> {
        from_js_value(value).map(|inner| JsCmoTranche { inner })
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// CMO Waterfall
// =============================================================================

/// JavaScript representation of a CMO waterfall.
#[wasm_bindgen(js_name = CmoWaterfall)]
#[derive(Clone, Debug)]
pub struct JsCmoWaterfall {
    pub(crate) inner: CmoWaterfall,
}

#[wasm_bindgen(js_class = CmoWaterfall)]
impl JsCmoWaterfall {
    /// Create a new waterfall from tranches.
    #[wasm_bindgen(constructor)]
    pub fn new(tranches: Vec<JsCmoTranche>) -> JsCmoWaterfall {
        let rust_tranches: Vec<CmoTranche> = tranches.into_iter().map(|t| t.inner).collect();
        JsCmoWaterfall {
            inner: CmoWaterfall::new(rust_tranches),
        }
    }

    /// Get the total current face.
    #[wasm_bindgen(js_name = totalCurrentFace)]
    pub fn total_current_face(&self) -> f64 {
        self.inner.total_current_face().amount()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCmoWaterfall, JsValue> {
        from_js_value(value).map(|inner| JsCmoWaterfall { inner })
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// =============================================================================
// Agency CMO
// =============================================================================

/// JavaScript representation of an agency CMO.
#[wasm_bindgen(js_name = AgencyCmo)]
#[derive(Clone, Debug)]
pub struct JsAgencyCmo {
    pub(crate) inner: AgencyCmo,
}

impl InstrumentWrapper for JsAgencyCmo {
    type Inner = AgencyCmo;
    fn from_inner(inner: AgencyCmo) -> Self {
        JsAgencyCmo { inner }
    }
    fn inner(&self) -> AgencyCmo {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = AgencyCmo)]
impl JsAgencyCmo {
    /// Create a new agency CMO.
    ///
    /// @param instrumentId - Unique identifier
    /// @param dealName - Deal/series name
    /// @param agency - Agency program ("fnma", "fhlmc", "gnma")
    /// @param issueDate - Deal issue date
    /// @param waterfall - Waterfall structure with tranches
    /// @param referenceTrancheId - ID of the tranche being valued
    /// @param discountCurveId - Discount curve ID
    /// @param collateralWac - Optional collateral WAC
    /// @param collateralWam - Optional collateral WAM (months)
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        deal_name: &str,
        agency: &str,
        issue_date: &JsDate,
        waterfall: &JsCmoWaterfall,
        reference_tranche_id: &str,
        discount_curve_id: &str,
        collateral_wac: Option<f64>,
        collateral_wam: Option<u32>,
    ) -> Result<JsAgencyCmo, JsValue> {
        let agency_enum = match agency.to_lowercase().as_str() {
            "fnma" | "fannie" => AgencyProgram::Fnma,
            "fhlmc" | "freddie" => AgencyProgram::Fhlmc,
            "gnma" | "ginnie" => AgencyProgram::Gnma,
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Invalid agency: '{}'. Must be 'fnma', 'fhlmc', or 'gnma'",
                    agency
                )));
            }
        };

        let mut builder = AgencyCmo::builder()
            .id(InstrumentId::new(instrument_id))
            .deal_name(deal_name.to_string())
            .agency(agency_enum)
            .issue_date(issue_date.inner())
            .waterfall(waterfall.inner.clone())
            .reference_tranche_id(reference_tranche_id.to_string())
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(wac) = collateral_wac {
            builder = builder.collateral_wac_opt(Some(wac));
        }
        if let Some(wam) = collateral_wam {
            builder = builder.collateral_wam_opt(Some(wam));
        }

        let cmo = builder
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsAgencyCmo::from_inner(cmo))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dealName)]
    pub fn deal_name(&self) -> String {
        self.inner.deal_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn agency(&self) -> String {
        match self.inner.agency {
            AgencyProgram::Fnma => "fnma".to_string(),
            AgencyProgram::Fhlmc => "fhlmc".to_string(),
            AgencyProgram::Gnma => "gnma".to_string(),
        }
    }

    #[wasm_bindgen(getter, js_name = issueDate)]
    pub fn issue_date(&self) -> JsDate {
        JsDate::from_core(self.inner.issue_date)
    }

    #[wasm_bindgen(getter, js_name = referenceTrancheId)]
    pub fn reference_tranche_id(&self) -> String {
        self.inner.reference_tranche_id.clone()
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsAgencyCmo, JsValue> {
        from_js_value(value).map(JsAgencyCmo::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
