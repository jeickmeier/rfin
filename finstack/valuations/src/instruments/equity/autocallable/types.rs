//! Autocallable structured product instrument definition.
//!
//! # Barrier Monitoring Convention
//!
//! Autocallable barriers are monitored **discretely** at the specified observation dates.
//! This implementation does NOT apply continuous monitoring or the Broadie-Glasserman-Kou
//! adjustment for discrete monitoring of continuous barriers.
//!
//! ## Why No Adjustment
//!
//! The Broadie-Glasserman-Kou adjustment (see reference below) is designed to correct for
//! discretely sampling a barrier that is contractually monitored continuously:
//! ```text
//! H_adj = H × exp(±0.5826 × σ × √Δt)
//! ```
//!
//! However, for autocallables, barriers are typically **contractually discrete** - they
//! are only checked on specific observation dates as defined in the term sheet. Therefore:
//! - The `observation_dates` field specifies the exact barrier monitoring dates
//! - Monte Carlo paths are evaluated exactly at these dates (time grid includes them)
//! - No adjustment is needed because there is no approximation of continuous monitoring
//!
//! ## For Continuously Monitored Barriers
//!
//! If you need to price a product with continuous barrier monitoring (e.g., daily close
//! knock-in/knock-out), you should either:
//! 1. Apply the BGK adjustment externally to the barrier levels
//! 2. Use a finer time grid with many intraday steps
//!
//! # References
//!
//! - Broadie, M., Glasserman, P., & Kou, S. (1997). "A Continuity Correction for
//!   Discrete Barrier Options." *Mathematical Finance*, 7(4), 325-349.
//! - Haug, E. G. (2007). *The Complete Guide to Option Pricing Formulas*, Section 4.17.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use time::macros::date;

/// Final payoff type for autocallable products.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FinalPayoffType {
    /// Capital protection: max(floor, participation * min(S_T/S_0, cap))
    CapitalProtection {
        /// Minimum return floor (e.g., 1.0 for 100% protection)
        floor: f64,
    },
    /// Participation: 1 + participation_rate * max(0, S_T/S_0 - 1)
    Participation {
        /// Participation rate in upside (e.g., 1.0 for 100% participation)
        rate: f64,
    },
    /// Knock-in put: Put option if barrier breached, otherwise return principal
    KnockInPut {
        /// Strike price for knock-in put option
        strike: f64,
    },
}

/// Autocallable structured product instrument.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct Autocallable {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Underlying asset ticker symbol
    pub underlying_ticker: crate::instruments::equity::spot::Ticker,
    /// Observation dates for autocall and coupon checks.
    ///
    /// Barriers are monitored **discretely** at these exact dates only.
    /// The Monte Carlo time grid is constructed to include these dates precisely.
    pub observation_dates: Vec<Date>,
    /// Explicit terminal expiry date for the structure.
    pub expiry: Date,
    /// Autocall barrier levels (as ratios of initial spot, e.g., 1.0 = 100%).
    ///
    /// Each barrier corresponds to the observation date at the same index.
    /// If spot ≥ barrier × initial_spot on the observation date, the product autocalls.
    pub autocall_barriers: Vec<f64>,
    /// Coupon amounts paid if observation barrier is met
    pub coupons: Vec<f64>,
    /// Final barrier level for final payoff determination
    pub final_barrier: f64,
    /// Type of final payoff (capital protection, participation, knock-in put)
    pub final_payoff_type: FinalPayoffType,
    /// Participation rate in underlying performance
    pub participation_rate: f64,
    /// Cap level for final payoff (maximum return)
    pub cap_level: f64,
    /// Notional amount
    pub notional: Money,
    /// Day count convention for interest calculations
    pub day_count: finstack_core::dates::DayCount,
    /// Discount curve ID for present value calculations
    pub discount_curve_id: CurveId,
    /// Spot price identifier for underlying asset
    pub spot_id: String,
    /// Volatility surface ID for option pricing
    pub vol_surface_id: CurveId,
    /// Optional dividend yield curve ID
    pub div_yield_id: Option<CurveId>,
    /// Pricing overrides (manual price, yield, spread)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for scenario selection and grouping
    pub attributes: Attributes,
}

impl Autocallable {
    /// Create a canonical example autocallable (quarterly observations, simple barriers/coupons).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use finstack_core::dates::DayCount;
        let observation_dates = vec![
            date!(2024 - 03 - 29),
            date!(2024 - 06 - 28),
            date!(2024 - 09 - 30),
            date!(2024 - 12 - 31),
        ];
        let autocall_barriers = vec![1.0, 1.0, 1.0, 1.0]; // 100% of initial
        let coupons = vec![0.02, 0.02, 0.02, 0.02]; // 2% per observation if called
        Autocallable::builder()
            .id(InstrumentId::new("AUTO-SPX-QTR"))
            .underlying_ticker("SPX".to_string())
            .observation_dates(observation_dates)
            .expiry(date!(2024 - 12 - 31))
            .autocall_barriers(autocall_barriers)
            .coupons(coupons)
            .final_barrier(0.6) // 60% final KI barrier
            .final_payoff_type(FinalPayoffType::Participation { rate: 1.0 })
            .participation_rate(1.0)
            .cap_level(1.5) // 150% cap
            .notional(Money::new(100_000.0, Currency::USD))
            .day_count(DayCount::Act365F)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .spot_id("SPX-SPOT".to_string())
            .vol_surface_id(CurveId::new("SPX-VOL"))
            .div_yield_id_opt(Some(CurveId::new("SPX-DIV")))
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .unwrap_or_else(|_| {
                unreachable!("Example Autocallable with valid constants should never fail")
            })
    }
}

impl crate::instruments::common_impl::traits::Instrument for Autocallable {
    impl_instrument_base!(crate::pricer::InstrumentType::Autocallable);

    fn market_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::dependencies::MarketDependencies>
    {
        crate::instruments::common_impl::dependencies::MarketDependencies::from_curves_and_equity(
            self,
        )
    }

    fn value(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        #[cfg(feature = "mc")]
        {
            use crate::instruments::equity::autocallable::pricer;
            pricer::compute_pv(self, market, as_of)
        }
        #[cfg(not(feature = "mc"))]
        {
            let _ = (market, as_of);
            Err(finstack_core::Error::Validation(
                "MC feature required for Autocallable pricing".to_string(),
            ))
        }
    }

    fn effective_start_date(&self) -> Option<Date> {
        self.observation_dates.first().copied()
    }

    fn expiry(&self) -> Option<Date> {
        Some(self.expiry)
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}
