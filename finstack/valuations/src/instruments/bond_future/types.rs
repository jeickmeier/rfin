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

impl BondFutureSpecs {
    /// UST 10-year futures contract specifications.
    ///
    /// **Market**: U.S. Treasury
    /// **Exchange**: Chicago Board of Trade (CBOT)
    /// **Contract**: 10-Year T-Note Futures
    ///
    /// # Specifications
    ///
    /// - Contract size: $100,000
    /// - Tick size: 1/32 of a point (0.03125)
    /// - Tick value: $31.25 per tick
    /// - Standard coupon: 6% annual
    /// - Standard maturity: 10 years
    /// - Settlement: T+2 business days
    /// - Day count: Actual/Actual (ISDA)
    /// - Deliverable: U.S. Treasury notes with at least 6.5 years remaining maturity
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
    ///
    /// let specs = BondFutureSpecs::ust_10y();
    /// assert_eq!(specs.contract_size, 100_000.0);
    /// assert_eq!(specs.standard_coupon, 0.06);
    /// ```
    pub fn ust_10y() -> Self {
        Self::default()
    }

    /// UST 5-year futures contract specifications.
    ///
    /// **Market**: U.S. Treasury
    /// **Exchange**: Chicago Board of Trade (CBOT)
    /// **Contract**: 5-Year T-Note Futures
    ///
    /// # Specifications
    ///
    /// - Contract size: $100,000
    /// - Tick size: 1/4 of 1/32 of a point (0.0078125)
    /// - Tick value: $15.625 per tick
    /// - Standard coupon: 6% annual
    /// - Standard maturity: 5 years
    /// - Settlement: T+2 business days
    /// - Day count: Actual/Actual (ISDA)
    /// - Deliverable: U.S. Treasury notes with at least 4 years, 2 months remaining maturity
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
    ///
    /// let specs = BondFutureSpecs::ust_5y();
    /// assert_eq!(specs.contract_size, 100_000.0);
    /// assert_eq!(specs.tick_size, 1.0 / 128.0);
    /// assert_eq!(specs.standard_maturity_years, 5.0);
    /// ```
    pub fn ust_5y() -> Self {
        Self {
            contract_size: 100_000.0,
            tick_size: 1.0 / 128.0,     // 1/4 of 1/32 = 1/128
            tick_value: 15.625,         // $100,000 × 1/128 × 1% = $15.625
            standard_coupon: 0.06,      // 6%
            standard_maturity_years: 5.0,
            settlement_days: 2,
        }
    }

    /// UST 2-year futures contract specifications.
    ///
    /// **Market**: U.S. Treasury
    /// **Exchange**: Chicago Board of Trade (CBOT)
    /// **Contract**: 2-Year T-Note Futures
    ///
    /// # Specifications
    ///
    /// - Contract size: $200,000 (note: double the 5Y/10Y contracts)
    /// - Tick size: 1/4 of 1/32 of a point (0.0078125)
    /// - Tick value: $15.625 per tick (= $200,000 × 1/128 × 1% / 2)
    /// - Standard coupon: 6% annual
    /// - Standard maturity: 2 years
    /// - Settlement: T+2 business days
    /// - Day count: Actual/Actual (ISDA)
    /// - Deliverable: U.S. Treasury notes with at least 1 year, 9 months remaining maturity
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
    ///
    /// let specs = BondFutureSpecs::ust_2y();
    /// assert_eq!(specs.contract_size, 200_000.0);
    /// assert_eq!(specs.tick_size, 1.0 / 128.0);
    /// assert_eq!(specs.standard_maturity_years, 2.0);
    /// ```
    pub fn ust_2y() -> Self {
        Self {
            contract_size: 200_000.0,   // 2Y contracts are $200k (double 5Y/10Y)
            tick_size: 1.0 / 128.0,     // 1/4 of 1/32 = 1/128
            tick_value: 15.625,         // $200,000 × 1/128 × 1% / 2 = $15.625
            standard_coupon: 0.06,      // 6%
            standard_maturity_years: 2.0,
            settlement_days: 2,
        }
    }

    /// German Bund futures contract specifications.
    ///
    /// **Market**: Germany (Eurex)
    /// **Exchange**: Eurex Exchange
    /// **Contract**: Euro-Bund Futures
    ///
    /// # Specifications
    ///
    /// - Contract size: €100,000
    /// - Tick size: 0.01 (1 basis point)
    /// - Tick value: €10 per tick
    /// - Standard coupon: 6% annual
    /// - Standard maturity: 10 years
    /// - Settlement: T+2 business days
    /// - Day count: Actual/Actual (ISDA)
    /// - Deliverable: German Federal bonds with 8.5 to 10.5 years remaining maturity
    ///
    /// # Notes
    ///
    /// - Quoted in percentage points (e.g., 125.50 = 125.50%)
    /// - Different tick size from UST (decimal vs. 32nds)
    /// - Settlement follows TARGET2 calendar
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
    ///
    /// let specs = BondFutureSpecs::bund();
    /// assert_eq!(specs.contract_size, 100_000.0);
    /// assert_eq!(specs.tick_size, 0.01);
    /// assert_eq!(specs.tick_value, 10.0);
    /// ```
    pub fn bund() -> Self {
        Self {
            contract_size: 100_000.0,
            tick_size: 0.01,            // 1 basis point
            tick_value: 10.0,           // €100,000 × 0.01% = €10
            standard_coupon: 0.06,      // 6%
            standard_maturity_years: 10.0,
            settlement_days: 2,
        }
    }

