use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::curve_id_from_str;
use crate::valuations::common::opt_f64_to_decimal;
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, DayCount, StubKind, Tenor};
use finstack_valuations::instruments::rates::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use rust_decimal::prelude::ToPrimitive;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = BasisSwapLeg)]
#[derive(Clone, Debug)]
pub struct JsBasisSwapLeg {
    pub(crate) inner: BasisSwapLeg,
}

#[wasm_bindgen(js_class = BasisSwapLeg)]
impl JsBasisSwapLeg {
    /// Create a basis swap floating leg specification.
    ///
    /// Each leg owns its own dates, discount curve, calendar, and stub conventions.
    ///
    /// @param forward_curve - Forward curve ID (e.g. `"USD-SOFR-3M"`)
    /// @param discount_curve - Discount curve ID (e.g. `"USD-OIS"`)
    /// @param start - Leg start date
    /// @param end - Leg end date
    /// @param frequency - Optional payment/reset frequency (e.g. `"3M"`)
    /// @param day_count - Optional day count name (e.g. `"act_360"`)
    /// @param spread_bp - Optional spread in basis points added to the forward rate
    /// @param business_day_convention - Optional BDC name (e.g. `"modified_following"`)
    /// @param calendar_id - Optional calendar code (e.g. `"usny"`)
    /// @param stub - Optional stub kind string (e.g. `"short_front"`)
    /// @returns A `BasisSwapLeg`
    /// @throws {Error} If parsing fails
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        forward_curve: &str,
        discount_curve: &str,
        start: &JsDate,
        end: &JsDate,
        frequency: Option<String>,
        day_count: Option<String>,
        spread_bp: Option<f64>,
        business_day_convention: Option<String>,
        calendar_id: Option<String>,
        stub: Option<String>,
    ) -> Result<JsBasisSwapLeg, JsValue> {
        let freq = parse_optional_with_default(frequency, Tenor::quarterly())?;
        let dc = parse_optional_with_default(day_count, DayCount::Act360)?;
        let bdc = parse_optional_with_default(
            business_day_convention,
            BusinessDayConvention::ModifiedFollowing,
        )?;
        let stub_kind = parse_optional_with_default(stub, StubKind::ShortFront)?;

        Ok(JsBasisSwapLeg {
            inner: BasisSwapLeg {
                forward_curve_id: curve_id_from_str(forward_curve),
                discount_curve_id: curve_id_from_str(discount_curve),
                start: start.inner(),
                end: end.inner(),
                frequency: freq,
                day_count: dc,
                bdc,
                calendar_id,
                stub: stub_kind,
                spread_bp: opt_f64_to_decimal(spread_bp, "spread_bp")?,
                payment_lag_days: 0,
                reset_lag_days: 0,
            },
        })
    }

    #[wasm_bindgen(getter, js_name = forwardCurve)]
    pub fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn spread_bp(&self) -> f64 {
        self.inner.spread_bp.to_f64().unwrap_or(0.0)
    }
}

#[wasm_bindgen(js_name = BasisSwap)]
#[derive(Clone, Debug)]
pub struct JsBasisSwap {
    pub(crate) inner: BasisSwap,
}

impl InstrumentWrapper for JsBasisSwap {
    type Inner = BasisSwap;
    fn from_inner(inner: BasisSwap) -> Self {
        JsBasisSwap { inner }
    }
    fn inner(&self) -> BasisSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = BasisSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsBasisSwapBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    primary_leg: Option<BasisSwapLeg>,
    reference_leg: Option<BasisSwapLeg>,
}

#[wasm_bindgen(js_class = BasisSwapBuilder)]
impl JsBasisSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsBasisSwapBuilder {
        JsBasisSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsBasisSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = primaryLeg)]
    pub fn primary_leg(mut self, primary_leg: &JsBasisSwapLeg) -> JsBasisSwapBuilder {
        self.primary_leg = Some(primary_leg.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = referenceLeg)]
    pub fn reference_leg(mut self, reference_leg: &JsBasisSwapLeg) -> JsBasisSwapBuilder {
        self.reference_leg = Some(reference_leg.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsBasisSwap, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("BasisSwapBuilder: notional (money) is required".to_string())
        })?;
        let primary_leg = self
            .primary_leg
            .ok_or_else(|| js_error("BasisSwapBuilder: primaryLeg is required".to_string()))?;
        let reference_leg = self
            .reference_leg
            .ok_or_else(|| js_error("BasisSwapBuilder: referenceLeg is required".to_string()))?;

        BasisSwap::new(&self.instrument_id, notional, primary_leg, reference_leg)
            .map(JsBasisSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = BasisSwap)]
