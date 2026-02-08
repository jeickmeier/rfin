//! Equity option specific parameters.

use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Equity option specific parameters.
///
/// Groups parameters specific to equity options, including Money-denominated strike.
#[derive(Debug, Clone)]
pub struct EquityOptionParams {
    /// Strike price in Money (includes currency)
    pub strike: Money,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: SettlementType,
    /// Contract size (shares per contract)
    pub contract_size: f64,
}

impl EquityOptionParams {
    /// Create new equity option parameters
    pub fn new(strike: Money, expiry: Date, option_type: OptionType, contract_size: f64) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Physical,
            contract_size,
        }
    }

    /// Create European call option parameters
    pub fn european_call(strike: Money, expiry: Date, contract_size: f64) -> Self {
        Self::new(strike, expiry, OptionType::Call, contract_size)
    }

    /// Create European put option parameters
    pub fn european_put(strike: Money, expiry: Date, contract_size: f64) -> Self {
        Self::new(strike, expiry, OptionType::Put, contract_size)
    }

    /// Set exercise style
    pub fn with_exercise_style(mut self, style: ExerciseStyle) -> Self {
        self.exercise_style = style;
        self
    }

    /// Set settlement type
    pub fn with_settlement(mut self, settlement: SettlementType) -> Self {
        self.settlement = settlement;
        self
    }
}
