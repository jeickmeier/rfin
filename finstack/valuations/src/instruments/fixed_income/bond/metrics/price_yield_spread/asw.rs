use crate::cashflow::traits::CashflowProvider;
use crate::cashflow::{builder::CashFlowSchedule, primitives::CFKind};
use crate::instruments::bond::pricing::quote_engine::{
    fixed_leg_annuity, par_rate_and_annuity_from_discount,
};
use crate::instruments::bond::CashflowSpec;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::term_structures::DiscountCurve;
use rust_decimal::prelude::ToPrimitive;

/// Configuration for fixed-leg conventions used in ASW par/market metrics.
///
/// Controls the day-count, frequency, business day convention, calendar, and stub
/// rules for the asset swap fixed leg. When a field is `None`, the corresponding
/// convention falls back to the bond's own coupon conventions.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::fixed_income::bond::metrics::price_yield_spread::asw::AssetSwapConfig;
/// use finstack_core::dates::{DayCount, Tenor, BusinessDayConvention, StubKind};
///
/// let config = AssetSwapConfig {
///     fixed_leg_day_count: Some(DayCount::Act365F),
///     fixed_leg_frequency: Some(Tenor::semi_annual()),
///     fixed_leg_bdc: Some(BusinessDayConvention::ModifiedFollowing),
///     fixed_leg_calendar_id: Some("USGS".to_string()),
///     fixed_leg_stub: Some(StubKind::ShortFront),
/// };
/// ```
#[derive(Clone, Debug, Default)]
pub struct AssetSwapConfig {
    /// Day-count convention for the ASW fixed leg (annuity).
    pub fixed_leg_day_count: Option<DayCount>,
    /// Payment frequency for the ASW fixed leg.
    pub fixed_leg_frequency: Option<Tenor>,
    /// Business day convention for the ASW fixed-leg schedule.
    pub fixed_leg_bdc: Option<BusinessDayConvention>,
    /// Optional calendar identifier for business-day adjustment.
    pub fixed_leg_calendar_id: Option<String>,
    /// Stub convention for the ASW fixed-leg schedule.
    pub fixed_leg_stub: Option<StubKind>,
}

/// Asset swap par spread calculator using discount-curve annuity approximation.
///
/// Par ASW is the spread such that the PV of fixed coupons at `(df * (1 + s*α))`
/// equals par. Uses the closed-form approximation: `asw_par ≈ coupon - par_swap_rate`.
///
/// # Dependencies
///
/// Requires `Ytm` metric to be computed first (for par rate calculation).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // Par ASW is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct AssetSwapParCalculator {
    config: AssetSwapConfig,
}

/// Asset swap market spread calculator using market price.
///
/// Market ASW is the spread that equates the PV of the bond's fixed leg to the
/// dirty market price. Uses the approximation:
/// ```text
/// asw_mkt ≈ (dirty/Notional - price_pv/Notional)/annuity + coupon - par_rate
/// ```
///
/// # Dependencies
///
/// Requires `Accrued` metric to be computed first (for dirty price calculation).
///
/// # Examples
///
/// ```rust,no_run
/// use finstack_valuations::instruments::fixed_income::bond::Bond;
/// use finstack_valuations::metrics::{MetricRegistry, MetricId, MetricContext};
/// use finstack_core::market_data::context::MarketContext;
/// use finstack_core::dates::Date;
///
/// # let bond = Bond::example();
/// # let market = MarketContext::new();
/// # let as_of = Date::from_calendar_date(2024, time::Month::January, 15).unwrap();
/// // Market ASW is computed automatically when requesting bond metrics
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct AssetSwapMarketCalculator {
    config: AssetSwapConfig,
}

impl AssetSwapParCalculator {
    /// Create a par ASW calculator with default behaviour (bond conventions).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a par ASW calculator with explicit fixed-leg conventions.
    pub fn with_config(config: AssetSwapConfig) -> Self {
        Self { config }
    }
}

impl AssetSwapMarketCalculator {
    /// Create a market ASW calculator with default behaviour (bond conventions).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a market ASW calculator with explicit fixed-leg conventions.
    pub fn with_config(config: AssetSwapConfig) -> Self {
        Self { config }
    }
}

