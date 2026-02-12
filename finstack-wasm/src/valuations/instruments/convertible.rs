use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fixed_income::convertible::{
    AntiDilutionPolicy, ConversionEvent, ConversionPolicy, ConversionSpec, ConvertibleBond,
    DividendAdjustment,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = ConversionPolicy)]
#[derive(Clone, Debug)]
pub struct JsConversionPolicy {
    pub(crate) inner: ConversionPolicy,
}

#[wasm_bindgen(js_class = ConversionPolicy)]
impl JsConversionPolicy {
    /// Voluntary conversion (holder may convert at discretion).
    ///
    /// @returns ConversionPolicy
    #[wasm_bindgen(js_name = voluntary)]
    pub fn voluntary() -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::Voluntary,
        }
    }

    /// Mandatory conversion on a specific date.
    ///
    /// @param conversion_date - Date when conversion becomes mandatory
    /// @returns ConversionPolicy
    #[wasm_bindgen(js_name = mandatoryOn)]
    pub fn mandatory_on(conversion_date: &JsDate) -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::MandatoryOn(conversion_date.inner()),
        }
    }

    /// Event-triggered conversion based on a price threshold lookback.
    ///
    /// @param price_threshold - Price trigger threshold (absolute)
    /// @param lookback_days - Lookback window in days
    /// @returns ConversionPolicy
    #[wasm_bindgen(js_name = uponEvent)]
    pub fn upon_event(price_threshold: f64, lookback_days: u32) -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::UponEvent(ConversionEvent::PriceTrigger {
                threshold: price_threshold,
                lookback_days,
            }),
        }
    }

    /// Mandatory variable delivery conversion (PERCS/DECS/ACES).
    ///
    /// At `conversion_date`, the delivery ratio varies with the stock price:
    /// - Below `lower_conversion_price`: max shares (loss participation)
    /// - Between lower and upper: variable ratio delivering face value
    /// - Above `upper_conversion_price`: min shares (capped upside)
    ///
    /// @param conversion_date - Date of mandatory conversion
    /// @param upper_conversion_price - Price above which upside is capped
    /// @param lower_conversion_price - Price below which holder participates in loss
    /// @returns ConversionPolicy
    #[wasm_bindgen(js_name = mandatoryVariable)]
    pub fn mandatory_variable(
        conversion_date: &JsDate,
        upper_conversion_price: f64,
        lower_conversion_price: f64,
    ) -> JsConversionPolicy {
        JsConversionPolicy {
            inner: ConversionPolicy::MandatoryVariable {
                conversion_date: conversion_date.inner(),
                upper_conversion_price,
                lower_conversion_price,
            },
        }
    }
}

#[wasm_bindgen(js_name = ConversionSpec)]
#[derive(Clone, Debug)]
pub struct JsConversionSpec {
    pub(crate) inner: ConversionSpec,
}

#[wasm_bindgen(js_class = ConversionSpec)]
impl JsConversionSpec {
    /// Create a conversion specification for a convertible bond.
    ///
    /// Conventions:
    /// - Provide either `ratio` (shares per bond) or `price` (conversion price). At least one is required.
    ///
    /// @param policy - Conversion policy (voluntary, mandatory on date, or event-triggered)
    /// @param ratio - Optional conversion ratio (shares per bond)
    /// @param price - Optional conversion price (absolute price level)
    /// @returns A `ConversionSpec`
    /// @throws {Error} If neither `ratio` nor `price` is provided
    #[wasm_bindgen(constructor)]
    pub fn new(
        policy: &JsConversionPolicy,
        ratio: Option<f64>,
        price: Option<f64>,
    ) -> Result<JsConversionSpec, JsValue> {
        if ratio.is_none() && price.is_none() {
            return Err(js_error(
                "Provide either conversion ratio or conversion price".to_string(),
            ));
        }

        Ok(JsConversionSpec {
            inner: ConversionSpec {
                ratio,
                price,
                policy: policy.inner.clone(),
                anti_dilution: AntiDilutionPolicy::None,
                dividend_adjustment: DividendAdjustment::None,
                dilution_events: Vec::new(),
            },
        })
    }
}

#[wasm_bindgen(js_name = ConvertibleBond)]
#[derive(Clone, Debug)]
pub struct JsConvertibleBond {
    pub(crate) inner: ConvertibleBond,
}

