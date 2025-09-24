//! Swaption (option on interest rate swap) implementation with SABR volatility.
//!
//! This module defines the `Swaption` data structure and integrates with the
//! common instrument trait via `impl_instrument!`. All pricing math is
//! implemented in the `pricing/` submodule; metrics are provided in the
//! `metrics/` submodule. The type exposes helper methods for forward swap
//! rate, annuity, and day-count based year fractions that reuse core library
//! functionality.

use crate::instruments::common::models::SABRParameters;
use crate::instruments::common::traits::Attributes;
use crate::instruments::common::parameters::OptionType;
use crate::instruments::PricingOverrides;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
use finstack_core::F;

use super::parameters::SwaptionParams;

/// Swaption settlement type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionSettlement {
    Physical,
    Cash,
}

/// Swaption exercise style
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwaptionExercise {
    European,
    Bermudan,
    American,
}

/// Swaption instrument
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct Swaption {
    pub id: String,
    pub option_type: OptionType,
    pub notional: Money,
    pub strike_rate: F,
    pub expiry: Date,
    pub swap_start: Date,
    pub swap_end: Date,
    pub fixed_freq: Frequency,
    pub float_freq: Frequency,
    pub day_count: DayCount,
    pub exercise: SwaptionExercise,
    pub settlement: SwaptionSettlement,
    pub disc_id: &'static str,
    pub forward_id: &'static str,
    pub vol_id: &'static str,
    pub pricing_overrides: PricingOverrides,
    pub sabr_params: Option<SABRParameters>,
    pub attributes: Attributes,
}

impl Swaption {
    /// Create a new payer swaption using parameter structs.
    pub fn new_payer(
        id: impl Into<String>,
        params: &SwaptionParams,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Call,
            notional: params.notional,
            strike_rate: params.strike_rate,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            disc_id,
            forward_id,
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Create a new receiver swaption using parameter structs.
    pub fn new_receiver(
        id: impl Into<String>,
        params: &SwaptionParams,
        disc_id: &'static str,
        forward_id: &'static str,
        vol_id: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            option_type: OptionType::Put,
            notional: params.notional,
            strike_rate: params.strike_rate,
            expiry: params.expiry,
            swap_start: params.swap_start,
            swap_end: params.swap_end,
            fixed_freq: Frequency::semi_annual(),
            float_freq: Frequency::quarterly(),
            day_count: DayCount::Thirty360,
            exercise: SwaptionExercise::European,
            settlement: SwaptionSettlement::Physical,
            disc_id,
            forward_id,
            vol_id,
            pricing_overrides: PricingOverrides::default(),
            sabr_params: None,
            attributes: Attributes::default(),
        }
    }

    /// Attach SABR parameters to enable SABR-implied volatility pricing.
    pub fn with_sabr(mut self, params: SABRParameters) -> Self {
        self.sabr_params = Some(params);
        self
    }

    // Pricing helpers moved to pricing::engine. Keep types.rs free of pricing logic.
}

impl_instrument!(
    Swaption,
    "Swaption",
    pv = |s, curves, _as_of| {
        // Delegate PV to the pricing engine to keep instrument type slim
        let pricer = crate::instruments::swaption::pricing::SwaptionPricer;
        pricer.npv(s, curves, _as_of)
    },
);