fn build_future_dates_from_flows(
    flows: &[(finstack_core::dates::Date, finstack_core::money::Money)],
    as_of: finstack_core::dates::Date,
) -> Vec<finstack_core::dates::Date> {
    use finstack_core::dates::Date;
    use std::collections::BTreeSet;
    let mut set: BTreeSet<Date> = BTreeSet::new();
    for (d, _amt) in flows {
        if *d > as_of {
            set.insert(*d);
        }
    }
    let mut dates: Vec<Date> = Vec::with_capacity(set.len() + 1);
    dates.push(as_of);
    dates.extend(set);
    dates
}

/// PV of coupon-only leg from a custom schedule (excludes amortization and principal).
fn pv_coupon_from_custom_schedule(
    disc: &DiscountCurve,
    schedule: &CashFlowSchedule,
    as_of: Date,
) -> finstack_core::Result<f64> {
    use finstack_core::math::summation::NeumaierAccumulator;

    let mut pv = NeumaierAccumulator::new();
    for cf in &schedule.flows {
        if cf.date <= as_of {
            continue;
        }
        match cf.kind {
            CFKind::Fixed | CFKind::Stub => {
                let df = disc.df_on_date_curve(cf.date)?;
                pv.add(cf.amount.amount() * df);
            }
            _ => {}
        }
    }
    Ok(pv.total())
}

/// Compute Par ASW using a forward-based methodology with explicit parameters.
///
/// Note:
/// - This helper uses the bond's coupon day-count for the fixed-leg annuity.
/// - In many markets, par asset swaps are quoted on swap fixed-leg conventions
///   that may differ from the bond's coupon convention.
///
/// For explicit control over the fixed-leg convention (e.g., to align with
/// a swap market standard per currency), prefer
/// [`asw_par_with_forward_config`], which accepts an optional fixed-leg
/// day-count override.
pub fn asw_par_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
) -> finstack_core::Result<f64> {
    asw_par_with_forward_config(bond, curves, as_of, fwd_curve_id, float_spread_bp, None)
}

/// Compute Par ASW using a forward-based methodology with explicit fixed-leg
/// conventions.
///
/// - `fixed_leg_day_count`: when `Some`, this day-count is used to build the
///   fixed-leg annuity, allowing callers to align with swap fixed-leg market
///   conventions (e.g., 30E/360) instead of the bond's coupon convention.
/// - When `None`, this falls back to `bond.cashflow_spec.day_count()`.
pub fn asw_par_with_forward_config(
    bond: &Bond,
    curves: &finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
    fixed_leg_day_count: Option<DayCount>,
) -> finstack_core::Result<f64> {
    let disc = curves.get_discount(&bond.discount_curve_id)?;
    let fwd = curves.get_forward(fwd_curve_id)?;

    // Mirror the bond schedule via holder flows
    let flows = bond.build_dated_flows(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }

    let fixed_dc = fixed_leg_day_count.unwrap_or_else(|| bond.cashflow_spec.day_count());
    let ann = fixed_leg_annuity(disc.as_ref(), fixed_dc, &sched)?;
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let f_base = fwd.base_date();
    let f_dc = fwd.day_count();
    let spread = float_spread_bp * 1e-4;
    let mut pv_float = 0.0;
    let mut prev = sched[0];
    for &d in &sched[1..] {
        let t1 = f_dc.year_fraction(f_base, prev, finstack_core::dates::DayCountCtx::default())?;
        let t2 = f_dc.year_fraction(f_base, d, finstack_core::dates::DayCountCtx::default())?;
        let yf = f_dc.year_fraction(prev, d, finstack_core::dates::DayCountCtx::default())?;
        let rate = fwd.rate_period(t1, t2) + spread;
        let coupon_flt = bond.notional.amount() * rate * yf;
        let df = disc.df_on_date_curve(d)?;
        pv_float += coupon_flt * df;
        prev = d;
    }
    let par_rate = pv_float / (bond.notional.amount() * ann);

    // Equivalent fixed rate from coupon-only PV
    let eq_coupon = if let Some(custom) = &bond.custom_cashflows {
        let pv_coupon = pv_coupon_from_custom_schedule(disc.as_ref(), custom, as_of)?;
        pv_coupon / (bond.notional.amount() * ann)
    } else {
        // Extract fixed coupon rate from cashflow_spec (converting Decimal to f64)
        match &bond.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            _ => return Err(finstack_core::InputError::Invalid.into()),
        }
    };
    Ok(eq_coupon - par_rate)
}

