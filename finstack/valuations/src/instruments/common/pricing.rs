//! Common pricing patterns and shared infrastructure.
//!
//! This module provides generic pricer implementations and shared pricing utilities
//! to eliminate duplication across instrument pricing modules.

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::market_data::context::MarketContext;
use finstack_core::types::CurveId;
use std::marker::PhantomData;

/// Generic pricer for any instrument that implements the Instrument trait.
///
/// This eliminates the need for instrument-specific pricer implementations that just
/// forward to the instrument's `value()` method.
pub struct GenericInstrumentPricer<I> {
    instrument_type: InstrumentType,
    model_key: ModelKey,
    _phantom: PhantomData<I>,
}

impl<I> GenericInstrumentPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    /// Create a new generic pricer for the specified instrument and model type.
    pub fn new(instrument_type: InstrumentType, model_key: ModelKey) -> Self {
        Self {
            instrument_type,
            model_key,
            _phantom: PhantomData,
        }
    }
}

impl<I> Pricer for GenericInstrumentPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    fn key(&self) -> PricerKey {
        PricerKey::new(self.instrument_type, self.model_key)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument = instrument
            .as_any()
            .downcast_ref::<I>()
            .ok_or_else(|| PricingError::type_mismatch(self.instrument_type, instrument.key()))?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Removed USD/EUR heuristic helper; as_of is derived from instrument's own curve

/// Trait for instruments with a primary discount curve.
///
/// This trait is used by generic pricers and metric calculators to extract
/// discount curve IDs. All instruments with a discount curve should implement this.
///
/// **Note**: This is primarily an internal helper trait. End-users typically
/// don't need to interact with it directly.
pub trait HasDiscountCurve {
    /// Get the instrument's primary discount curve ID.
    fn discount_curve_id(&self) -> &CurveId;
}

/// Trait for instruments that reference forward/projection curves.
///
/// This trait is used by generic DV01 calculators to identify all forward curves
/// that should be bumped alongside the discount curve for parallel rate shifts.
/// Instruments with floating rate legs (FRAs, swaps, floating bonds, etc.) should
/// implement this trait.
pub trait HasForwardCurves {
    /// Get all forward curve IDs referenced by this instrument.
    fn forward_curve_ids(&self) -> Vec<CurveId>;
}

/// Generic discounting pricer for instruments that can be valued via simple discounting.
///
/// This pricer derives the valuation date from the instrument's discount curve
/// and delegates PV calculation to the instrument's `value()` method.
/// It eliminates boilerplate across instrument pricers.
pub struct GenericDiscountingPricer<I> {
    instrument_type: InstrumentType,
    _phantom: PhantomData<I>,
}

impl<I> GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    /// Create a new generic discounting pricer for the specified instrument type.
    pub fn new(instrument_type: InstrumentType) -> Self {
        Self {
            instrument_type,
            _phantom: PhantomData,
        }
    }
}

impl<I> Pricer for GenericDiscountingPricer<I>
where
    I: Instrument + HasDiscountCurve + 'static,
{
    fn key(&self) -> PricerKey {
        PricerKey::new(self.instrument_type, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let typed_instrument = instrument
            .as_any()
            .downcast_ref::<I>()
            .ok_or_else(|| PricingError::type_mismatch(self.instrument_type, instrument.key()))?;

        // Compute present value using the instrument's unified value method
        let pv = typed_instrument
            .value(market, as_of)
            .map_err(|e| PricingError::model_failure(e.to_string()))?;

        // Return stamped result
        Ok(ValuationResult::stamped(typed_instrument.id(), as_of, pv))
    }
}

// Removed per-instrument constructors; use GenericDiscountingPricer::<I>::new()

// Special case for CDS which uses HazardRate model
impl GenericInstrumentPricer<crate::instruments::CreditDefaultSwap> {
    /// Create a CDS hazard rate pricer.
    pub fn cds() -> Self {
        Self::new(InstrumentType::CDS, ModelKey::HazardRate)
    }
}

// ============================================================================
// TRS Pricing Engine
// ============================================================================

use crate::instruments::common::parameters::legs::FinancingLegSpec;
use crate::instruments::common::parameters::trs_common::TrsScheduleSpec;
use finstack_core::dates::{Date, DayCountCtx};
use finstack_core::money::Money;

/// Parameters for total return leg calculation.
#[derive(Debug, Clone)]
pub struct TotalReturnLegParams<'a> {
    /// Schedule specification for payment periods.
    pub schedule: &'a TrsScheduleSpec,
    /// Notional amount for the leg.
    pub notional: Money,
    /// Discount curve identifier.
    pub discount_curve_id: &'a str,
    /// Contract size multiplier for the underlying.
    pub contract_size: f64,
    /// Initial level of the underlying (if known).
    pub initial_level: Option<f64>,
}

/// Trait for underlying-specific total return models.
///
/// Implementations of this trait provide the logic for calculating
/// total returns over a period for different underlying types (equity vs fixed income).
pub trait TrsReturnModel {
    /// Computes total return over a period given times from as_of and initial level.
    ///
    /// # Arguments
    /// * `period_start` — Start date of the period
    /// * `period_end` — End date of the period
    /// * `t_start` — Time from as_of to period start (year fraction)
    /// * `t_end` — Time from as_of to period end (year fraction)
    /// * `initial_level` — Initial level of the underlying
    /// * `context` — Market context for data access
    ///
    /// # Returns
    /// Total return as a decimal (e.g., 0.05 for 5% return).
    fn period_return(
        &self,
        period_start: Date,
        period_end: Date,
        t_start: f64,
        t_end: f64,
        initial_level: f64,
        context: &MarketContext,
    ) -> finstack_core::Result<f64>;
}

