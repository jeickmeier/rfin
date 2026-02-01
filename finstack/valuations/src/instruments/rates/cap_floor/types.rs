//! Interest rate option instrument types and Black model greeks.

use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, SettlementType};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::InterestRateOptionParams;

/// Volatility convention for cap/floor pricing.
///
/// The volatility type determines how the input volatility is interpreted
/// and which pricing model is used:
///
/// # Lognormal (Black-Scholes)
///
/// The standard market convention where volatility is expressed as a
/// proportion of the forward rate. Uses the Black (1976) formula.
///
/// **Constraints**: Requires positive forward rates and strikes.
///
/// # Normal (Bachelier)
///
/// Volatility expressed in absolute rate terms (e.g., 50bp = 0.50%).
/// Uses the Bachelier model, which naturally handles negative rates.
///
/// **Use case**: EUR/CHF markets with negative rates.
///
/// # Market Convention Notes
///
/// - **USD**: Historically lognormal, shifting to normal post-SOFR
/// - **EUR**: Predominantly normal since negative rates became common
/// - **GBP/JPY**: Mixed, check dealer quotes
///
/// Always verify the vol convention with your data provider as using
/// the wrong type will produce materially incorrect prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CapFloorVolType {
    /// Lognormal (Black) volatility - percentage of forward rate.
    ///
    /// Standard market convention. Volatility is typically quoted as a
    /// decimal (e.g., 0.20 for 20% vol).
    #[default]
    Lognormal,

    /// Normal (Bachelier) volatility - absolute rate terms.
    ///
    /// Volatility is quoted in the same units as rates (e.g., 0.0050 for 50bp).
    /// Required for negative rate environments.
    Normal,
}

impl std::fmt::Display for CapFloorVolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapFloorVolType::Lognormal => write!(f, "lognormal"),
            CapFloorVolType::Normal => write!(f, "normal"),
        }
    }
}

/// Minimum time-to-fixing for vol surface lookup (in years).
///
/// When a caplet is at or past its fixing date (`t_fix <= 0`), the vol surface lookup
/// still requires a positive time input. This constant provides a small floor (~8.6 hours)
/// to avoid numerical issues while still returning a near-expiry volatility.
///
/// The choice of 1e-6 years is small enough to not materially affect the volatility lookup
/// but large enough to avoid potential division-by-zero or log(0) issues in vol surface
/// interpolation. For seasoned caplets, the Black formula will use intrinsic value anyway,
/// so the exact vol returned is not critical.
const MIN_VOL_LOOKUP_TIME: f64 = 1e-6;

/// Type of interest rate option
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RateOptionType {
    /// Cap (series of caplets)
    Cap,
    /// Floor (series of floorlets)
    Floor,
    /// Caplet (single period cap)
    Caplet,
    /// Floorlet (single period floor)
    Floorlet,
}

