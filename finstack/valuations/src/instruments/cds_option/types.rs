//! CdsOption instrument: option on a CDS spread.
//!
//! This module defines the `CdsOption` data structure and integrates with the
//! common instrument trait via `impl_instrument!`. All pricing math and metrics
//! are implemented in the `pricing/` and `metrics/` submodules.

use crate::instruments::common::parameters::CreditParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::{Date, DayCount};
use finstack_core::money::Money;
use finstack_core::types::InstrumentId;
use finstack_core::F;

use super::parameters::CdsOptionParams;

/// Credit option instrument (option on CDS spread)
#[derive(Clone, Debug)]
pub struct CdsOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike spread in basis points
    pub strike_spread_bp: F,
    /// Option type (Call = right to buy protection, Put = right to sell protection)
    pub option_type: OptionType,
    /// Exercise style
    pub exercise_style: ExerciseStyle,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Day count convention for time calculations
    pub day_count: DayCount,
    /// Notional amount
    pub notional: Money,
    /// Settlement type
    pub settlement: SettlementType,
    /// Recovery rate assumption
    pub recovery_rate: F,
    /// Discount curve identifier
    pub disc_id: finstack_core::types::CurveId,
    /// Hazard curve identifier
    pub credit_id: finstack_core::types::CurveId,
    /// Volatility surface identifier
    pub vol_id: &'static str,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
    /// If true, the underlying is a CDS index; else single-name CDS
    pub underlying_is_index: bool,
    /// Optional index factor scaling for index underlying
    pub index_factor: Option<F>,
    /// Forward spread adjustment (bp) to apply for forward computation
    pub forward_spread_adjust_bp: F,
}

impl CdsOption {
    /// Create a new credit option using parameter structs.
    ///
    /// Inputs separation:
    /// - `option_params`: deal-level fields (strike in bp, expiry, CDS maturity, notional, option type)
    /// - `credit_params`: reference entity, recovery rate, and the hazard `credit_id`
    /// - `disc_id`: discount curve identifier for discounting cashflows
    /// - `vol_id`: volatility surface identifier for the CDS option
    ///
    /// Note: `credit_id` is sourced from `credit_params` to avoid duplication.
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &CdsOptionParams,
        credit_params: &CreditParams,
        disc_id: impl Into<finstack_core::types::CurveId>,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            strike_spread_bp: option_params.strike_spread_bp,
            option_type: option_params.option_type,
            exercise_style: ExerciseStyle::European,
            expiry: option_params.expiry,
            cds_maturity: option_params.cds_maturity,
            day_count: DayCount::Act360,
            notional: option_params.notional,
            settlement: SettlementType::Cash,
            recovery_rate: credit_params.recovery_rate,
            disc_id: disc_id.into(),
            credit_id: credit_params.credit_curve_id.clone(),
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
            underlying_is_index: option_params.underlying_is_index,
            index_factor: option_params.index_factor,
            forward_spread_adjust_bp: option_params.forward_spread_adjust_bp,
        }
    }

    /// Calculate the net present value of this CDS option
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.npv(self, curves, as_of)
    }

    /// Calculate delta of this CDS option
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::F> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(self.credit_id.clone())?;
        let current_tenor = self.day_count.year_fraction(
            as_of,
            self.cds_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            self.strike_spread_bp
        };

        let sigma = if let Some(v) = self.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface_ref(self.vol_id)?
                .value_clamped(t, self.strike_spread_bp)
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let delta = pricer.delta(self, fwd_bp, sigma, t);
        Ok(delta * self.notional.amount())
    }

    /// Calculate gamma of this CDS option
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::F> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(self.credit_id.clone())?;
        let current_tenor = self.day_count.year_fraction(
            as_of,
            self.cds_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            self.strike_spread_bp
        };

        let sigma = if let Some(v) = self.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface_ref(self.vol_id)?
                .value_clamped(t, self.strike_spread_bp)
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let gamma = pricer.gamma(self, fwd_bp, sigma, t);
        Ok(gamma * self.notional.amount())
    }

    /// Calculate vega of this CDS option
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::F> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(self.credit_id.clone())?;
        let current_tenor = self.day_count.year_fraction(
            as_of,
            self.cds_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            self.strike_spread_bp
        };

        let sigma = if let Some(v) = self.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface_ref(self.vol_id)?
                .value_clamped(t, self.strike_spread_bp)
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let vega = pricer.vega(self, fwd_bp, sigma, t);
        Ok(vega * self.notional.amount())
    }

    /// Calculate theta of this CDS option
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::F> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Risk-free rate proxy from discount curve at expiry
        let disc = curves.get_discount_ref(self.disc_id.clone())?;
        let r = disc.zero(t);

        // Forward spread in bp
        let hazard_curve = curves.get_hazard(self.credit_id.clone())?;
        let current_tenor = self.day_count.year_fraction(
            as_of,
            self.cds_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            self.strike_spread_bp
        };

        let sigma = if let Some(v) = self.pricing_overrides.implied_volatility {
            v
        } else {
            curves
                .surface(self.vol_id)?
                .value_clamped(t, self.strike_spread_bp)
        };

        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        let theta = pricer.theta(self, fwd_bp, r, sigma, t);
        Ok(theta * self.notional.amount())
    }

    /// Calculate implied volatility of this CDS option
    pub fn implied_vol(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        target_price: finstack_core::F,
        initial_guess: Option<finstack_core::F>,
    ) -> finstack_core::Result<finstack_core::F> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.implied_vol(self, curves, as_of, target_price, initial_guess)
    }
}

impl_instrument!(
    CdsOption,
    "CdsOption",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
    }
);

impl crate::instruments::common::HasDiscountCurve for CdsOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}
