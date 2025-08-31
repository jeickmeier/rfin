//! Bond-specific metric calculators.
//!
//! Provides comprehensive metric calculators for fixed-rate bonds including
//! yield to maturity, duration, convexity, accrued interest, and credit spreads.
//! These metrics are essential for bond valuation, risk management, and
//! portfolio analysis.

use super::helpers::{df_from_yield, periods_per_year, YieldCompounding};
use super::oas_pricer::OASCalculator;
use crate::cashflow::primitives::CFKind;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::Bond;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::prelude::*;
use finstack_core::F;

/// Calculates accrued interest for bonds.
///
/// Computes the accrued interest since the last coupon payment up to the
/// valuation date. This is essential for determining the dirty price and
/// other bond metrics that depend on accrued interest.
///
/// See unit tests and `examples/` for usage.
pub struct AccruedInterestCalculator;

impl MetricCalculator for AccruedInterestCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Scope the bond borrow to avoid conflicts with &mut context later
        let (last, next, period_coupon_amount, disc_id, dc, maybe_flows) = {
            let bond: &Bond = context.instrument_as()?;

            // Determine coupon periods from actual schedule when available
            let (last, next, period_coupon_amount) = if let Some(ref custom) = bond.custom_cashflows
            {
                // Use coupon flows (Fixed/Stub) from custom schedule
                let mut coupon_dates: Vec<(
                    finstack_core::dates::Date,
                    finstack_core::money::Money,
                )> = Vec::new();
                for cf in &custom.flows {
                    if cf.kind == CFKind::Fixed || cf.kind == CFKind::Stub {
                        coupon_dates.push((cf.date, cf.amount));
                    }
                }
                if coupon_dates.len() < 2 {
                    return Ok(0.0);
                }
                // Find window around as_of and the coupon amount on the next date
                let mut found = None;
                for w in coupon_dates.windows(2) {
                    let (a, _a_amt) = w[0];
                    let (b, b_amt) = w[1];
                    if a <= context.as_of && context.as_of < b {
                        found = Some((a, b, b_amt));
                        break;
                    }
                }
                match found {
                    Some((a, b, amt)) => (a, b, amt),
                    None => return Ok(0.0),
                }
            } else {
                // Fallback to canonical schedule using bond fields
                let sched = crate::cashflow::builder::build_dates(
                    bond.issue,
                    bond.maturity,
                    bond.freq,
                    finstack_core::dates::StubKind::None,
                    finstack_core::dates::BusinessDayConvention::Following,
                    None,
                );
                let dates = sched.dates;
                if dates.len() < 2 {
                    return Ok(0.0);
                }
                let mut last = dates[0];
                let mut next = dates[1];
                let mut found = false;
                for w in dates.windows(2) {
                    let (a, b) = (w[0], w[1]);
                    if a <= context.as_of && context.as_of < b {
                        last = a;
                        next = b;
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(0.0);
                }
                // Period coupon amount based on notional × rate × yf
                let yf = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
                    last,
                    next,
                    bond.dc,
                );
                let coupon_amt = bond.notional * (bond.coupon * yf);
                (last, next, coupon_amt)
            };

            // Prepare potential flows for caching (build now, assign later)
            let maybe_flows = if context.cashflows.is_none() {
                Some(bond.build_schedule(&context.curves, context.as_of)?)
            } else {
                None
            };

            (
                last,
                next,
                period_coupon_amount,
                bond.disc_id,
                bond.dc,
                maybe_flows,
            )
        };

        // Calculate accrued interest linearly within the coupon period
        use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
        let yf_total = DiscountCurve::year_fraction(last, next, dc);
        if yf_total <= 0.0 {
            return Ok(0.0);
        }
        let elapsed = DiscountCurve::year_fraction(last, context.as_of, dc).max(0.0);
        let accrued = period_coupon_amount * (elapsed / yf_total);

        // Cache basic context hints for downstream metrics
        context.discount_curve_id = Some(disc_id);
        context.day_count = Some(dc);
        // Also cache full holder cashflows for downstream risk metrics
        if context.cashflows.is_none() {
            if let Some(flows) = maybe_flows {
                context.cashflows = Some(flows);
            }
        }

        Ok(accrued.amount())
    }
}

/// Calculates yield to maturity for bonds.
///
/// Computes the internal rate of return that equates the present value of
/// all future cashflows to the current market price. This is a fundamental
/// metric for bond valuation and comparison across different bonds.
///
/// # Dependencies
/// Requires `Accrued` metric to be computed first.
///
/// See unit tests and `examples/` for usage.
pub struct YtmCalculator;