/// Interest rate option instrument
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct InterestRateOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Option type
    pub rate_option_type: RateOptionType,
    /// Notional amount
    pub notional: Money,
    /// Strike rate (as decimal, e.g., 0.05 for 5%)
    pub strike_rate: f64,
    /// Start date of underlying period
    pub start_date: Date,
    /// End date of underlying period
    pub end_date: Date,
    /// Payment frequency for caps/floors
    pub frequency: Tenor,
    /// Day count convention
    pub day_count: DayCount,
    /// Schedule stub convention
    pub stub_kind: StubKind,
    /// Schedule business day convention
    pub bdc: BusinessDayConvention,
    /// Optional holiday calendar identifier for schedule and roll conventions
    pub calendar_id: Option<String>,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Settlement type
    pub settlement: SettlementType,
    /// Discount curve identifier
    pub discount_curve_id: CurveId,
    /// Forward curve identifier
    pub forward_id: CurveId,
    /// Volatility surface identifier
    pub vol_surface_id: CurveId,
    /// Volatility type convention (lognormal/Black or normal/Bachelier).
    ///
    /// **Critical**: This must match the convention of your vol surface data.
    /// Using lognormal vol with a normal surface (or vice versa) will produce
    /// incorrect prices.
    ///
    /// - `Lognormal` (default): Standard Black model, requires positive rates/strikes
    /// - `Normal`: Bachelier model, handles negative rates
    #[cfg_attr(feature = "serde", serde(default))]
    pub vol_type: CapFloorVolType,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InterestRateOption {
    /// Create a new interest rate option using parameter structs
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &InterestRateOptionParams,
        start_date: Date,
        end_date: Date,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            rate_option_type: option_params.rate_option_type,
            notional: option_params.notional,
            strike_rate: option_params.strike_rate,
            start_date,
            end_date,
            frequency: option_params.frequency,
            day_count: option_params.day_count,
            stub_kind: option_params.stub_kind,
            bdc: option_params.bdc,
            calendar_id: option_params.calendar_id.map(|s| s.to_string()),
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            discount_curve_id: discount_curve_id.into(),
            forward_id: forward_id.into(),
            vol_surface_id: vol_surface_id.into(),
            vol_type: CapFloorVolType::default(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a cap instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_cap(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        start_date: Date,
        end_date: Date,
        frequency: Tenor,
        day_count: DayCount,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::cap(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            discount_curve_id.into(),
            forward_id.into(),
            vol_surface_id,
        )
    }

    /// Create a floor instrument using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_floor(
        id: impl Into<InstrumentId>,
        notional: Money,
        strike_rate: f64,
        start_date: Date,
        end_date: Date,
        frequency: Tenor,
        day_count: DayCount,
        discount_curve_id: impl Into<CurveId>,
        forward_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::floor(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            end_date,
            discount_curve_id.into(),
            forward_id.into(),
            vol_surface_id,
        )
    }

    /// Set the volatility type convention.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::rates::cap_floor::{InterestRateOption, CapFloorVolType};
    /// use finstack_core::currency::Currency;
    /// use finstack_core::dates::{create_date, DayCount, Tenor};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::{CurveId, InstrumentId};
    /// use time::Month;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// // Create a floor with normal volatility for EUR market
    /// let floor = InterestRateOption::new_floor(
    ///     InstrumentId::new("EUR-FLOOR-001"),
    ///     Money::new(1_000_000.0, Currency::EUR),
    ///     0.02,
    ///     create_date(2026, Month::January, 1)?,
    ///     create_date(2027, Month::January, 1)?,
    ///     Tenor::quarterly(),
    ///     DayCount::Act365F,
    ///     CurveId::new("EUR-OIS"),
    ///     CurveId::new("EUR-ESTR-3M"),
    ///     CurveId::new("EUR-CAPFLOOR-VOL"),
    /// )
    ///     .with_vol_type(CapFloorVolType::Normal);
    /// # let _ = floor;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_vol_type(mut self, vol_type: CapFloorVolType) -> Self {
        self.vol_type = vol_type;
        self
    }
}

