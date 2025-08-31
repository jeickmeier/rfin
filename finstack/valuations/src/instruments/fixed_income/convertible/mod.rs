//! Convertible bond instrument boilerplate.

pub mod metrics;

use finstack_core::prelude::*;
use finstack_core::F;

use crate::instruments::traits::Attributes;
use crate::instruments::fixed_income::bond::CallPutSchedule;

/// Simplified convertible bond placeholder.
///
/// This is a fixed income instrument with an embedded equity conversion option.
/// Initial boilerplate supports identity PV (0) and no default metrics.
#[derive(Clone, Debug)]
pub struct ConvertibleBond {
    /// Unique identifier for the instrument.
    pub id: String,
    /// Principal amount.
    pub notional: Money,
    /// Issue date.
    pub issue: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Discount curve identifier for the debt component.
    pub disc_id: &'static str,
    /// Conversion terms for equity conversion.
    pub conversion: ConversionSpec,
    /// Optional underlying equity identifier (ticker or instrument id).
    pub underlying_equity_id: Option<String>,
    /// Optional call/put schedule (issuer/holder redemption before maturity).
    pub call_put: Option<CallPutSchedule>,
    /// Attributes for selection and tagging.
    pub attributes: Attributes,
}

/// Defines how and when conversion can occur.
#[derive(Clone, Debug)]
pub enum ConversionPolicy {
    /// Holder may convert at any time (subject to window, if any).
    Voluntary,
    /// Bond will mandatorily convert on the specified date.
    MandatoryOn(Date),
    /// Holder may convert within a window.
    Window { start: Date, end: Date },
    /// Conversion tied to an external event or condition.
    UponEvent(ConversionEvent),
}

/// Events that may trigger conversion.
#[derive(Clone, Debug)]
pub enum ConversionEvent {
    QualifiedIpo,
    ChangeOfControl,
    /// Forced conversion if share price meets threshold for a lookback period.
    PriceTrigger { threshold: F, lookback_days: u32 },
}

/// Anti-dilution protection applied to conversion terms.
#[derive(Clone, Debug)]
pub enum AntiDilutionPolicy {
    None,
    FullRatchet,
    WeightedAverage,
}

/// How dividends affect conversion terms.
#[derive(Clone, Debug)]
pub enum DividendAdjustment {
    None,
    AdjustPrice,
    AdjustRatio,
}

/// Conversion specification for the instrument.
#[derive(Clone, Debug)]
pub struct ConversionSpec {
    /// Conversion ratio (shares per bond). If not provided, derive from price.
    pub ratio: Option<F>,
    /// Conversion price (price per share). If not provided, derive from ratio.
    pub price: Option<F>,
    /// Policy governing conversion timing/conditions.
    pub policy: ConversionPolicy,
    /// Anti-dilution protection policy.
    pub anti_dilution: AntiDilutionPolicy,
    /// Dividend adjustment mechanism.
    pub dividend_adjustment: DividendAdjustment,
}

impl_instrument!(
    ConvertibleBond, "ConvertibleBond",
    pv = |_s, _curves, _as_of| {
        // Placeholder PV; real implementation will separate debt and option legs
        Ok(Money::new(0.0, _s.notional.currency()))
    },
    metrics = |_s| {
        // No standard metrics yet; to be expanded with equity sensitivity, parity, etc.
        vec![]
    }
);

// Builder for ConvertibleBond
impl_builder!(
    ConvertibleBond,
    ConvertibleBondBuilder,
    required: [
        id: String,
        notional: Money,
        issue: Date,
        maturity: Date,
        disc_id: &'static str,
        conversion: ConversionSpec
    ],
    optional: [
        underlying_equity_id: String,
        call_put: CallPutSchedule
    ]
);