impl JsBasisSwap {
    /// Create a float/float basis swap.
    ///
    /// Each leg owns its own dates, discount curve, calendar, and stub conventions.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Swap notional (currency-tagged)
    /// @param primary_leg - Primary floating leg specification
    /// @param reference_leg - Reference floating leg specification
    /// @returns A new `BasisSwap`
    /// @throws {Error} If inputs are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { BasisSwap, BasisSwapLeg, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const start = new FsDate(2024, 1, 2);
    /// const end = new FsDate(2029, 1, 2);
    /// const leg3m = new BasisSwapLeg("USD-SOFR-3M", "USD-OIS", start, end, "3M", "act_360", 5.0);
    /// const leg1m = new BasisSwapLeg("USD-SOFR-1M", "USD-OIS", start, end, "1M", "act_360", 0.0);
    /// const swap = BasisSwap.fromLegs("basis_1", Money.fromCode(10_000_000, "USD"), leg3m, leg1m);
    /// ```
    #[wasm_bindgen(js_name = fromLegs)]
    pub fn from_legs(
        instrument_id: &str,
        notional: &JsMoney,
        primary_leg: &JsBasisSwapLeg,
        reference_leg: &JsBasisSwapLeg,
    ) -> Result<JsBasisSwap, JsValue> {
        BasisSwap::new(
            instrument_id,
            notional.inner(),
            primary_leg.inner.clone(),
            reference_leg.inner.clone(),
        )
        .map(JsBasisSwap::from_inner)
        .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsBasisSwap, JsValue> {
        from_js_value(value).map(JsBasisSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Get projected floating-leg cashflows for this basis swap.
    ///
    /// Returns an array of cashflow tuples: [date, amount, kind, outstanding_balance]
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        use finstack_core::dates::DateExt;
        use finstack_core::dates::DayCountCtx;

        let disc = market
            .inner()
            .get_discount(self.inner.primary_leg.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let result = Array::new();

        for (leg_kind, sign, leg) in [
            ("PrimaryFloat", 1.0, &self.inner.primary_leg),
            ("ReferenceFloat", -1.0, &self.inner.reference_leg),
        ] {
            let schedule = self
                .inner
                .leg_schedule(leg)
                .map_err(|e| js_error(e.to_string()))?;
            if schedule.dates.len() < 2 {
                continue;
            }

            let fwd = market
                .inner()
                .get_forward(leg.forward_curve_id.as_str())
                .map_err(|e| js_error(e.to_string()))?;
            let fwd_dc = fwd.day_count();
            let fwd_base = fwd.base_date();

            let cal = leg
                .calendar_id
                .as_deref()
                .and_then(|id| finstack_core::dates::CalendarRegistry::global().resolve_str(id));

            for i in 1..schedule.dates.len() {
                let period_start = schedule.dates[i - 1];
                let period_end = schedule.dates[i];

                let payment_date = if leg.payment_lag_days == 0 {
                    period_end
                } else if let Some(cal) = cal {
                    period_end
                        .add_business_days(leg.payment_lag_days, cal)
                        .map_err(|e| js_error(e.to_string()))?
                } else {
                    period_end + time::Duration::days(leg.payment_lag_days as i64)
                };

                if payment_date <= as_of {
                    continue;
                }

                let t_start = fwd_dc
                    .year_fraction(fwd_base, period_start, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;
                let t_end = fwd_dc
                    .year_fraction(fwd_base, period_end, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;
                let forward_rate = fwd.rate_period(t_start, t_end);

                let accrual = leg
                    .day_count
                    .year_fraction(period_start, period_end, DayCountCtx::default())
                    .map_err(|e| js_error(e.to_string()))?;

                let coupon = sign
                    * self.inner.notional.amount()
                    * (forward_rate + leg.spread_bp.to_f64().unwrap_or(0.0) / 10_000.0)
                    * accrual;
                let entry = Array::new();
                entry.push(&JsDate::from_core(payment_date).into());
                entry.push(
                    &JsMoney::from_inner(finstack_core::money::Money::new(
                        coupon,
                        self.inner.notional.currency(),
                    ))
                    .into(),
                );
                entry.push(&JsValue::from_str(leg_kind));
                entry.push(&JsValue::NULL);
                result.push(&entry);
            }
        }

        Ok(result)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::BasisSwap.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("BasisSwap(id='{}')", self.inner.id)
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBasisSwap {
        JsBasisSwap::from_inner(self.inner.clone())
    }
}
