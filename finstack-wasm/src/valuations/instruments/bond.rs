use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::dates::frequency::JsFrequency;
use crate::core::dates::schedule::JsStubKind;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::cashflow::JsAmortizationSpec;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::Date as CoreDate;
use finstack_core::money::Money as CoreMoney;
use finstack_valuations::cashflow::builder::specs::{
    CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
};
use finstack_valuations::instruments::fixed_income::bond::{
    Bond, CallPut, CallPutSchedule, CashflowSpec,
};
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::pricer::InstrumentType;
use js_sys::{Array, Reflect};
use time::Month;
use wasm_bindgen::prelude::*;

fn parse_iso_date(value: &str) -> Result<CoreDate, JsValue> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(js_error(format!(
            "Date '{value}' must be in ISO format YYYY-MM-DD"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| js_error(format!("Invalid year component in date '{value}'")))?;
    let month: u8 = parts[1]
        .parse()
        .map_err(|_| js_error(format!("Invalid month component in date '{value}'")))?;
    let day: u8 = parts[2]
        .parse()
        .map_err(|_| js_error(format!("Invalid day component in date '{value}'")))?;
    let month_enum = Month::try_from(month)
        .map_err(|_| js_error(format!("Month component must be 1-12 in date '{value}'")))?;
    CoreDate::from_calendar_date(year, month_enum, day)
        .map_err(|e| js_error(format!("Invalid date '{value}': {e}")))
}

fn parse_call_put_entries(
    entries: Option<Array>,
    label: &str,
) -> Result<Option<Vec<CallPut>>, JsValue> {
    if let Some(array) = entries {
        let mut out: Vec<CallPut> = Vec::with_capacity(array.length() as usize);
        for item in array.iter() {
            if Array::is_array(&item) {
                let pair = Array::from(&item);
                if pair.length() != 2 {
                    return Err(js_error(format!(
                        "{label} entries must be [dateString, pricePct] pairs"
                    )));
                }
                let date_value = pair.get(0);
                let price_value = pair.get(1);
                let date_str = date_value.as_string().ok_or_else(|| {
                    js_error(format!(
                        "{label} entries must provide date strings in ISO format"
                    ))
                })?;
                let price_pct = price_value.as_f64().ok_or_else(|| {
                    js_error(format!("{label} entries must provide numeric prices"))
                })?;
                out.push(CallPut {
                    date: parse_iso_date(&date_str)?,
                    price_pct_of_par: price_pct,
                });
            } else if item.is_object() {
                let date_value = Reflect::get(&item, &JsValue::from_str("date")).map_err(|_| {
                    js_error(format!(
                        "{label} objects must contain a 'date' property as ISO string"
                    ))
                })?;
                let price_value =
                    Reflect::get(&item, &JsValue::from_str("pricePct")).map_err(|_| {
                        js_error(format!(
                            "{label} objects must contain a 'pricePct' numeric property"
                        ))
                    })?;
                let date_str = date_value.as_string().ok_or_else(|| {
                    js_error(format!(
                        "{label} objects must provide date strings in ISO format"
                    ))
                })?;
                let price_pct = price_value.as_f64().ok_or_else(|| {
                    js_error(format!(
                        "{label} objects must provide numeric pricePct values"
                    ))
                })?;
                out.push(CallPut {
                    date: parse_iso_date(&date_str)?,
                    price_pct_of_par: price_pct,
                });
            } else {
                return Err(js_error(format!(
                    "{label} entries must be arrays [dateString, pricePct] or objects"
                )));
            }
        }
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

#[wasm_bindgen(js_name = BondBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsBondBuilder {
    instrument_id: String,
    notional: Option<CoreMoney>,
    issue: Option<CoreDate>,
    maturity: Option<CoreDate>,
    discount_curve: Option<String>,
    credit_curve: Option<String>,
    coupon_rate: Option<f64>,
    frequency: Option<finstack_core::dates::Tenor>,
    day_count: Option<finstack_core::dates::DayCount>,
    business_day_convention: Option<finstack_core::dates::BusinessDayConvention>,
    calendar_id: Option<String>,
    stub_kind: Option<finstack_core::dates::StubKind>,
    amortization: Option<finstack_valuations::cashflow::builder::AmortizationSpec>,
    call_schedule: Option<Array>,
    put_schedule: Option<Array>,
    quoted_clean_price: Option<f64>,
    forward_curve: Option<String>,
    float_margin_bp: Option<f64>,
    float_gearing: Option<f64>,
    float_reset_lag_days: Option<i32>,
}

impl JsBondBuilder {
    fn build_cashflow_spec(&self) -> Result<CashflowSpec, JsValue> {
        let freq = self
            .frequency
            .unwrap_or_else(finstack_core::dates::Tenor::semi_annual);
        let bdc = self
            .business_day_convention
            .unwrap_or(finstack_core::dates::BusinessDayConvention::Following);
        let stub = self
            .stub_kind
            .unwrap_or(finstack_core::dates::StubKind::None);
        let calendar_id = self.calendar_id.clone().unwrap_or_else(|| {
            finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID.to_string()
        });

        let base = if let Some(curve) = &self.forward_curve {
            // Floating rate bond
            let dc = self
                .day_count
                .unwrap_or(finstack_core::dates::DayCount::Act360);
            CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: curve_id_from_str(curve),
                    spread_bp: rust_decimal::Decimal::from_f64_retain(
                        self.float_margin_bp.unwrap_or(0.0),
                    )
                    .unwrap_or_default(),
                    gearing: rust_decimal::Decimal::from_f64_retain(
                        self.float_gearing.unwrap_or(1.0),
                    )
                    .unwrap_or(rust_decimal::Decimal::ONE),
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: freq,
                    reset_lag_days: self.float_reset_lag_days.unwrap_or(2),
                    dc,
                    bdc,
                    calendar_id: calendar_id.clone(),
                    fixing_calendar_id: Some(calendar_id.clone()),
                    end_of_month: false,
                    overnight_compounding: None,
                    payment_lag_days: 0,
                },
                coupon_type: CouponType::Cash,
                freq,
                stub,
            })
        } else {
            // Fixed rate bond
            let dc = self
                .day_count
                .unwrap_or(finstack_core::dates::DayCount::Thirty360);
            CashflowSpec::Fixed(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: rust_decimal::Decimal::from_f64_retain(self.coupon_rate.unwrap_or(0.0))
                    .unwrap_or_default(),
                freq,
                dc,
                bdc,
                calendar_id: calendar_id.clone(),
                stub,
                end_of_month: false,
                payment_lag_days: 0,
            })
        };

        // Wrap in amortization if present
        if let Some(amort) = &self.amortization {
            Ok(CashflowSpec::Amortizing {
                base: Box::new(base),
                schedule: amort.clone(),
            })
        } else {
            Ok(base)
        }
    }

    fn build_call_put(&self) -> Result<Option<CallPutSchedule>, JsValue> {
        if self.call_schedule.is_none() && self.put_schedule.is_none() {
            return Ok(None);
        }
        let mut schedule = CallPutSchedule::default();
        if let Some(calls) = parse_call_put_entries(self.call_schedule.clone(), "Call schedule")? {
            schedule.calls = calls;
        }
        if let Some(puts) = parse_call_put_entries(self.put_schedule.clone(), "Put schedule")? {
            schedule.puts = puts;
        }
        Ok(Some(schedule))
    }

    fn effective_issue(&self, maturity: CoreDate) -> CoreDate {
        self.issue.unwrap_or_else(|| {
            maturity
                .checked_sub(time::Duration::days(365))
                .unwrap_or(maturity)
        })
    }
}

