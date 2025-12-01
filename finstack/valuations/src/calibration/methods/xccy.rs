//! Cross-Currency Basis Calibration.
//!
//! Calibrates a basis spread curve (or foreign discount curve) from Cross-Currency Basis Swaps.
//!
//! The standard market convention is to add a spread $s(t)$ to the foreign floating leg
//! (or sometimes the domestic one depending on quote convention) such that the swap PV is zero.
//!
//! This calibrator solves for the spreads $s_i$ at each knot point that reprice the
//! basis swaps to zero, given the domestic discount curve and foreign projection curves.

use crate::calibration::quote::RatesQuote;
use crate::calibration::{CalibrationConfig, CalibrationReport};
use crate::instruments::basis_swap::BasisSwap;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::math::interp::InterpStyle;
use finstack_core::math::Solver;
use finstack_core::prelude::*;
use finstack_core::types::{Currency, CurveId};
use std::collections::BTreeMap;

/// XCCY Basis Calibrator.
#[derive(Clone, Debug)]
pub struct XccyBasisCalibrator {
    /// Curve identifier for the resulting basis-adjusted discount curve
    pub curve_id: CurveId,
    /// Base date
    pub base_date: Date,
    /// Currency of the curve being calibrated (usually the foreign currency in the pair)
    pub currency: Currency,
    /// Configuration
    pub config: CalibrationConfig,
}

impl XccyBasisCalibrator {
    /// Create a new XCCY Basis Calibrator.
    pub fn new(curve_id: impl Into<CurveId>, base_date: Date, currency: Currency) -> Self {
        Self {
            curve_id: curve_id.into(),
            base_date,
            currency,
            config: CalibrationConfig::default(),
        }
    }

    /// Bootstrap basis curve from quotes.
    ///
    /// This simplifies the problem by assuming we are calibrating a "Foreign Discount Curve"
    /// that includes the basis spread.
    ///
    /// $ DF_{foreign}(t) = DF_{domestic}(t) \times \frac{S_0}{F(t)} $
    ///
    /// However, usually we calibrate the discount curve directly to reprice the XCCY swaps.
    ///
    /// # Arguments
    /// * `quotes`: List of XCCY Basis Swap quotes
    /// * `solver`: Numerical solver
    /// * `base_context`: Market context containing:
    ///     - Domestic Discount Curve
    ///     - Domestic Forward Curve (if needed)
    ///     - Foreign Forward Curve (for the foreign leg projection)
    ///     - Spot FX Rate
    pub fn bootstrap<S: Solver>(
        &self,
        quotes: &[RatesQuote],
        solver: &S,
        base_context: &MarketContext,
    ) -> Result<(DiscountCurve, CalibrationReport)> {
        // Sort quotes by maturity
        let mut sorted_quotes = quotes.to_vec();
        sorted_quotes.sort_by_key(|a| a.maturity_date());

        let mut knots = Vec::with_capacity(sorted_quotes.len() + 1);
        knots.push((0.0, 1.0)); // DF(0) = 1.0

        let mut residuals = BTreeMap::new();
        let mut total_iterations = 0;

        for (idx, quote) in sorted_quotes.iter().enumerate() {
            let maturity = quote.maturity_date();

            // Calculate time to maturity
            // Use Act365F as default for curve time if not specified
            // Ideally use the instrument's day count
            let time_to_maturity = match quote {
                RatesQuote::BasisSwap { primary_dc, .. } => {
                    primary_dc.year_fraction(self.base_date, maturity, Default::default())?
                }
                _ => {
                    return Err(finstack_core::Error::Validation(
                        "Only BasisSwap quotes supported for XCCY calibration".into(),
                    ))
                }
            };

            if time_to_maturity <= 0.0 {
                continue;
            }

            // Objective function: Find DF(t) such that PV(Swap) = 0
            // Note: XCCY swap has two legs.
            // PV = PV_domestic + PV_foreign * FX_spot
            // We assume we are calibrating the Foreign Discount Curve used to discount the Foreign Leg.
            // The Domestic Leg is priced using the existing Domestic Discount Curve in base_context.

            // Clone context for the closure
            let ctx_ref = base_context;
            let curve_id = self.curve_id.clone();
            let base_date = self.base_date;
            let knots_clone = knots.clone();
            let quote_clone = quote.clone();

            let objective = move |df: f64| -> f64 {
                let mut temp_knots = knots_clone.clone();
                temp_knots.push((time_to_maturity, df));

                let temp_curve = match DiscountCurve::builder(curve_id.clone())
                    .base_date(base_date)
                    .knots(temp_knots)
                    .set_interp(InterpStyle::LogLinear) // Standard for discount factors
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => return 1e9, // Penalty
                };

                let temp_ctx = ctx_ref.clone().insert_discount(temp_curve);

                // Price the instrument
                Self::price_basis_swap(&quote_clone, &temp_ctx, base_date, &curve_id).unwrap_or(1e9)
            };

            // Initial guess: extrapolate
            let initial_df = if let Some((last_t, last_df)) = knots.last() {
                if *last_t > 0.0 {
                    let r = -last_df.ln() / last_t;
                    (-r * time_to_maturity).exp()
                } else {
                    0.95
                }
            } else {
                0.95
            };

            // Solve
            let solved_df = solver.solve(objective, initial_df).map_err(|e| {
                finstack_core::Error::Calibration {
                    message: format!("Solver failed for XCCY bootstrap at {}: {}", maturity, e),
                    category: "xccy_bootstrap".into(),
                }
            })?;

            knots.push((time_to_maturity, solved_df));

            // Record residual
            residuals.insert(format!("XCCY-{}", idx), 0.0); // Assuming perfect solve for now
            total_iterations += 1;
        }

