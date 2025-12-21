//! Bond future core types.
//!
//! This module defines the data structures for bond futures, including
//! the deliverable basket, contract specifications, and the main BondFuture type.

use crate::instruments::common::traits::Attributes;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

// Re-export Position from ir_future module
pub use crate::instruments::ir_future::Position;

/// A bond in the deliverable basket with its conversion factor.
///
/// Each bond future contract has a basket of deliverable bonds that can be delivered
/// to satisfy the contract. The conversion factor normalizes bonds with different
/// coupons and maturities to a standard notional bond.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::bond_future::DeliverableBond;
/// use finstack_core::types::InstrumentId;
///
/// let deliverable = DeliverableBond {
///     bond_id: InstrumentId::new("US912828XG33"),
///     conversion_factor: 0.8234,
/// };
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeliverableBond {
    /// Identifier of the deliverable bond
    pub bond_id: InstrumentId,
    /// Conversion factor for this bond (published by exchange)
    pub conversion_factor: f64,
}

/// Contract specifications for bond futures.
///
/// Defines the standard parameters for a bond future contract including
/// contract size, tick size, and the notional bond parameters used for
/// conversion factor calculations.
///
/// # Examples
///
/// ```rust
/// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
///
/// // UST 10-year contract specs
/// let specs = BondFutureSpecs::default(); // UST 10Y defaults
/// assert_eq!(specs.contract_size, 100_000.0);
/// assert_eq!(specs.standard_coupon, 0.06);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BondFutureSpecs {
    /// Face value of a single contract (e.g., $100,000 for UST)
    pub contract_size: f64,
    /// Minimum price increment (e.g., 1/32 = 0.03125 for UST)
    pub tick_size: f64,
    /// Value of one tick in currency units (e.g., $31.25 for UST)
    pub tick_value: f64,
    /// Standard coupon rate for conversion factor calculation (e.g., 0.06 for 6%)
    pub standard_coupon: f64,
    /// Standard maturity in years for conversion factor calculation
    pub standard_maturity_years: f64,
    /// Number of business days for settlement after expiry
    pub settlement_days: u32,
}

impl Default for BondFutureSpecs {
    /// Default specifications for UST 10-year futures.
    ///
    /// Standard parameters:
    /// - Contract size: $100,000
    /// - Tick size: 1/32 of a point (0.03125)
    /// - Tick value: $31.25 (= $100,000 × 1/32 × 1%)
    /// - Standard coupon: 6% (0.06)
    /// - Standard maturity: 10 years
    /// - Settlement: 2 business days
    fn default() -> Self {
        Self {
            contract_size: 100_000.0,
            tick_size: 1.0 / 32.0,      // 1/32 of a point
            tick_value: 31.25,          // $100,000 × 1/32 × 1% = $31.25
            standard_coupon: 0.06,      // 6%
            standard_maturity_years: 10.0,
            settlement_days: 2,
        }
    }
}

/// Bond future instrument.
///
/// A bond future is a standardized contract to buy or sell a government bond at a
/// specified price on a future date. The contract has a basket of deliverable bonds,
/// each with a conversion factor. The holder of the short position chooses which
/// bond to deliver (typically the Cheapest-to-Deliver or CTD bond).
///
/// # Contract Mechanics
///
/// - **Deliverable Basket**: Multiple bonds eligible for delivery
/// - **Conversion Factors**: Published by exchange to normalize different bonds
/// - **CTD Selection**: Short side chooses which bond to deliver (user-specified in this implementation)
/// - **Invoice Price**: (Futures Price × Conversion Factor) + Accrued Interest
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_valuations::instruments::bond_future::{BondFuture, BondFutureBuilder, DeliverableBond, Position};
/// use finstack_core::money::Money;
/// use finstack_core::currency::Currency;
/// use finstack_core::dates::Date;
/// use finstack_core::types::{InstrumentId, CurveId};
/// use time::Month;
///
/// // Create a UST 10-year future
/// let future = BondFutureBuilder::new()
///     .id(InstrumentId::new("TYH5"))
///     .notional(Money::new(1_000_000.0, Currency::USD))
///     .expiry_date(Date::from_calendar_date(2025, Month::March, 20).unwrap())
///     .delivery_start(Date::from_calendar_date(2025, Month::March, 21).unwrap())
///     .delivery_end(Date::from_calendar_date(2025, Month::March, 31).unwrap())
///     .quoted_price(125.50)
///     .position(Position::Long)
///     .contract_specs(BondFutureSpecs::default())
///     .deliverable_basket(vec![
///         DeliverableBond {
///             bond_id: InstrumentId::new("US912828XG33"),
///             conversion_factor: 0.8234,
///         },
///     ])
///     .ctd_bond_id(InstrumentId::new("US912828XG33"))
///     .discount_curve_id(CurveId::new("USD-TREASURY"))
///     .build()
///     .expect("Valid bond future");
/// ```
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BondFuture {
    /// Unique identifier for the contract
    pub id: InstrumentId,
    
    /// Notional exposure in currency units.
    /// For multiple contracts, use notional = contract_specs.contract_size × num_contracts
    pub notional: Money,
    
    /// Future expiry date (last trading day)
    pub expiry_date: Date,
    
    /// First delivery date
    pub delivery_start: Date,
    
    /// Last delivery date
    pub delivery_end: Date,
    
    /// Quoted futures price (e.g., 125.50 for 125-16/32)
    pub quoted_price: f64,
    
    /// Position side (Long or Short)
    pub position: Position,
    
    /// Contract specifications (tick size, standard coupon, etc.)
    pub contract_specs: BondFutureSpecs,
    
    /// Basket of deliverable bonds with conversion factors
    pub deliverable_basket: Vec<DeliverableBond>,
    
    /// Cheapest-to-Deliver (CTD) bond identifier.
    /// User must specify which bond in the basket to use for pricing.
    /// In production systems, this would be calculated automatically.
    pub ctd_bond_id: InstrumentId,
    
    /// Discount curve identifier for present value calculations
    pub discount_curve_id: CurveId,
    
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_deliverable_bond_construction() {
        let db = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };
        assert_eq!(db.conversion_factor, 0.8234);
        assert_eq!(db.bond_id.as_str(), "US912828XG33");
    }

    #[test]
    fn test_bond_future_specs_default() {
        let specs = BondFutureSpecs::default();
        assert_eq!(specs.contract_size, 100_000.0);
        assert_eq!(specs.tick_size, 1.0 / 32.0);
        assert_eq!(specs.tick_value, 31.25);
        assert_eq!(specs.standard_coupon, 0.06);
        assert_eq!(specs.standard_maturity_years, 10.0);
        assert_eq!(specs.settlement_days, 2);
    }

    #[test]
    fn test_bond_future_construction() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .build()
            .expect("Valid bond future");

        assert_eq!(future.id.as_str(), "TYH5");
        assert_eq!(future.quoted_price, 125.50);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_position_long() {
        let pos = Position::Long;
        assert_eq!(pos, Position::Long);
        assert_eq!(format!("{}", pos), "long");
    }

    #[test]
    fn test_position_short() {
        let pos = Position::Short;
        assert_eq!(pos, Position::Short);
        assert_eq!(format!("{}", pos), "short");
    }
}
