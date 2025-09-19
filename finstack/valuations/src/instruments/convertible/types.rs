//! Convertible bond instrument types and implementation.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

use crate::cashflow::builder::types::{FixedCouponSpec, FloatingCouponSpec};
use crate::instruments::bond::CallPutSchedule;
use crate::instruments::traits::Attributes;

use super::model;

/// Convertible bond instrument with embedded equity conversion option.
///
/// This fixed income instrument combines debt characteristics (coupons, principal)
/// with equity optionality (conversion rights). Uses the CashflowBuilder for
/// robust schedule generation and tree-based pricing for the hybrid valuation.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    #[builder(optional)]
    pub underlying_equity_id: Option<String>,
    /// Optional call/put schedule (issuer/holder redemption before maturity).
    #[builder(optional)]
    pub call_put: Option<CallPutSchedule>,
    /// Fixed coupon specification (if applicable).
    #[builder(optional)]
    pub fixed_coupon: Option<FixedCouponSpec>,
    /// Floating coupon specification (if applicable).
    #[builder(optional)]
    pub floating_coupon: Option<FloatingCouponSpec>,
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
    PriceTrigger {
        threshold: F,
        lookback_days: u32,
    },
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
    ConvertibleBond,
    "ConvertibleBond",
    pv = |s, curves, _as_of| {
        // Use the new tree-based pricing model
        model::price_convertible_bond(s, curves, model::ConvertibleTreeType::default())
    }
);
