//! Calibration method bindings for WASM.

use super::config::JsCalibrationConfig;
use super::quote::{JsCreditQuote, JsInflationQuote, JsRatesQuote, JsVolQuote};
use super::report::JsCalibrationReport;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::core::market_data::term_structures::{
    JsDiscountCurve, JsForwardCurve, JsHazardCurve, JsInflationCurve,
};
use crate::core::market_data::VolSurface as JsVolSurface;
use finstack_valuations::calibration::methods::{
    DiscountCurveCalibrator, ForwardCurveCalibrator, HazardCurveCalibrator,
    InflationCurveCalibrator, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::{
    Calibrator, CreditQuote, InflationQuote, RatesQuote, VolQuote,
};
use wasm_bindgen::prelude::*;

/// Discount curve calibrator for bootstrapping OIS/Treasury curves.
#[wasm_bindgen(js_name = DiscountCurveCalibrator)]
#[derive(Clone)]
pub struct JsDiscountCurveCalibrator {
    inner: DiscountCurveCalibrator,
}

#[wasm_bindgen(js_class = DiscountCurveCalibrator)]
impl JsDiscountCurveCalibrator {
    /// Create a new discount curve calibrator.
    ///
    /// @param {string} curve_id - Identifier for the calibrated curve (e.g., "USD-OIS")
    /// @param {Date} base_date - Valuation date corresponding to t=0
    /// @param {string} currency - Currency code ("USD", "EUR", etc.)
    /// @returns {DiscountCurveCalibrator} Calibrator ready to fit curves to market quotes
    /// @throws {Error} If currency code is invalid
    ///
    /// @example
    /// ```javascript
    /// const calibrator = new DiscountCurveCalibrator(
    ///   "USD-OIS",
    ///   new Date(2024, 1, 2),
    ///   "USD"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        curve_id: &str,
        base_date: &FsDate,
        currency: &str,
    ) -> Result<JsDiscountCurveCalibrator, JsValue> {
        let ccy: finstack_core::currency::Currency = currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", currency)))?;
        Ok(Self {
            inner: DiscountCurveCalibrator::new(curve_id, base_date.inner(), ccy),
        })
    }

    /// Configure the calibrator with solver settings and tolerances.
    ///
    /// @param {CalibrationConfig} config - Configuration with solver kind, tolerance, iterations
    /// @returns {DiscountCurveCalibrator} New calibrator with updated configuration
    ///
    /// @example
    /// ```javascript
    /// const config = CalibrationConfig.multiCurve()
    ///   .withSolverKind(SolverKind.Hybrid())
    ///   .withMaxIterations(40);
    ///
    /// const calibrator = new DiscountCurveCalibrator("USD-OIS", baseDate, "USD")
    ///   .withConfig(config);
    /// ```
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(&self, config: &JsCalibrationConfig) -> JsDiscountCurveCalibrator {
        Self {
            inner: self.inner.clone().with_config(config.inner()),
        }
    }

    /// Calibrate the discount curve to market quotes.
    ///
    /// Fits the curve to deposit and swap quotes using numerical optimization.
    /// Returns a tuple of [calibrated_curve, calibration_report].
    ///
    /// @param {Array<RatesQuote>} quotes - Market quotes (deposits, swaps) to fit
    /// @param {MarketContext | null} market - Optional existing market context
    /// @returns {Array} Tuple [DiscountCurve, CalibrationReport]
    /// @throws {Error} If calibration fails or quotes are insufficient
    ///
    /// @example
    /// ```javascript
    /// const quotes = [
    ///   RatesQuote.deposit(new Date(2024, 2, 1), 0.0450, 'act_360'),
    ///   RatesQuote.swap(new Date(2025, 1, 2), 0.0475, Frequency.annual(),
    ///                   Frequency.quarterly(), '30_360', 'act_360', 'USD-SOFR')
    /// ];
    ///
    /// const [curve, report] = calibrator.calibrate(quotes, null);
    /// console.log('Success:', report.success);
    /// console.log('Iterations:', report.iterations);
    /// console.log('DF at 1Y:', curve.df(1.0));
    /// ```
    #[wasm_bindgen]
    pub fn calibrate(
        &self,
        quotes: Vec<JsRatesQuote>,
        market: Option<JsMarketContext>,
    ) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<RatesQuote> = quotes.iter().map(|q| q.inner()).collect();
        let default_ctx = finstack_core::market_data::context::MarketContext::new();
        let base_context = market.as_ref().map(|m| m.inner()).unwrap_or(&default_ctx);

        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, base_context)
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [curve, report] array
        let result = js_sys::Array::new();
        result.push(&JsDiscountCurve::from_arc(std::sync::Arc::new(curve)).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}

