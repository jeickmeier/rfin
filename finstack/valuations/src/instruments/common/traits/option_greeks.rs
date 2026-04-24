// ================================================================================================
// Option risk metric providers
// ================================================================================================

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;

/// Supported option greek requests for the consolidated provider API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionGreekKind {
    /// Cash delta in instrument metric convention.
    Delta,
    /// Cash gamma in instrument metric convention.
    Gamma,
    /// Cash vega per 1 vol point.
    Vega,
    /// Theta per instrument day-count convention.
    Theta,
    /// Domestic rho per 1bp.
    Rho,
    /// Foreign/dividend rho per 1bp.
    ForeignRho,
    /// Vanna in instrument bump convention.
    Vanna,
    /// Volga in instrument bump convention.
    Volga,
}

/// Inputs needed to request a specific option greek.
///
/// `base_pv` is required only for [`OptionGreekKind::Volga`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionGreeksRequest {
    /// The greek being requested.
    pub greek: OptionGreekKind,
    /// Base PV required by some greeks such as volga.
    pub base_pv: Option<f64>,
}

impl OptionGreeksRequest {
    /// Return the requested base PV or an error when it is required but missing.
    pub fn require_base_pv(self) -> finstack_core::Result<f64> {
        self.base_pv.ok_or_else(|| {
            finstack_core::Error::Validation(
                "OptionGreekKind::Volga requires base_pv in OptionGreeksRequest".to_string(),
            )
        })
    }
}

/// Sparse option greek payload returned by [`OptionGreeksProvider`].
///
/// Providers should populate the requested field when it is supported for the
/// instrument and leave unsupported greeks as `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OptionGreeks {
    /// Cash delta in instrument metric convention.
    pub delta: Option<f64>,
    /// Cash gamma in instrument metric convention.
    pub gamma: Option<f64>,
    /// Cash vega per 1 vol point.
    pub vega: Option<f64>,
    /// Theta per instrument day-count convention.
    pub theta: Option<f64>,
    /// Domestic rho per 1bp.
    pub rho_bp: Option<f64>,
    /// Foreign/dividend rho per 1bp.
    pub foreign_rho_bp: Option<f64>,
    /// Vanna in instrument bump convention.
    pub vanna: Option<f64>,
    /// Volga in instrument bump convention.
    pub volga: Option<f64>,
}

/// Consolidated option greek provider.
///
/// Implementations return a sparse [`OptionGreeks`] payload keyed by the
/// requested [`OptionGreekKind`]. Callers should interpret `None` as "not
/// supported for this instrument" rather than as a zero-valued greek.
pub trait OptionGreeksProvider {
    /// Return the requested greek in a sparse [`OptionGreeks`] payload.
    fn option_greeks(
        &self,
        market: &MarketContext,
        as_of: Date,
        request: &OptionGreeksRequest,
    ) -> finstack_core::Result<OptionGreeks>;
}

// Single-greek helpers remain crate-private while instrument implementations
// converge on `OptionGreeksProvider`.

