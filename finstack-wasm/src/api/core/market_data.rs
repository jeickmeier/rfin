//! WASM bindings for `finstack_core::market_data` term structures and FX.

use std::sync::Arc;

use crate::utils::to_js_err;
use finstack_core::currency::Currency as RustCurrency;
use finstack_core::dates::Date;
use finstack_core::dates::DayCount;
use finstack_core::market_data::term_structures::{
    DiscountCurve as RustDiscountCurve, ForwardCurve as RustForwardCurve,
};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::money::fx::{
    FxConversionPolicy as RustFxConversionPolicy, FxMatrix as RustFxMatrix, FxQuery,
    SimpleFxProvider,
};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an ISO date string (`"YYYY-MM-DD"`) into a Rust [`Date`].
fn parse_iso_date(s: &str) -> Result<Date, JsValue> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(to_js_err(format!("expected YYYY-MM-DD, got {s:?}")));
    }
    let year: i32 = parts[0].parse().map_err(to_js_err)?;
    let month_num: u8 = parts[1].parse().map_err(to_js_err)?;
    let day: u8 = parts[2].parse().map_err(to_js_err)?;
    let month = time::Month::try_from(month_num).map_err(to_js_err)?;
    Date::from_calendar_date(year, month, day).map_err(to_js_err)
}

/// Format a [`Date`] as `"YYYY-MM-DD"`.
fn date_to_iso(d: Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month() as u8, d.day())
}

/// Parse a day-count string.
fn parse_day_count(s: &str) -> Result<DayCount, JsValue> {
    s.parse::<DayCount>().map_err(to_js_err)
}

/// Parse an interpolation style string.
fn parse_interp_style(s: &str) -> Result<InterpStyle, JsValue> {
    s.parse::<InterpStyle>().map_err(to_js_err)
}

/// Parse an extrapolation policy string.
fn parse_extrapolation(s: &str) -> Result<ExtrapolationPolicy, JsValue> {
    s.parse::<ExtrapolationPolicy>().map_err(to_js_err)
}

// ---------------------------------------------------------------------------
// DiscountCurve
// ---------------------------------------------------------------------------

/// Discount factor curve for present-value calculations.
#[wasm_bindgen(js_name = DiscountCurve)]
pub struct DiscountCurve {
    #[wasm_bindgen(skip)]
    pub(crate) inner: Arc<RustDiscountCurve>,
}

#[wasm_bindgen(js_class = DiscountCurve)]
impl DiscountCurve {
    /// Construct from an array of `[time, df]` pairs.
    ///
    /// # Arguments
    /// * `id` - Curve identifier.
    /// * `baseDate` - ISO date string (``"YYYY-MM-DD"``).
    /// * `knots` - Flat `[t0, df0, t1, df1, …]` array.
    /// * `interp` - Interpolation style (default ``"monotone_convex"``).
    /// * `extrapolation` - Extrapolation policy (default ``"flat_forward"``).
    /// * `dayCount` - Day-count convention (default ``"act_365f"``).
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        base_date: &str,
        knots: &[f64],
        interp: Option<String>,
        extrapolation: Option<String>,
        day_count: Option<String>,
    ) -> Result<DiscountCurve, JsValue> {
        let base = parse_iso_date(base_date)?;
        let style = match interp {
            Some(ref s) => parse_interp_style(s)?,
            None => InterpStyle::MonotoneConvex,
        };
        let extrap = match extrapolation {
            Some(ref s) => parse_extrapolation(s)?,
            None => ExtrapolationPolicy::FlatForward,
        };
        let dc = match day_count {
            Some(ref s) => parse_day_count(s)?,
            None => DayCount::Act365F,
        };

        if !knots.len().is_multiple_of(2) {
            return Err(to_js_err("knots array must have even length (t, df pairs)"));
        }
        let pairs: Vec<(f64, f64)> = knots.chunks_exact(2).map(|c| (c[0], c[1])).collect();

        let curve = RustDiscountCurve::builder(id)
            .base_date(base)
            .day_count(dc)
            .knots(pairs)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(to_js_err)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Discount factor at year fraction `t`.
    pub fn df(&self, t: f64) -> f64 {
        self.inner.df(t)
    }

    /// Continuously-compounded zero rate at year fraction `t`.
    pub fn zero(&self, t: f64) -> f64 {
        self.inner.zero(t)
    }

    /// Continuously-compounded forward rate between `t1` and `t2`.
    #[wasm_bindgen(js_name = forwardRate)]
    pub fn forward_rate(&self, t1: f64, t2: f64) -> Result<f64, JsValue> {
        self.inner.forward(t1, t2).map_err(to_js_err)
    }

    /// Curve identifier.
    #[wasm_bindgen(getter, js_name = id)]
    pub fn id(&self) -> String {
        self.inner.id().as_str().to_string()
    }

    /// Base date as ISO string.
    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> String {
        date_to_iso(self.inner.base_date())
    }
}