/// Forward curve calibrator.
#[wasm_bindgen(js_name = ForwardCurveCalibrator)]
#[derive(Clone)]
pub struct JsForwardCurveCalibrator {
    inner: ForwardCurveCalibrator,
}

#[wasm_bindgen(js_class = ForwardCurveCalibrator)]
impl JsForwardCurveCalibrator {
    /// Create a new forward curve calibrator.
    #[wasm_bindgen(constructor)]
    pub fn new(
        curve_id: &str,
        tenor_years: f64,
        base_date: &FsDate,
        currency: &str,
        discount_curve_id: &str,
    ) -> Result<JsForwardCurveCalibrator, JsValue> {
        let ccy: finstack_core::currency::Currency = currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", currency)))?;
        Ok(Self {
            inner: ForwardCurveCalibrator::new(
                curve_id,
                tenor_years,
                base_date.inner(),
                ccy,
                discount_curve_id,
            ),
        })
    }

    /// Set calibration configuration.
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(&self, config: &JsCalibrationConfig) -> JsForwardCurveCalibrator {
        Self {
            inner: self.inner.clone().with_config(config.inner()),
        }
    }

    /// Calibrate to market quotes.
    #[wasm_bindgen]
    pub fn calibrate(
        &self,
        quotes: Vec<JsRatesQuote>,
        market: &JsMarketContext,
    ) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<RatesQuote> = quotes.iter().map(|q| q.inner()).collect();

        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, market.inner())
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [curve, report] array
        let result = js_sys::Array::new();
        result.push(&JsForwardCurve::from_arc(std::sync::Arc::new(curve)).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}

/// Hazard curve calibrator.
#[wasm_bindgen(js_name = HazardCurveCalibrator)]
#[derive(Clone)]
pub struct JsHazardCurveCalibrator {
    inner: HazardCurveCalibrator,
}

#[wasm_bindgen(js_class = HazardCurveCalibrator)]
impl JsHazardCurveCalibrator {
    /// Create a new hazard curve calibrator.
    #[wasm_bindgen(constructor)]
    pub fn new(
        entity: &str,
        seniority: &str,
        recovery_rate: f64,
        base_date: &FsDate,
        currency: &str,
        discount_curve_id: Option<String>,
    ) -> Result<JsHazardCurveCalibrator, JsValue> {
        let sen: finstack_core::market_data::term_structures::Seniority = seniority
            .parse()
            .map_err(|e: String| JsValue::from_str(&e))?;
        let ccy: finstack_core::currency::Currency = currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", currency)))?;

        let inner = if let Some(ref curve_id) = discount_curve_id {
            HazardCurveCalibrator::new(
                entity,
                sen,
                recovery_rate,
                base_date.inner(),
                ccy,
                curve_id.as_str(),
            )
        } else {
            HazardCurveCalibrator::new_with_default_discount(
                entity,
                sen,
                recovery_rate,
                base_date.inner(),
                ccy,
            )
        };

        Ok(Self { inner })
    }

    /// Set calibration configuration.
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(&self, config: &JsCalibrationConfig) -> JsHazardCurveCalibrator {
        Self {
            inner: self.inner.clone().with_config(config.inner()),
        }
    }

    /// Calibrate to credit quotes.
    #[wasm_bindgen]
    pub fn calibrate(
        &self,
        quotes: Vec<JsCreditQuote>,
        market: &JsMarketContext,
    ) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<CreditQuote> = quotes.iter().map(|q| q.inner()).collect();

        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, market.inner())
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [curve, report] array
        let result = js_sys::Array::new();
        result.push(&JsHazardCurve::from_arc(std::sync::Arc::new(curve)).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}

/// Inflation curve calibrator.
#[wasm_bindgen(js_name = InflationCurveCalibrator)]
#[derive(Clone)]
pub struct JsInflationCurveCalibrator {
    inner: InflationCurveCalibrator,
}

