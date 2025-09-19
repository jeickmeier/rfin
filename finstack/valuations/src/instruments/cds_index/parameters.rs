//! CDS Index specific parameters.

use crate::instruments::cds::{CDSConvention, PayReceive};
use finstack_core::money::Money;
use finstack_core::F;

/// CDS Index specific parameters.
///
/// Groups parameters specific to CDS indices.
#[derive(Clone, Debug)]
pub struct CDSIndexParams {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe")
    pub index_name: String,
    /// Index series number
    pub series: u16,
    /// Index version number
    pub version: u16,
    /// Fixed coupon in basis points
    pub fixed_coupon_bp: F,
}

impl CDSIndexParams {
    /// Create new CDS index parameters
    pub fn new(
        index_name: impl Into<String>,
        series: u16,
        version: u16,
        fixed_coupon_bp: F,
    ) -> Self {
        Self {
            index_name: index_name.into(),
            series,
            version,
            fixed_coupon_bp,
        }
    }

    /// Create CDX North America Investment Grade parameters
    pub fn cdx_na_ig(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("CDX.NA.IG", series, version, fixed_coupon_bp)
    }

    /// Create CDX North America High Yield parameters
    pub fn cdx_na_hy(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("CDX.NA.HY", series, version, fixed_coupon_bp)
    }

    /// Create iTraxx Europe parameters
    pub fn itraxx_europe(series: u16, version: u16, fixed_coupon_bp: F) -> Self {
        Self::new("iTraxx Europe", series, version, fixed_coupon_bp)
    }
}

/// Complete CDS Index construction parameters.
///
/// Groups all parameters needed for CDS Index construction to reduce argument count.
#[derive(Clone, Debug)]
pub struct CDSIndexConstructionParams {
    /// Notional amount
    pub notional: Money,
    /// Protection side (pay/receive)
    pub side: PayReceive,
    /// CDS convention
    pub convention: CDSConvention,
}

impl CDSIndexConstructionParams {
    /// Create new CDS index construction parameters
    pub fn new(
        notional: Money,
        side: PayReceive,
        convention: CDSConvention,
    ) -> Self {
        Self {
            notional,
            side,
            convention,
        }
    }

    /// Create standard protection buyer parameters
    pub fn buy_protection(notional: Money) -> Self {
        Self::new(
            notional,
            PayReceive::PayProtection,
            CDSConvention::IsdaNa,
        )
    }
}