/// Compute Market ASW using forward-based methodology with explicit parameters.
///
/// Note:
/// - This helper requires an explicit dirty market price in currency.
///   Callers **must** pass `Some(dirty_price_ccy)` even when interpreting
///   ASW relative to par (in which case, pass `bond.notional.amount()`).
/// - In many markets, the fixed leg follows swap fixed-leg conventions that
///   may differ from the bond's coupon convention.
///
/// For explicit control over fixed-leg conventions (e.g., swap day-count),
/// prefer [`asw_market_with_forward_config`], which accepts an optional
/// fixed-leg day-count override.
pub fn asw_market_with_forward(
    bond: &Bond,
    curves: &finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
    dirty_price_ccy: Option<f64>,
) -> finstack_core::Result<f64> {
    asw_market_with_forward_config(
        bond,
        curves,
        as_of,
        fwd_curve_id,
        float_spread_bp,
        dirty_price_ccy,
        None,
    )
}

/// Compute Market ASW using forward-based methodology with explicit
/// fixed-leg conventions.
///
/// - `dirty_price_ccy`: dirty market price expressed in currency. When
///   `None`, this returns `InputError::NotFound { id: "dirty_price_ccy" }`
///   instead of silently assuming par.
/// - `fixed_leg_day_count`: when `Some`, this day-count is used to build
///   the fixed-leg annuity; otherwise the bond's coupon day-count is used.
pub fn asw_market_with_forward_config(
    bond: &Bond,
    curves: &finstack_core::market_data::context::MarketContext,
    as_of: finstack_core::dates::Date,
    fwd_curve_id: &str,
    float_spread_bp: f64,
    dirty_price_ccy: Option<f64>,
    fixed_leg_day_count: Option<DayCount>,
) -> finstack_core::Result<f64> {
    let disc = curves.get_discount(&bond.discount_curve_id)?;
    let flows = bond.build_dated_flows(curves, as_of)?;
    let sched = build_future_dates_from_flows(&flows, as_of);
    if sched.len() < 2 {
        return Ok(0.0);
    }
    let fixed_dc = fixed_leg_day_count.unwrap_or_else(|| bond.cashflow_spec.day_count());
    let ann = fixed_leg_annuity(disc.as_ref(), fixed_dc, &sched)?;
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let par_asw = asw_par_with_forward_config(
        bond,
        curves,
        as_of,
        fwd_curve_id,
        float_spread_bp,
        fixed_leg_day_count,
    )?;
    let notional = bond.notional.amount();
    let dirty = match dirty_price_ccy {
        Some(v) => v,
        None => {
            return Err(finstack_core::InputError::NotFound {
                id: "dirty_price_ccy".to_string(),
            }
            .into());
        }
    };
    let price_pct = dirty / notional;
    // Market ASW = Par ASW + (Market Price % - 100%) / Annuity
    Ok(par_asw + (price_pct - 1.0) / ann)
}

