//! XVA (Valuation Adjustments) framework WASM bindings.
//!
//! Wraps CVA computation, exposure profiling, netting, and collateral from
//! `finstack-margin`. Provides configuration types, result containers, and
//! the core computational functions for counterparty credit risk.

use crate::core::error::{core_to_js, js_error};
use crate::core::market_data::{DiscountCurve, HazardCurve};
use finstack_margin::xva as margin_xva;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// FundingConfig
// ---------------------------------------------------------------------------

/// Funding cost/benefit configuration for FVA calculations.
///
/// @example
/// ```javascript
/// const funding = new FundingConfig(50.0);  // 50 bps funding spread
/// console.log(funding.effectiveBenefitBps);  // 50 (symmetric)
/// ```
#[wasm_bindgen(js_name = FundingConfig)]
#[derive(Clone)]
pub struct JsFundingConfig {
    inner: margin_xva::types::FundingConfig,
}

impl JsFundingConfig {
    pub(crate) fn from_inner(inner: margin_xva::types::FundingConfig) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = FundingConfig)]
impl JsFundingConfig {
    /// Create a new funding configuration.
    ///
    /// @param fundingSpreadBps - Funding spread in basis points
    /// @param fundingBenefitBps - Optional funding benefit spread (defaults to symmetric)
    #[wasm_bindgen(constructor)]
    pub fn new(funding_spread_bps: f64, funding_benefit_bps: Option<f64>) -> Self {
        Self {
            inner: margin_xva::types::FundingConfig {
                funding_spread_bps,
                funding_benefit_bps,
            },
        }
    }

    /// Funding spread in basis points.
    #[wasm_bindgen(getter, js_name = fundingSpreadBps)]
    pub fn funding_spread_bps(&self) -> f64 {
        self.inner.funding_spread_bps
    }

    /// Funding benefit spread in basis points (if set).
    #[wasm_bindgen(getter, js_name = fundingBenefitBps)]
    pub fn funding_benefit_bps(&self) -> Option<f64> {
        self.inner.funding_benefit_bps
    }

    /// Effective benefit spread (falls back to funding spread if not explicitly set).
    #[wasm_bindgen(getter, js_name = effectiveBenefitBps)]
    pub fn effective_benefit_bps(&self) -> f64 {
        self.inner.effective_benefit_bps()
    }
}

// ---------------------------------------------------------------------------
// XvaConfig
// ---------------------------------------------------------------------------

/// Configuration for XVA calculations.
///
/// @example
/// ```javascript
/// const config = new XvaConfig();  // defaults: quarterly to 30Y, 40% recovery
/// console.log(config.recoveryRate);  // 0.4
/// ```
#[wasm_bindgen(js_name = XvaConfig)]
#[derive(Clone)]
pub struct JsXvaConfig {
    inner: margin_xva::types::XvaConfig,
}

#[allow(dead_code)]
impl JsXvaConfig {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: margin_xva::types::XvaConfig) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = XvaConfig)]
impl JsXvaConfig {
    /// Create an XVA configuration with optional overrides.
    ///
    /// @param timeGrid - Time points (years). Defaults to quarterly to 30Y.
    /// @param recoveryRate - Recovery upon default (0 to 1, default 0.40).
    /// @param ownRecoveryRate - Own recovery for DVA (optional).
    /// @param funding - FundingConfig for FVA (optional).
    #[wasm_bindgen(constructor)]
    pub fn new(
        time_grid: Option<Vec<f64>>,
        recovery_rate: Option<f64>,
        own_recovery_rate: Option<f64>,
        funding: Option<JsFundingConfig>,
    ) -> Result<JsXvaConfig, JsValue> {
        let defaults = margin_xva::types::XvaConfig::default();
        let recovery_rate = recovery_rate.unwrap_or(defaults.recovery_rate);
        let time_grid = time_grid.unwrap_or_else(|| defaults.time_grid.clone());
        let inner = margin_xva::types::XvaConfig {
            time_grid,
            recovery_rate,
            own_recovery_rate,
            funding: funding.map(|cfg| cfg.inner),
        };
        inner.validate().map_err(core_to_js)?;
        Ok(Self { inner })
    }

    /// Time grid for exposure simulation (years from today).
    #[wasm_bindgen(getter, js_name = timeGrid)]
    pub fn time_grid(&self) -> Vec<f64> {
        self.inner.time_grid.clone()
    }

    /// Recovery rate for counterparty default.
    #[wasm_bindgen(getter, js_name = recoveryRate)]
    pub fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    /// Recovery rate for own default (DVA).
    #[wasm_bindgen(getter, js_name = ownRecoveryRate)]
    pub fn own_recovery_rate(&self) -> Option<f64> {
        self.inner.own_recovery_rate
    }

    /// Funding configuration for FVA.
    #[wasm_bindgen(getter)]
    pub fn funding(&self) -> Option<JsFundingConfig> {
        self.inner
            .funding
            .as_ref()
            .map(|cfg| JsFundingConfig::from_inner(cfg.clone()))
    }
}

// ---------------------------------------------------------------------------
// CsaTerms
// ---------------------------------------------------------------------------

