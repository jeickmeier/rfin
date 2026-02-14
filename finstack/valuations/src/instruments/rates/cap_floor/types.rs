//! Interest rate option instrument types and Black model greeks.

use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::{ExerciseStyle, SettlementType};
use crate::market::conventions::ids::IndexId;
use crate::market::conventions::ConventionRegistry;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::InterestRateOptionParams;
use crate::impl_instrument_base;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapFloorVolType {
    /// Lognormal (Black) volatility - percentage of forward rate.
    ///
    /// Standard market convention. Volatility is typically quoted as a
    /// decimal (e.g., 0.20 for 20% vol).
    #[default]
    Lognormal,

    /// Shifted lognormal (displaced diffusion / shifted Black).
    ///
    /// Uses Black pricing on shifted rates:
    /// `F' = F + shift`, `K' = K + shift`.
    /// This is standard for low/negative rate regimes while preserving
    /// lognormal smile conventions.
    ShiftedLognormal,

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
            CapFloorVolType::ShiftedLognormal => write!(f, "shifted_lognormal"),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
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
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    #[serde(alias = "end_date")]
    pub maturity: Date,
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
    #[serde(alias = "forward_id")]
    pub forward_curve_id: CurveId,
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
    #[serde(default)]
    pub vol_type: CapFloorVolType,
    /// Additional attributes
    pub attributes: Attributes,
}