impl MetricCalculator for YtmCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Extract fields we need from the bond without cloning it
        let (clean_px, currency, dc, disc_id, notional, coupon, freq, built_flows) = {
            let bond: &Bond = context.instrument_as()?;

            let built_flows = if context.cashflows.is_none() {
                Some(bond.build_schedule(&context.curves, context.as_of)?)
            } else {
                None
            };

            (
                bond.quoted_clean.ok_or_else(|| {
                    finstack_core::Error::from(finstack_core::error::InputError::NotFound)
                })?,
                bond.notional.currency(),
                bond.dc,
                bond.disc_id,
                bond.notional,
                bond.coupon,
                bond.freq,
                built_flows,
            )
        };

        // Get accrued from computed metrics
        let ai = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Compute dirty price
        let dirty_amt = clean_px + ai;
        let dirty = Money::new(dirty_amt, currency);

        // Cache flows and hints if we built them
        if context.cashflows.is_none() {
            if let Some(flows) = &built_flows {
                context.cashflows = Some(flows.clone());
            }
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
        }

        let flows: Vec<(Date, Money)> = if let Some(f) = &context.cashflows {
            f.clone()
        } else {
            // Should not happen, but fallback to empty
            built_flows.unwrap_or_default()
        };

        // Solve for YTM using shared solver with Street compounding (default)
        let ytm = super::ytm_solver::solve_ytm(
            &flows,
            context.as_of,
            dirty,
            super::ytm_solver::YtmPricingSpec {
                day_count: dc,
                notional,
                coupon_rate: coupon,
                compounding: YieldCompounding::Street,
                frequency: freq,
            },
        )?;

        Ok(ytm)
    }
}

impl YtmCalculator {}

/// Calculates Macaulay duration for bonds.
pub struct MacaulayDurationCalculator;

impl MetricCalculator for MacaulayDurationCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Build or reuse flows without cloning the instrument
        let flows: Vec<(Date, Money)> = if let Some(f) = &context.cashflows {
            f.clone()
        } else {
            let bond: &Bond = context.instrument_as()?;
            let disc_id = bond.disc_id;
            let dc = bond.dc;
            let built_flows = bond.build_schedule(&context.curves, context.as_of)?;
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
            context.cashflows = Some(built_flows.clone());
            built_flows
        };

        // Calculate price from flows to ensure consistency
        let price = {
            let bond: &Bond = context.instrument_as()?;
            super::helpers::price_from_ytm(bond, &flows, context.as_of, ytm)?
        };
        if price == 0.0 {
            return Ok(0.0);
        }

        // Calculate Macaulay duration
        let mut weighted_time = 0.0;

        {
            let bond: &Bond = context.instrument_as()?;
            for &(date, amount) in &flows {
                if date <= context.as_of {
                    continue;
                }
                let t = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(context.as_of, date, bond.dc).max(0.0);
                let df = df_from_yield(ytm, t, YieldCompounding::Street, bond.freq).unwrap_or(0.0);
                weighted_time += t * amount.amount() * df;
            }
        }

        Ok(weighted_time / price)
    }
}

impl MacaulayDurationCalculator {}

/// Calculates modified duration for bonds.
pub struct ModifiedDurationCalculator;

impl MetricCalculator for ModifiedDurationCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::DurationMac]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        let d_mac = context
            .computed
            .get(&MetricId::DurationMac)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Modified duration depends on compounding; default to Street (periodic with bond freq)
        let m = periods_per_year(bond.freq).unwrap_or(1.0).max(1.0);
        Ok(d_mac / (1.0 + ytm / m))
    }
}

/// Calculates convexity for bonds.
pub struct ConvexityCalculator;