/// Credit Support Annex terms for collateralization.
///
/// @example
/// ```javascript
/// const csa = new CsaTerms(0.0, 500_000, 10, 0.0);
/// ```
#[wasm_bindgen(js_name = XvaCsaTerms)]
#[derive(Clone)]
pub struct JsXvaCsaTerms {
    inner: margin_xva::types::CsaTerms,
}

#[allow(dead_code)]
impl JsXvaCsaTerms {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: margin_xva::types::CsaTerms) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = XvaCsaTerms)]
impl JsXvaCsaTerms {
    /// Create CSA terms.
    ///
    /// @param threshold - Threshold below which no collateral is required
    /// @param mta - Minimum transfer amount
    /// @param mporDays - Margin period of risk in calendar days
    /// @param independentAmount - Additional collateral independent of MtM
    #[wasm_bindgen(constructor)]
    pub fn new(threshold: f64, mta: f64, mpor_days: u32, independent_amount: f64) -> Self {
        Self {
            inner: margin_xva::types::CsaTerms {
                threshold,
                mta,
                mpor_days,
                independent_amount,
            },
        }
    }

    /// Threshold below which no collateral is required.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    /// Minimum transfer amount.
    #[wasm_bindgen(getter)]
    pub fn mta(&self) -> f64 {
        self.inner.mta
    }

    /// Margin period of risk in calendar days.
    #[wasm_bindgen(getter, js_name = mporDays)]
    pub fn mpor_days(&self) -> u32 {
        self.inner.mpor_days
    }

    /// Additional collateral independent of MtM.
    #[wasm_bindgen(getter, js_name = independentAmount)]
    pub fn independent_amount(&self) -> f64 {
        self.inner.independent_amount
    }
}

// ---------------------------------------------------------------------------
// XvaNettingSet
// ---------------------------------------------------------------------------

/// Netting set specification under an ISDA master agreement.
///
/// @example
/// ```javascript
/// const ns = new XvaNettingSet("NS1", "CPTY_A");
/// ```
#[wasm_bindgen(js_name = XvaNettingSet)]
#[derive(Clone)]
pub struct JsXvaNettingSet {
    inner: margin_xva::types::NettingSet,
}

#[allow(dead_code)]
impl JsXvaNettingSet {
    #[allow(dead_code)]
    pub(crate) fn from_inner(inner: margin_xva::types::NettingSet) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = XvaNettingSet)]
impl JsXvaNettingSet {
    /// Create a netting set specification.
    ///
    /// @param id - Netting set identifier
    /// @param counterpartyId - Counterparty identifier
    /// @param csa - Optional CSA terms
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, counterparty_id: &str, csa: Option<JsXvaCsaTerms>) -> Self {
        Self {
            inner: margin_xva::types::NettingSet {
                id: id.to_string(),
                counterparty_id: counterparty_id.to_string(),
                csa: csa.map(|c| c.inner),
                reporting_currency: None,
            },
        }
    }

    /// Netting set identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Counterparty identifier.
    #[wasm_bindgen(getter, js_name = counterpartyId)]
    pub fn counterparty_id(&self) -> String {
        self.inner.counterparty_id.clone()
    }
}

// ---------------------------------------------------------------------------
// ExposureProfile
// ---------------------------------------------------------------------------

/// Exposure profile computed at each time grid point.
///
/// Contains times, mark-to-market values, expected positive exposure (EPE),
/// and expected negative exposure (ENE).
#[wasm_bindgen(js_name = ExposureProfile)]
#[derive(Clone)]
pub struct JsExposureProfile {
    inner: margin_xva::types::ExposureProfile,
}

#[allow(dead_code)]
impl JsExposureProfile {
    pub(crate) fn from_inner(inner: margin_xva::types::ExposureProfile) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = ExposureProfile)]
impl JsExposureProfile {
    /// Time points in years from the valuation date.
    #[wasm_bindgen(getter)]
    pub fn times(&self) -> Vec<f64> {
        self.inner.times.clone()
    }

    /// Portfolio mark-to-market at each time point.
    #[wasm_bindgen(getter, js_name = mtmValues)]
    pub fn mtm_values(&self) -> Vec<f64> {
        self.inner.mtm_values.clone()
    }

    /// Expected positive exposure at each time point.
    #[wasm_bindgen(getter)]
    pub fn epe(&self) -> Vec<f64> {
        self.inner.epe.clone()
    }

    /// Expected negative exposure at each time point.
    #[wasm_bindgen(getter)]
    pub fn ene(&self) -> Vec<f64> {
        self.inner.ene.clone()
    }
}

// ---------------------------------------------------------------------------
// XvaResult
// ---------------------------------------------------------------------------

/// Result of XVA calculations including CVA, DVA, FVA, and exposure profiles.
#[wasm_bindgen(js_name = XvaResult)]
#[derive(Clone)]
pub struct JsXvaResult {
    inner: margin_xva::types::XvaResult,
}

impl JsXvaResult {
    pub(crate) fn from_inner(inner: margin_xva::types::XvaResult) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = XvaResult)]