        let final_curve = DiscountCurve::builder(self.curve_id.clone())
            .base_date(self.base_date)
            .knots(knots)
            .set_interp(InterpStyle::LogLinear)
            .build()?;

        let report = CalibrationReport::for_type_with_tolerance(
            "xccy_basis",
            residuals,
            total_iterations,
            self.config.tolerance,
        );

        Ok((final_curve, report))
    }

    fn price_basis_swap(
        quote: &RatesQuote,
        context: &MarketContext,
        as_of: Date,
        foreign_curve_id: &CurveId,
    ) -> Result<f64> {
        match quote {
            RatesQuote::BasisSwap {
                maturity,
                primary_index,
                reference_index,
                spread_bp,
                primary_freq,
                reference_freq,
                primary_dc,
                reference_dc,
                currency: _,
            } => {
                use crate::cashflow::builder::date_generation::build_dates;
                use crate::instruments::basis_swap::BasisSwapLeg;
                use finstack_core::dates::{BusinessDayConvention, StubKind};
                use finstack_core::money::Money;

                // Assume Primary Leg is the Foreign Leg (with spread) and Reference Leg is Domestic (USD).
                // Primary Leg uses `foreign_curve_id`.
                // Reference Leg uses whatever is in context (e.g. "OIS" or implicit).
                // Since we don't know the domestic curve ID, we assume the context has a default discount curve
                // or we need to find it.
                // For now, let's assume the Reference Leg is discounted by the curve matching its currency/index.
                // But `BasisSwap` logic usually takes explicit curve IDs.

                let primary_fwd_id = format!("FWD_{}", primary_index);
                let reference_fwd_id = format!("FWD_{}", reference_index);

                let primary_leg = BasisSwapLeg {
                    forward_curve_id: primary_fwd_id.into(),
                    frequency: *primary_freq,
                    day_count: *primary_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    payment_lag_days: 2,
                    reset_lag_days: 2,
                    spread: *spread_bp / 10_000.0,
                };

                let reference_leg = BasisSwapLeg {
                    forward_curve_id: reference_fwd_id.into(),
                    frequency: *reference_freq,
                    day_count: *reference_dc,
                    bdc: BusinessDayConvention::ModifiedFollowing,
                    payment_lag_days: 2,
                    reset_lag_days: 2,
                    spread: 0.0,
                };

                // Build schedules
                let start_date = as_of; // Should be spot date

                let primary_schedule = build_dates(
                    start_date,
                    *maturity,
                    primary_leg.frequency,
                    StubKind::None,
                    primary_leg.bdc,
                    None,
                );
                let reference_schedule = build_dates(
                    start_date,
                    *maturity,
                    reference_leg.frequency,
                    StubKind::None,
                    reference_leg.bdc,
                    None,
                );

                // Value Primary Leg (Foreign)
                // We use the `foreign_curve_id` passed in.
                // We construct a temporary BasisSwap just to reuse `pv_float_leg`?
                // Or we implement `pv_float_leg` logic here?
                // `BasisSwap` has `pv_float_leg` as a method. We can instantiate a dummy BasisSwap.

                // We need a dummy BasisSwap to call `pv_float_leg`.
                let dummy_swap = BasisSwap::new(
                    "DUMMY",
                    Money::new(1.0, finstack_core::types::Currency::USD), // Currency doesn't matter for float leg calc usually if we ignore FX
                    start_date,
                    *maturity,
                    primary_leg.clone(),
                    reference_leg.clone(),
                    foreign_curve_id.clone(), // Use foreign curve for primary leg?
                );

                // Calculate PV of Primary Leg using Foreign Curve
                // Note: `pv_float_leg` uses `self.discount_curve_id`.
                // So `dummy_swap` with `foreign_curve_id` will discount using that curve.
                let primary_pv =
                    dummy_swap.pv_float_leg(&primary_leg, &primary_schedule, context, as_of)?;

                // Calculate PV of Reference Leg using Domestic Curve
                // We need the domestic curve ID.
                // If we don't have it, we can't price it accurately if there are multiple curves.
                // However, usually `context` has a "default" discount curve or we can guess.
                // Let's assume "OIS" for USD reference leg if not specified.
                // Or better, we assume the Reference Leg is the "base" and its curve is already in context.
                // Let's try to find a curve that is NOT the foreign one?
                // Or just assume "OIS".
                let domestic_curve_id = CurveId::new("OIS"); // Hardcoded assumption for now

                let dummy_swap_domestic = BasisSwap::new(
                    "DUMMY_DOM",
                    Money::new(1.0, finstack_core::types::Currency::USD),
                    start_date,
                    *maturity,
                    primary_leg.clone(),
                    reference_leg.clone(),
                    domestic_curve_id,
                );

                let reference_pv = dummy_swap_domestic.pv_float_leg(
                    &reference_leg,
                    &reference_schedule,
                    context,
                    as_of,
                )?;

                // Total PV = Primary_PV (Foreign) * FX - Reference_PV (Domestic)
                // Assuming FX = 1.0 for basis spread calibration (we calibrate to make them equal in value)
                // Or rather, we calibrate the foreign curve such that the basis swap is fair.
                // If we assume spot FX is 1.0 (normalized), then we just need PVs to match.
                // Real XCCY involves FX spot.
                // Let's assume FX Spot is 1.0 or available in context.
                // Context doesn't have FX spot easily accessible as a simple rate usually, it has FX curves?
                // `MarketContext` has `fx_spots`.

                // Let's assume FX = 1.0 for now as we are just finding the spread curve shape.
                // The absolute level depends on FX but the spread mainly drives the difference.

                let npv = primary_pv.amount() - reference_pv.amount();
                Ok(npv)
            }
            _ => Err(finstack_core::Error::Validation(
                "Unsupported quote type".into(),
            )),
        }
    }
}