/// Common TRS pricing engine for shared calculations.
///
/// Provides utility functions for calculating present values of TRS legs
/// and other common pricing operations shared between equity and fixed income TRS.
pub struct TrsEngine;

impl TrsEngine {
    /// Calculates the present value of a total return leg using shared logic.
    ///
    /// This method contains the common period iteration and discounting logic,
    /// while delegating underlying-specific return calculations to the model.
    ///
    /// # Arguments
    /// * `params` — Parameters for the total return leg calculation
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    /// * `model` — Model implementing TrsReturnModel for underlying-specific logic
    ///
    /// # Returns
    /// Present value of the total return leg in the instrument's currency.
    pub fn pv_total_return_leg_with_model(
        params: TotalReturnLegParams,
        context: &MarketContext,
        as_of: Date,
        model: &impl TrsReturnModel,
    ) -> finstack_core::Result<Money> {
        // Get discount curve
        let disc = context.get_discount_ref(params.discount_curve_id)?;

        // Build schedule
        let period_schedule = params.schedule.period_schedule()?;

        let mut total_pv = 0.0;
        let currency = params.notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Time fractions
            let t_start = params
                .schedule
                .params
                .dc
                .year_fraction(as_of, period_start, ctx)?;
            let t_end = params
                .schedule
                .params
                .dc
                .year_fraction(as_of, period_end, ctx)?;

            // Calculate underlying return for this period (delegated to underlying-specific logic)
            let total_return = model.period_return(
                period_start,
                period_end,
                t_start,
                t_end,
                params.initial_level.unwrap_or(1.0),
                context,
            )?;

            // Payment amount
            let payment = params.notional.amount() * total_return * params.contract_size;

            // Discount to present
            let df = disc.df(t_end);
            total_pv += payment * df;
        }

        Ok(Money::new(total_pv, currency))
    }

    /// Calculates the present value of the financing leg.
    ///
    /// This is shared by both equity and fixed income TRS.
    ///
    /// # Arguments
    /// * `financing` — Financing leg specification
    /// * `schedule` — Schedule specification for payment periods
    /// * `notional` — Notional amount for the leg
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Present value of the financing leg in the instrument's currency.
    pub fn pv_financing_leg(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        // Get curves
        let disc_curve_id = financing.discount_curve_id.as_str();
        let fwd_curve_id = financing.forward_curve_id.as_str();

        let disc = context.get_discount_ref(disc_curve_id)?;
        let fwd = context.get_forward_ref(fwd_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule()?;

        let mut total_pv = 0.0;
        let currency = notional.currency();
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .params
                .dc
                .year_fraction(period_start, period_end, ctx)?;

            // Forward rate for the period
            let t_start = schedule.params.dc.year_fraction(as_of, period_start, ctx)?;
            let t_end = schedule.params.dc.year_fraction(as_of, period_end, ctx)?;
            let fwd_rate = fwd.rate_period(t_start, t_end);

            // Add spread
            let total_rate = fwd_rate + financing.spread_bp / 10000.0;

            // Payment amount
            let payment = notional.amount() * total_rate * yf;

            // Discount to present
            let df = disc.df(t_end);
            total_pv += payment * df;
        }

        Ok(Money::new(total_pv, currency))
    }

    /// Calculates the financing annuity for par spread calculation.
    ///
    /// # Arguments
    /// * `financing` — Financing leg specification
    /// * `schedule` — Schedule specification for payment periods
    /// * `notional` — Notional amount for the leg
    /// * `context` — Market context containing curves and market data
    /// * `as_of` — Valuation date
    ///
    /// # Returns
    /// Financing annuity (sum of discounted year fractions × notional).
    pub fn financing_annuity(
        financing: &FinancingLegSpec,
        schedule: &TrsScheduleSpec,
        notional: Money,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        // Get discount curve
        let disc_curve_id = financing.discount_curve_id.as_str();
        let disc = context.get_discount_ref(disc_curve_id)?;

        // Build schedule
        let period_schedule = schedule.period_schedule()?;

        let mut annuity = 0.0;
        let ctx = DayCountCtx::default();

        // Iterate through periods
        for i in 1..period_schedule.dates.len() {
            let period_start = period_schedule.dates[i - 1];
            let period_end = period_schedule.dates[i];

            // Year fraction for accrual
            let yf = schedule
                .params
                .dc
                .year_fraction(period_start, period_end, ctx)?;

            // Discount factor to payment date
            let t_pay = schedule.params.dc.year_fraction(as_of, period_end, ctx)?;
            let df = disc.df(t_pay);

            annuity += df * yf;
        }

        Ok(annuity * notional.amount())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_pricer_keys() {
        let bond_pricer =
            GenericDiscountingPricer::<crate::instruments::Bond>::new(InstrumentType::Bond);
        assert_eq!(
            bond_pricer.key(),
            PricerKey::new(InstrumentType::Bond, ModelKey::Discounting)
        );

        let deposit_pricer =
            GenericDiscountingPricer::<crate::instruments::Deposit>::new(InstrumentType::Deposit);
        assert_eq!(
            deposit_pricer.key(),
            PricerKey::new(InstrumentType::Deposit, ModelKey::Discounting)
        );
    }
}
