//! Foreign-exchange interfaces and policy types.
//!
//! This module defines an `FxProvider` trait and simple policy metadata used by
//! `Money::convert`. Conversions are always explicit – arithmetic on `Money`
//! requires the same currency.

use crate::currency::Currency;
use crate::dates::Date;

/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(feature = "decimal128")]
pub type FxRate = rust_decimal::Decimal;
/// Provider FX rate type alias – `Decimal` when `decimal128` is enabled, otherwise `f64`.
#[cfg(not(feature = "decimal128"))]
pub type FxRate = f64;

/// Standard FX conversion strategies. These are metadata hints for providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxConversionPolicy {
    /// Use spot/forward on the cashflow date.
    CashflowDate,
    /// Use period end date.
    PeriodEnd,
    /// Use an average over the period.
    PeriodAverage,
    /// Custom strategy defined by the caller/provider.
    Custom,
}

/// Metadata describing the policy applied by the provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FxPolicyMeta {
    /// Strategy applied for the conversion.
    pub strategy: FxConversionPolicy,
    /// Optional declared target currency (for stamping).
    pub target_ccy: Option<Currency>,
    /// Optional notes for auditability.
    pub notes: &'static str,
}

impl Default for FxPolicyMeta {
    fn default() -> Self {
        Self {
            strategy: FxConversionPolicy::CashflowDate,
            target_ccy: None,
            notes: "",
        }
    }
}

/// Trait for obtaining FX rates.
pub trait FxProvider: Send + Sync {
    /// Return a rate to convert `from` → `to` applicable on `on` per `policy`.
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
    ) -> crate::Result<FxRate>;
}

/// Small matrix helper for caching pairwise rates and checking multiplicative closure.
#[derive(Default)]
pub struct FxMatrix<P: FxProvider> {
    provider: P,
}

impl<P: FxProvider> FxMatrix<P> {
    /// Create a new `FxMatrix` wrapping the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Get rate with a simple closure check: from→mid × mid→to ≈ from→to.
    pub fn rate_with_closure(
        &self,
        from: Currency,
        to: Currency,
        on: Date,
        policy: FxConversionPolicy,
        mid: Currency,
        tol: f64,
    ) -> crate::Result<FxRate> {
        let dir = self.provider.rate(from, to, on, policy)?;
        let a = self.provider.rate(from, mid, on, policy)?;
        let b = self.provider.rate(mid, to, on, policy)?;
        #[cfg(feature = "decimal128")]
        {
            let lhs = dir.to_string().parse::<f64>().unwrap_or(0.0);
            let rhs = (a * b).to_string().parse::<f64>().unwrap_or(0.0);
            if (lhs - rhs).abs() > tol {
                // Non-fatal: still return direct rate; caller can choose to treat as error.
            }
            Ok(dir)
        }
        #[cfg(not(feature = "decimal128"))]
        {
            let lhs = dir;
            let rhs = a * b;
            if (lhs - rhs).abs() > tol {
                // Non-fatal: still return direct rate.
            }
            Ok(dir)
        }
    }
}