    /// UK Gilt futures contract specifications.
    ///
    /// **Market**: United Kingdom
    /// **Exchange**: ICE Futures Europe (LIFFE)
    /// **Contract**: Long Gilt Futures
    ///
    /// # Specifications
    ///
    /// - Contract size: £100,000
    /// - Tick size: 0.01 (1 basis point)
    /// - Tick value: £10 per tick
    /// - Standard coupon: 4% annual (note: different from UST/Bund 6%)
    /// - Standard maturity: 10 years
    /// - Settlement: T+2 business days
    /// - Day count: Actual/Actual (ISDA)
    /// - Deliverable: UK Gilts with 8.75 to 13 years remaining maturity
    ///
    /// # Notes
    ///
    /// - Quoted in percentage points (e.g., 125.50 = 125.50%)
    /// - Standard coupon is 4%, not 6% like other major markets
    /// - Settlement follows UK bank holidays
    /// - Long Gilt contract covers 8.75-13 year maturity range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_valuations::instruments::bond_future::BondFutureSpecs;
    ///
    /// let specs = BondFutureSpecs::gilt();
    /// assert_eq!(specs.contract_size, 100_000.0);
    /// assert_eq!(specs.tick_size, 0.01);
    /// assert_eq!(specs.standard_coupon, 0.04);  // 4%, not 6%
    /// ```
    pub fn gilt() -> Self {
        Self {
            contract_size: 100_000.0,
            tick_size: 0.01,            // 1 basis point
            tick_value: 10.0,           // £100,000 × 0.01% = £10
            standard_coupon: 0.04,      // 4% (different from UST/Bund)
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

impl BondFuture {
    /// Validate the BondFuture parameters.
    ///
    /// This method checks the following invariants:
    /// - Date ordering: expiry_date < delivery_start < delivery_end
    /// - Deliverable basket is non-empty
    /// - CTD bond exists in deliverable basket
    /// - All conversion factors are positive
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`](finstack_core::Error::Validation) if any validation fails.
    fn validate(&self) -> finstack_core::Result<()> {
        // Date ordering validation
        if self.expiry_date >= self.delivery_start {
            return Err(finstack_core::Error::Validation(format!(
                "expiry_date ({}) must be before delivery_start ({})",
                self.expiry_date, self.delivery_start
            )));
        }
        if self.delivery_start >= self.delivery_end {
            return Err(finstack_core::Error::Validation(format!(
                "delivery_start ({}) must be before delivery_end ({})",
                self.delivery_start, self.delivery_end
            )));
        }

        // Deliverable basket validation
        if self.deliverable_basket.is_empty() {
            return Err(finstack_core::Error::Validation(
                "deliverable_basket cannot be empty".to_string(),
            ));
        }

        // CTD bond exists in basket validation
        let ctd_exists = self
            .deliverable_basket
            .iter()
            .any(|bond| bond.bond_id == self.ctd_bond_id);
        if !ctd_exists {
            return Err(finstack_core::Error::Validation(format!(
                "ctd_bond_id ({}) not found in deliverable_basket",
                self.ctd_bond_id.as_str()
            )));
        }

        // Conversion factors validation
        for deliverable in &self.deliverable_basket {
            if deliverable.conversion_factor <= 0.0 {
                return Err(finstack_core::Error::Validation(format!(
                    "conversion_factor must be positive for bond {}, got {}",
                    deliverable.bond_id.as_str(),
                    deliverable.conversion_factor
                )));
            }
        }

        Ok(())
    }

    /// Create a UST 10-year futures contract.
    ///
    /// **Market**: U.S. Treasury 10-Year Note Futures (CBOT)
    ///
    /// This is a convenience constructor that automatically sets:
    /// - Contract specifications: [`BondFutureSpecs::ust_10y()`]
    /// - Attributes: empty [`Attributes::new()`]
    ///
    /// # Arguments
    ///
    /// * `id` - Contract identifier (e.g., "TYH5" for March 2025)
    /// * `notional` - Total notional exposure (contract_size × num_contracts)
    /// * `expiry_date` - Last trading day
    /// * `delivery_start` - First delivery date
    /// * `delivery_end` - Last delivery date
    /// * `quoted_price` - Futures price (e.g., 125.50 for 125-16/32)
    /// * `position` - Long or Short
    /// * `deliverable_basket` - Eligible bonds with conversion factors
    /// * `ctd_bond_id` - Cheapest-to-Deliver bond identifier
    /// * `discount_curve_id` - Discount curve for pricing
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::bond_future::{BondFuture, DeliverableBond, Position};
    /// use finstack_core::money::Money;
    /// use finstack_core::types::{InstrumentId, CurveId};
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// let future = BondFuture::ust_10y(
    ///     InstrumentId::new("TYH5"),
    ///     Money::from_code(1_000_000.0, "USD"),
    ///     Date::from_calendar_date(2025, Month::March, 20).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 21).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 31).unwrap(),
    ///     125.50,
    ///     Position::Long,
    ///     vec![DeliverableBond {
    ///         bond_id: InstrumentId::new("US912828XG33"),
    ///         conversion_factor: 0.8234,
    ///     }],
    ///     InstrumentId::new("US912828XG33"),
    ///     CurveId::new("USD-TREASURY"),
    /// ).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (see [`BondFuture::validate`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ust_10y(
        id: InstrumentId,
        notional: Money,
        expiry_date: Date,
        delivery_start: Date,
        delivery_end: Date,
        quoted_price: f64,
        position: Position,
        deliverable_basket: Vec<DeliverableBond>,
        ctd_bond_id: InstrumentId,
        discount_curve_id: CurveId,
    ) -> finstack_core::Result<Self> {
        BondFutureBuilder::new()
            .id(id)
            .notional(notional)
            .expiry_date(expiry_date)
            .delivery_start(delivery_start)
            .delivery_end(delivery_end)
            .quoted_price(quoted_price)
            .position(position)
            .contract_specs(BondFutureSpecs::ust_10y())
            .deliverable_basket(deliverable_basket)
            .ctd_bond_id(ctd_bond_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new())
            .try_build()
    }

    /// Create a UST 5-year futures contract.
    ///
    /// **Market**: U.S. Treasury 5-Year Note Futures (CBOT)
    ///
    /// This is a convenience constructor that automatically sets:
    /// - Contract specifications: [`BondFutureSpecs::ust_5y()`]
    /// - Attributes: empty [`Attributes::new()`]
    ///
    /// # Arguments
    ///
    /// * `id` - Contract identifier (e.g., "FVH5" for March 2025)
    /// * `notional` - Total notional exposure (contract_size × num_contracts)
    /// * `expiry_date` - Last trading day
    /// * `delivery_start` - First delivery date
    /// * `delivery_end` - Last delivery date
    /// * `quoted_price` - Futures price
    /// * `position` - Long or Short
    /// * `deliverable_basket` - Eligible bonds with conversion factors
    /// * `ctd_bond_id` - Cheapest-to-Deliver bond identifier
    /// * `discount_curve_id` - Discount curve for pricing
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let future = BondFuture::ust_5y(
    ///     InstrumentId::new("FVH5"),
    ///     Money::from_code(500_000.0, "USD"),
    ///     Date::from_calendar_date(2025, Month::March, 20).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 21).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 31).unwrap(),
    ///     118.75,
    ///     Position::Long,
    ///     vec![/* deliverable bonds */],
    ///     InstrumentId::new("US912828XG33"),
    ///     CurveId::new("USD-TREASURY"),
    /// ).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (see [`BondFuture::validate`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ust_5y(
        id: InstrumentId,
        notional: Money,
        expiry_date: Date,
        delivery_start: Date,
        delivery_end: Date,
        quoted_price: f64,
        position: Position,
        deliverable_basket: Vec<DeliverableBond>,
        ctd_bond_id: InstrumentId,
        discount_curve_id: CurveId,
    ) -> finstack_core::Result<Self> {
        BondFutureBuilder::new()
            .id(id)
            .notional(notional)
            .expiry_date(expiry_date)
            .delivery_start(delivery_start)
            .delivery_end(delivery_end)
            .quoted_price(quoted_price)
            .position(position)
            .contract_specs(BondFutureSpecs::ust_5y())
            .deliverable_basket(deliverable_basket)
            .ctd_bond_id(ctd_bond_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new())
            .try_build()
    }

    /// Create a UST 2-year futures contract.
    ///
    /// **Market**: U.S. Treasury 2-Year Note Futures (CBOT)
    ///
    /// This is a convenience constructor that automatically sets:
    /// - Contract specifications: [`BondFutureSpecs::ust_2y()`]
    /// - Attributes: empty [`Attributes::new()`]
    ///
    /// **Note**: 2-year contracts have a larger contract size ($200,000) than 5Y/10Y.
    ///
    /// # Arguments
    ///
    /// * `id` - Contract identifier (e.g., "TUH5" for March 2025)
    /// * `notional` - Total notional exposure (contract_size × num_contracts)
    /// * `expiry_date` - Last trading day
    /// * `delivery_start` - First delivery date
    /// * `delivery_end` - Last delivery date
    /// * `quoted_price` - Futures price
    /// * `position` - Long or Short
    /// * `deliverable_basket` - Eligible bonds with conversion factors
    /// * `ctd_bond_id` - Cheapest-to-Deliver bond identifier
    /// * `discount_curve_id` - Discount curve for pricing
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let future = BondFuture::ust_2y(
    ///     InstrumentId::new("TUH5"),
    ///     Money::from_code(400_000.0, "USD"),  // 2 contracts × $200k
    ///     Date::from_calendar_date(2025, Month::March, 20).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 21).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 31).unwrap(),
    ///     105.25,
    ///     Position::Long,
    ///     vec![/* deliverable bonds */],
    ///     InstrumentId::new("US912828XG33"),
    ///     CurveId::new("USD-TREASURY"),
    /// ).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (see [`BondFuture::validate`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ust_2y(
        id: InstrumentId,
        notional: Money,
        expiry_date: Date,
        delivery_start: Date,
        delivery_end: Date,
        quoted_price: f64,
        position: Position,
        deliverable_basket: Vec<DeliverableBond>,
        ctd_bond_id: InstrumentId,
        discount_curve_id: CurveId,
    ) -> finstack_core::Result<Self> {
        BondFutureBuilder::new()
            .id(id)
            .notional(notional)
            .expiry_date(expiry_date)
            .delivery_start(delivery_start)
            .delivery_end(delivery_end)
            .quoted_price(quoted_price)
            .position(position)
            .contract_specs(BondFutureSpecs::ust_2y())
            .deliverable_basket(deliverable_basket)
            .ctd_bond_id(ctd_bond_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new())
            .try_build()
    }

    /// Create a German Bund futures contract.
    ///
    /// **Market**: Euro-Bund Futures (Eurex)
    ///
    /// This is a convenience constructor that automatically sets:
    /// - Contract specifications: [`BondFutureSpecs::bund()`]
    /// - Attributes: empty [`Attributes::new()`]
    ///
    /// # Arguments
    ///
    /// * `id` - Contract identifier (e.g., "FGBLH5" for March 2025)
    /// * `notional` - Total notional exposure (contract_size × num_contracts)
    /// * `expiry_date` - Last trading day
    /// * `delivery_start` - First delivery date
    /// * `delivery_end` - Last delivery date
    /// * `quoted_price` - Futures price (decimal, e.g., 125.50)
    /// * `position` - Long or Short
    /// * `deliverable_basket` - Eligible bonds with conversion factors
    /// * `ctd_bond_id` - Cheapest-to-Deliver bond identifier
    /// * `discount_curve_id` - Discount curve for pricing
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let future = BondFuture::bund(
    ///     InstrumentId::new("FGBLH5"),
    ///     Money::from_code(1_000_000.0, "EUR"),
    ///     Date::from_calendar_date(2025, Month::March, 20).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 21).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 31).unwrap(),
    ///     132.15,
    ///     Position::Long,
    ///     vec![/* deliverable bonds */],
    ///     InstrumentId::new("DE0001102473"),
    ///     CurveId::new("EUR-BUNDS"),
    /// ).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (see [`BondFuture::validate`]).
    #[allow(clippy::too_many_arguments)]
    pub fn bund(
        id: InstrumentId,
        notional: Money,
        expiry_date: Date,
        delivery_start: Date,
        delivery_end: Date,
        quoted_price: f64,
        position: Position,
        deliverable_basket: Vec<DeliverableBond>,
        ctd_bond_id: InstrumentId,
        discount_curve_id: CurveId,
    ) -> finstack_core::Result<Self> {
        BondFutureBuilder::new()
            .id(id)
            .notional(notional)
            .expiry_date(expiry_date)
            .delivery_start(delivery_start)
            .delivery_end(delivery_end)
            .quoted_price(quoted_price)
            .position(position)
            .contract_specs(BondFutureSpecs::bund())
            .deliverable_basket(deliverable_basket)
            .ctd_bond_id(ctd_bond_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new())
            .try_build()
    }

    /// Create a UK Gilt futures contract.
    ///
    /// **Market**: Long Gilt Futures (ICE Futures Europe/LIFFE)
    ///
    /// This is a convenience constructor that automatically sets:
    /// - Contract specifications: [`BondFutureSpecs::gilt()`]
    /// - Attributes: empty [`Attributes::new()`]
    ///
    /// **Note**: Gilts use a 4% standard coupon, different from UST/Bund 6%.
    ///
    /// # Arguments
    ///
    /// * `id` - Contract identifier (e.g., "H5" for March 2025)
    /// * `notional` - Total notional exposure (contract_size × num_contracts)
    /// * `expiry_date` - Last trading day
    /// * `delivery_start` - First delivery date
    /// * `delivery_end` - Last delivery date
    /// * `quoted_price` - Futures price (decimal, e.g., 115.25)
    /// * `position` - Long or Short
    /// * `deliverable_basket` - Eligible bonds with conversion factors
    /// * `ctd_bond_id` - Cheapest-to-Deliver bond identifier
    /// * `discount_curve_id` - Discount curve for pricing
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let future = BondFuture::gilt(
    ///     InstrumentId::new("GILTH5"),
    ///     Money::from_code(500_000.0, "GBP"),
    ///     Date::from_calendar_date(2025, Month::March, 20).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 21).unwrap(),
    ///     Date::from_calendar_date(2025, Month::March, 31).unwrap(),
    ///     115.25,
    ///     Position::Long,
    ///     vec![/* deliverable bonds */],
    ///     InstrumentId::new("GB00B128DH60"),
    ///     CurveId::new("GBP-GILTS"),
    /// ).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails (see [`BondFuture::validate`]).
    #[allow(clippy::too_many_arguments)]
    pub fn gilt(
        id: InstrumentId,
        notional: Money,
        expiry_date: Date,
        delivery_start: Date,
        delivery_end: Date,
        quoted_price: f64,
        position: Position,
        deliverable_basket: Vec<DeliverableBond>,
        ctd_bond_id: InstrumentId,
        discount_curve_id: CurveId,
    ) -> finstack_core::Result<Self> {
        BondFutureBuilder::new()
            .id(id)
            .notional(notional)
            .expiry_date(expiry_date)
            .delivery_start(delivery_start)
            .delivery_end(delivery_end)
            .quoted_price(quoted_price)
            .position(position)
            .contract_specs(BondFutureSpecs::gilt())
            .deliverable_basket(deliverable_basket)
            .ctd_bond_id(ctd_bond_id)
            .discount_curve_id(discount_curve_id)
            .attributes(Attributes::new())
            .try_build()
    }

    /// Calculate the invoice price for settlement of the bond future.
    ///
    /// The invoice price is the amount the buyer pays to the seller when taking delivery
    /// of the underlying bond at expiry. It consists of:
    /// - The futures price scaled by the conversion factor (bringing CTD bond to standard terms)
    /// - Plus accrued interest on the CTD bond at settlement
    ///
    /// Formula: `Invoice = (Futures_Price × Conversion_Factor) + Accrued_Interest`
    ///
    /// # Arguments
    ///
    /// * `ctd_bond` - The cheapest-to-deliver bond reference
    /// * `market` - Market context containing curves for accrued interest calculation
    /// * `settlement_date` - Settlement date (typically T+2 after expiry)
    ///
    /// # Returns
    ///
    /// Invoice price in the same currency as the futures contract notional.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - CTD bond is not found in the deliverable basket
    /// - Cashflow schedule building fails
    /// - Accrued interest calculation fails
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::bond_future::BondFuture;
    /// use finstack_valuations::instruments::bond::Bond;
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::dates::Date;
    /// use time::Month;
    ///
    /// # let future = create_test_bond_future();
    /// # let ctd_bond = create_test_bond();
    /// # let market = MarketContext::new();
    ///
    /// // Calculate invoice price for settlement 2 days after expiry
    /// let settlement = Date::from_calendar_date(2025, Month::March, 23).unwrap();
    /// let invoice = future.invoice_price(&ctd_bond, &market, settlement)?;
    ///
    /// // For futures price 125.50 and CF 0.8234:
    /// // Invoice = (125.50 × 0.8234) + accrued
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn invoice_price(
        &self,
        ctd_bond: &crate::instruments::bond::Bond,
        market: &finstack_core::market_data::context::MarketContext,
        settlement_date: Date,
    ) -> finstack_core::Result<Money> {
        // Find the conversion factor for the CTD bond
        let conversion_factor = self
            .deliverable_basket
            .iter()
            .find(|db| db.bond_id == self.ctd_bond_id)
            .ok_or_else(|| {
                finstack_core::error::InputError::NotFound {
                    id: format!(
                        "CTD bond {} not found in deliverable basket",
                        self.ctd_bond_id.as_str()
                    ),
                }
            })?
            .conversion_factor;

        // Get the CTD bond's cashflow schedule
        let schedule = ctd_bond.get_full_schedule(market)?;

        // Calculate accrued interest at settlement date
        use crate::cashflow::accrual::{accrued_interest_amount, AccrualConfig, ExCouponRule};
        
        let accrual_config = AccrualConfig {
            method: ctd_bond.accrual_method.clone(),
            ex_coupon: ctd_bond.ex_coupon_days.map(|days| ExCouponRule {
                days_before_coupon: days,
                calendar_id: ctd_bond.ex_coupon_calendar_id.clone(),
            }),
            include_pik: true,
        };

        let accrued_amount = accrued_interest_amount(&schedule, settlement_date, &accrual_config)?;

        // Convert accrued interest from absolute currency units to percentage of par
        // accrued_amount is in dollars (e.g., $7,000 on $100,000 notional)
        // We need to express it as dollars per $100 face (e.g., $7 per $100)
        let ctd_notional = ctd_bond.notional.amount();
        let accrued_pct = (accrued_amount / ctd_notional) * 100.0;

        // Calculate invoice price per contract
        // Invoice = (Futures_Price × CF) + Accrued
        // Note: Futures price is quoted per $100 face value, so we scale appropriately
        let futures_price_pct = self.quoted_price; // e.g., 125.50 for 125-16/32
        let invoice_pct = (futures_price_pct * conversion_factor) + accrued_pct;

        // Convert from percentage to actual money amount for contract size
        // Contract size is typically $100,000, so invoice_pct is per $100 face
        let invoice_per_contract = (invoice_pct / 100.0) * self.contract_specs.contract_size;

        // Scale by number of contracts (notional / contract_size)
        let num_contracts = self.notional.amount() / self.contract_specs.contract_size;
        let total_invoice = invoice_per_contract * num_contracts;

        Ok(Money::new(total_invoice, self.notional.currency()))
    }
}

// Manually implement a validated builder method
impl BondFutureBuilder {
    /// Build the BondFuture with validation.
    ///
    /// This is a wrapper around the generated `build()` method that adds
    /// validation checks after construction.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any required field is missing (from the generated builder)
    /// - Validation fails (from [`BondFuture::validate`])
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let future = BondFutureBuilder::new()
    ///     .id(InstrumentId::new("TYH5"))
    ///     // ... set all fields
    ///     .try_build()?; // Validates after construction
    /// ```
    pub fn try_build(self) -> finstack_core::Result<BondFuture> {
        let bond_future = self.build().map_err(|e| {
            finstack_core::Error::Validation(format!("BondFuture construction failed: {}", e))
        })?;
        bond_future.validate()?;
        Ok(bond_future)
    }
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
    fn test_ust_10y_specs() {
        let specs = BondFutureSpecs::ust_10y();
        assert_eq!(specs.contract_size, 100_000.0);
        assert_eq!(specs.tick_size, 1.0 / 32.0);
        assert_eq!(specs.tick_value, 31.25);
        assert_eq!(specs.standard_coupon, 0.06);
        assert_eq!(specs.standard_maturity_years, 10.0);
        assert_eq!(specs.settlement_days, 2);
    }

    #[test]
    fn test_ust_5y_specs() {
        let specs = BondFutureSpecs::ust_5y();
        assert_eq!(specs.contract_size, 100_000.0);
        assert_eq!(specs.tick_size, 1.0 / 128.0);
        assert_eq!(specs.tick_value, 15.625);
        assert_eq!(specs.standard_coupon, 0.06);
        assert_eq!(specs.standard_maturity_years, 5.0);
        assert_eq!(specs.settlement_days, 2);
    }

    #[test]
    fn test_ust_2y_specs() {
        let specs = BondFutureSpecs::ust_2y();
        assert_eq!(specs.contract_size, 200_000.0);  // Note: 2Y is $200k
        assert_eq!(specs.tick_size, 1.0 / 128.0);
        assert_eq!(specs.tick_value, 15.625);
        assert_eq!(specs.standard_coupon, 0.06);
        assert_eq!(specs.standard_maturity_years, 2.0);
        assert_eq!(specs.settlement_days, 2);
    }

    #[test]
    fn test_bund_specs() {
        let specs = BondFutureSpecs::bund();
        assert_eq!(specs.contract_size, 100_000.0);
        assert_eq!(specs.tick_size, 0.01);
        assert_eq!(specs.tick_value, 10.0);
        assert_eq!(specs.standard_coupon, 0.06);
        assert_eq!(specs.standard_maturity_years, 10.0);
        assert_eq!(specs.settlement_days, 2);
    }

    #[test]
    fn test_gilt_specs() {
        let specs = BondFutureSpecs::gilt();
        assert_eq!(specs.contract_size, 100_000.0);
        assert_eq!(specs.tick_size, 0.01);
        assert_eq!(specs.tick_value, 10.0);
        assert_eq!(specs.standard_coupon, 0.04);  // Different from UST/Bund
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

    #[test]
    fn test_validation_date_ordering_expiry_after_delivery() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        // expiry_date >= delivery_start (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date")) // Wrong: same as delivery_end
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("expiry_date") && err_msg.contains("delivery_start"));
    }

    #[test]
    fn test_validation_date_ordering_delivery_start_after_end() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        // delivery_start >= delivery_end (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date")) // Wrong: after delivery_end
            .delivery_end(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("delivery_start") && err_msg.contains("delivery_end"));
    }

    #[test]
    fn test_validation_empty_basket() {
        // Empty deliverable basket (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![]) // Invalid: empty
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("deliverable_basket") && err_msg.contains("empty"));
    }