impl MetricCalculator for AssetSwapParCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let bond: &Bond = context.instrument_as()?;

        // If the bond has custom cashflows, compute ASW using a forward-based
        // custom-swap constructed on the same schedule. Requires a float spec.
        if bond.custom_cashflows.is_some() {
            match &bond.cashflow_spec {
                CashflowSpec::Floating(spec) => {
                    return asw_par_with_forward(
                        bond,
                        &context.curves,
                        context.as_of,
                        spec.rate_spec.index_id.as_str(),
                        spec.rate_spec.spread_bp.to_f64().unwrap_or(0.0),
                    );
                }
                _ => {
                    return Err(finstack_core::InputError::NotFound {
                        id: "bond.cashflow_spec.floating".to_string(),
                    }
                    .into());
                }
            }
        }

        let discount_curve_id = bond.discount_curve_id.to_owned();
        let maturity = bond.maturity;
        let bond_dc = bond.cashflow_spec.day_count();
        let disc = context.curves.get_discount(&discount_curve_id)?;

        // Extract schedule params from cashflow_spec, allowing ASW config to
        // override fixed-leg conventions when provided.
        let (bond_freq, bond_bdc, bond_calendar_id, bond_stub) = match &bond.cashflow_spec {
            CashflowSpec::Fixed(spec) => {
                (spec.freq, spec.bdc, spec.calendar_id.as_deref(), spec.stub)
            }
            CashflowSpec::Floating(spec) => (
                spec.freq,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
                spec.stub,
            ),
            CashflowSpec::Amortizing { base, .. } => match &**base {
                CashflowSpec::Fixed(spec) => {
                    (spec.freq, spec.bdc, spec.calendar_id.as_deref(), spec.stub)
                }
                CashflowSpec::Floating(spec) => (
                    spec.freq,
                    spec.rate_spec.bdc,
                    spec.rate_spec.calendar_id.as_deref(),
                    spec.stub,
                ),
                _ => return Err(finstack_core::InputError::Invalid.into()),
            },
        };

        let freq = self.config.fixed_leg_frequency.unwrap_or(bond_freq);
        let bdc = self.config.fixed_leg_bdc.unwrap_or(bond_bdc);
        let calendar_id = self
            .config
            .fixed_leg_calendar_id
            .as_deref()
            .or(bond_calendar_id);
        let stub = self.config.fixed_leg_stub.unwrap_or(bond_stub);

        // Market standard: Par swap rate via discount ratio on the ASW fixed-leg
        // schedule. By default this matches the bond schedule; callers may
        // override fixed-leg conventions via AssetSwapConfig.
        let mut builder = finstack_core::dates::ScheduleBuilder::new(context.as_of, maturity)?
            .frequency(freq)
            .stub_rule(stub);

        if let Some(id) = calendar_id {
            if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
                builder = builder.adjust_with(bdc, cal);
            }
        }

        let sched: Vec<Date> = builder.build()?.into_iter().collect();
        if sched.len() < 2 {
            return Ok(0.0);
        }
        let dc_fixed = self.config.fixed_leg_day_count.unwrap_or(bond_dc);
        let (par_rate, ann) = par_rate_and_annuity_from_discount(disc.as_ref(), dc_fixed, &sched)?;
        if ann == 0.0 {
            return Ok(0.0);
        }
        // Use stated coupon for non-custom bonds; for custom bonds, this branch is not reached
        let coupon = match &bond.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
            _ => return Err(finstack_core::InputError::Invalid.into()),
        };
        Ok(coupon - par_rate)
    }
}

