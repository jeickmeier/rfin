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

use super::parameters::CdsOptionParams;

/// Credit option instrument (option on CDS spread)
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CdsOption {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Strike spread in basis points
    pub strike_spread_bp: f64,
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
    pub recovery_rate: f64,
    /// Discount curve identifier
    pub discount_curve_id: finstack_core::types::CurveId,
    /// Credit curve identifier
    pub credit_curve_id: finstack_core::types::CurveId,
    /// Volatility surface identifier
    pub vol_surface_id: finstack_core::types::CurveId,
    /// Pricing overrides (including implied volatility)
    pub pricing_overrides: PricingOverrides,
    /// Additional attributes
    pub attributes: Attributes,
    /// If true, the underlying is a CDS index; else single-name CDS
    pub underlying_is_index: bool,
    /// Optional index factor scaling for index underlying
    pub index_factor: Option<f64>,
    /// Forward spread adjustment (bp) to apply for forward computation
    pub forward_spread_adjust_bp: f64,
}

// Implement HasCreditCurve for generic CS01 calculator
impl crate::metrics::HasCreditCurve for CdsOption {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.credit_curve_id
    }
}

impl CdsOption {
    /// Create a canonical example CDS option (call on CDS spread).
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;
        let option_params = CdsOptionParams::call(
            100.0,
            Date::from_calendar_date(2025, Month::June, 20).unwrap(),
            Date::from_calendar_date(2030, Month::June, 20).unwrap(),
            Money::new(10_000_000.0, Currency::USD),
        );
        let credit_params =
            crate::instruments::common::parameters::CreditParams::corporate_standard(
                "CORP",
                "CORP-HAZARD",
            );
        CdsOption::new(
            InstrumentId::new("CDSOPT-CALL-CORP-5Y"),
            &option_params,
            &credit_params,
            "USD-OIS",
            "CDSOPT-VOL",
        )
    }

    /// Create a new credit option using parameter structs.
    ///
    /// Inputs separation:
    /// - `option_params`: deal-level fields (strike in bp, expiry, CDS maturity, notional, option type)
    /// - `credit_params`: reference entity, recovery rate, and the hazard `credit_id`
    /// - `discount_curve_id`: discount curve identifier for discounting cashflows
    /// - `vol_surface_id`: volatility surface identifier for the CDS option
    ///
    /// Note: `credit_id` is sourced from `credit_params` to avoid duplication.
    pub fn new(
        id: impl Into<InstrumentId>,
        option_params: &CdsOptionParams,
        credit_params: &CreditParams,
        discount_curve_id: impl Into<finstack_core::types::CurveId>,
        vol_surface_id: impl Into<finstack_core::types::CurveId>,
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
            discount_curve_id: discount_curve_id.into(),
            credit_curve_id: credit_params.credit_curve_id.to_owned(),
            vol_surface_id: vol_surface_id.into(),
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
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(&self.credit_curve_id)?;
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
                .surface_ref(self.vol_surface_id.as_str())?
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
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(&self.credit_curve_id)?;
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
                .surface_ref(self.vol_surface_id.as_str())?
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
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Forward spread in bp
        let hazard_curve = curves.get_hazard_ref(&self.credit_curve_id)?;
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
                .surface_ref(self.vol_surface_id.as_str())?
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
    ) -> finstack_core::Result<f64> {
        let t = self.day_count.year_fraction(
            as_of,
            self.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Risk-free rate proxy from discount curve at expiry
        let disc = curves.get_discount_ref(&self.discount_curve_id)?;
        let r = disc.zero(t);

        // Forward spread in bp
        let hazard_curve = curves.get_hazard(&self.credit_curve_id)?;
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
                .surface(self.vol_surface_id.as_str())?
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
        target_price: f64,
        initial_guess: Option<f64>,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_option::pricer::CdsOptionPricer::default();
        pricer.implied_vol(self, curves, as_of, target_price, initial_guess)
    }
}

impl crate::instruments::common::traits::Instrument for CdsOption {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDSOption
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
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
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
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CdsOption {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}