    #[test]
    fn test_validation_ctd_not_in_basket() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        // CTD bond not in basket (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable])
            .ctd_bond_id(InstrumentId::new("UNKNOWN_BOND_ID")) // Invalid: not in basket
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("ctd_bond_id") && err_msg.contains("not found"));
    }

    #[test]
    fn test_validation_negative_conversion_factor() {
        let deliverable_valid = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };
        let deliverable_invalid = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG34"),
            conversion_factor: -0.5, // Invalid: negative
        };

        // Negative conversion factor (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable_valid, deliverable_invalid])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("conversion_factor") && err_msg.contains("positive"));
    }

    #[test]
    fn test_validation_zero_conversion_factor() {
        let deliverable_invalid = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.0, // Invalid: zero
        };

        // Zero conversion factor (invalid)
        let result = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"))
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable_invalid])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(Attributes::new())
            .try_build();

        assert!(result.is_err());
        let err_msg = format!("{}", result.expect_err("Should have validation error"));
        assert!(err_msg.contains("conversion_factor") && err_msg.contains("positive"));
    }

    #[test]
    fn test_validation_success() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        // All validations should pass
        let result = BondFutureBuilder::new()
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
            .try_build();

        assert!(result.is_ok());
        let future = result.expect("Should build valid BondFuture");
        assert_eq!(future.id.as_str(), "TYH5");
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    // Convenience constructor tests
    #[test]
    fn test_ust_10y_constructor() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFuture::ust_10y(
            InstrumentId::new("TYH5"),
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            125.50,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("US912828XG33"),
            CurveId::new("USD-TREASURY"),
        )
        .expect("Valid UST 10Y future");

        assert_eq!(future.id.as_str(), "TYH5");
        assert_eq!(future.quoted_price, 125.50);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.contract_specs.standard_coupon, 0.06);
        assert_eq!(future.contract_specs.standard_maturity_years, 10.0);
        assert_eq!(future.contract_specs.contract_size, 100_000.0);
        assert_eq!(future.contract_specs.tick_size, 1.0 / 32.0);
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_ust_5y_constructor() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.7890,
        };

        let future = BondFuture::ust_5y(
            InstrumentId::new("FVH5"),
            Money::new(500_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            118.75,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("US912828XG33"),
            CurveId::new("USD-TREASURY"),
        )
        .expect("Valid UST 5Y future");

        assert_eq!(future.id.as_str(), "FVH5");
        assert_eq!(future.quoted_price, 118.75);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.contract_specs.standard_coupon, 0.06);
        assert_eq!(future.contract_specs.standard_maturity_years, 5.0);
        assert_eq!(future.contract_specs.contract_size, 100_000.0);
        assert_eq!(future.contract_specs.tick_size, 1.0 / 128.0);
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_ust_2y_constructor() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.9123,
        };

        let future = BondFuture::ust_2y(
            InstrumentId::new("TUH5"),
            Money::new(400_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            105.25,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("US912828XG33"),
            CurveId::new("USD-TREASURY"),
        )
        .expect("Valid UST 2Y future");

        assert_eq!(future.id.as_str(), "TUH5");
        assert_eq!(future.quoted_price, 105.25);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.contract_specs.standard_coupon, 0.06);
        assert_eq!(future.contract_specs.standard_maturity_years, 2.0);
        assert_eq!(future.contract_specs.contract_size, 200_000.0); // 2Y is $200k
        assert_eq!(future.contract_specs.tick_size, 1.0 / 128.0);
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_bund_constructor() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("DE0001102473"),
            conversion_factor: 0.8567,
        };

        let future = BondFuture::bund(
            InstrumentId::new("FGBLH5"),
            Money::new(1_000_000.0, Currency::EUR),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            132.15,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("DE0001102473"),
            CurveId::new("EUR-BUNDS"),
        )
        .expect("Valid Bund future");

        assert_eq!(future.id.as_str(), "FGBLH5");
        assert_eq!(future.quoted_price, 132.15);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.contract_specs.standard_coupon, 0.06);
        assert_eq!(future.contract_specs.standard_maturity_years, 10.0);
        assert_eq!(future.contract_specs.contract_size, 100_000.0);
        assert_eq!(future.contract_specs.tick_size, 0.01); // Decimal, not 32nds
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_gilt_constructor() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("GB00B128DH60"),
            conversion_factor: 0.7234,
        };

        let future = BondFuture::gilt(
            InstrumentId::new("GILTH5"),
            Money::new(500_000.0, Currency::GBP),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            115.25,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("GB00B128DH60"),
            CurveId::new("GBP-GILTS"),
        )
        .expect("Valid Gilt future");

        assert_eq!(future.id.as_str(), "GILTH5");
        assert_eq!(future.quoted_price, 115.25);
        assert_eq!(future.position, Position::Long);
        assert_eq!(future.contract_specs.standard_coupon, 0.04); // 4%, not 6%
        assert_eq!(future.contract_specs.standard_maturity_years, 10.0);
        assert_eq!(future.contract_specs.contract_size, 100_000.0);
        assert_eq!(future.contract_specs.tick_size, 0.01);
        assert_eq!(future.deliverable_basket.len(), 1);
    }

    #[test]
    fn test_convenience_constructor_short_position() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFuture::ust_10y(
            InstrumentId::new("TYH5"),
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            125.50,
            Position::Short, // Short position
            vec![deliverable],
            InstrumentId::new("US912828XG33"),
            CurveId::new("USD-TREASURY"),
        )
        .expect("Valid short future");

        assert_eq!(future.position, Position::Short);
    }

    #[test]
    fn test_convenience_constructor_validation_error() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        // Invalid: expiry after delivery start
        let result = BondFuture::ust_10y(
            InstrumentId::new("TYH5"),
            Money::new(1_000_000.0, Currency::USD),
            Date::from_calendar_date(2025, Month::March, 25).expect("Valid date"), // After delivery_start
            Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"),
            125.50,
            Position::Long,
            vec![deliverable],
            InstrumentId::new("US912828XG33"),
            CurveId::new("USD-TREASURY"),
        );

        assert!(result.is_err());
    }
}

