//! CDS Index types and implementations.
//!
//! This module defines the `CDSIndex` instrument along with its pricing
//! configuration and constituents. The index can be priced in two modes:
//! - `SingleCurve`: delegate to a synthetic single-name CDS priced off a
//!   single index hazard curve.
//! - `Constituents`: expand into per-name CDS positions with weights and
//!   aggregate results across names.

use crate::instruments::cds::CreditParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::money::Money;

// Reuse CDS components for conventions and legs
use crate::instruments::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive as CdsPayReceive, PremiumLegSpec,
    ProtectionLegSpec, SettlementType,
};

use super::parameters::CDSIndexConstituentParam;
use super::parameters::{CDSIndexConstructionParams, CDSIndexParams};

/// Pricing mode for CDS indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IndexPricing {
    /// Price the index against a single index hazard curve (synthetic CDS)
    SingleCurve,
    /// Price each issuer separately and aggregate by weight
    Constituents,
}

/// Constituent in a CDS index with weight and credit parameters.
#[derive(Clone, Debug)]
pub struct CDSIndexConstituent {
    /// Credit configuration for the issuer (includes hazard curve id and recovery)
    pub credit: CreditParams,
    /// Weight of the issuer in the index notional (e.g., 1/125.0 for CDX IG)
    pub weight: finstack_core::F,
}

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
    /// Index factor (fraction of surviving notional since series inception)
    pub index_factor: finstack_core::F,
    /// Protection buyer/seller perspective
    pub side: CdsPayReceive,
    /// Regional ISDA convention
    pub convention: CDSConvention,
    /// Premium leg specification (coupon schedule and discounting)
    pub premium: PremiumLegSpec,
    /// Protection leg specification (credit curve and settlement)
    pub protection: ProtectionLegSpec,
    /// Pricing aggregation mode
    pub pricing: IndexPricing,
    /// Optional list of constituents when using `IndexPricing::Constituents`
    pub constituents: Vec<CDSIndexConstituent>,
    /// Pricing overrides (including upfront payment)
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CDSIndex {
    /// Create a new CDS Index with standard ISDA conventions using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_standard(
        id: impl Into<String>,
        index_params: &CDSIndexParams,
        construction_params: &CDSIndexConstructionParams,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
        credit_params: &CreditParams,
        disc_id: &'static str,
        credit_id: &'static str,
    ) -> Self {
        let dc = construction_params.convention.day_count();
        let freq = construction_params.convention.frequency();
        let bdc = construction_params.convention.business_day_convention();
        let stub = construction_params.convention.stub_convention();

        let mut s = Self {
            id: id.into(),
            index_name: index_params.index_name.clone(),
            series: index_params.series,
            version: index_params.version,
            notional: construction_params.notional,
            index_factor: index_params.index_factor.unwrap_or(1.0),
            side: construction_params.side,
            convention: construction_params.convention,
            premium: PremiumLegSpec {
                start,
                end,
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: index_params.fixed_coupon_bp,
                disc_id,
            },
            protection: ProtectionLegSpec {
                credit_id,
                recovery_rate: credit_params.recovery_rate,
                settlement: SettlementType::Cash,
                settlement_delay: 3,
            },
            pricing: IndexPricing::SingleCurve,
            constituents: Vec::new(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        };

        if let Some(cons) = &index_params.constituents {
            if !cons.is_empty() {
                s.pricing = IndexPricing::Constituents;
                s.constituents = cons
                    .iter()
                    .map(|c: &CDSIndexConstituentParam| CDSIndexConstituent {
                        credit: c.credit.clone(),
                        weight: c.weight,
                    })
                    .collect();
            }
        }

        s
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

    /// Configure equal-weight constituents by credit parameter set per name.
    /// Each name receives weight = 1/(n names). Mode switches to `Constituents`.
    pub fn with_constituents_equal_weight(
        mut self,
        names: impl IntoIterator<Item = CreditParams>,
    ) -> Self {
        let list: Vec<CreditParams> = names.into_iter().collect();
        if list.is_empty() {
            self.constituents.clear();
            self.pricing = IndexPricing::SingleCurve;
            return self;
        }
        let w = 1.0 / (list.len() as finstack_core::F);
        self.constituents = list
            .into_iter()
            .map(|credit| CDSIndexConstituent { credit, weight: w })
            .collect();
        self.pricing = IndexPricing::Constituents;
        self
    }

    /// Configure explicit constituents with custom weights.
    pub fn with_constituents(mut self, constituents: Vec<CDSIndexConstituent>) -> Self {
        if constituents.is_empty() {
            self.constituents.clear();
            self.pricing = IndexPricing::SingleCurve;
        } else {
            self.constituents = constituents;
            self.pricing = IndexPricing::Constituents;
        }
        self
    }
}

impl_instrument!(
    CDSIndex,
    "CDSIndex",
    pv = |s, curves, as_of| {
        // Delegate to the CDS Index pricing engine so pricing mode is honored.
        let pricer = crate::instruments::cds_index::pricing::CDSIndexPricer::new();
        pricer.npv(s, curves, as_of)
    }
);
