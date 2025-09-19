//! Credit option specific parameters.

use crate::instruments::OptionType;
use finstack_core::{dates::Date, money::Money, F};

/// Credit option specific parameters.
///
/// Groups parameters specific to credit options (options on CDS).
#[derive(Clone, Debug)]
pub struct CreditOptionParams {
    /// Strike spread in basis points
    pub strike_spread_bp: F,
    /// Option expiry date
    pub expiry: Date,
    /// Underlying CDS maturity date
    pub cds_maturity: Date,
    /// Notional amount
    pub notional: Money,
    /// Option type (Call/Put)
    pub option_type: OptionType,
}

impl CreditOptionParams {
    /// Create new credit option parameters
    pub fn new(
        strike_spread_bp: F,
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
        }
    }

    /// Create credit call option parameters (option to buy protection)
    pub fn call(
        strike_spread_bp: F,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            OptionType::Call,
        )
    }

    /// Create credit put option parameters (option to sell protection)
    pub fn put(
        strike_spread_bp: F,
        expiry: Date,
        cds_maturity: Date,
        notional: Money,
    ) -> Self {
        Self::new(
            strike_spread_bp,
            expiry,
            cds_maturity,
            notional,
            OptionType::Put,
        )
    }
}