// Implement Instrument trait for BondFuture
impl crate::instruments::common::traits::Instrument for BondFuture {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::BondFuture
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
        _market: &finstack_core::market_data::context::MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        // Bond futures require the CTD bond and conversion factor to be provided explicitly
        // for pricing. Use the BondFuturePricer directly or the pricing registry with
        // proper CTD bond setup.
        Err(finstack_core::Error::Input(
            finstack_core::error::InputError::NotFound {
                id: format!(
                    "BondFuture::value() requires CTD bond ({}) and conversion factor. \
                     Use BondFuturePricer::calculate_npv() or the pricing registry instead.",
                    self.ctd_bond_id.as_str()
                ),
            },
        ))
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        // Similar to value(), bond futures need CTD bond and conversion factor
        // This will be properly implemented when we add the pricing registry integration
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
        )
    }

    fn required_discount_curves(&self) -> Vec<finstack_core::types::CurveId> {
        vec![self.discount_curve_id.clone()]
    }
}

// Implement HasDiscountCurve for BondFuture
impl crate::instruments::common::pricing::HasDiscountCurve for BondFuture {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculators
impl crate::instruments::common::traits::CurveDependencies for BondFuture {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

#[cfg(test)]
mod instrument_trait_tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_instrument_trait_key() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::traits::Instrument;
        use crate::pricer::InstrumentType;