impl JsXvaResult {
    /// Unilateral CVA (positive = cost).
    #[wasm_bindgen(getter)]
    pub fn cva(&self) -> f64 {
        self.inner.cva
    }

    /// Debit Valuation Adjustment.
    #[wasm_bindgen(getter)]
    pub fn dva(&self) -> Option<f64> {
        self.inner.dva
    }

    /// Funding Valuation Adjustment.
    #[wasm_bindgen(getter)]
    pub fn fva(&self) -> Option<f64> {
        self.inner.fva
    }

    /// Bilateral CVA.
    #[wasm_bindgen(getter, js_name = bilateralCva)]
    pub fn bilateral_cva(&self) -> Option<f64> {
        self.inner.bilateral_cva
    }

    /// Maximum potential future exposure.
    #[wasm_bindgen(getter, js_name = maxPfe)]
    pub fn max_pfe(&self) -> f64 {
        self.inner.max_pfe
    }

    /// Effective EPE (time-weighted average per Basel III SA-CCR).
    #[wasm_bindgen(getter, js_name = effectiveEpe)]
    pub fn effective_epe(&self) -> f64 {
        self.inner.effective_epe
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// Apply close-out netting to instrument mark-to-market values.
///
/// @returns Net positive exposure max(sum(values), 0)
#[wasm_bindgen(js_name = applyNetting)]
pub fn apply_netting(instrument_values: Vec<f64>) -> f64 {
    margin_xva::netting::apply_netting(&instrument_values)
}

/// Apply CSA collateral terms to reduce gross exposure.
///
/// @returns Net exposure after collateral (always non-negative)
#[wasm_bindgen(js_name = applyXvaCollateral)]
pub fn apply_collateral(gross_exposure: f64, csa: &JsXvaCsaTerms) -> f64 {
    margin_xva::netting::apply_collateral(gross_exposure, &csa.inner)
}

/// Compute unilateral CVA from an exposure profile.
///
/// @returns XvaResult with CVA value and exposure metrics
#[wasm_bindgen(js_name = computeCva)]
pub fn compute_cva(
    exposure_profile: &JsExposureProfile,
    hazard_curve: &HazardCurve,
    discount_curve: &DiscountCurve,
    recovery_rate: f64,
) -> Result<JsXvaResult, JsValue> {
    if !(0.0..=1.0).contains(&recovery_rate) {
        return Err(js_error("recovery_rate must be between 0 and 1"));
    }
    let hc = hazard_curve.inner();
    let dc = discount_curve.inner();
    let result = margin_xva::cva::compute_cva(&exposure_profile.inner, &hc, &dc, recovery_rate)
        .map_err(core_to_js)?;
    Ok(JsXvaResult::from_inner(result))
}

/// Compute debit valuation adjustment from the negative exposure profile.
#[wasm_bindgen(js_name = computeDva)]
pub fn compute_dva(
    exposure_profile: &JsExposureProfile,
    own_hazard_curve: &HazardCurve,
    discount_curve: &DiscountCurve,
    own_recovery_rate: f64,
) -> Result<f64, JsValue> {
    if !(0.0..=1.0).contains(&own_recovery_rate) {
        return Err(js_error("own_recovery_rate must be between 0 and 1"));
    }
    let hc = own_hazard_curve.inner();
    let dc = discount_curve.inner();
    margin_xva::cva::compute_dva(&exposure_profile.inner, &hc, &dc, own_recovery_rate)
        .map_err(core_to_js)
}

/// Compute funding valuation adjustment from the exposure profile.
#[wasm_bindgen(js_name = computeFva)]
pub fn compute_fva(
    exposure_profile: &JsExposureProfile,
    discount_curve: &DiscountCurve,
    funding_spread_bps: f64,
    funding_benefit_bps: f64,
) -> Result<f64, JsValue> {
    let dc = discount_curve.inner();
    margin_xva::cva::compute_fva(
        &exposure_profile.inner,
        &dc,
        funding_spread_bps,
        funding_benefit_bps,
    )
    .map_err(core_to_js)
}

/// Compute bilateral XVA including CVA, DVA, and optional FVA.
#[wasm_bindgen(js_name = computeBilateralXva)]
pub fn compute_bilateral_xva(
    exposure_profile: &JsExposureProfile,
    counterparty_hazard_curve: &HazardCurve,
    own_hazard_curve: &HazardCurve,
    discount_curve: &DiscountCurve,
    counterparty_recovery_rate: f64,
    own_recovery_rate: f64,
    funding: Option<JsFundingConfig>,
) -> Result<JsXvaResult, JsValue> {
    let chc = counterparty_hazard_curve.inner();
    let ohc = own_hazard_curve.inner();
    let dc = discount_curve.inner();
    let result = margin_xva::cva::compute_bilateral_xva(
        &exposure_profile.inner,
        &chc,
        &ohc,
        &dc,
        counterparty_recovery_rate,
        own_recovery_rate,
        funding.as_ref().map(|cfg| &cfg.inner),
    )
    .map_err(core_to_js)?;
    Ok(JsXvaResult::from_inner(result))
}