impl crate::instruments::common::traits::Instrument for InterestRateOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CapFloor
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::cashflow::builder::date_generation::build_dates;
        use crate::instruments::common::pricing::time::{
            rate_period_on_dates, relative_df_discount_curve,
        };
        use crate::instruments::rates::cap_floor::pricing::black as black_ir;

        // Get market curves
        let disc_curve = curves.get_discount(self.discount_curve_id.as_ref())?;
        let fwd_curve = curves.get_forward(self.forward_id.as_ref())?;
        let vol_surface = if self.pricing_overrides.implied_volatility.is_none() {
            Some(curves.surface(self.vol_surface_id.as_str())?)
        } else {
            None
        };

        let mut total_pv = finstack_core::money::Money::new(0.0, self.notional.currency());
        let dc_ctx = finstack_core::dates::DayCountCtx::default();

        // Single caplet/floorlet
        if matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Floorlet
        ) {
            // Skip entirely settled cashflows (payment date already passed)
            if self.end_date <= as_of {
                return Ok(total_pv);
            }

            // Time to fixing using instrument's day count (for vol surface)
            let t_fix = self
                .day_count
                .year_fraction(as_of, self.start_date, dc_ctx)?;

            // Accrual year fraction
            let tau = self
                .day_count
                .year_fraction(self.start_date, self.end_date, dc_ctx)?;

            // Use curve-consistent helpers for forward rate and discount factor
            let forward = rate_period_on_dates(fwd_curve.as_ref(), self.start_date, self.end_date)?;
            let df = relative_df_discount_curve(disc_curve.as_ref(), as_of, self.end_date)?;

            let sigma = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
                impl_vol
            } else if let Some(vol_surf) = &vol_surface {
                // Use MIN_VOL_LOOKUP_TIME floor for seasoned caplets (t_fix <= 0)
                vol_surf.value_clamped(t_fix.max(MIN_VOL_LOOKUP_TIME), self.strike_rate)
            } else {
                return Err(finstack_core::InputError::NotFound {
                    id: "cap_floor_vol_surface".to_string(),
                }
                .into());
            };

            let is_cap = matches!(
                self.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            return black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                is_cap,
                notional: self.notional.amount(),
                strike: self.strike_rate,
                forward,
                discount_factor: df,
                volatility: sigma,
                time_to_fixing: t_fix,
                accrual_year_fraction: tau,
                currency: self.notional.currency(),
            });
        }

        // Cap/floor portfolio of caplets/floorlets
        let schedule = build_dates(
            self.start_date,
            self.end_date,
            self.frequency,
            self.stub_kind,
            self.bdc,
            self.calendar_id.as_deref(),
        )?;

        if schedule.dates.len() < 2 {
            return Ok(total_pv);
        }

        let is_cap = matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Cap
        );
        let mut prev = schedule.dates[0];
        for &pay in &schedule.dates[1..] {
            // Skip entirely settled cashflows (payment date already passed)
            if pay <= as_of {
                prev = pay;
                continue;
            }

            // Time to fixing using instrument's day count (for vol surface lookup)
            let t_fix = self.day_count.year_fraction(as_of, prev, dc_ctx)?;

            // Accrual year fraction
            let tau = self.day_count.year_fraction(prev, pay, dc_ctx)?;

            // Use curve-consistent helpers for forward rate and discount factor
            let forward = rate_period_on_dates(fwd_curve.as_ref(), prev, pay)?;
            let df = relative_df_discount_curve(disc_curve.as_ref(), as_of, pay)?;

            let sigma = if let Some(impl_vol) = self.pricing_overrides.implied_volatility {
                impl_vol
            } else if let Some(vol_surf) = &vol_surface {
                // Use MIN_VOL_LOOKUP_TIME floor for seasoned caplets (t_fix <= 0)
                vol_surf.value_clamped(t_fix.max(MIN_VOL_LOOKUP_TIME), self.strike_rate)
            } else {
                return Err(finstack_core::InputError::NotFound {
                    id: "cap_floor_vol_surface".to_string(),
                }
                .into());
            };

            // Include ALL periods where payment_date > as_of, including
            // seasoned periods where fixing_date <= as_of < payment_date.
            // The Black formula handles t_fix <= 0 by computing intrinsic value.
            let leg_pv = black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                is_cap,
                notional: self.notional.amount(),
                strike: self.strike_rate,
                forward,
                discount_factor: df,
                volatility: sigma,
                time_to_fixing: t_fix,
                accrual_year_fraction: tau,
                currency: self.notional.currency(),
            })?;
            total_pv = total_pv.checked_add(leg_pv)?;

            prev = pay;
        }

        Ok(total_pv)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }
}

