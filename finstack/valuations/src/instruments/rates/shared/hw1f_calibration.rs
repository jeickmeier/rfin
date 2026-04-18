//! Resolve HW1F short-rate parameters (κ, σ) for exotic rate products.
//!
//! Precedence:
//! 1. Explicit overrides in `PricingOverrides.model_config` (keys `hw1f_kappa`, `hw1f_sigma`).
//! 2. Calibrated from the instrument's swaption vol surface (if provided).
//! 3. [`HullWhiteParams::default()`] with a `tracing::warn!` log.

// NOTE: swaption::pricer::HullWhiteParams is used (not
// calibration::hull_white's) because this type has `impl Default` and
// an infallible `new`, which the precedence-resolver needs. Converging
// the two is tracked as a follow-up.
use crate::instruments::rates::swaption::pricer::HullWhiteParams;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Input for HW1F parameter resolution.
pub struct Hw1fResolveRequest<'a> {
    /// Optional vol surface id to calibrate against. When `Some`, the
    /// resolver tries to fit (κ, σ) to the surface's ATM-short-expiry
    /// section before falling back to default.
    pub vol_surface_id: Option<&'a str>,
    /// Optional pricing-override JSON blob (from `PricingOverrides.model_config`).
    pub overrides: Option<&'a serde_json::Value>,
    /// Context label for logs/warns (e.g., "TARN TARN-USD-5Y").
    pub context: &'a str,
}

/// Resolve HW1F parameters following the documented precedence.
///
/// Never returns an error for the "no surface + no overrides" case; instead
/// emits a `tracing::warn!` and returns [`HullWhiteParams::default()`].
/// An error is only returned when overrides are malformed.
///
/// `_market` is currently unused and reserved for the surface-calibration
/// branch (follow-up PR). It is part of the public signature so that
/// wiring it up later is not a breaking change.
pub fn resolve_hw1f_params(
    req: &Hw1fResolveRequest<'_>,
    _market: &MarketContext,
) -> Result<HullWhiteParams> {
    if let Some(obj) = req.overrides.and_then(|v| v.as_object()) {
        let kappa = obj.get("hw1f_kappa").and_then(|x| x.as_f64());
        let sigma = obj.get("hw1f_sigma").and_then(|x| x.as_f64());
        if let (Some(k), Some(s)) = (kappa, sigma) {
            if !k.is_finite() || k <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "hw1f_kappa override must be positive and finite, got {k}"
                )));
            }
            if !s.is_finite() || s <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "hw1f_sigma override must be positive and finite, got {s}"
                )));
            }
            return Ok(HullWhiteParams::new(k, s));
        }
    }

    if let Some(surface_id) = req.vol_surface_id {
        tracing::warn!(
            target = "finstack.exotic_rates",
            context = req.context,
            vol_surface_id = %surface_id,
            "HW1F calibration-from-surface not yet implemented; falling back to HullWhiteParams::default(). Tracked in docs/superpowers/plans/2026-04-16-exotic-rate-products-roadmap.md."
        );
    }

    let defaults = HullWhiteParams::default();
    tracing::warn!(
        target = "finstack.exotic_rates",
        context = req.context,
        kappa = defaults.kappa,
        sigma = defaults.sigma,
        "no HW1F overrides or surface provided; using HullWhiteParams::default()"
    );
    Ok(defaults)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::market_data::context::MarketContext;
    use serde_json::json;

    fn empty_market() -> MarketContext {
        MarketContext::new()
    }

    #[test]
    fn overrides_are_used_when_present() {
        let overrides = json!({ "hw1f_kappa": 0.05, "hw1f_sigma": 0.012 });
        let req = Hw1fResolveRequest {
            vol_surface_id: None,
            overrides: Some(&overrides),
            context: "test",
        };
        let params = resolve_hw1f_params(&req, &empty_market()).expect("ok");
        assert!((params.kappa - 0.05).abs() < 1e-12);
        assert!((params.sigma - 0.012).abs() < 1e-12);
    }

    #[test]
    fn defaults_when_nothing_provided() {
        let req = Hw1fResolveRequest {
            vol_surface_id: None,
            overrides: None,
            context: "test",
        };
        let params = resolve_hw1f_params(&req, &empty_market()).expect("ok");
        let default = HullWhiteParams::default();
        assert!((params.kappa - default.kappa).abs() < 1e-12);
        assert!((params.sigma - default.sigma).abs() < 1e-12);
    }

    #[test]
    fn negative_kappa_errors() {
        let overrides = json!({ "hw1f_kappa": -0.05, "hw1f_sigma": 0.01 });
        let req = Hw1fResolveRequest {
            vol_surface_id: None,
            overrides: Some(&overrides),
            context: "test",
        };
        let err = resolve_hw1f_params(&req, &empty_market()).expect_err("should error");
        assert!(format!("{err}").contains("hw1f_kappa"));
    }

    #[test]
    fn zero_sigma_errors() {
        // Note: JSON does not support NaN/Inf (serde_json drops them to Null), so
        // the `is_finite` branch is unreachable via JSON input. The positivity
        // check is exercised here with `sigma = 0.0`, which must still error.
        let overrides = json!({ "hw1f_kappa": 0.03, "hw1f_sigma": 0.0 });
        let req = Hw1fResolveRequest {
            vol_surface_id: None,
            overrides: Some(&overrides),
            context: "test",
        };
        let err = resolve_hw1f_params(&req, &empty_market()).expect_err("should error");
        assert!(format!("{err}").contains("hw1f_sigma"));
    }

    #[test]
    fn partial_override_falls_through_to_default() {
        let overrides = json!({ "hw1f_kappa": 0.07 });
        let req = Hw1fResolveRequest {
            vol_surface_id: None,
            overrides: Some(&overrides),
            context: "test",
        };
        let params = resolve_hw1f_params(&req, &empty_market()).expect("ok");
        let default = HullWhiteParams::default();
        assert!((params.kappa - default.kappa).abs() < 1e-12);
        assert!((params.sigma - default.sigma).abs() < 1e-12);
    }

    #[test]
    fn surface_id_alone_falls_through_to_default() {
        let req = Hw1fResolveRequest {
            vol_surface_id: Some("USD_SWAPTION_ATM"),
            overrides: None,
            context: "test",
        };
        let params = resolve_hw1f_params(&req, &empty_market()).expect("ok");
        let default = HullWhiteParams::default();
        assert!((params.kappa - default.kappa).abs() < 1e-12);
        assert!((params.sigma - default.sigma).abs() < 1e-12);
    }
}
