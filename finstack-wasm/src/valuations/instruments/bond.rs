use crate::core::dates::calendar::JsBusinessDayConvention;
use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::dates::frequency::JsFrequency;
use crate::core::dates::schedule::JsStubKind;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::cashflow::JsAmortizationSpec;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::Date as CoreDate;
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
    pub fn builder(
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
                    fixing_calendar_id: calendar_id.clone(),
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

    /// Create a simple fixed-rate bond with semi-annual coupons.
    ///
    /// Conventions:
    /// - `coupon_rate` is a **decimal rate** (e.g. `0.05` for 5%).
    /// - `quoted_clean_price` is a **clean price** in **percent of par** (e.g. `99.25`).
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param coupon_rate - Annual coupon rate (decimal)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID (must exist in `MarketContext` when pricing)
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @returns A new `Bond`
    ///
    /// @example
    /// ```javascript
    /// import init, { Bond, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const bond = Bond.fixedSemiannual(
    ///   "bond_1",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   0.05,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2034, 1, 2),
    ///   "USD-OIS",
    ///   99.25
    /// );
    /// ```
    #[wasm_bindgen(js_name = fixedSemiannual)]
    #[allow(clippy::too_many_arguments)]
    pub fn fixed_semiannual(
        instrument_id: &str,
        notional: &JsMoney,
        coupon_rate: f64,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
    ) -> JsBond {
        let mut bond = Bond::fixed(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            coupon_rate,
            issue.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
        )
        .expect("Bond::fixed should succeed with valid parameters");
        if let Some(price) = quoted_clean_price {
            bond.pricing_overrides = PricingOverrides::default().with_clean_price(price);
        }
        JsBond::from_inner(bond)
    }

    /// Create a US Treasury-style bond using the `USTreasury` convention.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param coupon_rate - Annual coupon rate (decimal)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @returns A new `Bond`
    #[wasm_bindgen(js_name = treasury)]
    pub fn treasury(
        instrument_id: &str,
        notional: &JsMoney,
        coupon_rate: f64,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
    ) -> JsBond {
        use finstack_valuations::instruments::BondConvention;
        let mut bond = Bond::with_convention(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            coupon_rate,
            issue.inner(),
            maturity.inner(),
            BondConvention::USTreasury,
            discount_curve,
        )
        .expect("Bond::with_convention should succeed for US Treasury");
        if let Some(price) = quoted_clean_price {
            bond.pricing_overrides = PricingOverrides::default().with_clean_price(price);
        }
        JsBond::from_inner(bond)
    }

    /// Create a zero-coupon bond.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @returns A new `Bond`
    #[wasm_bindgen(js_name = zeroCoupon)]
    pub fn zero_coupon(
        instrument_id: &str,
        notional: &JsMoney,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
    ) -> JsBond {
        let mut bond = Bond::fixed(
            instrument_id_from_str(instrument_id),
            notional.inner(),
            0.0, // Zero coupon
            issue.inner(),
            maturity.inner(),
            curve_id_from_str(discount_curve),
        )
        .expect("Bond::fixed should succeed with valid parameters");
        if let Some(price) = quoted_clean_price {
            bond.pricing_overrides = PricingOverrides::default().with_clean_price(price);
        }
        JsBond::from_inner(bond)
    }

    /// Create a floating-rate bond with a single forward index and margin.
    ///
    /// Conventions:
    /// - `margin_bp` is in **basis points** (e.g. `150.0` for +150bp).
    /// - The underlying forward index is provided by `forward_curve` (curve ID).
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param forward_curve - Forward curve ID (e.g. `"USD-SOFR-3M"`)
    /// @param margin_bp - Spread in basis points
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @returns A new `Bond`
    /// @throws {Error} If construction fails due to invalid inputs
    #[wasm_bindgen(js_name = floating)]
    #[allow(clippy::too_many_arguments)]
    pub fn floating(
        instrument_id: &str,
        notional: &JsMoney,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        forward_curve: &str,
        margin_bp: f64,
        quoted_clean_price: Option<f64>,
    ) -> Result<JsBond, JsValue> {
        use finstack_core::dates::{DayCount, Tenor};

        let pricing_overrides = if let Some(price) = quoted_clean_price {
            PricingOverrides::default().with_clean_price(price)
        } else {
            PricingOverrides::default()
        };

        let mut bond = Bond::floating(
            instrument_id,
            notional.inner(),
            curve_id_from_str(forward_curve),
            margin_bp,
            issue.inner(),
            maturity.inner(),
            Tenor::quarterly(),
            DayCount::Act360,
            curve_id_from_str(discount_curve),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        bond.pricing_overrides = pricing_overrides;
        Ok(JsBond::from_inner(bond))
    }

    /// Create a bond with a split cash/PIK coupon.
    ///
    /// Conventions:
    /// - `coupon_rate` is a **decimal rate**.
    /// - `cash_pct` / `pik_pct` are fractions in **decimal** (typically sum to 1.0).
    /// - Requires a `MarketContext` to build floating/curve-aware schedules.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param coupon_rate - Annual coupon rate (decimal)
    /// @param cash_pct - Fraction of coupon paid in cash (decimal)
    /// @param pik_pct - Fraction of coupon paid in PIK (decimal)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @param market - Market context (used to build schedule)
    /// @returns A new `Bond`
    /// @throws {Error} If schedule construction fails
    ///
    /// @example
    /// ```javascript
    /// import init, { Bond, Money, FsDate, MarketContext } from "finstack-wasm";
    ///
    /// await init();
    /// const market = new MarketContext();
    /// const bond = Bond.pikToggle(
    ///   "pik_1",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   0.12,
    ///   0.5,
    ///   0.5,
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2029, 1, 2),
    ///   "USD-OIS",
    ///   null,
    ///   market
    /// );
    /// ```
    #[wasm_bindgen(js_name = pikToggle)]
    #[allow(clippy::too_many_arguments)]
    pub fn pik_toggle(
        instrument_id: &str,
        notional: &JsMoney,
        coupon_rate: f64,
        cash_pct: f64,
        pik_pct: f64,
        issue: &JsDate,
        maturity: &JsDate,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<JsBond, JsValue> {
        use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
        use finstack_valuations::cashflow::builder::{
            CashFlowSchedule, CouponType, FixedCouponSpec,
        };

        // Build cashflow schedule with PIK split
        let custom_schedule = CashFlowSchedule::builder()
            .principal(notional.inner(), issue.inner(), maturity.inner())
            .fixed_cf(FixedCouponSpec {
                coupon_type: CouponType::Split {
                    cash_pct: rust_decimal::Decimal::from_f64_retain(cash_pct).unwrap_or_default(),
                    pik_pct: rust_decimal::Decimal::from_f64_retain(pik_pct).unwrap_or_default(),
                },
                rate: rust_decimal::Decimal::from_f64_retain(coupon_rate).unwrap_or_default(),
                freq: Tenor::semi_annual(),
                dc: DayCount::Thirty360,
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            })
            .build_with_curves(Some(market.inner()))
            .map_err(|e| js_error(e.to_string()))?;

        Bond::from_cashflows(
            instrument_id_from_str(instrument_id),
            custom_schedule,
            curve_id_from_str(discount_curve),
            quoted_clean_price,
        )
        .map(JsBond::from_inner)
        .map_err(|e| js_error(e.to_string()))
    }

    /// Create a bond that switches from fixed to floating coupons on a given date.
    ///
    /// Conventions:
    /// - `fixed_rate` is a **decimal rate**.
    /// - `margin_bp` is in **basis points**.
    /// - Requires a `MarketContext` to build the schedule.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param fixed_rate - Fixed coupon rate before switch (decimal)
    /// @param switch_date - Date where floating coupons begin
    /// @param forward_curve - Forward curve ID for floating coupons
    /// @param margin_bp - Floating spread in bps
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param frequency - Coupon frequency
    /// @param day_count - Day count convention
    /// @param discount_curve - Discount curve ID
    /// @param quoted_clean_price - Optional clean price override (percent of par)
    /// @param market - Market context (used to build schedule)
    /// @returns A new `Bond`
    /// @throws {Error} If schedule construction fails
    #[wasm_bindgen(js_name = fixedToFloating)]
    #[allow(clippy::too_many_arguments)]
    pub fn fixed_to_floating(
        instrument_id: &str,
        notional: &JsMoney,
        fixed_rate: f64,
        switch_date: &JsDate,
        forward_curve: &str,
        margin_bp: f64,
        issue: &JsDate,
        maturity: &JsDate,
        frequency: &JsFrequency,
        day_count: &JsDayCount,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<JsBond, JsValue> {
        use finstack_core::dates::{BusinessDayConvention, StubKind};
        use finstack_valuations::cashflow::builder::{
            CashFlowSchedule, CouponType, FloatCouponParams, ScheduleParams,
        };

        // Build cashflow schedule with fixed then floating windows
        let mut b = CashFlowSchedule::builder();
        let _ = b.principal(notional.inner(), issue.inner(), maturity.inner());

        // Fixed window: issue to switch date
        let _ = b.add_fixed_coupon_window(
            issue.inner(),
            switch_date.inner(),
            fixed_rate,
            ScheduleParams {
                freq: frequency.inner(),
                dc: day_count.inner(),
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            },
            CouponType::Cash,
        );

        // Floating window: switch date to maturity
        let _ = b.add_float_coupon_window(
            switch_date.inner(),
            maturity.inner(),
            FloatCouponParams {
                index_id: curve_id_from_str(forward_curve),
                margin_bp: rust_decimal::Decimal::from_f64_retain(margin_bp).unwrap_or_default(),
                gearing: rust_decimal::Decimal::ONE,
                reset_lag_days: 2,
            },
            ScheduleParams {
                freq: frequency.inner(),
                dc: day_count.inner(),
                bdc: BusinessDayConvention::Following,
                calendar_id: None,
                stub: StubKind::None,
            },
            CouponType::Cash,
        );

        let custom_schedule = b
            .build_with_curves(Some(market.inner()))
            .map_err(|e| js_error(e.to_string()))?;

        Bond::from_cashflows(
            instrument_id_from_str(instrument_id),
            custom_schedule,
            curve_id_from_str(discount_curve),
            quoted_clean_price,
        )
        .map(JsBond::from_inner)
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
