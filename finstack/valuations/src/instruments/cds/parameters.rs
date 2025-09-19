//! CDS construction parameters.

use finstack_core::{money::Money, F};

use super::types::{CDSConvention, PayReceive};

/// Complete CDS construction parameters.
///
/// Groups all parameters needed for CDS construction to reduce argument count.
#[derive(Clone, Debug)]
pub struct CDSConstructionParams {
    /// Notional amount
    pub notional: Money,
    /// Protection side (pay/receive)
    pub side: PayReceive,
    /// CDS convention
    pub convention: CDSConvention,
    /// Spread in basis points
    pub spread_bp: F,
}

impl CDSConstructionParams {
    /// Create new CDS construction parameters
    pub fn new(
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
        spread_bp: F,
    ) -> Self {
        Self {
            notional,
            side,
            convention,
            spread_bp,
        }
    }

    /// Create standard protection buyer parameters
    pub fn buy_protection(
        notional: Money,
        spread_bp: F,
    ) -> Self {
        Self::new(
            notional,
            PayReceive::PayProtection,
            CDSConvention::IsdaNa,
            spread_bp,
        )
    }

    /// Create standard protection seller parameters
    pub fn sell_protection(
        notional: Money,
        spread_bp: F,
    ) -> Self {
        Self::new(
            notional,
            PayReceive::ReceiveProtection,
            CDSConvention::IsdaNa,
            spread_bp,
        )
    }
}
