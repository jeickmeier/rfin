//! CDS Index types and implementations.

use crate::instruments::common::{CDSIndexConstructionParams, CDSIndexParams, CreditParams, DateRange, MarketRefs, PricingOverrides};
use crate::instruments::traits::Attributes;
use finstack_core::money::Money;

// Reuse CDS components for conventions and legs
use crate::instruments::fixed_income::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive as CdsPayReceive, PremiumLegSpec,
    ProtectionLegSpec, SettlementType,
};

/// CDS Index instrument definition
#[derive(Clone, Debug, finstack_macros::FinancialBuilder)]
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
    /// Pricing overrides (including upfront payment)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CDSIndex {

    /// Create a new CDS Index with standard ISDA conventions using parameter structs
    pub fn new_standard(
        id: impl Into<String>,
        index_params: &CDSIndexParams,
        construction_params: &CDSIndexConstructionParams,
        date_range: &DateRange,
        credit_params: &CreditParams,
        market_refs: &MarketRefs,
    ) -> Self {
        let dc = construction_params.convention.day_count();
        let freq = construction_params.convention.frequency();
        let bdc = construction_params.convention.business_day_convention();
        let stub = construction_params.convention.stub_convention();

        let credit_id = market_refs
            .credit_id
            .as_ref()
            .expect("Credit curve required for CDS index");

        Self {
            id: id.into(),
            index_name: index_params.index_name.clone(),
            series: index_params.series,
            version: index_params.version,
            notional: construction_params.notional,
            side: construction_params.side,
            convention: construction_params.convention,
            premium: PremiumLegSpec {
                start: date_range.start,
                end: date_range.end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: index_params.fixed_coupon_bp,
                disc_id: Box::leak(market_refs.disc_id.to_string().into_boxed_str()),
            },
            protection: ProtectionLegSpec {
                credit_id: Box::leak(credit_id.to_string().into_boxed_str()),
                recovery_rate: credit_params.recovery_rate,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            },
            pricing_overrides: PricingOverrides::default(),
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
            pricing_overrides: self.pricing_overrides.clone(),
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