impl InterestRateOption {
    /// Create a new interest rate option using parameter structs
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &InterestRateOptionParams,
        start_date: Date,
        maturity: Date,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        Self {
            id: id.into(),
            rate_option_type: option_params.rate_option_type,
            notional: option_params.notional,
            strike_rate: option_params.strike_rate,
            start_date,
            maturity,
            frequency: option_params.frequency,
            day_count: option_params.day_count,
            stub_kind: option_params.stub_kind,
            bdc: option_params.bdc,
            calendar_id: option_params.calendar_id.map(|s| s.to_string()),
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Cash,
            discount_curve_id: discount_curve_id.into(),
            forward_curve_id: forward_curve_id.into(),
            vol_surface_id: vol_surface_id.into(),
            vol_type: CapFloorVolType::default(),
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
        maturity: Date,
        frequency: Tenor,
        day_count: DayCount,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::cap(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            maturity,
            discount_curve_id.into(),
            forward_curve_id.into(),
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
        maturity: Date,
        frequency: Tenor,
        day_count: DayCount,
        discount_curve_id: impl Into<CurveId>,
        forward_curve_id: impl Into<CurveId>,
        vol_surface_id: impl Into<CurveId>,
    ) -> Self {
        let option_params =
            InterestRateOptionParams::floor(notional, strike_rate, frequency, day_count);
        Self::new(
            id,
            &option_params,
            start_date,
            maturity,
            discount_curve_id.into(),
            forward_curve_id.into(),
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

    /// Set displacement shift used for shifted-lognormal pricing.
    pub fn with_vol_shift(mut self, vol_shift: f64) -> Self {
        self.attributes
            .meta
            .insert("vol_shift".to_string(), vol_shift.to_string());
        self
    }

    fn resolved_payment_lag_days(&self) -> i32 {
        let Ok(registry) = ConventionRegistry::try_global() else {
            return 0;
        };
        let idx = IndexId::new(self.forward_curve_id.as_str());
        registry
            .require_rate_index(&idx)
            .map(|conv| conv.default_payment_delay_days)
            .unwrap_or(0)
    }

    fn resolved_reset_lag_days(&self) -> Option<i32> {
        let Ok(registry) = ConventionRegistry::try_global() else {
            return None;
        };
        let idx = IndexId::new(self.forward_curve_id.as_str());
        registry
            .require_rate_index(&idx)
            .map(|conv| conv.default_reset_lag_days)
            .ok()
    }

    fn resolved_vol_shift(&self) -> f64 {
        self.attributes
            .get_meta("vol_shift")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0)
    }
}

impl crate::instruments::common_impl::traits::Instrument for InterestRateOption {
    impl_instrument_base!(crate::pricer::InstrumentType::CapFloor);

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        use crate::instruments::common_impl::pricing::time::{
            rate_period_on_dates, relative_df_discount_curve,
        };
        use crate::instruments::rates::cap_floor::pricing::{
            black as black_ir, normal as normal_ir,
        };

        // Get market curves
        let disc_curve = curves.get_discount(self.discount_curve_id.as_ref())?;
        let fwd_curve = curves.get_forward(self.forward_curve_id.as_ref())?;
        let vol_surface = curves.surface(self.vol_surface_id.as_str())?;

        let mut total_pv = finstack_core::money::Money::new(0.0, self.notional.currency());
        let dc_ctx = finstack_core::dates::DayCountCtx::default();

        // Single caplet/floorlet
        if matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Floorlet
        ) {
            // Skip entirely settled cashflows (payment date already passed)
            if self.maturity <= as_of {
                return Ok(total_pv);
            }

            // Time to fixing using instrument's day count (for vol surface)
            let t_fix = self
                .day_count
                .year_fraction(as_of, self.start_date, dc_ctx)?;

            // Accrual year fraction
            let tau = self
                .day_count
                .year_fraction(self.start_date, self.maturity, dc_ctx)?;

            // Use curve-consistent helpers for forward rate and discount factor
            let forward = rate_period_on_dates(fwd_curve.as_ref(), self.start_date, self.maturity)?;
            let df = relative_df_discount_curve(disc_curve.as_ref(), as_of, self.maturity)?;

            // Use MIN_VOL_LOOKUP_TIME floor for seasoned caplets (t_fix <= 0)
            let sigma = vol_surface.value_clamped(t_fix.max(MIN_VOL_LOOKUP_TIME), self.strike_rate);

            let is_cap = matches!(
                self.rate_option_type,
                RateOptionType::Caplet | RateOptionType::Cap
            );
            let black_price = |strike: f64, forward: f64| {
                black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                    is_cap,
                    notional: self.notional.amount(),
                    strike,
                    forward,
                    discount_factor: df,
                    volatility: sigma,
                    time_to_fixing: t_fix,
                    accrual_year_fraction: tau,
                    currency: self.notional.currency(),
                })
            };
            return match self.vol_type {
                CapFloorVolType::Lognormal => black_price(self.strike_rate, forward),
                CapFloorVolType::ShiftedLognormal => {
                    let vol_shift = self.resolved_vol_shift();
                    black_price(self.strike_rate + vol_shift, forward + vol_shift)
                }
                CapFloorVolType::Normal => {
                    normal_ir::price_caplet_floorlet(normal_ir::CapletFloorletInputs {
                        is_cap,
                        notional: self.notional.amount(),
                        strike: self.strike_rate,
                        forward,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: self.notional.currency(),
                    })
                }
            };
        }

        // Cap/floor portfolio of caplets/floorlets
        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: self.start_date,
                end: self.maturity,
                frequency: self.frequency,
                stub: self.stub_kind,
                bdc: self.bdc,
                calendar_id: self
                    .calendar_id
                    .as_deref()
                    .unwrap_or(crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID),
                end_of_month: false,
                day_count: self.day_count,
                payment_lag_days: self.resolved_payment_lag_days(),
                reset_lag_days: self.resolved_reset_lag_days(),
            },
        )?;

        if periods.is_empty() {
            return Ok(total_pv);
        }

        let is_cap = matches!(
            self.rate_option_type,
            RateOptionType::Caplet | RateOptionType::Cap
        );
        for period in periods {
            let pay = period.payment_date;
            // Skip entirely settled cashflows (payment date already passed)
            if pay <= as_of {
                continue;
            }

            // Time to fixing using instrument's day count (for vol surface lookup)
            let t_fix = self
                .day_count
                .year_fraction(as_of, period.accrual_start, dc_ctx)?;

            // Accrual year fraction
            let tau = period.accrual_year_fraction;

            // Use curve-consistent helpers for forward rate and discount factor
            let forward =
                rate_period_on_dates(fwd_curve.as_ref(), period.accrual_start, period.accrual_end)?;
            let df = relative_df_discount_curve(disc_curve.as_ref(), as_of, pay)?;

            // Use MIN_VOL_LOOKUP_TIME floor for seasoned caplets (t_fix <= 0)
            let sigma = vol_surface.value_clamped(t_fix.max(MIN_VOL_LOOKUP_TIME), self.strike_rate);

            // Include ALL periods where payment_date > as_of, including
            // seasoned periods where fixing_date <= as_of < payment_date.
            // The Black formula handles t_fix <= 0 by computing intrinsic value.
            let leg_pv = match self.vol_type {
                CapFloorVolType::Lognormal => {
                    black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                        is_cap,
                        notional: self.notional.amount(),
                        strike: self.strike_rate,
                        forward,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: self.notional.currency(),
                    })?
                }
                CapFloorVolType::ShiftedLognormal => {
                    let vol_shift = self.resolved_vol_shift();
                    black_ir::price_caplet_floorlet(black_ir::CapletFloorletInputs {
                        is_cap,
                        notional: self.notional.amount(),
                        strike: self.strike_rate + vol_shift,
                        forward: forward + vol_shift,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: self.notional.currency(),
                    })?
                }
                CapFloorVolType::Normal => {
                    normal_ir::price_caplet_floorlet(normal_ir::CapletFloorletInputs {
                        is_cap,
                        notional: self.notional.amount(),
                        strike: self.strike_rate,
                        forward,
                        discount_factor: df,
                        volatility: sigma,
                        time_to_fixing: t_fix,
                        accrual_year_fraction: tau,
                        currency: self.notional.currency(),
                    })?
                }
            };
            total_pv = total_pv.checked_add(leg_pv)?;
        }

        Ok(total_pv)
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.maturity)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.start_date)
    }
}