impl MetricCalculator for ConvexityCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let ytm = context
            .computed
            .get(&MetricId::Ytm)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Build or reuse flows
        let flows: Vec<(Date, Money)> = if let Some(f) = &context.cashflows {
            f.clone()
        } else {
            let bond: &Bond = context.instrument_as()?;
            let disc_id = bond.disc_id;
            let dc = bond.dc;
            let built = bond.build_schedule(&context.curves, context.as_of)?;
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
            context.cashflows = Some(built.clone());
            built
        };

        let dy = 1e-4; // 1 basis point for numerical differentiation

        // Calculate prices with yield bumps for numerical convexity
        let (p0, p_up, p_dn) = {
            let bond: &Bond = context.instrument_as()?;
            let p0 = super::helpers::price_from_ytm(bond, &flows, context.as_of, ytm)?;
            let p_up = super::helpers::price_from_ytm(bond, &flows, context.as_of, ytm + dy)?;
            let p_dn = super::helpers::price_from_ytm(bond, &flows, context.as_of, ytm - dy)?;
            (p0, p_up, p_dn)
        };

        if p0 == 0.0 || dy == 0.0 {
            return Ok(0.0);
        }

        // Convexity = (P+ + P- - 2*P0) / (P0 * dy^2)
        Ok((p_up + p_dn - 2.0 * p0) / (p0 * dy * dy))
    }
}

impl ConvexityCalculator {}

/// Calculates yield-to-worst for bonds with call/put schedules.
pub struct YtwCalculator;

impl MetricCalculator for YtwCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        // Build or reuse flows; also assemble candidates while borrowing bond
        let (flows, candidates) = {
            let bond: &Bond = context.instrument_as()?;

            let flows = if let Some(f) = &context.cashflows {
                f.clone()
            } else {
                bond.build_schedule(&context.curves, context.as_of)?
            };

            // Build candidate exercise dates
            let mut candidates: Vec<(Date, Money)> = Vec::new();
            if let Some(cp) = &bond.call_put {
                for c in &cp.calls {
                    if c.date >= context.as_of && c.date <= bond.maturity {
                        let redemption = bond.notional * (c.price_pct_of_par / 100.0);
                        candidates.push((c.date, redemption));
                    }
                }
                for p in &cp.puts {
                    if p.date >= context.as_of && p.date <= bond.maturity {
                        let redemption = bond.notional * (p.price_pct_of_par / 100.0);
                        candidates.push((p.date, redemption));
                    }
                }
            }
            // Always include maturity
            candidates.push((bond.maturity, bond.notional));

            (flows, candidates)
        };

        // Cache flows and hints if not already cached
        if context.cashflows.is_none() {
            // Re-borrow to access fields for hints
            let bond: &Bond = context.instrument_as()?;
            let disc_id = bond.disc_id;
            let dc = bond.dc;
            context.cashflows = Some(flows.clone());
            context.discount_curve_id = Some(disc_id);
            context.day_count = Some(dc);
        }

        // Get current dirty price from PV
        let dirty_now = context.base_value;

        // Find worst yield
        let mut best_ytm = f64::INFINITY;
        for (exercise_date, redemption) in candidates {
            let y = {
                let bond: &Bond = context.instrument_as()?;
                self.solve_ytm_with_exercise(
                    bond,
                    &flows,
                    context.as_of,
                    dirty_now,
                    exercise_date,
                    redemption,
                )?
            };

            if y < best_ytm {
                best_ytm = y;
            }
        }

        Ok(best_ytm)
    }
}

impl YtwCalculator {
    fn solve_ytm_with_exercise(
        &self,
        bond: &Bond,
        flows: &[(Date, Money)],
        as_of: Date,
        target_price: Money,
        exercise_date: Date,
        redemption: Money,
    ) -> finstack_core::Result<F> {
        // Build truncated flows up to exercise plus redemption and reuse solver
        let mut ex_flows: Vec<(Date, Money)> = Vec::with_capacity(flows.len());
        for &(date, amount) in flows {
            if date <= as_of || date > exercise_date {
                continue;
            }
            ex_flows.push((date, amount));
        }
        ex_flows.push((exercise_date, redemption));

        super::ytm_solver::solve_ytm(
            &ex_flows,
            as_of,
            target_price,
            super::ytm_solver::YtmPricingSpec {
                day_count: bond.dc,
                notional: bond.notional,
                coupon_rate: bond.coupon,
                compounding: YieldCompounding::Street,
                frequency: bond.freq,
            },
        )
    }
}

/// Calculates dirty price for bonds (clean price + accrued interest).
pub struct DirtyPriceCalculator;

impl MetricCalculator for DirtyPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        // Dirty price only makes sense if we have a quoted clean price
        let clean_px = bond.quoted_clean.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound)
        })?;

        // Get accrued from computed metrics
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Dirty price = clean price + accrued interest
        Ok(clean_px + accrued)
    }
}

/// Calculates clean price for bonds (dirty price - accrued interest).
pub struct CleanPriceCalculator;

