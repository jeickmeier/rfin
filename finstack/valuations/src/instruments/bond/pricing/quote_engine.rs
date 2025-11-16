//! Bond quote engine for mapping between price, yields, and spreads.
//!
//! This module provides a small, opinionated API that takes **one**
//! quote input (price, yield, or spread) and produces a consistent
//! set of derived bond quotes using the existing pricing and metric
//! infrastructure.
//!
//! All spread-style quantities exposed here use **decimal units**:
//! `0.01` corresponds to **100 basis points**.

use super::helpers;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::bond::Bond;
use crate::instruments::common::traits::Instrument;
use crate::metrics::{MetricContext, MetricId};
use crate::metrics::{standard_registry, MetricRegistry};
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::Result;
use std::sync::Arc;

/// Quote input for the bond quote engine.
///
/// All spreads are expressed in **decimal** (`0.01 = 100bp`).
pub enum BondQuoteInput {
    /// Clean price quoted as percentage of par (e.g., 99.5 = 99.5% of par).
    CleanPricePct(f64),
    /// Dirty price in currency units.
    DirtyPriceCcy(f64),
    /// Yield to maturity (decimal).
    Ytm(f64),
    /// Z-spread over the discount curve (decimal).
    ZSpread(f64),
    /// Discount margin for FRNs (decimal).
    DiscountMargin(f64),
    /// Option-adjusted spread (decimal).
    Oas(f64),
    /// Asset swap market spread (decimal).
    AswMarket(f64),
    /// I-spread (decimal).
    ISpread(f64),
}

/// Full quote set produced by the quote engine.
///
/// - Prices are returned both in currency and as % of par.
/// - All spreads are decimal (`0.01 = 100bp`).
#[derive(Debug, Clone)]
pub struct BondQuoteSet {
    /// Clean price in currency.
    pub clean_price_ccy: f64,
    /// Clean price as percentage of par (quote convention).
    pub clean_price_pct: f64,
    /// Dirty price in currency.
    pub dirty_price_ccy: f64,
    /// Yield to maturity (decimal), if applicable.
    pub ytm: Option<f64>,
    /// Yield to worst (decimal), if applicable.
    pub ytw: Option<f64>,
    /// Z-spread over discount curve (decimal), if applicable.
    pub z_spread: Option<f64>,
    /// Discount margin for FRNs (decimal), if applicable.
    pub discount_margin: Option<f64>,
    /// Option-adjusted spread (decimal), if applicable.
    pub oas: Option<f64>,
    /// Asset swap par spread (decimal), if applicable.
    pub asw_par: Option<f64>,
    /// Asset swap market spread (decimal), if applicable.
    pub asw_market: Option<f64>,
    /// I-spread (decimal), if applicable.
    pub i_spread: Option<f64>,
}