        assert_eq!(future.key(), InstrumentType::BondFuture);
        assert_eq!(future.id(), "TYH5");
    }

    #[test]
    fn test_instrument_trait_id() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::traits::Instrument;

        assert_eq!(future.id(), "TYH5");
    }

    #[test]
    fn test_instrument_trait_attributes() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let attrs = Attributes::new()
            .with_tag("futures")
            .with_meta("exchange", "CBOT");

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
            .delivery_end(Date::from_calendar_date(2025, Month::March, 31).expect("Valid date"))
            .quoted_price(125.50)
            .position(Position::Long)
            .contract_specs(BondFutureSpecs::default())
            .deliverable_basket(vec![deliverable])
            .ctd_bond_id(InstrumentId::new("US912828XG33"))
            .discount_curve_id(CurveId::new("USD-TREASURY"))
            .attributes(attrs)
            .build()
            .expect("Valid bond future");

        use crate::instruments::common::traits::Instrument;

        assert!(future.attributes().has_tag("futures"));
        assert_eq!(future.attributes().get_meta("exchange"), Some("CBOT"));
    }

    #[test]
    fn test_instrument_trait_clone_box() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::traits::Instrument;

        let cloned = future.clone_box();
        assert_eq!(cloned.id(), "TYH5");
        assert_eq!(cloned.key(), crate::pricer::InstrumentType::BondFuture);
    }

    #[test]
    fn test_instrument_trait_as_any_downcast() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::traits::Instrument;

        let instrument: &dyn Instrument = &future;
        let concrete_future: Option<&BondFuture> = instrument.as_any().downcast_ref::<BondFuture>();
        assert!(concrete_future.is_some());
        assert_eq!(concrete_future.expect("Should be BondFuture").id.as_str(), "TYH5");
    }

    #[test]
    fn test_required_discount_curves() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::traits::Instrument;

        let curves = future.required_discount_curves();
        assert_eq!(curves.len(), 1);
        assert_eq!(curves[0].as_str(), "USD-TREASURY");
    }

    #[test]
    fn test_curve_dependencies() {
        let deliverable = DeliverableBond {
            bond_id: InstrumentId::new("US912828XG33"),
            conversion_factor: 0.8234,
        };

        let future = BondFutureBuilder::new()
            .id(InstrumentId::new("TYH5"))
            .notional(Money::new(1_000_000.0, Currency::USD))
            .expiry_date(Date::from_calendar_date(2025, Month::March, 20).expect("Valid date"))
            .delivery_start(
                Date::from_calendar_date(2025, Month::March, 21).expect("Valid date"),
            )
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

        use crate::instruments::common::pricing::HasDiscountCurve;
        use crate::instruments::common::traits::CurveDependencies;

        // Test HasDiscountCurve trait
        let discount_id = future.discount_curve_id();
        assert_eq!(discount_id.as_str(), "USD-TREASURY");

        // Test CurveDependencies trait
        let curves = future.curve_dependencies();
        assert_eq!(curves.discount_curves.len(), 1);
        assert_eq!(curves.discount_curves[0].as_str(), "USD-TREASURY");
        assert_eq!(curves.forward_curves.len(), 0);
        assert_eq!(curves.credit_curves.len(), 0);
        assert!(!curves.is_empty());
        assert_eq!(curves.len(), 1);
    }
}