#[wasm_bindgen(js_class = InflationCurveCalibrator)]
impl JsInflationCurveCalibrator {
    /// Create a new inflation curve calibrator.
    #[wasm_bindgen(constructor)]
    pub fn new(
        curve_id: &str,
        base_date: &FsDate,
        currency: &str,
        base_cpi: f64,
        discount_curve_id: &str,
    ) -> Result<JsInflationCurveCalibrator, JsValue> {
        let ccy: finstack_core::currency::Currency = currency
            .parse()
            .map_err(|_| JsValue::from_str(&format!("Unknown currency: {}", currency)))?;

        Ok(Self {
            inner: InflationCurveCalibrator::new(
                curve_id,
                base_date.inner(),
                ccy,
                base_cpi,
                discount_curve_id,
            ),
        })
    }

    /// Set calibration configuration.
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(&self, config: &JsCalibrationConfig) -> JsInflationCurveCalibrator {
        Self {
            inner: self.inner.clone().with_config(config.inner()),
        }
    }

    /// Calibrate to inflation quotes.
    #[wasm_bindgen]
    pub fn calibrate(
        &self,
        quotes: Vec<JsInflationQuote>,
        market: &JsMarketContext,
    ) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<InflationQuote> = quotes.iter().map(|q| q.inner()).collect();

        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, market.inner())
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [curve, report] array
        let result = js_sys::Array::new();
        result.push(&JsInflationCurve::from_arc(std::sync::Arc::new(curve)).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}

/// Volatility surface calibrator.
#[wasm_bindgen(js_name = VolSurfaceCalibrator)]
#[derive(Clone)]
pub struct JsVolSurfaceCalibrator {
    inner: VolSurfaceCalibrator,
}

#[wasm_bindgen(js_class = VolSurfaceCalibrator)]
impl JsVolSurfaceCalibrator {
    /// Create a new vol surface calibrator.
    #[wasm_bindgen(constructor)]
    pub fn new(
        surface_id: &str,
        beta: f64,
        target_expiries: Vec<f64>,
        target_strikes: Vec<f64>,
    ) -> Result<JsVolSurfaceCalibrator, JsValue> {
        if target_expiries.is_empty() {
            return Err(JsValue::from_str("target_expiries must not be empty"));
        }
        if target_strikes.len() < 3 {
            return Err(JsValue::from_str(
                "target_strikes must contain at least three points",
            ));
        }

        Ok(Self {
            inner: VolSurfaceCalibrator::new(surface_id, beta, target_expiries, target_strikes),
        })
    }

    /// Set base date for the surface.
    #[wasm_bindgen(js_name = withBaseDate)]
    pub fn with_base_date(&self, base_date: &FsDate) -> JsVolSurfaceCalibrator {
        Self {
            inner: self.inner.clone().with_base_date(base_date.inner()),
        }
    }

    /// Set calibration configuration.
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(&self, config: &JsCalibrationConfig) -> JsVolSurfaceCalibrator {
        Self {
            inner: self.inner.clone().with_config(config.inner()),
        }
    }

    /// Set discount curve identifier.
    #[wasm_bindgen(js_name = withDiscountId)]
    pub fn with_discount_id(&self, discount_curve_id: &str) -> JsVolSurfaceCalibrator {
        Self {
            inner: self.inner.clone().with_discount_id(discount_curve_id),
        }
    }

    /// Calibrate to volatility quotes.
    #[wasm_bindgen]
    pub fn calibrate(
        &self,
        quotes: Vec<JsVolQuote>,
        market: &JsMarketContext,
    ) -> Result<JsValue, JsValue> {
        let rust_quotes: Vec<VolQuote> = quotes.iter().map(|q| q.inner()).collect();

        let (surface, report) = self
            .inner
            .calibrate(&rust_quotes, market.inner())
            .map_err(|e| JsValue::from_str(&format!("Calibration failed: {}", e)))?;

        // Return as [surface, report] array
        let result = js_sys::Array::new();
        result.push(&JsVolSurface::from_arc(std::sync::Arc::new(surface)).into());
        result.push(&JsCalibrationReport::from_inner(report).into());
        Ok(result.into())
    }
}