// ---------------------------------------------------------------------------
// ForwardCurve
// ---------------------------------------------------------------------------

/// Forward rate curve for a floating-rate index with a fixed tenor.
#[wasm_bindgen(js_name = ForwardCurve)]
pub struct ForwardCurve {
    #[wasm_bindgen(skip)]
    pub(crate) inner: Arc<RustForwardCurve>,
}

#[wasm_bindgen(js_class = ForwardCurve)]
impl ForwardCurve {
    /// Construct from an array of `[time, rate]` pairs.
    ///
    /// # Arguments
    /// * `id` - Curve identifier.
    /// * `tenor` - Index tenor in years.
    /// * `baseDate` - ISO date string.
    /// * `knots` - Flat `[t0, rate0, t1, rate1, …]` array.
    /// * `dayCount` - Day-count convention (default ``"act_360"``).
    /// * `interp` - Interpolation style (default ``"linear"``).
    /// * `extrapolation` - Extrapolation policy (default ``"flat_forward"``).
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        tenor: f64,
        base_date: &str,
        knots: &[f64],
        day_count: Option<String>,
        interp: Option<String>,
        extrapolation: Option<String>,
    ) -> Result<ForwardCurve, JsValue> {
        let base = parse_iso_date(base_date)?;
        let dc = match day_count {
            Some(ref s) => parse_day_count(s)?,
            None => DayCount::Act360,
        };
        let style = match interp {
            Some(ref s) => parse_interp_style(s)?,
            None => InterpStyle::Linear,
        };
        let extrap = match extrapolation {
            Some(ref s) => parse_extrapolation(s)?,
            None => ExtrapolationPolicy::FlatForward,
        };

        if !knots.len().is_multiple_of(2) {
            return Err(to_js_err(
                "knots array must have even length (t, rate pairs)",
            ));
        }
        let pairs: Vec<(f64, f64)> = knots.chunks_exact(2).map(|c| (c[0], c[1])).collect();

        let curve = RustForwardCurve::builder(id, tenor)
            .base_date(base)
            .day_count(dc)
            .knots(pairs)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(to_js_err)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Forward rate at year fraction `t`.
    pub fn rate(&self, t: f64) -> f64 {
        self.inner.rate(t)
    }

    /// Curve identifier.
    #[wasm_bindgen(getter, js_name = id)]
    pub fn id(&self) -> String {
        self.inner.id().as_str().to_string()
    }

    /// Base date as ISO string.
    #[wasm_bindgen(getter, js_name = baseDate)]
    pub fn base_date(&self) -> String {
        date_to_iso(self.inner.base_date())
    }
}

// ---------------------------------------------------------------------------
// FxMatrix
// ---------------------------------------------------------------------------

/// Foreign-exchange rate matrix for currency conversion.
#[wasm_bindgen(js_name = FxMatrix)]
pub struct FxMatrix {
    provider: Arc<SimpleFxProvider>,
    inner: RustFxMatrix,
}

