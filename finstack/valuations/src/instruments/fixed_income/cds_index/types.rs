//! CDS Index types and implementations.

use crate::instruments::traits::Attributes;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::F;

// Reuse CDS components for conventions and legs
use crate::instruments::fixed_income::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive as CdsPayReceive, PremiumLegSpec,
    ProtectionLegSpec, SettlementType,
};

/// CDS Index instrument definition
#[derive(Clone, Debug)]
pub struct CDSIndex {
    /// Unique instrument identifier
    pub id: String,
    /// Index name, e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx Europe"
    pub index_name: String,
    /// Series number (e.g., 42)
    pub series: u16,
    /// Version number within series
    pub version: u16,
    /// Notional amount of the index
    pub notional: Money,
    /// Protection buyer/seller perspective
    pub side: CdsPayReceive,
    /// Regional ISDA convention
    pub convention: CDSConvention,
    /// Premium leg specification (coupon schedule and discounting)
    pub premium: PremiumLegSpec,
    /// Protection leg specification (credit curve and settlement)
    pub protection: ProtectionLegSpec,
    /// Optional upfront payment
    pub upfront: Option<Money>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CDSIndex {
    /// Create a new CDS Index builder
    pub fn builder() -> crate::instruments::fixed_income::cds_index::builder::CDSIndexBuilder {
        crate::instruments::fixed_income::cds_index::builder::CDSIndexBuilder::new()
    }

    /// Create a new CDS Index with standard ISDA conventions
    #[allow(clippy::too_many_arguments)]
    pub fn new_standard(
        id: impl Into<String>,
        index_name: impl Into<String>,
        series: u16,
        version: u16,
        notional: Money,
        side: CdsPayReceive,
        convention: CDSConvention,
        start: Date,
        end: Date,
        fixed_coupon_bp: F,
        credit_id: &'static str,
        recovery_rate: F,
        disc_id: &'static str,
    ) -> Self {
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: id.into(),
            index_name: index_name.into(),
            series,
            version,
            notional,
            side,
            convention,
            premium: PremiumLegSpec {
                start,
                end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: fixed_coupon_bp,
                disc_id,
            },
            protection: ProtectionLegSpec {
                credit_id,
                recovery_rate,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            },
            upfront: None,
            attributes: Attributes::new(),
        }
    }

    /// Map this index to a synthetic single-name CDS for valuation reuse
    pub fn to_synthetic_cds(&self) -> CreditDefaultSwap {
        CreditDefaultSwap {
            id: self.id.clone(),
            notional: self.notional,
            reference_entity: self.index_name.clone(),
            side: self.side,
            convention: self.convention,
            premium: self.premium.clone(),
            protection: self.protection.clone(),
            upfront: self.upfront,
            attributes: self.attributes.clone(),
        }
    }
}

impl_instrument!(
    CDSIndex,
    "CDSIndex",
    pv = |s, curves, as_of| {
        let cds = s.to_synthetic_cds();
        cds.value(curves, as_of)
    }
);