#[wasm_bindgen(js_class = BondBuilder)]
impl JsBondBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsBondBuilder {
        JsBondBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsBondBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = issue)]
    pub fn issue(mut self, issue: &JsDate) -> JsBondBuilder {
        self.issue = Some(issue.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsBondBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsBondBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = hazardCurve)]
    pub fn hazard_curve(mut self, hazard_curve: &str) -> JsBondBuilder {
        self.credit_curve = Some(hazard_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = couponRate)]
    pub fn coupon_rate(mut self, coupon_rate: f64) -> JsBondBuilder {
        self.coupon_rate = Some(coupon_rate);
        self
    }

    #[wasm_bindgen(js_name = frequency)]
    pub fn frequency(mut self, frequency: JsFrequency) -> JsBondBuilder {
        self.frequency = Some(frequency.inner());
        self
    }

    #[wasm_bindgen(js_name = dayCount)]
    pub fn day_count(mut self, day_count: JsDayCount) -> JsBondBuilder {
        self.day_count = Some(day_count.inner());
        self
    }

    #[wasm_bindgen(js_name = businessDayConvention)]
    pub fn business_day_convention(mut self, bdc: JsBusinessDayConvention) -> JsBondBuilder {
        self.business_day_convention = Some(bdc.into());
        self
    }

    #[wasm_bindgen(js_name = calendarId)]
    pub fn calendar_id(mut self, calendar_id: String) -> JsBondBuilder {
        self.calendar_id = Some(calendar_id);
        self
    }

    #[wasm_bindgen(js_name = stubKind)]
    pub fn stub_kind(mut self, stub_kind: JsStubKind) -> JsBondBuilder {
        self.stub_kind = Some(stub_kind.inner());
        self
    }

    #[wasm_bindgen(js_name = amortization)]
    pub fn amortization(mut self, amortization: JsAmortizationSpec) -> JsBondBuilder {
        self.amortization = Some(amortization.inner());
        self
    }

    #[wasm_bindgen(js_name = callSchedule)]
    pub fn call_schedule(mut self, call_schedule: Array) -> JsBondBuilder {
        self.call_schedule = Some(call_schedule);
        self
    }

    #[wasm_bindgen(js_name = putSchedule)]
    pub fn put_schedule(mut self, put_schedule: Array) -> JsBondBuilder {
        self.put_schedule = Some(put_schedule);
        self
    }

    #[wasm_bindgen(js_name = quotedCleanPrice)]
    pub fn quoted_clean_price(mut self, quoted_clean_price: f64) -> JsBondBuilder {
        self.quoted_clean_price = Some(quoted_clean_price);
        self
    }

    #[wasm_bindgen(js_name = forwardCurve)]
    pub fn forward_curve(mut self, forward_curve: String) -> JsBondBuilder {
        self.forward_curve = Some(forward_curve);
        self
    }

    #[wasm_bindgen(js_name = floatMarginBp)]
    pub fn float_margin_bp(mut self, float_margin_bp: f64) -> JsBondBuilder {
        self.float_margin_bp = Some(float_margin_bp);
        self
    }

    #[wasm_bindgen(js_name = floatGearing)]
    pub fn float_gearing(mut self, float_gearing: f64) -> JsBondBuilder {
        self.float_gearing = Some(float_gearing);
        self
    }

    #[wasm_bindgen(js_name = floatResetLagDays)]
    pub fn float_reset_lag_days(mut self, float_reset_lag_days: i32) -> JsBondBuilder {
        self.float_reset_lag_days = Some(float_reset_lag_days);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsBond, JsValue> {
        let notional = self
            .notional
            .ok_or_else(|| js_error("BondBuilder: notional (money) is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("BondBuilder: maturity is required".to_string()))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("BondBuilder: discountCurve is required".to_string()))?;

        let issue = self.effective_issue(maturity);
        let cashflow_spec = self.build_cashflow_spec()?;

        let mut builder = Bond::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .issue(issue)
            .maturity(maturity)
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(curve_id_from_str(discount_curve));

        if let Some(price) = self.quoted_clean_price {
            builder =
                builder.pricing_overrides(PricingOverrides::default().with_clean_price(price));
        }
        if let Some(hazard) = self.credit_curve.as_deref() {
            builder = builder.credit_curve_id_opt(Some(curve_id_from_str(hazard)));
        }
        if let Some(schedule) = self.build_call_put()? {
            builder = builder.call_put_opt(Some(schedule));
        }

        builder
            .build()
            .map(JsBond::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = Bond)]
#[derive(Clone, Debug)]
pub struct JsBond {
    pub(crate) inner: Bond,
}

impl InstrumentWrapper for JsBond {
    type Inner = Bond;
    fn from_inner(inner: Bond) -> Self {
        JsBond { inner }
    }
    fn inner(&self) -> Bond {
        self.inner.clone()
    }
}

// Public accessor for use in other modules
impl JsBond {
    /// Get a clone of the inner Bond (for internal WASM use).
    pub(crate) fn inner_bond(&self) -> Bond {
        self.inner.clone()
    }
}

// Manually implement JsCast for wasm_bindgen structs
// wasm_bindgen doesn't automatically implement JsCast for structs with private fields

#[wasm_bindgen(js_class = Bond)]
impl JsBond {
    /// Construct a bond with full control over schedule and coupon conventions.
    ///
    /// This constructor supports both **fixed-rate** and **floating-rate** bonds:
    /// - If `forward_curve` is `null`/`undefined`, the bond is treated as **fixed-rate** and
    ///   `coupon_rate` is used.
    /// - If `forward_curve` is provided, the bond is treated as **floating-rate** and
    ///   `float_margin_bp` / `float_gearing` / `float_reset_lag_days` are used.
    ///
    /// Conventions:
    /// - `coupon_rate` is a **decimal rate** (e.g. `0.05` for 5%).
    /// - `float_margin_bp` and `float_margin_bp`-style fields are in **basis points** (e.g. `120.0`).
    /// - `quoted_clean_price` is a **clean price** in **percent of par** (e.g. `99.25`).
    /// - `call_schedule` / `put_schedule` entries are **percent of par** (e.g. `100.0`).
    ///
    /// @param instrument_id - Unique identifier for this instrument
    /// @param notional - Face amount (currency-tagged)
    /// @param issue - Issue/settlement start date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID (must exist in `MarketContext` when pricing)
    /// @param coupon_rate - Fixed coupon rate (decimal). Ignored for floating bonds
    /// @param frequency - Coupon frequency (optional; defaults to semi-annual)
    /// @param day_count - Day count convention for accrual (optional; fixed defaults to 30/360, float defaults to Act/360)
    /// @param business_day_convention - Business day adjustment convention (optional)
    /// @param calendar_id - Calendar registry code (optional, e.g. `"usny"`, `"gblo"`)
    /// @param stub_kind - Stub rule (optional)
    /// @param amortization - Optional amortization schedule
    /// @param call_schedule - Optional call schedule (array of `[\"YYYY-MM-DD\", pricePct]` or `{date, pricePct}` objects)
    /// @param put_schedule - Optional put schedule (array of `[\"YYYY-MM-DD\", pricePct]` or `{date, pricePct}` objects)
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @param forward_curve - Optional forward curve ID; when set the bond is floating-rate
    /// @param float_margin_bp - Floating spread in **bps** (e.g. `150.0` for +150bp)
    /// @param float_gearing - Floating gearing multiplier (default `1.0`)
    /// @param float_reset_lag_days - Reset lag in business days (default `2`)
    /// @param hazard_curve - Optional hazard/credit curve ID (for credit-risky bond pricing)
    /// @returns A new `Bond` instance
    /// @throws {Error} If inputs are invalid (e.g., malformed call schedule dates)
    ///
    /// @example
    /// ```javascript
    /// import init, { Bond, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const notional = Money.fromCode(1_000_000, "USD");
    /// const issue = new FsDate(2024, 1, 2);
    /// const maturity = new FsDate(2034, 1, 2);
    ///
    /// // Fixed-rate bond with custom conventions
    /// const bond = new Bond(
    ///   "bond_1",
    ///   notional,
    ///   issue,
    ///   maturity,
    ///   "USD-OIS",
    ///   0.05,     // 5% coupon (decimal)
    ///   null,
    ///   null,
    ///   null,
    ///   "usny",
    ///   null,
    ///   null,
    ///   null,
    ///   null,
    ///   99.25
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        coupon_rate: Option<f64>,
        frequency: Option<JsFrequency>,
        day_count: Option<JsDayCount>,
        business_day_convention: Option<JsBusinessDayConvention>,
        calendar_id: Option<String>,
        stub_kind: Option<JsStubKind>,
        amortization: Option<JsAmortizationSpec>,
        call_schedule: Option<Array>,
        put_schedule: Option<Array>,
        quoted_clean_price: Option<f64>,
        forward_curve: Option<String>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
        hazard_curve: Option<String>,
    ) -> Result<JsBond, JsValue> {
        let calendar_id = calendar_id.unwrap_or_else(|| {
            finstack_valuations::cashflow::builder::calendar::WEEKENDS_ONLY_ID.to_string()
        });
        // Build CashflowSpec based on whether it's floating or fixed
        let base_spec = if let Some(curve) = &forward_curve {
            // Floating rate bond
            CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: curve_id_from_str(curve),
                    spread_bp: rust_decimal::Decimal::from_f64_retain(
                        float_margin_bp.unwrap_or(0.0),
                    )
                    .unwrap_or_default(),
                    gearing: rust_decimal::Decimal::from_f64_retain(float_gearing.unwrap_or(1.0))
                        .unwrap_or(rust_decimal::Decimal::ONE),
                    gearing_includes_spread: true,
                    floor_bp: None,
                    all_in_floor_bp: None,
                    cap_bp: None,
                    index_cap_bp: None,
                    reset_freq: frequency
                        .map(|f| f.inner())
                        .unwrap_or_else(finstack_core::dates::Tenor::semi_annual),
                    reset_lag_days: float_reset_lag_days.unwrap_or(2),
                    dc: day_count
                        .map(|dc| dc.inner())
                        .unwrap_or(finstack_core::dates::DayCount::Act360),
                    bdc: business_day_convention
                        .map(|c| c.into())
                        .unwrap_or(finstack_core::dates::BusinessDayConvention::Following),
                    calendar_id: calendar_id.clone(),
                    fixing_calendar_id: Some(calendar_id.clone()),
                    end_of_month: false,
                    overnight_compounding: None,
                    payment_lag_days: 0,
                },
                coupon_type: CouponType::Cash,
                freq: frequency
                    .map(|f| f.inner())
                    .unwrap_or_else(finstack_core::dates::Tenor::semi_annual),
                stub: stub_kind
                    .map(|s| s.inner())
                    .unwrap_or(finstack_core::dates::StubKind::None),
            })
        } else {
            // Fixed rate bond
            CashflowSpec::Fixed(FixedCouponSpec {
                coupon_type: CouponType::Cash,
                rate: rust_decimal::Decimal::from_f64_retain(coupon_rate.unwrap_or(0.0))
                    .unwrap_or_default(),
                freq: frequency
                    .map(|f| f.inner())
                    .unwrap_or_else(finstack_core::dates::Tenor::semi_annual),
                dc: day_count
                    .map(|dc| dc.inner())
                    .unwrap_or(finstack_core::dates::DayCount::Thirty360),
                bdc: business_day_convention
                    .map(|c| c.into())
                    .unwrap_or(finstack_core::dates::BusinessDayConvention::Following),
                calendar_id: calendar_id.clone(),
                stub: stub_kind
                    .map(|s| s.inner())
                    .unwrap_or(finstack_core::dates::StubKind::None),
                end_of_month: false,
                payment_lag_days: 0,
            })
        };

        // Wrap in amortization if present
        let cashflow_spec = if let Some(amort) = amortization {
            CashflowSpec::Amortizing {
                base: Box::new(base_spec),
                schedule: amort.inner(),
            }
        } else {
            base_spec
        };

        let mut builder = Bond::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .issue(issue.inner())
            .maturity(maturity.inner())
            .cashflow_spec(cashflow_spec)
            .discount_curve_id(curve_id_from_str(discount_curve));
        if let Some(price) = quoted_clean_price {
            builder =
                builder.pricing_overrides(PricingOverrides::default().with_clean_price(price));
        }
        if let Some(hazard) = hazard_curve {
            builder = builder.credit_curve_id_opt(Some(curve_id_from_str(&hazard)));
        }

        if call_schedule.is_some() || put_schedule.is_some() {
            let mut schedule = CallPutSchedule::default();
            if let Some(calls) = parse_call_put_entries(call_schedule, "Call schedule")? {
                schedule.calls = calls;
            }
            if let Some(puts) = parse_call_put_entries(put_schedule, "Put schedule")? {
                schedule.puts = puts;
            }
            builder = builder.call_put_opt(Some(schedule));
        }

        builder
            .build()
            .map(JsBond::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    /// Parse a bond from a JSON value (as produced by `toJson`).
    ///
    /// @param value - JSON value matching the Bond schema
    /// @returns A new `Bond`
    /// @throws {Error} If deserialization fails
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsBond, JsValue> {
        from_js_value(value).map(JsBond::from_inner)
    }

    /// Serialize this bond to a JSON value.
    ///
    /// @returns JSON value
    /// @throws {Error} If serialization fails
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter)]
    pub fn coupon(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        // Extract coupon from cashflow_spec - return 0 for floating or amortizing
        match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            CashflowSpec::Amortizing { base, .. } => match base.as_ref() {
                CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn frequency(&self) -> JsFrequency {
        let freq = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.freq,
            CashflowSpec::Floating(spec) => spec.freq,
            CashflowSpec::Amortizing { base, .. } => base.frequency(),
        };
        JsFrequency::from_inner(freq)
    }

    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        let dc = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.dc,
            CashflowSpec::Floating(spec) => spec.rate_spec.dc,
            CashflowSpec::Amortizing { base, .. } => base.day_count(),
        };
        format!("{:?}", dc)
    }

    #[wasm_bindgen(getter, js_name = businessDayConvention)]
    pub fn business_day_convention(&self) -> JsBusinessDayConvention {
        let bdc = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.bdc,
            CashflowSpec::Floating(spec) => spec.rate_spec.bdc,
            CashflowSpec::Amortizing { base, .. } => match base.as_ref() {
                CashflowSpec::Fixed(spec) => spec.bdc,
                CashflowSpec::Floating(spec) => spec.rate_spec.bdc,
                _ => finstack_core::dates::BusinessDayConvention::Following,
            },
        };
        bdc.into()
    }

    #[wasm_bindgen(getter)]
    pub fn issue(&self) -> JsDate {
        JsDate::from_core(self.inner.issue)
    }

    #[wasm_bindgen(getter)]
    pub fn maturity(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = hazardCurve)]
    pub fn hazard_curve(&self) -> Option<String> {
        self.inner
            .credit_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[wasm_bindgen(getter, js_name = quotedCleanPrice)]
    pub fn quoted_clean_price(&self) -> Option<f64> {
        self.inner.pricing_overrides.quoted_clean_price
    }

    /// Get the cashflow schedule for this bond.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    /// For floating rate bonds, the amounts are computed using the market context.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use crate::core::dates::date::JsDate;
        use crate::core::money::JsMoney;
        use finstack_core::cashflow::CFKind;

        // Use the Bond's get_full_schedule method with market curves
        let sched = self
            .inner
            .get_full_schedule(market.inner())
            .map_err(|e| js_error(e.to_string()))?;

        // Get outstanding path (properly calculated by the Rust library)
        let outstanding_path = sched
            .outstanding_path_per_flow()
            .map_err(|e| js_error(e.to_string()))?;

        // Convert to JS arrays
        let result = Array::new();

        for (idx, cf) in sched.flows.iter().enumerate() {
            let entry = Array::new();
            entry.push(&JsDate::from_core(cf.date).into());
            entry.push(&JsMoney::from_inner(cf.amount).into());

            // Add kind as string
            // Stubs are treated as their underlying type (Fixed/Float/PIK) not as a separate category
            let kind_str = match cf.kind {
                CFKind::Fixed | CFKind::Stub => {
                    // Classify stub based on bond type
                    let is_floating = match &self.inner.cashflow_spec {
                        CashflowSpec::Floating(_) => true,
                        CashflowSpec::Amortizing { base, .. } => {
                            matches!(**base, CashflowSpec::Floating(_))
                        }
                        _ => false,
                    };
                    if is_floating {
                        "Float"
                    } else {
                        "Fixed"
                    }
                }
                CFKind::FloatReset => "Float",
                CFKind::Notional => "Notional",
                CFKind::PIK => "PIK",
                CFKind::Amortization => "Amortization",
                CFKind::Fee => "Fee",
                _ => "Other",
            };
            entry.push(&JsValue::from_str(kind_str));

            // Get outstanding balance from the path
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
        InstrumentType::Bond as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        let coupon = self.coupon(); // Use the getter method
        format!(
            "Bond(id='{}', coupon={:.4}, maturity='{}')",
            self.inner.id, coupon, self.inner.maturity
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBond {
        JsBond::from_inner(self.inner.clone())
    }
}