impl Default for FxMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen(js_class = FxMatrix)]
impl FxMatrix {
    /// Create an empty FX matrix.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let provider = Arc::new(SimpleFxProvider::new());
        let matrix = RustFxMatrix::new(provider.clone());
        Self {
            provider,
            inner: matrix,
        }
    }

    /// Set an explicit FX quote.
    ///
    /// **Note:** Each call rebuilds the internal rate matrix.  When setting
    /// many quotes at once, prefer calling `setQuote` in a batch and
    /// querying rates only after all quotes are loaded.
    ///
    /// # Arguments
    /// * `base` - Base (from) currency ISO code.
    /// * `quote` - Quote (to) currency ISO code.
    /// * `rate` - Conversion rate.
    #[wasm_bindgen(js_name = setQuote)]
    pub fn set_quote(&mut self, base: &str, quote: &str, rate: f64) -> Result<(), JsValue> {
        let base_ccy: RustCurrency = base.parse().map_err(to_js_err)?;
        let quote_ccy: RustCurrency = quote.parse().map_err(to_js_err)?;
        self.provider
            .set_quote(base_ccy, quote_ccy, rate)
            .map_err(to_js_err)?;
        self.inner = RustFxMatrix::new(self.provider.clone());
        Ok(())
    }

    /// Look up an FX rate.
    ///
    /// # Arguments
    /// * `base` - Base (from) currency ISO code.
    /// * `quote` - Quote (to) currency ISO code.
    /// * `date` - ISO date string.
    /// * `policy` - Conversion policy string (default ``"cashflow_date"``).
    pub fn rate(
        &self,
        base: &str,
        quote: &str,
        date: &str,
        policy: Option<String>,
    ) -> Result<f64, JsValue> {
        let base_ccy: RustCurrency = base.parse().map_err(to_js_err)?;
        let quote_ccy: RustCurrency = quote.parse().map_err(to_js_err)?;
        let d = parse_iso_date(date)?;
        let pol: RustFxConversionPolicy = match policy {
            Some(ref s) => s.parse().map_err(to_js_err)?,
            None => RustFxConversionPolicy::CashflowDate,
        };

        let query = FxQuery::with_policy(base_ccy, quote_ccy, d, pol);
        let result = self.inner.rate(query).map_err(to_js_err)?;
        Ok(result.rate)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::dates::{DayCount, Month};
    use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};

    #[test]
    fn parse_iso_date_components_and_roundtrip() {
        let d = parse_iso_date("2024-01-15").expect("valid ISO date");
        assert_eq!(d.year(), 2024);
        assert_eq!(d.month(), Month::January);
        assert_eq!(d.day(), 15);
        assert_eq!(date_to_iso(d), "2024-01-15");
    }

    #[test]
    fn date_to_iso_roundtrips_parse() {
        let s = "2024-06-30";
        let d = parse_iso_date(s).expect("valid ISO date");
        assert_eq!(date_to_iso(d), s);
    }

    #[test]
    fn parse_day_count_act_variants() {
        assert_eq!(
            parse_day_count("act_365f").expect("act_365f"),
            DayCount::Act365F
        );
        assert_eq!(
            parse_day_count("act_360").expect("act_360"),
            DayCount::Act360
        );
    }

    #[test]
    fn parse_interp_style_variants() {
        assert_eq!(
            parse_interp_style("linear").expect("linear"),
            InterpStyle::Linear
        );
        assert_eq!(
            parse_interp_style("monotone_convex").expect("monotone_convex"),
            InterpStyle::MonotoneConvex
        );
    }

    #[test]
    fn parse_extrapolation_variants() {
        assert_eq!(
            parse_extrapolation("flat_forward").expect("flat_forward"),
            ExtrapolationPolicy::FlatForward
        );
        assert_eq!(
            parse_extrapolation("flat").expect("flat"),
            ExtrapolationPolicy::FlatZero
        );
    }

    #[test]
    fn discount_curve_new_and_accessors() {
        let curve = DiscountCurve::new(
            "USD-OIS",
            "2024-01-15",
            &[0.5, 0.99, 1.0, 0.98, 2.0, 0.96],
            None,
            None,
            None,
        )
        .expect("discount curve");
        assert_eq!(curve.id(), "USD-OIS");
        assert_eq!(curve.base_date(), "2024-01-15");
        assert!((curve.df(0.5) - 0.99).abs() < 1e-6);
        assert!((curve.df(1.0) - 0.98).abs() < 1e-6);
        assert!(curve.zero(1.0) > 0.0);
        let f = curve.forward_rate(0.5, 1.0).expect("forward rate");
        assert!(f > 0.0);
    }

    #[test]
    fn forward_curve_new_and_accessors() {
        let curve = ForwardCurve::new(
            "USD-3M",
            0.25,
            "2024-01-15",
            &[0.5, 0.04, 1.0, 0.045, 2.0, 0.05],
            None,
            None,
            None,
        )
        .expect("forward curve");
        assert_eq!(curve.id(), "USD-3M");
        assert_eq!(curve.base_date(), "2024-01-15");
        assert!((curve.rate(1.0) - 0.045).abs() < 1e-6);
    }

    #[test]
    fn fx_matrix_quote_and_rate() {
        let mut m = FxMatrix::new();
        m.set_quote("USD", "EUR", 0.92).expect("set quote");
        let r = m.rate("USD", "EUR", "2024-01-15", None).expect("fx rate");
        assert!((r - 0.92).abs() < 1e-9);
    }
}
