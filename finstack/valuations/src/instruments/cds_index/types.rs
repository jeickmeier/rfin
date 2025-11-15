//! CDS Index types and implementations.
//!
//! This module defines the `CDSIndex` instrument along with its pricing
//! configuration and constituents. The index can be priced in two modes:
//! - `SingleCurve`: delegate to a synthetic single-name CDS priced off a
//!   single index hazard curve.
//! - `Constituents`: expand into per-name CDS positions with weights and
//!   aggregate results across names.

use crate::instruments::common::parameters::CreditParams;
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

// Reuse CDS components for conventions and legs
use crate::instruments::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive, PremiumLegSpec, ProtectionLegSpec,
};

use super::parameters::CDSIndexConstituentParam;
use super::parameters::{CDSIndexConstructionParams, CDSIndexParams};

/// Pricing mode for CDS indices.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IndexPricing {
    /// Price the index against a single index hazard curve (synthetic CDS)
    SingleCurve,
    /// Price each issuer separately and aggregate by weight
    Constituents,
}

/// Constituent in a CDS index with weight and credit parameters.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CDSIndexConstituent {
    /// Credit configuration for the issuer (includes hazard curve id and recovery)
    pub credit: CreditParams,
    /// Weight of the issuer in the index notional (e.g., 1/125.0 for CDX IG)
    pub weight: f64,
}

/// CDS Index instrument definition
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CDSIndex {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Index name, e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx Europe"
    pub index_name: String,
    /// Series number (e.g., 42)
    pub series: u16,
    /// Version number within series
    pub version: u16,
    /// Notional amount of the index
    pub notional: Money,
    /// Index factor (fraction of surviving notional since series inception)
    pub index_factor: f64,
    /// Protection buyer/seller perspective
    pub side: PayReceive,
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

// Implement HasCreditCurve for generic CS01 calculator
impl crate::metrics::HasCreditCurve for CDSIndex {
    fn credit_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.protection.credit_curve_id
    }
}

impl CDSIndex {
    /// Create a canonical example CDS Index for testing and documentation.
    ///
    /// Returns a CDX.NA.IG series 42 index with standard conventions.
    pub fn example() -> Self {
        let convention = CDSConvention::IsdaNa;
        let dc = convention.day_count();
        let freq = convention.frequency();
        let bdc = convention.business_day_convention();
        let stub = convention.stub_convention();

        Self {
            id: InstrumentId::new("CDX-IG-42"),
            index_name: "CDX.NA.IG".to_string(),
            series: 42,
            version: 1,
            notional: Money::new(10_000_000.0, Currency::USD),
            index_factor: 1.0,
            side: PayReceive::PayFixed,
            convention,
            premium: PremiumLegSpec {
                start: Date::from_calendar_date(2024, time::Month::March, 20).unwrap(),
                end: Date::from_calendar_date(2029, time::Month::December, 20).unwrap(),
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: 60.0,
                discount_curve_id: CurveId::new("USD-OIS"),
            },
            protection: ProtectionLegSpec {
                credit_curve_id: CurveId::new("CDX.NA.IG.HAZARD"),
                recovery_rate: 0.40,
                settlement_delay: convention.settlement_delay(),
            },
            pricing: IndexPricing::SingleCurve,
            constituents: Vec::new(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Attributes::new(),
        }
    }

    /// Create a new CDS Index with standard ISDA conventions using parameter structs
    #[allow(clippy::too_many_arguments)]
    pub fn new_standard(
        id: impl Into<InstrumentId>,
        index_params: &CDSIndexParams,
        construction_params: &CDSIndexConstructionParams,
        start: finstack_core::dates::Date,
        end: finstack_core::dates::Date,
        credit_params: &CreditParams,
        discount_curve_id: impl Into<CurveId>,
        credit_id: impl Into<CurveId>,
    ) -> Self {
        let dc = construction_params.convention.day_count();
        let freq = construction_params.convention.frequency();
        let bdc = construction_params.convention.business_day_convention();
        let stub = construction_params.convention.stub_convention();

        let mut s = Self {
            id: id.into(),
            index_name: index_params.index_name.to_owned(),
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
                discount_curve_id: discount_curve_id.into(),
            },
            protection: ProtectionLegSpec {
                credit_curve_id: credit_id.into(),
                recovery_rate: credit_params.recovery_rate,
                settlement_delay: construction_params.convention.settlement_delay(),
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
            id: self.id.to_owned(),
            notional: self.notional,
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
        let w = 1.0 / (list.len() as f64);
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

    /// Calculate the net present value of this CDS Index
    pub fn npv(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.npv(self, curves, as_of)
    }

    /// Calculate protection leg PV
    pub fn pv_protection_leg(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_protection_leg(self, curves, as_of)
    }

    /// Calculate premium leg PV
    pub fn pv_premium_leg(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_premium_leg(self, curves, as_of)
    }

    /// Calculate par spread
    pub fn par_spread(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.par_spread(self, curves, as_of)
    }

    /// Calculate risky PV01
    pub fn risky_pv01(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.risky_pv01(self, curves, as_of)
    }

    /// Calculate CS01
    pub fn cs01(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer = crate::instruments::cds_index::pricer::CDSIndexPricer::new();
        pricer.cs01(self, curves, as_of)
    }
}

impl crate::instruments::common::traits::Instrument for CDSIndex {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDSIndex
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn attributes(&self) -> &crate::instruments::common::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common::traits::Instrument> {
        Box::new(self.clone())
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
        )
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CDSIndex {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.premium.discount_curve_id
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common::traits::CurveDependencies for CDSIndex {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.premium.discount_curve_id.clone())
            .credit(self.protection.credit_curve_id.clone())
            .build()
    }
}
