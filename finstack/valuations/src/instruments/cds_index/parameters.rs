//! CDS Index specific parameters.

use crate::instruments::cds::{CDSConvention, PayReceive};
use crate::instruments::cds::CreditParams;
use finstack_core::money::Money;
use finstack_core::F;

/// Constituent definition for CDS Index parameters (credit + weight).
#[derive(Clone, Debug)]
pub struct CDSIndexConstituentParam {
    /// Credit configuration for the issuer
    pub credit: CreditParams,
    /// Weight in the index notional (sum across names typically = 1.0)
    pub weight: F,
}

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
    /// Optional basket of underlying issuers (credit params + weights)
    pub constituents: Option<Vec<CDSIndexConstituentParam>>,
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
            constituents: None,
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

    /// Attach explicit constituents to these params.
    pub fn with_constituents(mut self, constituents: Vec<CDSIndexConstituentParam>) -> Self {
        self.constituents = if constituents.is_empty() { None } else { Some(constituents) };
        self
    }

    /// Attach equal-weight constituents from a list of credit params.
    pub fn with_constituents_equal_weight(
        mut self,
        names: impl IntoIterator<Item = CreditParams>,
    ) -> Self {
        let list: Vec<CreditParams> = names.into_iter().collect();
        if list.is_empty() {
            self.constituents = None;
            return self;
        }
        let w = 1.0 / (list.len() as F);
        let cons = list
            .into_iter()
            .map(|credit| CDSIndexConstituentParam { credit, weight: w })
            .collect();
        self.constituents = Some(cons);
        self
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
    pub fn new(notional: Money, side: PayReceive, convention: CDSConvention) -> Self {
        Self {
            notional,
            side,
            convention,
        }
    }

    /// Create standard protection buyer parameters
    pub fn buy_protection(notional: Money) -> Self {
        Self::new(notional, PayReceive::PayProtection, CDSConvention::IsdaNa)
    }
}
