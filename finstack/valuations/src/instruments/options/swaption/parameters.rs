//! Swaption-specific parameters.

use crate::instruments::fixed_income::irs::PayReceive;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

/// Swaption-specific parameters.
///
/// Groups swaption parameters beyond basic option parameters.
#[derive(Clone, Debug)]
pub struct SwaptionParams {
    /// Notional amount
    pub notional: Money,
    /// Strike rate (fixed rate)
    pub strike_rate: F,
    /// Swaption expiry date
    pub expiry: Date,
    /// Underlying swap start date
    pub swap_start: Date,
    /// Underlying swap end date
    pub swap_end: Date,
    /// Payer/receiver side
    pub side: PayReceive,
}

impl SwaptionParams {
    /// Create payer swaption parameters
    pub fn payer(
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::PayFixed,
        }
    }

    /// Create receiver swaption parameters
    pub fn receiver(
        notional: Money,
        strike_rate: F,
        expiry: Date,
        swap_start: Date,
        swap_end: Date,
    ) -> Self {
        Self {
            notional,
            strike_rate,
            expiry,
            swap_start,
            swap_end,
            side: PayReceive::ReceiveFixed,
        }
    }
}
