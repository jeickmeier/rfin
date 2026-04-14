//! WASM bindings for `finstack_core::market_data` term structures and FX.

use std::sync::Arc;

use crate::utils::to_js_err;
use finstack_core::currency::Currency as RustCurrency;
use finstack_core::dates::Date;
use finstack_core::dates::DayCount;
use finstack_core::market_data::surfaces::VolCube as RustVolCube;
use finstack_core::market_data::term_structures::{
    DiscountCurve as RustDiscountCurve, ForwardCurve as RustForwardCurve,
};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use finstack_core::math::volatility::sabr::SabrParams;
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

// ---------------------------------------------------------------------------
// VolCube
// ---------------------------------------------------------------------------

/// SABR volatility cube for swaption pricing.
///
/// Stores calibrated SABR parameters on an expiry × tenor grid and evaluates
/// implied volatilities via bilinear parameter interpolation followed by the
/// Hagan (2002) approximation.
#[wasm_bindgen(js_name = VolCube)]
pub struct VolCube {
    #[wasm_bindgen(skip)]
    pub(crate) inner: Arc<RustVolCube>,
}

#[wasm_bindgen(js_class = VolCube)]
impl VolCube {
    /// Construct a vol cube from a flat SABR parameter array.
    ///
    /// # Arguments
    /// * `id` - Curve identifier.
    /// * `expiries` - Option expiry axis in years (strictly increasing).
    /// * `tenors` - Swap tenor axis in years (strictly increasing).
    /// * `params_flat` - Row-major flat array of SABR parameters:
    ///   `[alpha0, beta0, rho0, nu0, shift0, alpha1, …]`.
    ///   Length must equal `expiries.len() * tenors.len() * 5`.
    ///   Pass `NaN` for the shift element of a node to omit the shift.
    /// * `forwards` - Row-major forward rates, one per grid node.
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        expiries: &[f64],
        tenors: &[f64],
        params_flat: &[f64],
        forwards: &[f64],
    ) -> Result<VolCube, JsValue> {
        let n_nodes = expiries.len() * tenors.len();
        if params_flat.len() != n_nodes * 5 {
            return Err(JsValue::from_str(&format!(
                "params_flat length {} != {} nodes * 5 params",
                params_flat.len(),
                n_nodes
            )));
        }
        let mut sabr_params = Vec::with_capacity(n_nodes);
        for i in 0..n_nodes {
            let base = i * 5;
            let mut p = SabrParams::new(
                params_flat[base],     // alpha
                params_flat[base + 1], // beta
                params_flat[base + 2], // rho
                params_flat[base + 3], // nu
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
            let shift = params_flat[base + 4];
            if shift.is_finite() {
                p = p.with_shift(shift);
            }
            sabr_params.push(p);
        }
        let cube = RustVolCube::from_grid(id, expiries, tenors, &sabr_params, forwards)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(Self {
            inner: Arc::new(cube),
        })
    }

    /// Implied volatility at `(expiry, tenor, strike)`.
    ///
    /// Returns `Err` if `expiry` or `tenor` falls outside the grid.
    pub fn vol(&self, expiry: f64, tenor: f64, strike: f64) -> Result<f64, JsValue> {
        self.inner
            .vol(expiry, tenor, strike)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Implied volatility with clamped extrapolation.
    ///
    /// Clamps `expiry` and `tenor` to the grid edges before interpolation.
    /// Never returns `Err`.
    pub fn vol_clamped(&self, expiry: f64, tenor: f64, strike: f64) -> f64 {
        self.inner.vol_clamped(expiry, tenor, strike)
    }

    /// Cube identifier.
    #[wasm_bindgen(getter, js_name = id)]
    pub fn id(&self) -> String {
        self.inner.id().as_str().to_string()
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

    #[test]
    fn vol_cube_new_and_accessors() {
        // 2 expiries x 2 tenors = 4 nodes, 20 params total
        let expiries = [1.0_f64, 2.0];
        let tenors = [5.0_f64, 10.0];
        // [alpha, beta, rho, nu, shift(NaN = none)] per node
        #[rustfmt::skip]
        let params_flat = [
            0.035, 0.5, -0.2, 0.4, f64::NAN,  // expiry=1, tenor=5
            0.040, 0.5, -0.25, 0.45, f64::NAN, // expiry=1, tenor=10
            0.030, 0.5, -0.15, 0.35, f64::NAN, // expiry=2, tenor=5
            0.038, 0.5, -0.22, 0.42, f64::NAN, // expiry=2, tenor=10
        ];
        let forwards = [0.03_f64, 0.032, 0.035, 0.037];

        let cube = VolCube::new("USD-SWAPTION", &expiries, &tenors, &params_flat, &forwards)
            .expect("vol cube construction");
        assert_eq!(cube.id(), "USD-SWAPTION");

        // vol at an interior grid node with ATM strike
        let v = cube.vol(1.0, 5.0, 0.03).expect("vol");
        assert!(v > 0.0, "vol should be positive, got {v}");

        // vol_clamped extrapolates without error
        let v_ext = cube.vol_clamped(0.5, 3.0, 0.03);
        assert!(v_ext > 0.0, "vol_clamped should be positive, got {v_ext}");

        // out-of-bounds vol returns Err
        assert!(cube.vol(0.5, 5.0, 0.03).is_err());
    }

    #[test]
    fn vol_cube_params_flat_length_error() {
        let expiries = [1.0_f64];
        let tenors = [5.0_f64];
        // 1 node needs 5 params — supply 4 to trigger error
        let result = VolCube::new("BAD", &expiries, &tenors, &[0.035, 0.5, -0.2, 0.4], &[0.03]);
        assert!(result.is_err());
    }
}