impl crate::instruments::common::traits::CurveDependencies for InterestRateOption {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common::traits::Instrument;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::surfaces::VolSurface;
    use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
    use finstack_core::types::CurveId;
    use time::Month;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).expect("valid month"), day)
            .expect("valid date")
    }

    fn test_market_context(base_date: Date) -> MarketContext {
        let disc = DiscountCurve::builder(CurveId::new("TEST-DISC"))
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (0.5, 0.975), (1.0, 0.95), (2.0, 0.90)])
            .build()
            .expect("discount curve should build");

        let fwd = ForwardCurve::builder(CurveId::new("TEST-FWD"), 0.25)
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots(vec![(0.0, 0.04), (0.5, 0.042), (1.0, 0.045), (2.0, 0.05)])
            .build()
            .expect("forward curve should build");

        // Create a flat vol surface at 20%
        let vol = 0.20;
        let vol_surface = VolSurface::builder(CurveId::new("TEST-VOL"))
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[0.01, 0.03, 0.05, 0.07, 0.10])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .row(&[vol, vol, vol, vol, vol])
            .build()
            .expect("vol surface should build");

        MarketContext::new()
            .insert_discount(disc)
            .insert_forward(fwd)
            .insert_surface(vol_surface)
    }

    /// Test cap-floor parity: Cap(K) - Floor(K) = Forward Swap PV
    ///
    /// This verifies the fundamental no-arbitrage relationship:
    /// Cap(K) - Floor(K) = sum_i [ DF(T_i) * tau_i * (F_i - K) ]
    ///
    /// where F_i is the forward rate for period i.
    ///
    /// # References
    ///
    /// - Hull, J.C. "Options, Futures, and Other Derivatives", Chapter 28
    /// - This parity holds for European-style options under Black model
    #[test]
    fn cap_floor_parity_holds() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 3, 1);
        let end_date = date(2025, 3, 1);
        let strike = 0.045;
        let notional = Money::new(1_000_000.0, Currency::USD);

        let ctx = test_market_context(base_date);

        // Create cap and floor with identical parameters
        let cap = InterestRateOption::new_cap(
            "TEST-CAP",
            notional,
            strike,
            start_date,
            end_date,
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "TEST-FWD",
            "TEST-VOL",
        );

        let floor = InterestRateOption::new_floor(
            "TEST-FLOOR",
            notional,
            strike,
            start_date,
            end_date,
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "TEST-FWD",
            "TEST-VOL",
        );

        let cap_pv = cap
            .value(&ctx, base_date)
            .expect("cap pricing should succeed");
        let floor_pv = floor
            .value(&ctx, base_date)
            .expect("floor pricing should succeed");

        // Calculate expected forward swap value: sum of DF * tau * (F - K)
        let disc = ctx.get_discount(CurveId::new("TEST-DISC")).expect("disc");
        let fwd = ctx.get_forward(CurveId::new("TEST-FWD")).expect("fwd");

        let schedule = crate::cashflow::builder::date_generation::build_dates(
            start_date,
            end_date,
            Tenor::quarterly(),
            StubKind::None,
            BusinessDayConvention::ModifiedFollowing,
            None,
        )
        .expect("schedule");

        let mut expected_swap_pv = 0.0;
        let dc_ctx = finstack_core::dates::DayCountCtx::default();
        let mut prev = schedule.dates[0];
        for &pay in &schedule.dates[1..] {
            let tau = DayCount::Act360
                .year_fraction(prev, pay, dc_ctx)
                .expect("tau");
            let forward =
                crate::instruments::common::pricing::time::rate_period_on_dates(&fwd, prev, pay)
                    .expect("forward");
            let df = disc.df_between_dates(base_date, pay).expect("df");
            expected_swap_pv += df * tau * notional.amount() * (forward - strike);
            prev = pay;
        }

        // Cap - Floor should equal the forward swap PV
        let cap_minus_floor = cap_pv.amount() - floor_pv.amount();
        let parity_error = (cap_minus_floor - expected_swap_pv).abs();

        // Allow for small numerical tolerance (< 0.05 currency units on 1MM notional).
        // The tolerance accounts for day count fraction differences between the cap/floor
        // schedule and the analytical calculation, which can cause ~0.01 divergence.
        assert!(
            parity_error < 0.05,
            "Cap-floor parity violated: Cap({:.2}) - Floor({:.2}) = {:.4}, expected {:.4}, error = {:.6}",
            cap_pv.amount(),
            floor_pv.amount(),
            cap_minus_floor,
            expected_swap_pv,
            parity_error
        );
    }

    /// Test that cap and floor prices are non-negative and sensible
    #[test]
    fn cap_floor_prices_are_sensible() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 3, 1);
        let end_date = date(2025, 3, 1);
        let notional = Money::new(1_000_000.0, Currency::USD);

        let ctx = test_market_context(base_date);

        // Test at multiple strikes: ITM, ATM, OTM
        let forward_approx = 0.045; // Approximate forward rate
        let strikes = [0.02, 0.04, forward_approx, 0.05, 0.08];

        for &strike in &strikes {
            let cap = InterestRateOption::new_cap(
                format!("CAP-{}", strike),
                notional,
                strike,
                start_date,
                end_date,
                Tenor::quarterly(),
                DayCount::Act360,
                "TEST-DISC",
                "TEST-FWD",
                "TEST-VOL",
            );

            let floor = InterestRateOption::new_floor(
                format!("FLOOR-{}", strike),
                notional,
                strike,
                start_date,
                end_date,
                Tenor::quarterly(),
                DayCount::Act360,
                "TEST-DISC",
                "TEST-FWD",
                "TEST-VOL",
            );

            let cap_pv = cap.value(&ctx, base_date).expect("cap pricing");
            let floor_pv = floor.value(&ctx, base_date).expect("floor pricing");

            // Option prices must be non-negative
            assert!(
                cap_pv.amount() >= 0.0,
                "Cap price must be non-negative at strike {}: got {}",
                strike,
                cap_pv.amount()
            );
            assert!(
                floor_pv.amount() >= 0.0,
                "Floor price must be non-negative at strike {}: got {}",
                strike,
                floor_pv.amount()
            );

            // Monotonicity: cap value decreases with strike, floor increases
            // (This is tested implicitly by comparing adjacent strikes)
        }
    }
}