/// Convert between price, yield, and spread metrics for a bond.
///
/// The engine:
/// - Normalizes the chosen `quote_input` into a **canonical dirty price in currency**.
/// - Derives the corresponding clean price (% of par) and stamps it into
///   `pricing_overrides.quoted_clean_price` on an internal bond clone.
/// - Uses the standard metrics registry to compute the remaining metrics.
pub fn compute_quotes(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    quote_input: BondQuoteInput,
) -> Result<BondQuoteSet> {
    // Work on a local clone so we never mutate the caller's bond instance.
    let mut bond_for_metrics = bond.clone();

    // Accrued interest in currency using the context-aware helper
    // so FRNs are handled correctly.
    let accrued_ccy =
        helpers::compute_accrued_interest_with_context(&bond_for_metrics, curves, as_of)?;

    let notional = bond_for_metrics.notional.amount();
    if notional == 0.0 {
        return Ok(BondQuoteSet {
            clean_price_ccy: 0.0,
            clean_price_pct: 0.0,
            dirty_price_ccy: 0.0,
            ytm: None,
            ytw: None,
            z_spread: None,
            discount_margin: None,
            oas: None,
            asw_par: None,
            asw_market: None,
            i_spread: None,
        });
    }

    // 1) Normalize the input into canonical dirty + clean prices.
    let (clean_price_pct, clean_price_ccy, dirty_price_ccy) = match quote_input {
        BondQuoteInput::CleanPricePct(clean_pct) => {
            let clean_ccy = clean_pct * notional / 100.0;
            let dirty_ccy = clean_ccy + accrued_ccy;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::DirtyPriceCcy(dirty_ccy) => {
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::Ytm(ytm) => {
            // Use standard holder-view flows and price_from_ytm helper.
            let flows =
                <Bond as CashflowProvider>::build_schedule(&bond_for_metrics, curves, as_of)?;
            let dirty_ccy = helpers::price_from_ytm(&bond_for_metrics, &flows, as_of, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ZSpread(z) => {
            let dirty_ccy = helpers::price_from_z_spread(&bond_for_metrics, curves, as_of, z)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::DiscountMargin(dm) => {
            let dirty_ccy = helpers::price_from_dm(&bond_for_metrics, curves, as_of, dm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::Oas(oas_decimal) => {
            let dirty_ccy = helpers::price_from_oas(&bond_for_metrics, curves, as_of, oas_decimal)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::AswMarket(asw_mkt) => {
            let dirty_ccy = price_from_asw_market(&bond_for_metrics, curves, as_of, asw_mkt)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
        BondQuoteInput::ISpread(i_spread) => {
            // I-spread = YTM - par_swap_rate → YTM = ISpread + par_swap_rate.
            let par_swap_rate = par_swap_rate_from_discount(bond, curves, as_of)?;
            let ytm = i_spread + par_swap_rate;
            let flows =
                <Bond as CashflowProvider>::build_schedule(&bond_for_metrics, curves, as_of)?;
            let dirty_ccy = helpers::price_from_ytm(&bond_for_metrics, &flows, as_of, ytm)?;
            let clean_ccy = dirty_ccy - accrued_ccy;
            let clean_pct = clean_ccy / notional * 100.0;
            (clean_pct, clean_ccy, dirty_ccy)
        }
    };

    // Stamp the canonical clean price quote into pricing_overrides so that all
    // existing metric calculators interpret this as the market price.
    bond_for_metrics
        .pricing_overrides
        .quoted_clean_price = Some(clean_price_pct);

    // 2) Build metric context and use the standard registry for the rest.
    let base_value = bond_for_metrics.value(curves, as_of)?;
    let registry: MetricRegistry = standard_registry();

    let instrument_arc: Arc<dyn Instrument> = Arc::new(bond_for_metrics.clone());
    let curves_arc = Arc::new(curves.clone());
    let mut ctx = MetricContext::new(instrument_arc, curves_arc, as_of, base_value);
    ctx.notional = Some(bond_for_metrics.notional);

    // Pre-populate accrued since we've already computed it.
    ctx.computed.insert(MetricId::Accrued, accrued_ccy);

    // Request the core price/yield/spread metrics.
    let metric_ids = [
        MetricId::Ytm,
        MetricId::Ytw,
        MetricId::ZSpread,
        MetricId::DiscountMargin,
        MetricId::Oas,
        MetricId::ASWPar,
        MetricId::ASWMarket,
        MetricId::ISpread,
    ];

    // We don't actually care about the HashMap return; we just want
    // the side-effect of populating `ctx.computed`.
    let _ = registry.compute(&metric_ids, &mut ctx)?;

    // Read back the metrics we care about.
    let ytm = ctx.computed.get(&MetricId::Ytm).copied();
    let ytw = ctx.computed.get(&MetricId::Ytw).copied();
    let z_spread = ctx.computed.get(&MetricId::ZSpread).copied();
    let discount_margin = ctx.computed.get(&MetricId::DiscountMargin).copied();
    let oas = ctx.computed.get(&MetricId::Oas).copied();
    let asw_par = ctx.computed.get(&MetricId::ASWPar).copied();
    let asw_market = ctx.computed.get(&MetricId::ASWMarket).copied();
    let i_spread = ctx.computed.get(&MetricId::ISpread).copied();

    Ok(BondQuoteSet {
        clean_price_ccy,
        clean_price_pct,
        dirty_price_ccy,
        ytm,
        ytw,
        z_spread,
        discount_margin,
        oas,
        asw_par,
        asw_market,
        i_spread,
    })
}

/// Compute the par swap fixed rate used in the I-Spread definition
/// (`ISpread = YTM - par_swap_rate`) using the same convention as the
/// `ISpreadCalculator` (annual Act/Act proxy fixed leg by default).
fn par_swap_rate_from_discount(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
) -> Result<f64> {
    use finstack_core::dates::{BusinessDayConvention, DayCount, DayCountCtx, Frequency, StubKind};

    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;

    // Mirror the schedule used in ISpreadCalculator (annual Act/Act, ShortFront stub).
    let dates = crate::instruments::bond::pricing::schedule_helpers::build_bond_schedule(
        as_of,
        bond.maturity,
        Frequency::annual(),
        StubKind::ShortFront,
        BusinessDayConvention::Following,
        None,
    );
    if dates.len() < 2 {
        return Ok(0.0);
    }

    let p0 = disc.df_on_date_curve(dates[0]);
    let pn = disc.df_on_date_curve(
        *dates
            .last()
            .expect("Dates should not be empty"),
    );
    let num = p0 - pn;
    let mut den = 0.0;
    for w in dates.windows(2) {
        let (a, b) = (w[0], w[1]);
        let alpha =
            DayCount::ActAct.year_fraction(a, b, DayCountCtx::default())?;
        let p = disc.df_on_date_curve(b);
        den += alpha * p;
    }
    if den == 0.0 {
        return Ok(0.0);
    }
    Ok(num / den)
}

/// Price from market asset swap spread (decimal) using the same
/// approximation as `AssetSwapMarketCalculator` for non-custom,
/// fixed-rate bonds:
///
/// `ASW_mkt = (coupon - par_rate) + (price_pct - 1.0) / annuity`
///
/// where `price_pct = dirty / notional`. Inverting:
///
/// `price_pct = 1.0 + (ASW_mkt - (coupon - par_rate)) * annuity`.
fn price_from_asw_market(
    bond: &Bond,
    curves: &MarketContext,
    as_of: Date,
    asw_market: f64,
) -> Result<f64> {
    use crate::instruments::bond::CashflowSpec;
    use crate::instruments::bond::pricing::schedule_helpers;
    use finstack_core::dates::DayCountCtx;

    // Only well-defined for fixed-rate, non-custom bonds in this helper.
    if bond.custom_cashflows.is_some() {
        return Err(finstack_core::error::InputError::Invalid.into());
    }
    let (coupon, freq, stub, bdc, calendar_id) = match &bond.cashflow_spec {
        CashflowSpec::Fixed(spec) => (spec.rate, spec.freq, spec.stub, spec.bdc, spec.calendar_id.as_deref()),
        _ => return Err(finstack_core::error::InputError::Invalid.into()),
    };

    let disc = curves.get_discount_ref(&bond.discount_curve_id)?;

    // Mirror the schedule and annuity definition used by AssetSwapMarketCalculator
    // (discount-ratio approximation on the fixed-leg schedule).
    let sched = schedule_helpers::build_bond_schedule(
        as_of,
        bond.maturity,
        freq,
        stub,
        bdc,
        calendar_id,
    );
    if sched.len() < 2 {
        return Ok(0.0);
    }

    let dc = bond.cashflow_spec.day_count();
    let mut ann = 0.0;
    let mut prev = sched[0];
    for &d in &sched[1..] {
        let alpha = dc.year_fraction(prev, d, DayCountCtx::default())?;
        let p = disc.df_on_date_curve(d);
        ann += alpha * p;
        prev = d;
    }
    if ann == 0.0 || bond.notional.amount() == 0.0 {
        return Ok(0.0);
    }

    let p0 = disc.df_on_date_curve(sched[0]);
    let pn = disc.df_on_date_curve(
        *sched
            .last()
            .expect("Schedule should not be empty"),
    );
    let par_rate = (p0 - pn) / ann;

    let par_asw = coupon - par_rate;
    let price_pct = 1.0 + (asw_market - par_asw) * ann;
    Ok(price_pct * bond.notional.amount())
}


