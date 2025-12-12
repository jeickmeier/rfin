//! Cross-Currency Basis Calibration.
//!
//! Calibrates a basis spread curve (or foreign discount curve) from Cross-Currency Basis Swaps.
//!
//! The standard market convention is to add a spread $s(t)$ to the foreign floating leg
//! (or sometimes the domestic one depending on quote convention) such that the swap PV is zero.
//!
//! This calibrator solves for the spreads $s_i$ at each knot point that reprice the
//! basis swaps to zero, given the domestic discount curve and foreign projection curves.
//!
//! # Status
//!
//! This module is currently **disabled** by design.
//!
//! The existing implementation treated XCCY basis swaps as if they were a single-currency
//! `BasisSwap` and relied on implicit/hardcoded assumptions (domestic curve ID, FX=1.0) and
//! schedule fallbacks that could silently produce empty schedules. That is not market-standard
//! and can yield materially wrong results.
//!
//! A market-standard implementation requires:
//! - explicit domestic/foreign discounting conventions,
//! - explicit FX spot/forward usage and conversion policy,
//! - multi-currency cashflow modelling (legs in different currencies),
//! - spot/settlement calendars and date conventions.
//!
//! Until the instrument layer supports multi-currency legs (or a dedicated XCCY instrument),
//! we fail fast to avoid providing misleading results.

use crate::calibration::quote::RatesQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::Solver;
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};

/// XCCY Basis Calibrator.
#[derive(Clone, Debug)]
pub struct XccyBasisCalibrator {
    /// Curve identifier for the resulting basis-adjusted discount curve
    pub curve_id: CurveId,
    /// Base date
    pub base_date: Date,
    /// Currency of the curve being calibrated (usually the foreign currency in the pair)
    pub currency: Currency,
    /// Configuration
    pub config: CalibrationConfig,
}

impl XccyBasisCalibrator {
    /// Create a new XCCY Basis Calibrator.
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            currency,
            config: CalibrationConfig::default(),
        }
    }

    /// Bootstrap basis curve from quotes.
    ///
    /// This simplifies the problem by assuming we are calibrating a "Foreign Discount Curve"
    /// that includes the basis spread.
    ///
    /// $ DF_{foreign}(t) = DF_{domestic}(t) \times \frac{S_0}{F(t)} $
    ///
    /// However, usually we calibrate the discount curve directly to reprice the XCCY swaps.
    ///
    /// # Arguments
    /// * `quotes`: List of XCCY Basis Swap quotes
    /// * `solver`: Numerical solver
    /// * `base_context`: Market context containing:
    ///     - Domestic Discount Curve
    ///     - Domestic Forward Curve (if needed)
    ///     - Foreign Forward Curve (for the foreign leg projection)
    ///     - Spot FX Rate
    pub fn bootstrap<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        let _ = (quotes, solver, base_context);
        Err(finstack_core::Error::Calibration {
            category: "xccy_not_implemented".to_string(),
            message: "XCCY basis calibration is disabled: current implementation was not market-standard (single-currency basis swap proxy, hardcoded curve/FX assumptions, and potential silent schedule fallbacks). Use/implement a dedicated multi-currency XCCY instrument + explicit FX/discounting conventions before enabling."
                .to_string(),
        })
    }

    // NOTE: Previously this module included a single-currency proxy pricer using `BasisSwap`.
    // That approach cannot represent multi-currency legs and is intentionally removed.
}
