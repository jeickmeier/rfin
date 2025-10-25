//! Credit option specific parameters.

use crate::instruments::common::parameters::OptionType;
use finstack_core::{dates::Date, money::Money};

/// Credit option specific parameters.
///
/// Deal-level inputs for an option on a CDS spread.
/// Ownership clarifications to avoid duplication with `CreditParams`:
/// - This struct holds strike (bp), expiry, underlying CDS maturity, notional, option type.
/// - Reference entity, recovery rate, and hazard `credit_id` live in `CreditParams`.
/// - Discount `disc_id` and vol `vol_id` are instrument-level market IDs passed to `CdsOption::new`.
///
/// Cds Option Params structure.
#[derive(Clone, Debug)]
pub struct CdsOptionParams {
    /// Strike spread in basis points
    pub strike_spread_bp: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Notional amount
    pub notional: Money,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Whether the underlying is a CDS index (vs single-name CDS)
    pub underlying_is_index: bool,
    /// Optional index factor scaling for index underlyings (e.g., 0.8)
    pub index_factor: Option<f64>,
    /// Forward spread adjustment in bp (e.g., to reflect front-end protection on indices)
    pub forward_spread_adjust_bp: f64,
}

impl CdsOptionParams {
    /// Create new credit option parameters
    pub fn new(
        strike_spread_bp: f64,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
        option_type: OptionType,
    ) -> Self {
        Self {
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            option_type,
            underlying_is_index: false,
            index_factor: None,
            forward_spread_adjust_bp: 0.0,
        }
    }

    /// Create credit call option parameters (option to buy protection)
    pub fn call(strike_spread_bp: f64, expiry: Date, cds_maturity: Date, notional: Money) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            OptionType::Call,
        )
    }

    /// Create credit put option parameters (option to sell protection)
    pub fn put(strike_spread_bp: f64, expiry: Date, cds_maturity: Date, notional: Money) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            OptionType::Put,
        )
    }

    /// Mark this option as referencing a CDS index and set an index factor.
    pub fn as_index(mut self, index_factor: f64) -> Self {
        self.underlying_is_index = true;
        self.index_factor = Some(index_factor);
        self
    }

    /// Apply a forward spread adjustment in bp (e.g., to reflect FEP for index options).
    pub fn with_forward_spread_adjust_bp(mut self, adjust_bp: f64) -> Self {
        self.forward_spread_adjust_bp = adjust_bp;
        self
    }
}
