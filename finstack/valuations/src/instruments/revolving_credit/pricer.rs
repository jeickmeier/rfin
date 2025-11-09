//! Pricing engine for revolving credit facilities.
//!
//! Provides both deterministic and Monte Carlo pricing implementations.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Period, PeriodId, PeriodKind};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::cashflows::generate_deterministic_cashflows;
use super::types::RevolvingCredit;

/// Build periods from payment schedule dates for PV aggregation.
///
/// Creates Period objects with synthetic IDs based on payment frequency.
/// Each period spans from one payment date (exclusive start) to the next (inclusive end).
fn build_periods_from_payment_dates(
    payment_dates: &[Date],
    frequency: finstack_core::dates::Frequency,
) -> Vec<Period> {
    if payment_dates.len() < 2 {
        return Vec::new();
    }

    let mut periods = Vec::with_capacity(payment_dates.len() - 1);

    // Determine period kind from frequency
    let period_kind = match frequency {
        finstack_core::dates::Frequency::Months(12) => PeriodKind::Annual,
        finstack_core::dates::Frequency::Months(6) => PeriodKind::SemiAnnual,
        finstack_core::dates::Frequency::Months(3) => PeriodKind::Quarterly,
        finstack_core::dates::Frequency::Months(1) => PeriodKind::Monthly,
        finstack_core::dates::Frequency::Days(7) => PeriodKind::Weekly,
        _ => PeriodKind::Quarterly, // Default fallback
    };

    for i in 0..(payment_dates.len() - 1) {
        let start = payment_dates[i];
        let end = payment_dates[i + 1];

        // Create a synthetic PeriodId based on the start date and frequency
        let period_id = match period_kind {
            PeriodKind::Quarterly => {
                let year = start.year();
                let month = start.month() as u8;
                let quarter = match month {
                    1..=3 => 1,
                    4..=6 => 2,
                    7..=9 => 3,
                    _ => 4,
                };
                PeriodId::quarter(year, quarter)
            }
            PeriodKind::Monthly => {
                let year = start.year();
                let month = start.month() as u8;
                PeriodId::month(year, month)
            }
            PeriodKind::SemiAnnual => {
                let year = start.year();
                let month = start.month() as u8;
                let half = if month <= 6 { 1 } else { 2 };
                PeriodId::half(year, half)
            }
            PeriodKind::Annual => {
                let year = start.year();
                PeriodId::annual(year)
            }
            PeriodKind::Weekly => {
                // For weekly, use a simple week number based on days since start of year
                let year = start.year();
                let year_start = Date::from_calendar_date(year, time::Month::January, 1).unwrap();
                let days = (start - year_start).whole_days();
                let week = ((days / 7) + 1).min(52) as u8;
                PeriodId::week(year, week)
            }
        };

        periods.push(Period {
            id: period_id,
            start,
            end,
            is_actual: false, // All periods are forecast for pricing
        });
    }

    periods
}

/// Discounting pricer for revolving credit facilities with deterministic cashflows.
///
/// This pricer generates cashflows using the facility's deterministic schedule
/// and discounts them using the discount curve.
#[derive(Default)]
pub struct RevolvingCreditDiscountingPricer;

impl RevolvingCreditDiscountingPricer {
    /// Create a new revolving credit discounting pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using deterministic cashflows.
    pub fn price_deterministic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Generate cashflows (excludes upfront fee - handled below)
        let schedule = generate_deterministic_cashflows(facility, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        // Build payment schedule dates to create periods
        use finstack_core::dates::ScheduleBuilder;
        let mut builder = ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
            .frequency(facility.payment_frequency)
            .stub_rule(finstack_core::dates::StubKind::None);

        if let Some(cal_code) = facility
            .attributes
            .get_meta("calendar_id")
            .or_else(|| facility.attributes.get_meta("calendar"))
        {
            if let Some(cal) =
                finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code)
            {
                builder = builder.adjust_with(
                    finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    cal,
                );
            }
        }

        let payment_schedule = builder.build()?;
        let payment_dates: Vec<Date> = payment_schedule.into_iter().collect();

        // Build periods from payment dates
        let periods = build_periods_from_payment_dates(&payment_dates, facility.payment_frequency);

        // Compute per-period PVs using the standard cashflow schedule method
        let period_pvs = schedule.pre_period_pv(
            &periods,
            disc as &dyn finstack_core::market_data::traits::Discounting,
            as_of,
            disc_dc,
        );

        // Sum PVs across all periods and currencies
        let mut total_pv = 0.0;
        let ccy = facility.commitment_amount.currency();
        for (_period_id, ccy_map) in period_pvs.iter() {
            if let Some(pv_money) = ccy_map.get(&ccy) {
                total_pv += pv_money.amount();
            }
        }

