//! Convertible bond instrument types and implementation.
//!
//! Data model for `ConvertibleBond` and related enums used by pricing and
//! metrics modules. Pricing logic is intentionally kept out of this file.

use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};


use crate::cashflow::builder::types::{FixedCouponSpec, FloatingCouponSpec};
use crate::instruments::bond::CallPutSchedule;
use crate::instruments::common::traits::Attributes;

use super::pricer;

/// Convertible bond instrument with embedded equity conversion option.
///
/// This fixed income instrument combines debt characteristics (coupons, principal)
/// with equity optionality (conversion rights). Uses the CashflowBuilder for
/// robust schedule generation and tree-based pricing for the hybrid valuation.
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
pub struct ConvertibleBond {
    /// Unique identifier for the instrument.
    pub id: InstrumentId,
    /// Principal amount.
    pub notional: Money,
    /// Issue date.
    pub issue: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Discount curve identifier for the debt component.
    pub disc_id: CurveId,
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
        threshold: f64,
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
    pub ratio: Option<f64>,
    /// Conversion price (price per share). If not provided, derive from ratio.
    pub price: Option<f64>,
    /// Policy governing conversion timing/conditions.
    pub policy: ConversionPolicy,
    /// Anti-dilution protection policy.
    pub anti_dilution: AntiDilutionPolicy,
    /// Dividend adjustment mechanism.
    pub dividend_adjustment: DividendAdjustment,
}

impl ConvertibleBond {
    /// Calculate the net present value of this convertible bond
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        pricer::price_convertible_bond(self, curves, pricer::ConvertibleTreeType::default())
    }

    /// Calculate parity ratio of this convertible bond
    pub fn parity(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let underlying_id = self
            .underlying_equity_id
            .as_ref()
            .ok_or(finstack_core::Error::Internal)?;

        let spot_price = curves.price(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };

        Ok(pricer::calculate_parity(self, spot))
    }

    /// Calculate conversion premium of this convertible bond  
    pub fn conversion_premium(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        bond_price: f64,
    ) -> finstack_core::Result<f64> {
        let underlying_id = self
            .underlying_equity_id
            .as_ref()
            .ok_or(finstack_core::Error::Internal)?;

        let spot_price = curves.price(underlying_id)?;
        let spot = match spot_price {
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
            finstack_core::market_data::scalars::MarketScalar::Unitless(value) => *value,
        };

        // Get conversion ratio
        let conversion_ratio = if let Some(ratio) = self.conversion.ratio {
            ratio
        } else if let Some(price) = self.conversion.price {
            self.notional.amount() / price
        } else {
            return Err(finstack_core::Error::Internal);
        };

        Ok(pricer::calculate_conversion_premium(
            bond_price,
            spot,
            conversion_ratio,
        ))
    }

    /// Calculate Greeks for this convertible bond
    pub fn greeks(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        tree_type: Option<pricer::ConvertibleTreeType>,
        bump_size: Option<f64>,
    ) -> finstack_core::Result<crate::instruments::common::models::TreeGreeks> {
        pricer::calculate_convertible_greeks(self, curves, tree_type.unwrap_or_default(), bump_size)
    }

    /// Calculate delta of this convertible bond
    pub fn delta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, None, None)?;
        Ok(greeks.delta)
    }

    /// Calculate gamma of this convertible bond
    pub fn gamma(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, None, None)?;
        Ok(greeks.gamma)
    }

    /// Calculate vega of this convertible bond
    pub fn vega(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, None, None)?;
        Ok(greeks.vega)
    }

    /// Calculate rho of this convertible bond
    pub fn rho(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, None, None)?;
        Ok(greeks.rho)
    }

    /// Calculate theta of this convertible bond
    pub fn theta(
        &self,
        curves: &finstack_core::market_data::MarketContext,
    ) -> finstack_core::Result<f64> {
        let greeks = self.greeks(curves, None, None)?;
        Ok(greeks.theta)
    }
}

impl_instrument!(
    ConvertibleBond,
    crate::pricer::InstrumentType::Convertible,
    "ConvertibleBond",
    pv = |s, curves, as_of| {
        // Call the instrument's own NPV method
        s.npv(curves, as_of)
    }
);

impl crate::instruments::common::HasDiscountCurve for ConvertibleBond {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}