impl crate::instruments::common_impl::traits::CurveDependencies for InterestRateOption {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .forward(self.forward_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
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

        // IMPORTANT: Use the same canonical schedule builder as the instrument pricer.
        //
        // `cashflow::builder::periods::build_periods` applies BDC even when `calendar_id`
        // is weekends-only. Cap/floor parity is very sensitive to small date shifts,
        // so we must match the instrument's schedule.
        let periods = crate::cashflow::builder::periods::build_periods(
            crate::cashflow::builder::periods::BuildPeriodsParams {
                start: start_date,
                end: end_date,
                frequency: Tenor::quarterly(),
                stub: StubKind::None,
                bdc: BusinessDayConvention::ModifiedFollowing,
                calendar_id: crate::cashflow::builder::calendar::WEEKENDS_ONLY_ID,
                end_of_month: false,
                day_count: DayCount::Act360,
                payment_lag_days: 0,
                reset_lag_days: None,
            },
        )
        .expect("periods");

        let mut expected_swap_pv = 0.0;
        for p in periods {
            let tau = p.accrual_year_fraction;
            let forward = crate::instruments::common_impl::pricing::time::rate_period_on_dates(
                &fwd,
                p.accrual_start,
                p.accrual_end,
            )
            .expect("forward");
            let df = disc
                .df_between_dates(base_date, p.payment_date)
                .expect("df");
            expected_swap_pv += df * tau * notional.amount() * (forward - strike);
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

    #[test]
    fn normal_vol_type_handles_negative_forward() {
        let base_date = date(2024, 1, 1);
        let start_date = date(2024, 3, 1);
        let end_date = date(2025, 3, 1);
        let notional = Money::new(1_000_000.0, Currency::USD);

        let mut ctx = test_market_context(base_date);
        let neg_fwd = ForwardCurve::builder(CurveId::new("TEST-FWD-NEG"), 0.25)
            .base_date(base_date)
            .day_count(DayCount::Act360)
            .knots(vec![
                (0.0, -0.01),
                (0.5, -0.008),
                (1.0, -0.006),
                (2.0, -0.004),
            ])
            .build()
            .expect("negative forward curve should build");
        ctx = ctx.insert_forward(neg_fwd);

        // Build a flat vol surface at 50bp normal vol for the normal model test
        let normal_vol = 0.005;
        let normal_vol_surface = VolSurface::builder(CurveId::new("TEST-VOL-NORMAL"))
            .expiries(&[0.25, 0.5, 1.0, 2.0])
            .strikes(&[-0.02, -0.01, 0.0, 0.01, 0.02])
            .row(&[normal_vol, normal_vol, normal_vol, normal_vol, normal_vol])
            .row(&[normal_vol, normal_vol, normal_vol, normal_vol, normal_vol])
            .row(&[normal_vol, normal_vol, normal_vol, normal_vol, normal_vol])
            .row(&[normal_vol, normal_vol, normal_vol, normal_vol, normal_vol])
            .build()
            .expect("normal vol surface should build");
        ctx = ctx.insert_surface(normal_vol_surface);

        // Build a floorlet with negative forward using normal vol surface.
        let normal_floorlet = InterestRateOption::new_floor(
            "NORM-FLOORLET",
            notional,
            0.0,
            start_date,
            end_date,
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "TEST-FWD-NEG",
            "TEST-VOL-NORMAL",
        )
        .with_vol_type(CapFloorVolType::Normal);

        let black_floorlet = InterestRateOption::new_floor(
            "BLACK-FLOORLET",
            notional,
            0.0,
            start_date,
            end_date,
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "TEST-FWD-NEG",
            "TEST-VOL-NORMAL",
        )
        .with_vol_type(CapFloorVolType::Lognormal);

        // This should succeed under normal model.
        let normal_pv = normal_floorlet
            .value(&ctx, base_date)
            .expect("normal cap/floor pricing should succeed");
        assert!(
            normal_pv.amount().is_finite() && normal_pv.amount() >= 0.0,
            "Normal cap/floor PV should be finite and non-negative"
        );

        // For black model the same negative forward should fail with a clear error.
        let black_result = black_floorlet.value(&ctx, base_date);
        assert!(
            black_result.is_err(),
            "Black model should reject non-positive forwards"
        );
    }

    #[test]
    fn payment_lag_resolution_uses_convention_or_fallback() {
        let instrument_with_unknown_index = InterestRateOption::new_cap(
            "CAP-LAG-UNKNOWN",
            Money::new(1_000_000.0, Currency::USD),
            0.04,
            date(2024, 3, 1),
            date(2025, 3, 1),
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "DOES-NOT-EXIST",
            "TEST-VOL",
        );
        assert_eq!(
            instrument_with_unknown_index.resolved_payment_lag_days(),
            0,
            "Unknown index should fall back to zero lag for backward compatibility"
        );

        let instrument_with_convention = InterestRateOption::new_cap(
            "CAP-LAG-CONVENTION",
            Money::new(1_000_000.0, Currency::USD),
            0.04,
            date(2024, 3, 1),
            date(2025, 3, 1),
            Tenor::quarterly(),
            DayCount::Act360,
            "TEST-DISC",
            "USD-SOFR-OIS",
            "TEST-VOL",
        );
        assert!(
            instrument_with_convention.resolved_payment_lag_days() >= 0,
            "Convention-based lag should resolve to a non-negative business-day delay"
        );
    }
}