impl MetricCalculator for AssetSwapMarketCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let (discount_curve_id, maturity, dc, notional_amt, quoted_clean, is_custom, coupon) = {
            let b: &Bond = context.instrument_as()?;
            let coupon_rate = match &b.cashflow_spec {
                CashflowSpec::Fixed(spec) => spec.rate.to_f64().unwrap_or(0.0),
                _ => 0.0, // Will be handled later if needed
            };
            (
                b.discount_curve_id.to_owned(),
                b.maturity,
                b.cashflow_spec.day_count(),
                b.notional.amount(),
                b.pricing_overrides.quoted_clean_price,
                b.custom_cashflows.is_some(),
                coupon_rate,
            )
        };
        let disc = context.curves.get_discount(&discount_curve_id)?;

        // Dirty market value in currency
        let dirty_ccy = if let Some(clean_px) = quoted_clean {
            let accrued = context
                .computed
                .get(&MetricId::Accrued)
                .copied()
                .ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::InputError::NotFound {
                        id: "metric:Accrued".to_string(),
                    })
                })?;
            clean_px * notional_amt / 100.0 + accrued
        } else {
            context.base_value.amount()
        };

        // If the bond has custom cashflows, compute forward-based ASW using the
        // bond's float spec on the same (custom) schedule. Requires a float spec.
        if is_custom {
            let bond: &Bond = context.instrument_as()?;
            match &bond.cashflow_spec {
                CashflowSpec::Floating(spec) => {
                    return asw_market_with_forward(
                        bond,
                        &context.curves,
                        context.as_of,
                        spec.rate_spec.index_id.as_str(),
                        spec.rate_spec.spread_bp.to_f64().unwrap_or(0.0),
                        Some(dirty_ccy),
                    );
                }
                _ => {
                    return Err(finstack_core::InputError::NotFound {
                        id: "bond.cashflow_spec.floating".to_string(),
                    }
                    .into());
                }
            }
        }

        // Fixed coupon-leg PV (exclude principal) at zero spread
        if context.cashflows.is_none() {
            let (disc_id_capture, dc_capture, built) = {
                let b: &Bond = context.instrument_as()?;
                (
                    b.discount_curve_id.to_owned(),
                    b.cashflow_spec.day_count(),
                    b.build_dated_flows(&context.curves, context.as_of)?,
                )
            };
            context.cashflows = Some(built);
            context.discount_curve_id = Some(disc_id_capture);
            context.day_count = Some(dc_capture);
        }
        let _flows = context.cashflows.as_ref().ok_or_else(|| {
            finstack_core::Error::from(finstack_core::InputError::NotFound {
                id: "cashflows".to_string(),
            })
        })?;
        // Note: we no longer pre-compute coupon-only PV here; custom-bond coupon PV is
        // computed below when needed to derive the equivalent coupon.

        // Forward-based path when configured for non-custom bonds is available
        // via explicit helper methods (ASW*Fwd calculators). Here we keep fallback-only.

        // Market standard: discount-ratio using ASW fixed-leg schedule (defaulting
        // to bond conventions but allowing overrides via AssetSwapConfig).
        let bond: &Bond = context.instrument_as()?;
        let (bond_freq, bond_stub, bond_bdc, bond_calendar_id) = match &bond.cashflow_spec {
            CashflowSpec::Fixed(spec) => {
                (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
            }
            CashflowSpec::Floating(spec) => (
                spec.freq,
                spec.stub,
                spec.rate_spec.bdc,
                spec.rate_spec.calendar_id.as_deref(),
            ),
            CashflowSpec::Amortizing { base, .. } => match &**base {
                CashflowSpec::Fixed(spec) => {
                    (spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref())
                }
                CashflowSpec::Floating(spec) => (
                    spec.freq,
                    spec.stub,
                    spec.rate_spec.bdc,
                    spec.rate_spec.calendar_id.as_deref(),
                ),
                _ => return Err(finstack_core::InputError::Invalid.into()),
            },
        };
        let freq = self.config.fixed_leg_frequency.unwrap_or(bond_freq);
        let stub = self.config.fixed_leg_stub.unwrap_or(bond_stub);
        let bdc = self.config.fixed_leg_bdc.unwrap_or(bond_bdc);
        let calendar_id = self
            .config
            .fixed_leg_calendar_id
            .as_deref()
            .or(bond_calendar_id);

        let mut builder = finstack_core::dates::ScheduleBuilder::new(context.as_of, maturity)?
            .frequency(freq)
            .stub_rule(stub);

        if let Some(id) = calendar_id {
            if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
                builder = builder.adjust_with(bdc, cal);
            }
        }

        let sched: Vec<Date> = builder.build()?.into_iter().collect();
        let dc_fixed = self.config.fixed_leg_day_count.unwrap_or(dc);
        let (par_rate, ann) = par_rate_and_annuity_from_discount(disc.as_ref(), dc_fixed, &sched)?;
        if ann == 0.0 || notional_amt == 0.0 {
            return Ok(0.0);
        }
        // Equivalent coupon from coupon PV only for custom bonds; otherwise stated coupon
        let eq_coupon = if let Some(custom) = &context.instrument_as::<Bond>()?.custom_cashflows {
            let pv_coupon = pv_coupon_from_custom_schedule(disc.as_ref(), custom, context.as_of)?;
            pv_coupon / (notional_amt * ann)
        } else {
            coupon
        };
        // Market ASW = Par ASW + (Market Price % - 100%) / Annuity
        let price_pct = dirty_ccy / notional_amt;
        let asw_mkt = (eq_coupon - par_rate) + (price_pct - 1.0) / ann;
        Ok(asw_mkt)
    }
}