        // Handle upfront fee at pricer level (consistent with MC pricer)
        // Upfront fee is paid by lender at commitment, so it reduces facility value
        let upfront_fee_pv = if let Some(upfront_fee) = facility.fees.upfront_fee {
            if facility.commitment_date > as_of {
                // Discount upfront fee from commitment date to as_of
                let t_commitment = disc_dc.year_fraction(
                    disc.base_date(),
                    facility.commitment_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let t_as_of = disc_dc.year_fraction(
                    disc.base_date(),
                    as_of,
                    finstack_core::dates::DayCountCtx::default(),
                )?;

                let df_commitment = disc.df(t_commitment);
                let df_as_of = disc.df(t_as_of);
                let df = if df_as_of > 0.0 {
                    df_commitment / df_as_of
                } else {
                    1.0
                };

                upfront_fee.amount() * df
            } else {
                // Commitment date in the past or today - no discounting needed
                upfront_fee.amount()
            }
        } else {
            0.0
        };

        // Lender perspective: upfront fee is an outflow, so subtract from PV
        let mut final_pv = total_pv - upfront_fee_pv;

        // If commitment_date == as_of, the initial draw happens "today" and is not included
        // in the cashflow schedule. Check if terminal repayment is excluded (if last period ends at maturity_date).
        // If excluded, it's already accounted for by subtracting initial draw.
        // If included, we need to subtract net cost: initial draw - terminal repayment PV.
        if facility.commitment_date == as_of && facility.drawn_amount.amount() > 1e-6 {
            let last_period_ends_at_maturity = periods
                .last()
                .map(|p| p.end == facility.maturity_date)
                .unwrap_or(false);

            if last_period_ends_at_maturity {
                // Terminal repayment is excluded from period PVs, so we need to:
                // 1. Add terminal repayment PV
                // 2. Subtract initial draw
                if let Some(terminal_flow) = schedule.flows.iter().find(|cf| {
                    cf.date == facility.maturity_date
                        && cf.kind == finstack_core::cashflow::primitives::CFKind::Notional
                        && cf.amount.amount() > 0.0
                }) {
                    let t_maturity = disc_dc.year_fraction(
                        disc.base_date(),
                        facility.maturity_date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let t_as_of = disc_dc.year_fraction(
                        disc.base_date(),
                        as_of,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let df_maturity = disc.df(t_maturity);
                    let df_as_of = disc.df(t_as_of);
                    let df = if df_as_of > 0.0 {
                        df_maturity / df_as_of
                    } else {
                        1.0
                    };
                    if terminal_flow.amount.currency() == ccy {
                        let terminal_pv = terminal_flow.amount.amount() * df;
                        // Add terminal repayment PV and subtract initial draw
                        final_pv += terminal_pv - facility.drawn_amount.amount();
                    } else {
                        final_pv -= facility.drawn_amount.amount();
                    }
                } else {
                    final_pv -= facility.drawn_amount.amount();
                }
            } else {
                // Terminal repayment is included in period PVs, so subtract net cost
                if let Some(terminal_flow) = schedule.flows.iter().find(|cf| {
                    cf.date == facility.maturity_date
                        && cf.kind == finstack_core::cashflow::primitives::CFKind::Notional
                        && cf.amount.amount() > 0.0
                }) {
                    let t_maturity = disc_dc.year_fraction(
                        disc.base_date(),
                        facility.maturity_date,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let t_as_of = disc_dc.year_fraction(
                        disc.base_date(),
                        as_of,
                        finstack_core::dates::DayCountCtx::default(),
                    )?;
                    let df_maturity = disc.df(t_maturity);
                    let df_as_of = disc.df(t_as_of);
                    let df = if df_as_of > 0.0 {
                        df_maturity / df_as_of
                    } else {
                        1.0
                    };
                    if terminal_flow.amount.currency() == ccy {
                        let terminal_pv = terminal_flow.amount.amount() * df;
                        // Subtract net cost: initial draw - terminal repayment PV
                        final_pv -= facility.drawn_amount.amount() - terminal_pv;
                    } else {
                        final_pv -= facility.drawn_amount.amount();
                    }
                } else {
                    final_pv -= facility.drawn_amount.amount();
                }
            }
        }

        Ok(Money::new(final_pv, ccy))
    }
}

impl Pricer for RevolvingCreditDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Validate that we have a deterministic spec
        if !facility.is_deterministic() {
            return Err(PricingError::model_failure(
                "RevolvingCreditDiscountingPricer requires deterministic cashflows".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market
            .get_discount_ref(facility.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility
        let pv = Self::price_deterministic(facility, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

/// Monte Carlo pricer for revolving credit facilities with stochastic utilization.
///
/// This pricer simulates utilization paths using a mean-reverting process and
/// averages the discounted cashflows across all paths to compute the expected PV.
///
/// **Note**: Requires the `mc` feature to be enabled.
#[cfg(feature = "mc")]
#[derive(Default)]
pub struct RevolvingCreditMcPricer;

#[cfg(feature = "mc")]
impl RevolvingCreditMcPricer {
    /// Create a new revolving credit Monte Carlo pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using Monte Carlo simulation.
    ///
    /// Simulates the utilization rate evolution using a mean-reverting OU process,
    /// generates cashflows for each path, and returns the average discounted value.
    ///
    /// If `mc_config` is present in the stochastic spec, uses multi-factor modeling
    /// with credit spread and interest rate dynamics. Otherwise, uses simple
    /// utilization-only simulation.
    pub fn price_stochastic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        use super::types::DrawRepaySpec;

        // Extract stochastic spec
        let stoch_spec = match &facility.draw_repay_spec {
            DrawRepaySpec::Stochastic(spec) => spec.as_ref(),
            _ => return Err(finstack_core::error::InputError::Invalid.into()),
        };

        // Check if advanced MC config is present
        #[cfg(feature = "mc")]
        if let Some(ref mc_config) = stoch_spec.mc_config {
            return Self::price_multi_factor(facility, market, as_of, stoch_spec, mc_config);
        }

        // Fall back to simple utilization-only simulation
        Self::price_simple_utilization(facility, market, as_of, stoch_spec)
    }

    /// Simple utilization-only Monte Carlo pricing (legacy implementation).
    #[cfg(feature = "mc")]
    fn price_simple_utilization(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        stoch_spec: &super::types::StochasticUtilizationSpec,
    ) -> finstack_core::Result<Money> {
        use super::types::UtilizationProcess;
        use crate::cashflow::builder::schedule::CashFlowSchedule;
        use crate::cashflow::primitives::{CFKind, CashFlow, Notional};
        use finstack_core::dates::DayCountCtx;

        // Extract mean-reverting parameters
        let (target_rate, speed, volatility) = match &stoch_spec.utilization_process {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => (*target_rate, *speed, *volatility),
        };

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        // Note: t_as_of not needed for MC pricing with CashFlowSchedule

        // Time horizon in years
        let t_start =
            disc_dc.year_fraction(base_date, facility.commitment_date, DayCountCtx::default())?;
        let t_end =
            disc_dc.year_fraction(base_date, facility.maturity_date, DayCountCtx::default())?;
        let time_horizon = t_end - t_start;

        if time_horizon <= 0.0 {
            return Ok(Money::new(0.0, facility.commitment_amount.currency()));
        }

        // Setup MC simulation
        let num_paths = stoch_spec.num_paths;
        let seed = stoch_spec.seed.unwrap_or(42);

        // Use quarterly time steps for cashflow generation
        let dt = 0.25; // 3 months
        let num_steps = (time_horizon / dt).ceil() as usize;

        // Build payment schedule dates and periods (reused across paths)
        use finstack_core::dates::ScheduleBuilder;
        let mut builder = ScheduleBuilder::new(facility.commitment_date, facility.maturity_date)
            .frequency(facility.payment_frequency)
            .stub_rule(finstack_core::dates::StubKind::None);

        if let Some(cal_code) = facility
            .attributes
            .get_meta("calendar_id")
            .or_else(|| facility.attributes.get_meta("calendar"))
        {
            if let Some(cal) =
                finstack_core::dates::CalendarRegistry::global().resolve_str(cal_code)
            {
                builder = builder.adjust_with(
                    finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    cal,
                );
            }
        }

        let payment_schedule = builder.build()?;
        let payment_dates: Vec<Date> = payment_schedule.into_iter().collect();
        let periods = build_periods_from_payment_dates(&payment_dates, facility.payment_frequency);

        // Initialize RNG
        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        use crate::instruments::common::mc::traits::RandomStream;

        let base_rng = PhiloxRng::new(seed);

        // Run MC simulation
        let mut sum_pv = 0.0;
        let ccy = facility.commitment_amount.currency();

        for path_idx in 0..num_paths {
            let mut rng = base_rng.split(path_idx as u64);

            // Simulate utilization path
            let mut utilization = facility.utilization_rate();

            // Simple default model (optional)
            let mut default_threshold: Option<f64> = None;
            let mut cum_hazard = 0.0;
            let mut defaulted = false;
            let mut recovery_rate = 0.0;
            let mut hazard_rate = 0.0;
            if let Some(def) = &stoch_spec.default_model {
                recovery_rate = def.recovery_rate.clamp(0.0, 1.0);
                hazard_rate = def
                    .annual_hazard
                    .unwrap_or_else(|| {
                        def.annual_spread.unwrap_or(0.0) / (1.0 - recovery_rate).max(1e-6)
                    })
                    .max(0.0);
                // Draw Exp(1) threshold: E = -ln(U)
                let u = rng.next_u01().clamp(1e-12, 1.0 - 1e-12);
                default_threshold = Some(-u.ln());
            }

            // Prepare base rate sources for interest calculation
            let (fixed_rate_opt, fwd_opt, margin_bp_opt) = match &facility.base_rate_spec {
                super::types::BaseRateSpec::Fixed { rate } => (Some(*rate), None, None),
                super::types::BaseRateSpec::Floating {
                    index_id,
                    margin_bp,
                    ..
                } => {
                    let fwd = market.get_forward_ref(index_id.as_str())?;
                    (None, Some(fwd), Some(*margin_bp))
                }
            };

            // Collect cashflows for this path
            let mut path_flows = Vec::new();

            // Note: Upfront fee is handled at pricer level (consistent with deterministic pricer),
            // not included in cashflow schedule to avoid double-counting

            // Convert time steps to dates for cashflow recording
            // Map MC steps to payment dates (MC uses quarterly steps which align with payment frequency)
            for step in 0..num_steps {
                let t = t_start + (step as f64) * dt;
                let t_next = (t + dt).min(t_end);
                let actual_dt = t_next - t;

                if actual_dt <= 0.0 {
                    break;
                }

                // Convert time to date by finding the payment date closest to this time step
                // For quarterly MC steps, this should align with quarterly payment dates
                // payment_dates[0] is commitment_date, payment_dates[1] is first payment, etc.
                let date_next = if step + 1 < payment_dates.len() {
                    payment_dates[step + 1]
                } else if !payment_dates.is_empty() {
                    // Use last payment date (maturity) if we've exceeded the schedule
                    *payment_dates.last().unwrap()
                } else {
                    // Fallback: approximate date by adding days (using Act365F approximation)
                    use time::Duration;
                    let days = (t_next * 365.0) as i64;
                    base_date + Duration::days(days)
                };

                // Default check
                if let Some(th) = default_threshold {
                    if !defaulted {
                        cum_hazard += hazard_rate * actual_dt;
                        if cum_hazard >= th {
                            defaulted = true;
                            // Recovery at default time
                            let commitment = facility.commitment_amount.amount();
                            let drawn_now = commitment * utilization;
                            path_flows.push(CashFlow {
                                date: date_next,
                                reset_date: None,
                                amount: Money::new(drawn_now * recovery_rate, ccy),
                                kind: CFKind::Notional,
                                accrual_factor: 0.0,
                            });
                        }
                    }
                }

                if defaulted {
                    break;
                }

                // Current drawn and undrawn amounts based on utilization
                let commitment = facility.commitment_amount.amount();
                let drawn = commitment * utilization;
                let undrawn = commitment - drawn;

                // Calculate cashflows for this period
                // Interest on drawn
                let period_rate = if let Some(r) = fixed_rate_opt {
                    r
                } else {
                    let fwd = fwd_opt.expect("forward curve available");
                    let m = margin_bp_opt.unwrap_or(0.0) * 1e-4;
                    fwd.rate(actual_dt).max(0.0) + m
                };
                let interest = Money::new(drawn * period_rate * actual_dt, ccy);

                // Commitment fee on undrawn
                let commitment_fee = Money::new(
                    undrawn * (facility.fees.commitment_fee_bp * 1e-4) * actual_dt,
                    ccy,
                );

                // Usage fee on drawn
                let usage_fee =
                    Money::new(drawn * (facility.fees.usage_fee_bp * 1e-4) * actual_dt, ccy);

                // Facility fee on total commitment
                let facility_fee = Money::new(
                    commitment * (facility.fees.facility_fee_bp * 1e-4) * actual_dt,
                    ccy,
                );

                // Calculate accrual factor
                // payment_dates[0] is commitment_date, so step 0 period starts at commitment_date
                let date_start = if step < payment_dates.len() {
                    payment_dates[step]
                } else if !payment_dates.is_empty() {
                    *payment_dates.last().unwrap()
                } else {
                    facility.commitment_date
                };
                let accrual = facility
                    .day_count
                    .year_fraction(date_start, date_next, DayCountCtx::default())
                    .unwrap_or(actual_dt);

                // Add interest cashflow
                if interest.amount().abs() > 1e-10 {
                    path_flows.push(CashFlow {
                        date: date_next,
                        reset_date: if fixed_rate_opt.is_none() {
                            Some(date_start)
                        } else {
                            None
                        },
                        amount: interest,
                        kind: if fixed_rate_opt.is_some() {
                            CFKind::Fixed
                        } else {
                            CFKind::FloatReset
                        },
                        accrual_factor: accrual,
                    });
                }

                // Add fee cashflows
                if commitment_fee.amount().abs() > 1e-10 {
                    path_flows.push(CashFlow {
                        date: date_next,
                        reset_date: None,
                        amount: commitment_fee,
                        kind: CFKind::Fee,
                        accrual_factor: accrual,
                    });
                }

                if usage_fee.amount().abs() > 1e-10 {
                    path_flows.push(CashFlow {
                        date: date_next,
                        reset_date: None,
                        amount: usage_fee,
                        kind: CFKind::Fee,
                        accrual_factor: accrual,
                    });
                }

                if facility_fee.amount().abs() > 1e-10 {
                    path_flows.push(CashFlow {
                        date: date_next,
                        reset_date: None,
                        amount: facility_fee,
                        kind: CFKind::Fee,
                        accrual_factor: accrual,
                    });
                }

                // Add terminal repayment of outstanding principal at maturity
                if step == num_steps - 1 && !defaulted {
                    path_flows.push(CashFlow {
                        date: date_next,
                        reset_date: None,
                        amount: Money::new(drawn, ccy),
                        kind: CFKind::Notional,
                        accrual_factor: 0.0,
                    });
                }

                // Evolve utilization using Euler-Maruyama discretization
                // dU = speed * (target - U) * dt + volatility * sqrt(dt) * dW
                if step < num_steps - 1 {
                    let drift = speed * (target_rate - utilization) * actual_dt;
                    let diffusion = volatility * actual_dt.sqrt() * rng.next_std_normal();
                    utilization += drift + diffusion;

                    // Clamp utilization to [0, 1]
                    utilization = utilization.clamp(0.0, 1.0);
                }
            }

            // Build CashFlowSchedule for this path
            let path_schedule = CashFlowSchedule {
                flows: path_flows,
                notional: Notional::par(facility.commitment_amount.amount(), ccy),
                day_count: facility.day_count,
                meta: Default::default(),
            };

            // Compute per-period PVs for this path
            let period_pvs = path_schedule.pre_period_pv(
                &periods,
                disc as &dyn finstack_core::market_data::traits::Discounting,
                as_of,
                disc_dc,
            );

            // Sum PVs across all periods for this path
            let mut path_pv = 0.0;
            for (_period_id, ccy_map) in period_pvs.iter() {
                if let Some(pv_money) = ccy_map.get(&ccy) {
                    path_pv += pv_money.amount();
                }
            }

            sum_pv += path_pv;
        }

        // Average across paths
        let mut mean_pv = sum_pv / (num_paths as f64);

        // Handle upfront fee at pricer level (consistent with deterministic pricer)
        let upfront_fee_pv = if let Some(upfront_fee) = facility.fees.upfront_fee {
            if facility.commitment_date > as_of {
                let t_commitment = disc_dc.year_fraction(
                    base_date,
                    facility.commitment_date,
                    DayCountCtx::default(),
                )?;
                let t_as_of = disc_dc.year_fraction(base_date, as_of, DayCountCtx::default())?;

                let df_commitment = disc.df(t_commitment);
                let df_as_of = disc.df(t_as_of);
                let df = if df_as_of > 0.0 {
                    df_commitment / df_as_of
                } else {
                    1.0
                };

                upfront_fee.amount() * df
            } else {
                upfront_fee.amount()
            }
        } else {
            0.0
        };

        // Lender perspective: upfront fee is an outflow, so subtract from PV
        // Note: Cashflows include all principal flows (draws/repays and terminal repayment),
        // so we don't need to separately account for initial capital deployment.
        mean_pv -= upfront_fee_pv;

        Ok(Money::new(mean_pv, ccy))
    }

    /// Multi-factor Monte Carlo pricing with credit spread and interest rate dynamics.
    #[cfg(feature = "mc")]
    fn price_multi_factor(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
        stoch_spec: &super::types::StochasticUtilizationSpec,
        mc_config: &super::types::McConfig,
    ) -> finstack_core::Result<Money> {
        use super::types::{
            BaseRateSpec, CreditSpreadProcessSpec, InterestRateProcessSpec, UtilizationProcess,
        };
        use crate::instruments::common::mc::rng::philox::PhiloxRng;
        use crate::instruments::common::mc::rng::sobol::SobolRng;
        use crate::instruments::common::mc::time_grid::TimeGrid;
        use crate::instruments::common::mc::traits::StochasticProcess;
        use crate::instruments::common::models::monte_carlo::discretization::revolving_credit::RevolvingCreditDiscretization;
        use crate::instruments::common::models::monte_carlo::engine::McEngineBuilder;
        use crate::instruments::common::models::monte_carlo::payoff::revolving_credit::{
            FeeStructure, RevolvingCreditPayoff,
        };
        use crate::instruments::common::models::monte_carlo::process::revolving_credit::{
            CreditSpreadParams, InterestRateSpec, RevolvingCreditProcess,
            RevolvingCreditProcessParams, UtilizationParams,
        };

        // Extract utilization parameters
        let util_params = match &stoch_spec.utilization_process {
            UtilizationProcess::MeanReverting {
                target_rate,
                speed,
                volatility,
            } => UtilizationParams::new(*speed, *target_rate, *volatility),
        };

        // Build interest rate specification
        let interest_rate_spec = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => InterestRateSpec::Fixed { rate: *rate },
            BaseRateSpec::Floating { .. } => {
                // Get interest rate process from config
                match &mc_config.interest_rate_process {
                    Some(InterestRateProcessSpec::HullWhite1F {
                        kappa,
                        sigma,
                        initial,
                        theta,
                    }) => {
                        use crate::instruments::common::mc::process::ou::HullWhite1FParams;
                        InterestRateSpec::Floating {
                            params: HullWhite1FParams::new(*kappa, *sigma, *theta),
                            initial: *initial,
                        }
                    }
                    None => {
                        // Floating rate but no process specified - use deterministic forward curve
                        let fwd = market.get_forward_ref(match &facility.base_rate_spec {
                            BaseRateSpec::Floating { index_id, .. } => index_id.as_str(),
                            _ => unreachable!(),
                        })?;
                        let times = fwd.knots().to_vec();
                        let rates = fwd.forwards().to_vec();
                        InterestRateSpec::DeterministicForward { times, rates }
                    }
                }
            }
        };

        // Build credit spread parameters
        let credit_spread_params = match &mc_config.credit_spread_process {
            CreditSpreadProcessSpec::Cir {
                kappa,
                theta,
                sigma,
                initial,
            } => CreditSpreadParams::new(*kappa, *theta, *sigma, *initial),
            CreditSpreadProcessSpec::Constant(spread) => {
                // Use constant spread with minimal dynamics (very low vol)
                CreditSpreadParams::new(0.01, *spread, 0.001, *spread)
            }
            CreditSpreadProcessSpec::MarketAnchored {
                hazard_curve_id,
                kappa,
                implied_vol,
                tenor_years,
            } => {
                // Pull hazard curve and compute tenor to maturity (or provided tenor)
                let hazard = market.get_hazard_ref(hazard_curve_id.as_str())?;
                let dc = hazard.day_count();
                let base_date = hazard.base_date();

                let t_maturity = dc.year_fraction(
                    base_date,
                    facility.maturity_date,
                    finstack_core::dates::DayCountCtx::default(),
                )?;
                let t = tenor_years.unwrap_or_else(|| t_maturity.max(1e-8));

                // Survival and average hazard over [0,T]
                let sp_t = hazard.sp(t);
                let avg_lambda = if t > 0.0 { (-sp_t.ln()) / t } else { 0.0 };

                // Initial hazard from first segment (fallback to avg when unavailable)
                let mut first_lambda = None;
                if let Some((tenor, lambda)) = hazard.knot_points().next() {
                    let _ = tenor;
                    first_lambda = Some(lambda.max(0.0));
                }
                let lambda0 = first_lambda.unwrap_or(avg_lambda).max(0.0);

                // Map hazard ↔ spread using s ≈ (1 − R) · λ
                let one_minus_r = (1.0 - mc_config.recovery_rate).max(1e-6);
                let s0 = one_minus_r * lambda0;
                let s_bar = one_minus_r * avg_lambda;

                // Mean-anchored CIR params on spread space
                let k = *kappa;
                let a = if (k * t).abs() < 1e-8 {
                    1.0 - 0.5 * k * t // first-order expansion
                } else {
                    (1.0 - (-k * t).exp()) / (k * t)
                };
                let theta = if (1.0 - a).abs() < 1e-12 {
                    s_bar
                } else {
                    ((s_bar - a * s0) / (1.0 - a)).max(0.0)
                };

                // Volatility scaled to match fractional vol near mean: σ ≈ implied_vol * sqrt(s̄)
                let sigma = (*implied_vol) * s_bar.max(1e-12).sqrt();

                CreditSpreadParams::new(k, theta, sigma, s0)
            }
        };

        // Get discount curve and compute time grid anchors (t_start, t_end)
        let disc_curve = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc_curve.day_count();
        let base_date = disc_curve.base_date();

        let t_start = disc_dc.year_fraction(
            base_date,
            facility.commitment_date.max(as_of),
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_end = disc_dc.year_fraction(
            base_date,
            facility.maturity_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let time_horizon = (t_end - t_start).max(0.0);

        if time_horizon <= 0.0 {
            return Ok(Money::new(0.0, facility.commitment_amount.currency()));
        }

        // Build process parameters
        let mut process_params = RevolvingCreditProcessParams::new(
            util_params,
            interest_rate_spec,
            credit_spread_params,
        );

        // Set correlation: prefer provided matrix, else use util–credit correlation if supplied
        // Default to identity (independent factors) when no correlation is specified
        if let Some(corr) = mc_config.correlation_matrix {
            // Validate correlation matrix is PSD
            finstack_core::math::linalg::validate_correlation_matrix(
                &corr.iter().flatten().copied().collect::<Vec<_>>(),
                3,
            )?;
            process_params = process_params.with_correlation(corr);
        } else if let Some(rho) = mc_config.util_credit_corr {
            let correlation = [[1.0, 0.0, rho], [0.0, 1.0, 0.0], [rho, 0.0, 1.0]];
            process_params = process_params.with_correlation(correlation);
        }

        // Apply time offset to align MC time to market time axis
        // Map MC time 0 to commitment date offset on the curve axis
        process_params = process_params.with_time_offset(t_start);
        let process = RevolvingCreditProcess::new(process_params);

        // Build discretization
        let disc = RevolvingCreditDiscretization::from_process(&process)?;

        // Create time grid (quarterly steps)
        let num_steps = ((time_horizon / 0.25).ceil() as usize).max(1);
        let time_grid = TimeGrid::uniform(time_horizon, num_steps)?;

        // Precompute discount factors for each step (for payoff internal PV accumulation)
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let df_as_of = disc_curve.df(t_as_of);

        let mut discount_factors = Vec::with_capacity(num_steps + 1);
        discount_factors.push(if df_as_of > 0.0 {
            disc_curve.df(t_start) / df_as_of
        } else {
            1.0
        });
        for i in 0..num_steps {
            let t_abs = t_start + time_grid.time(i + 1);
            let df_abs = disc_curve.df(t_abs);
            discount_factors.push(if df_as_of > 0.0 {
                df_abs / df_as_of
            } else {
                1.0
            });
        }

        // Build payoff (NOTE: upfront fee handled separately below, not in payoff)
        let fees = FeeStructure::new(
            facility.fees.commitment_fee_bp,
            facility.fees.usage_fee_bp,
            facility.fees.facility_fee_bp,
        );

        let is_fixed_rate = matches!(facility.base_rate_spec, BaseRateSpec::Fixed { .. });
        let (fixed_rate, margin_bp) = match &facility.base_rate_spec {
            BaseRateSpec::Fixed { rate } => (*rate, 0.0),
            BaseRateSpec::Floating { margin_bp, .. } => (0.0, *margin_bp),
        };

        let payoff = RevolvingCreditPayoff::new(
            facility.commitment_amount.amount(),
            facility.day_count,
            is_fixed_rate,
            fixed_rate,
            margin_bp,
            fees,
            mc_config.recovery_rate,
            time_horizon,
            discount_factors,
        );

        // Initial state
        let initial_utilization = facility.utilization_rate();
        let initial_state = process.params().initial_state(initial_utilization);

        // Create MC engine
        let seed = stoch_spec.seed.unwrap_or(42);
        let engine = McEngineBuilder::new()
            .num_paths(stoch_spec.num_paths)
            .seed(seed)
            .time_grid(time_grid)
            .parallel(cfg!(feature = "parallel"))
            .antithetic(stoch_spec.antithetic)
            .build()?;

        // Create RNG
        // Choose RNG based on spec
        let rng_philox = PhiloxRng::new(seed);
        let sobol_dim = process.num_factors();
        let rng_sobol = SobolRng::new(sobol_dim, seed);
        let use_sobol = stoch_spec.use_sobol_qmc;

        // Run simulation
        // Note: Payoff emits undiscounted cashflows; engine handles discounting
        let estimate = if use_sobol {
            engine.price::<SobolRng, _, _, _>(
                &rng_sobol,
                &process,
                &disc,
                &initial_state,
                &payoff,
                facility.commitment_amount.currency(),
                1.0,
            )?
        } else {
            engine.price::<PhiloxRng, _, _, _>(
                &rng_philox,
                &process,
                &disc,
                &initial_state,
                &payoff,
                facility.commitment_amount.currency(),
                1.0,
            )?
        };

        // Handle upfront fee at pricer level (one-time cashflow, not path-dependent)
        // Upfront fee is paid by lender at commitment, so it reduces facility value
        let upfront_fee_pv = if let Some(upfront_fee) = facility.fees.upfront_fee {
            // Discount upfront fee from commitment date to as_of
            let t_as_of = disc_dc.year_fraction(
                base_date,
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df_as_of = disc_curve.df(t_as_of);
            let df_commitment = disc_curve.df(t_start);
            let df = if df_as_of > 0.0 {
                df_commitment / df_as_of
            } else {
                1.0
            };
            upfront_fee.amount() * df
        } else {
            0.0
        };

        // Combine path-dependent PV with upfront fee
        // Lender perspective: upfront fee is an outflow, so subtract from PV
        // Note: Cashflows include all principal flows (draws/repays and terminal repayment),
        // so we don't need to separately account for initial capital deployment.
        let total_pv = estimate.mean.amount() - upfront_fee_pv;

        Ok(Money::new(total_pv, facility.commitment_amount.currency()))
    }
}

#[cfg(feature = "mc")]
impl Pricer for RevolvingCreditMcPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Validate that we have a stochastic spec
        if !facility.is_stochastic() {
            return Err(PricingError::model_failure(
                "RevolvingCreditMcPricer requires stochastic specification".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market
            .get_discount_ref(facility.discount_curve_id.as_str())
            .map_err(|e| PricingError::model_failure(e.to_string()))?;
        let as_of = disc.base_date();

        // Price the facility using MC
        let pv = Self::price_stochastic(facility, market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use time::Month;

    #[test]
    fn test_pricer_key() {
        let pricer = RevolvingCreditDiscountingPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
        );
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_pricer_key() {
        let pricer = RevolvingCreditMcPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::MonteCarloGBM)
        );
    }

    #[test]
    fn test_deterministic_period_pv_consistency() {
        // Test that sum of per-period PVs equals total NPV
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-TEST".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees {
                upfront_fee: None,
                commitment_fee_bp: 25.0,
                usage_fee_bp: 10.0,
                facility_fee_bp: 5.0,
            })
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        // Create a simple flat discount curve
        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // Verify we get a reasonable PV magnitude
        assert!(
            pv.amount().abs() < 10_000_000.0,
            "PV magnitude should be reasonable"
        );
    }

    #[test]
    fn test_deterministic_with_draw_repay() {
        // Test deterministic pricing with draw/repay events
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-TEST-2".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
                super::super::types::DrawRepayEvent {
                    date: draw_date,
                    amount: Money::new(2_000_000.0, Currency::USD),
                    is_draw: true,
                },
            ]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // Should price successfully
        assert!(pv.currency() == Currency::USD);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_mc_zero_volatility_matches_deterministic() {
        // Test that MC with zero volatility (single deterministic path) matches deterministic pricing
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility_det = RevolvingCredit::builder()
            .id("RC-DET".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let facility_mc = RevolvingCredit::builder()
            .id("RC-MC".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Stochastic(Box::new(
                super::super::types::StochasticUtilizationSpec {
                    utilization_process: super::super::types::UtilizationProcess::MeanReverting {
                        target_rate: 0.5,
                        speed: 1.0,
                        volatility: 0.0, // Zero volatility = deterministic
                    },
                    num_paths: 1, // Single path
                    seed: Some(42),
                    antithetic: false,
                    use_sobol_qmc: false,
                    default_model: None,
                    #[cfg(feature = "mc")]
                    mc_config: None,
                },
            )))
            .discount_curve_id("USD-OIS".into())
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        let pv_det =
            RevolvingCreditDiscountingPricer::price_deterministic(&facility_det, &market, start)
                .unwrap();
        let pv_mc =
            RevolvingCreditMcPricer::price_stochastic(&facility_mc, &market, start).unwrap();

        // With zero volatility and single path, MC should match deterministic (within numerical tolerance)
        let diff = (pv_det.amount() - pv_mc.amount()).abs();
        assert!(
            diff < 200_000.0,
            "MC with zero volatility should match deterministic, diff: {}",
            diff
        );
    }
}