impl MetricCalculator for CleanPriceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        // If we have quoted clean price, just return it
        if let Some(clean_px) = bond.quoted_clean {
            return Ok(clean_px);
        }

        // Otherwise calculate from base value (which should be dirty price)
        let dirty_px = context.base_value.amount();
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;

        // Clean price = dirty price - accrued interest
        Ok(dirty_px - accrued)
    }
}

/// Calculates CS01 (credit spread sensitivity) for bonds.
///
/// CS01 represents the price change for a 1 basis point parallel shift in credit spreads.
/// This implementation uses the bond's yield spread as a proxy for credit spread.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Ytm]
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;

        // Build cashflow schedule from Bond
        let flows = bond.build_schedule(&context.curves, context.as_of)?;

        // Get the base discount curve
        let disc_curve = context.curves.discount(bond.disc_id)?;

        // CS01 calculation using spread approximation
        let bp = 0.0001; // 1 basis point

        // Approximate CS01 by shifting the discount rates
        // This simulates a parallel credit spread shift
        let mut npv_up = 0.0;
        let mut npv_down = 0.0;

        for (date, amount) in &flows {
            if *date > context.as_of {
                let yf = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::year_fraction(
                    disc_curve.base_date(), *date, bond.dc
                );
                let df = disc_curve.df(yf);

                // Apply spread bumps to the discount factor
                // df_spread = df * exp(-spread * t)
                let df_up = df * (-bp * yf).exp();
                let df_down = df * (bp * yf).exp();

                npv_up += amount.amount() * df_up;
                npv_down += amount.amount() * df_down;
            }
        }

        // CS01 = (price with spread down - price with spread up) / 2
        // Scaled to per unit notional
        let cs01 = (npv_down - npv_up) / 2.0 / bond.notional.amount();

        Ok(cs01 * 10000.0) // Return in price per 100 notional terms
    }
}

impl Cs01Calculator {}

/// Calculates Option-Adjusted Spread for bonds with embedded options.
///
/// Uses short-rate trees to value callable/putable bonds and solve for the
/// spread that makes the model price equal to the market price.
pub struct OasCalculator;

impl MetricCalculator for OasCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[MetricId::Accrued] // Need accrued to calculate dirty target price
    }

    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<F> {
        let bond: &Bond = context.instrument_as()?;
        
        // Require quoted clean price
        let clean_price = bond.quoted_clean.ok_or_else(|| {
            finstack_core::Error::from(finstack_core::error::InputError::NotFound)
        })?;
        
        // Get accrued interest from computed metrics
        let accrued = context
            .computed
            .get(&MetricId::Accrued)
            .copied()
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound)
            })?;
            
        // Use MarketContext directly (no conversion needed)
        let market_context = context.curves.as_ref().clone();
        
        // Use OAS calculator to solve for OAS
        let oas_calculator = OASCalculator::new();
        let dirty_price = clean_price + accrued;
        
        oas_calculator.calculate_oas(bond, &market_context, context.as_of, dirty_price)
    }
}

/// Registers all bond metrics to a registry.
///
/// This function adds all bond-specific metrics to the provided metric
/// registry. Each metric is registered with the "Bond" instrument type
/// to ensure proper applicability filtering.
///
/// # Arguments
/// * `registry` - Metric registry to add bond metrics to
///
/// See unit tests and `examples/` for usage.
pub fn register_bond_metrics(registry: &mut crate::metrics::MetricRegistry) {
    use crate::metrics::MetricId;
    use std::sync::Arc;

    registry
        .register_metric(
            MetricId::Accrued,
            Arc::new(AccruedInterestCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::DirtyPrice,
            Arc::new(DirtyPriceCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::CleanPrice,
            Arc::new(CleanPriceCalculator),
            &["Bond"],
        )
        .register_metric(MetricId::Ytm, Arc::new(YtmCalculator), &["Bond"])
        .register_metric(
            MetricId::DurationMac,
            Arc::new(MacaulayDurationCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::DurationMod,
            Arc::new(ModifiedDurationCalculator),
            &["Bond"],
        )
        .register_metric(
            MetricId::Convexity,
            Arc::new(ConvexityCalculator),
            &["Bond"],
        )
        .register_metric(MetricId::Ytw, Arc::new(YtwCalculator), &["Bond"])
        .register_metric(MetricId::Oas, Arc::new(OasCalculator), &["Bond"])
        .register_metric(MetricId::Cs01, Arc::new(Cs01Calculator), &["Bond"]);
}
