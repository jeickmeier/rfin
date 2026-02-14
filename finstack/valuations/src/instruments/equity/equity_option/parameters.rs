//! Equity option specific parameters.

use crate::instruments::{ExerciseStyle, OptionType, SettlementType};
use finstack_core::dates::Date;
use finstack_core::money::Money;

/// Equity option specific parameters.
///
/// Groups parameters specific to equity options.
#[derive(Debug, Clone)]
pub struct EquityOptionParams {
    /// Strike price in underlying price units
    pub strike: f64,
    /// Option expiry date
    pub expiry: Date,
    /// Option type (Call/Put)
    pub option_type: OptionType,
    /// Exercise style (European/American/Bermudan)
    pub exercise_style: ExerciseStyle,
    /// Settlement type (Cash/Physical)
    pub settlement: SettlementType,
    /// Notional amount for valuation scaling.
    pub notional: Money,
}

impl EquityOptionParams {
    /// Create new equity option parameters
    pub fn new(strike: f64, expiry: Date, option_type: OptionType, notional: Money) -> Self {
        Self {
            strike,
            expiry,
            option_type,
            exercise_style: ExerciseStyle::European,
            settlement: SettlementType::Physical,
            notional,
        }
    }

    /// Create European call option parameters
    pub fn european_call(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Call, notional)
    }

    /// Create European put option parameters
    pub fn european_put(strike: f64, expiry: Date, notional: Money) -> Self {
        Self::new(strike, expiry, OptionType::Put, notional)
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