impl InstrumentWrapper for JsConvertibleBond {
    type Inner = ConvertibleBond;
    fn from_inner(inner: ConvertibleBond) -> Self {
        JsConvertibleBond { inner }
    }
    fn inner(&self) -> ConvertibleBond {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = ConvertibleBondBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsConvertibleBondBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    issue: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    conversion: Option<ConversionSpec>,
    underlying_equity_id: Option<String>,
}

#[wasm_bindgen(js_class = ConvertibleBondBuilder)]
impl JsConvertibleBondBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsConvertibleBondBuilder {
        JsConvertibleBondBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsConvertibleBondBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = issue)]
    pub fn issue(mut self, issue: &JsDate) -> JsConvertibleBondBuilder {
        self.issue = Some(issue.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsConvertibleBondBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsConvertibleBondBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = conversion)]
    pub fn conversion(mut self, conversion: &JsConversionSpec) -> JsConvertibleBondBuilder {
        self.conversion = Some(conversion.inner.clone());
        self
    }

    #[wasm_bindgen(js_name = underlyingEquityId)]
    pub fn underlying_equity_id(
        mut self,
        underlying_equity_id: String,
    ) -> JsConvertibleBondBuilder {
        self.underlying_equity_id = Some(underlying_equity_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsConvertibleBond, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("ConvertibleBondBuilder: notional (money) is required".to_string())
        })?;
        let issue = self
            .issue
            .ok_or_else(|| js_error("ConvertibleBondBuilder: issue is required".to_string()))?;
        let maturity = self
            .maturity
            .ok_or_else(|| js_error("ConvertibleBondBuilder: maturity is required".to_string()))?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("ConvertibleBondBuilder: discountCurve is required".to_string())
        })?;
        let conversion = self.conversion.ok_or_else(|| {
            js_error("ConvertibleBondBuilder: conversion is required".to_string())
        })?;

        let bond = ConvertibleBond {
            id: instrument_id_from_str(&self.instrument_id),
            notional,
            issue,
            maturity,
            discount_curve_id: curve_id_from_str(discount_curve),
            credit_curve_id: None,
            conversion,
            underlying_equity_id: self.underlying_equity_id,
            call_put: None,
            soft_call_trigger: None,
            fixed_coupon: None,
            floating_coupon: None,
            attributes: Default::default(),
        };

        Ok(JsConvertibleBond::from_inner(bond))
    }
}

#[wasm_bindgen(js_class = ConvertibleBond)]
impl JsConvertibleBond {
    /// Create a convertible bond.
    ///
    /// Conventions:
    /// - `discount_curve` is used for discounting cashflows.
    /// - `underlying_equity_id` should correspond to an equity/spot identifier used in pricing models.
    ///
    /// @param instrument_id - Unique identifier
    /// @param notional - Face amount (currency-tagged)
    /// @param issue - Issue date
    /// @param maturity - Maturity date
    /// @param discount_curve - Discount curve ID
    /// @param conversion - Conversion specification (ratio/price + policy)
    /// @param underlying_equity_id - Optional equity identifier for the underlying
    /// @returns A new `ConvertibleBond`
    ///
    /// @example
    /// ```javascript
    /// import init, { ConvertibleBond, ConversionSpec, ConversionPolicy, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const conversion = new ConversionSpec(ConversionPolicy.voluntary(), 20.0, null);
    /// const cb = new ConvertibleBond(
    ///   "cb_1",
    ///   Money.fromCode(1_000_000, "USD"),
    ///   new FsDate(2024, 1, 2),
    ///   new FsDate(2034, 1, 2),
    ///   "USD-OIS",
    ///   conversion,
    ///   "AAPL"
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
        conversion: &JsConversionSpec,
        underlying_equity_id: Option<String>,
    ) -> Result<JsConvertibleBond, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "ConvertibleBond constructor is deprecated; use ConvertibleBondBuilder instead.",
        ));
        let bond = ConvertibleBond {
            id: instrument_id_from_str(instrument_id),
            notional: notional.inner(),
            issue: issue.inner(),
            maturity: maturity.inner(),
            discount_curve_id: curve_id_from_str(discount_curve),
            credit_curve_id: None,
            conversion: conversion.inner.clone(),
            underlying_equity_id,
            call_put: None,
            soft_call_trigger: None,
            fixed_coupon: None,
            floating_coupon: None,
            attributes: Default::default(),
        };

        Ok(JsConvertibleBond::from_inner(bond))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = conversionRatio)]
    pub fn conversion_ratio(&self) -> Option<f64> {
        self.inner.conversion.ratio
    }

    #[wasm_bindgen(getter, js_name = conversionPrice)]
    pub fn conversion_price(&self) -> Option<f64> {
        self.inner.conversion.price
    }

    /// Get a simple cashflow view for this convertible bond.
    ///
    /// Currently returns principal repayment at maturity (coupons are not modeled in the WASM wrapper).
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(
        &self,
        market: &crate::core::market_data::context::JsMarketContext,
    ) -> Result<Array, JsValue> {
        let disc = market
            .inner()
            .get_discount(self.inner.discount_curve_id.as_str())
            .map_err(|e| js_error(e.to_string()))?;
        let as_of = disc.base_date();

        let result = Array::new();
        if self.inner.maturity > as_of {
            let entry = Array::new();
            entry.push(&JsDate::from_core(self.inner.maturity).into());
            entry.push(&JsMoney::from_inner(self.inner.notional).into());
            entry.push(&JsValue::from_str("Principal"));
            entry.push(&JsValue::NULL);
            result.push(&entry);
        }
        Ok(result)
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsConvertibleBond, JsValue> {
        from_js_value(value).map(JsConvertibleBond::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Convertible as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ConvertibleBond(id='{}', notional={})",
            self.inner.id, self.inner.notional
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsConvertibleBond {
        JsConvertibleBond::from_inner(self.inner.clone())
    }
}