/// Provide **cash delta** in the metric convention for this instrument.
///
/// Conventions:
/// - Return value is \(\partial PV / \partial S\) where \(S\) is the instrument's chosen
///   underlying "spot" driver (equity spot, FX spot, forward, etc.).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
/// - At/after expiry, return 0.0 unless the instrument explicitly defines an intrinsic
///   delta convention.
pub(crate) trait OptionDeltaProvider {
    /// Return cash delta per instrument conventions.
    fn option_delta(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide **cash gamma** in the metric convention for this instrument.
///
/// Conventions:
/// - Return value is \(\partial^2 PV / \partial S^2\) using the instrument's chosen
///   underlying "spot" driver \(S\).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
pub(crate) trait OptionGammaProvider {
    /// Return cash gamma per instrument conventions.
    fn option_gamma(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide **cash vega** (per 1 vol point) for this instrument.
///
/// Conventions:
/// - Return value is \(\partial PV / \partial \sigma\) scaled to a **0.01 absolute**
///   volatility move (1 vol point).
/// - The value should already include instrument scaling (notional / quantity / multiplier).
pub(crate) trait OptionVegaProvider {
    /// Return cash vega per instrument conventions (1 vol point).
    fn option_vega(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide theta (per day) in the instrument's convention.
///
/// Conventions:
/// - Return value is the PV change for **one day of time decay** (usually negative for long options).
/// - The day basis (calendar vs trading days) is instrument-specific and must match the
///   instrument's existing pricing/greeks conventions.
pub(crate) trait OptionThetaProvider {
    /// Return theta per instrument conventions (per day).
    fn option_theta(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide rho (domestic) per **1bp** move for this instrument.
///
/// Conventions:
/// - Return value is \(PV(r+1bp) - PV(r)\) for the relevant "domestic" discount driver.
/// - This should be a **finite-difference PV change**, not "per 1%" scaling.
pub(crate) trait OptionRhoProvider {
    /// Return domestic rho per instrument conventions (per 1bp).
    fn option_rho_bp(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Provide foreign/dividend rho per **1bp** move for this instrument.
///
/// Conventions:
/// - Return value is \(PV(q+1bp) - PV(q)\) where \(q\) is the foreign rate/dividend yield
///   driver used by the instrument.
pub(crate) trait OptionForeignRhoProvider {
    /// Return foreign/dividend rho per instrument conventions (per 1bp).
    fn option_foreign_rho_bp(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64>;
}

/// Compute vanna in the instrument's chosen bump conventions.
///
/// Conventions:
/// - Vanna is a mixed derivative (commonly \(\partial^2 PV / \partial S \partial \sigma\)).
/// - Implementations may use spot-then-vol or vol-then-spot bump logic as long as it is
///   consistent with the instrument's historical behavior and bump size settings.
pub(crate) trait OptionVannaProvider {
    /// Return vanna per instrument conventions.
    fn option_vanna(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64>;
}

/// Trait for instruments that can compute volga in their chosen bump conventions.
///
/// `base_pv` should be the already computed PV amount at `as_of` for the same market.
pub(crate) trait OptionVolgaProvider {
    /// Return volga per instrument conventions.
    fn option_volga(
        &self,
        market: &MarketContext,
        as_of: Date,
        base_pv: f64,
    ) -> finstack_core::Result<f64>;
}

/// Implement standard equity-exotic trait boilerplate for instruments with
/// `spot_id`, `vol_surface_id`, `pricing_overrides`, `day_count` fields.
///
/// # Variants
///
/// - With `curve_deps`: also implements `CurveDependencies` using `discount_curve_id`.
/// - For types with custom `HasExpiry`, use the internal `@equity`, `@mc_overrides`,
///   `@mc_daycount` arms directly and implement `HasExpiry` manually.
#[macro_export]
macro_rules! impl_equity_exotic_traits {
    ($ty:ty, curve_deps: true) => {
        impl $crate::instruments::common_impl::traits::CurveDependencies for $ty {
            fn curve_dependencies(
                &self,
            ) -> finstack_core::Result<$crate::instruments::common_impl::traits::InstrumentCurves>
            {
                $crate::instruments::common_impl::traits::InstrumentCurves::builder()
                    .discount(self.discount_curve_id.clone())
                    .build()
            }
        }

        $crate::impl_equity_exotic_traits!(@inner $ty);
    };

    ($ty:ty) => {
        $crate::impl_equity_exotic_traits!(@inner $ty);
    };

    (@inner $ty:ty) => {
        $crate::impl_equity_exotic_traits!(@equity $ty);
        $crate::impl_equity_exotic_traits!(@mc_overrides $ty);
        $crate::impl_equity_exotic_traits!(@mc_daycount $ty);


        impl $crate::metrics::HasExpiry for $ty {
            fn expiry(&self) -> finstack_core::dates::Date {
                self.expiry
            }
        }
    };

    (@equity $ty:ty) => {
        impl $crate::instruments::common_impl::traits::EquityDependencies for $ty {
            fn equity_dependencies(
                &self,
            ) -> finstack_core::Result<
                $crate::instruments::common_impl::traits::EquityInstrumentDeps,
            > {
                $crate::instruments::common_impl::traits::EquityInstrumentDeps::builder()
                    .spot(self.spot_id.as_str())
                    .vol_surface(self.vol_surface_id.as_str())
                    .build()
            }
        }
    };

    (@mc_overrides $ty:ty) => {

        impl $crate::metrics::HasPricingOverrides for $ty {
            fn pricing_overrides_mut(
                &mut self,
            ) -> &mut $crate::instruments::PricingOverrides {
                &mut self.pricing_overrides
            }
        }
    };

    (@mc_daycount $ty:ty) => {

        impl $crate::metrics::HasDayCount for $ty {
            fn day_count(&self) -> finstack_core::dates::DayCount {
                self.day_count
            }
        }
    };
}
