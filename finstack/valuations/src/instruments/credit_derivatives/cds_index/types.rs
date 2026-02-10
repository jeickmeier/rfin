//! CDS Index types and implementations.
//!
//! This module defines the `CDSIndex` instrument along with its pricing
//! configuration and constituents. The index can be priced in two modes:
//! - `SingleCurve`: delegate to a synthetic single-name CDS priced off a
//!   single index hazard curve.
//! - `Constituents`: expand into per-name CDS positions with weights and
//!   aggregate results across names.

use crate::instruments::common_impl::dependencies::MarketDependencies;
use crate::instruments::common_impl::parameters::CreditParams;
use crate::instruments::common_impl::traits::Attributes;
use crate::instruments::PricingOverrides;
use crate::margin::types::OtcMarginSpec;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use rust_decimal::Decimal;
use time::macros::date;

// Reuse CDS components for conventions and legs
use crate::instruments::credit_derivatives::cds::{
    CDSConvention, CreditDefaultSwap, PayReceive, PremiumLegSpec, ProtectionLegSpec,
};

use super::parameters::CDSIndexConstituentParam;
use super::parameters::{CDSIndexConstructionParams, CDSIndexParams};

/// Pricing mode for CDS indices.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IndexPricing {
    /// Price the index against a single index hazard curve (synthetic CDS)
    SingleCurve,
    /// Price each issuer separately and aggregate by weight
    Constituents,
}

/// Par spread denominator method for indices in constituents mode.
/// Method for computing par spread of a CDS index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParSpreadMethod {
    /// Par spread computed using risky annuity (RPV01) method
    RiskyAnnuity,
    /// Par spread with full premium and accrual-on-default
    FullPremiumAoD,
}

/// Constituent in a CDS index with weight and credit parameters.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CDSIndexConstituent {
    /// Credit configuration for the issuer (includes hazard curve id and recovery)
    pub credit: CreditParams,
    /// Weight of the issuer in the index notional (e.g., 1/125.0 for CDX IG)
    pub weight: f64,
    /// Whether the constituent has defaulted. Defaulted names are excluded from the
    /// premium leg but their settled protection payment is already reflected in `index_factor`.
    /// Per O'Kane (2008) Ch. 7: "On default, the protection payment is settled and the
    /// name is removed from the index. The index factor adjusts to reflect the reduced notional."
    #[serde(default)]
    pub defaulted: bool,
}

/// Per-constituent result entry for index-level analytics.
#[derive(Debug, Clone)]
pub struct ConstituentResult<T> {
    /// Hazard curve identifier for the constituent.
    pub credit_curve_id: CurveId,
    /// Recovery rate used for the constituent.
    pub recovery_rate: f64,
    /// Raw weight supplied on the index definition.
    pub weight_raw: f64,
    /// Effective weight used after optional normalization.
    pub weight_effective: f64,
    /// Computed value for the constituent.
    pub value: T,
}

/// Aggregate result for index-level analytics.
///
/// In `SingleCurve` mode, `constituents` is empty.
#[derive(Debug, Clone)]
pub struct IndexResult<T> {
    /// Total aggregated value.
    pub total: T,
    /// Optional per-constituent breakdown.
    pub constituents: Vec<ConstituentResult<T>>,
}

impl<T> IndexResult<T> {
    /// Construct a single-curve result with no breakdown.
    pub fn single_curve(total: T) -> Self {
        Self {
            total,
            constituents: Vec::new(),
        }
    }
}

/// Detailed par spread result for CDS indices.
///
/// Note: constituent par spreads are informational and are not additive.
#[derive(Debug, Clone)]
pub struct IndexParSpreadResult {
    /// Total par spread in basis points.
    pub total_spread_bp: f64,
    /// Per-constituent par spreads in basis points (informational).
    pub constituents_spread_bp: Vec<ConstituentResult<f64>>,
    /// Par spread denominator methodology.
    pub method: ParSpreadMethod,
    /// Aggregated protection PV used in the total calculation.
    pub numerator_protection_pv: Money,
    /// Aggregated denominator used in the total calculation.
    pub denominator: f64,
}

/// CDS Index instrument definition
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
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
    /// Optional OTC margin specification for VM/IM.
    ///
    /// CDS indices are typically cleared through ICE Clear Credit.
    /// Use `OtcMarginSpec::ice_clear_credit()` for standard cleared indices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_spec: Option<OtcMarginSpec>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
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
                start: date!(2024 - 03 - 20),
                end: date!(2029 - 12 - 20),
                freq,
                stub,
                bdc,
                calendar_id: None,
                dc,
                spread_bp: Decimal::from(60),
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
            margin_spec: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a new CDS Index with standard ISDA conventions using parameter structs.
    ///
    /// # Errors
    ///
    /// Returns an error if `fixed_coupon_bp` cannot be represented as Decimal.
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
    ) -> finstack_core::Result<Self> {
        let dc = construction_params.convention.day_count();
        let freq = construction_params.convention.frequency();
        let bdc = construction_params.convention.business_day_convention();
        let stub = construction_params.convention.stub_convention();

        let spread_bp_decimal = Decimal::try_from(index_params.fixed_coupon_bp).map_err(|e| {
            finstack_core::Error::Validation(format!(
                "fixed_coupon_bp {} cannot be represented as Decimal: {}",
                index_params.fixed_coupon_bp, e
            ))
        })?;

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
                spread_bp: spread_bp_decimal,
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
            margin_spec: None,
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
                        defaulted: false,
                    })
                    .collect();
            }
        }

        Ok(s)
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
            upfront: None,
            margin_spec: self.margin_spec.clone(),
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
            .map(|credit| CDSIndexConstituent {
                credit,
                weight: w,
                defaulted: false,
            })
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

    /// Calculate protection leg PV
    pub fn pv_protection_leg(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_protection_leg(self, curves, as_of)
    }

    /// Calculate premium leg PV
    pub fn pv_premium_leg(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_premium_leg(self, curves, as_of)
    }

    /// Calculate par spread
    pub fn par_spread(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.par_spread(self, curves, as_of)
    }

    /// Calculate risky PV01
    pub fn risky_pv01(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.risky_pv01(self, curves, as_of)
    }

    /// Calculate CS01
    pub fn cs01(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<f64> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.cs01(self, curves, as_of)
    }

    /// Calculate NPV with per-constituent breakdown (if applicable).
    pub fn npv_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexResult<Money>> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.npv_detailed(self, curves, as_of)
    }

    /// Calculate protection leg PV with per-constituent breakdown.
    pub fn pv_protection_leg_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexResult<Money>> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_protection_leg_detailed(self, curves, as_of)
    }

    /// Calculate premium leg PV with per-constituent breakdown.
    pub fn pv_premium_leg_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexResult<Money>> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.pv_premium_leg_detailed(self, curves, as_of)
    }

    /// Calculate par spread with per-constituent breakdown.
    pub fn par_spread_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexParSpreadResult> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.par_spread_detailed(self, curves, as_of)
    }

    /// Calculate risky PV01 with per-constituent breakdown.
    pub fn risky_pv01_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexResult<f64>> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.risky_pv01_detailed(self, curves, as_of)
    }

    /// Calculate CS01 with per-constituent breakdown.
    pub fn cs01_detailed(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<IndexResult<f64>> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.cs01_detailed(self, curves, as_of)
    }
}

impl crate::instruments::common_impl::traits::Instrument for CDSIndex {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDSIndex
    }

    fn as_any(&self) -> &dyn ::std::any::Any {
        self
    }

    fn as_marginable(&self) -> Option<&dyn crate::margin::traits::Marginable> {
        Some(self)
    }

    fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
        &mut self.attributes
    }

    fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
        Box::new(self.clone())
    }

    fn market_dependencies(&self) -> finstack_core::Result<MarketDependencies> {
        MarketDependencies::from_curve_dependencies(self)
    }

    fn value(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        let pricer =
            crate::instruments::credit_derivatives::cds_index::pricer::CDSIndexPricer::new();
        pricer.npv(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        crate::instruments::common_impl::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(curves.clone()),
            as_of,
            base_value,
            metrics,
            None,
            None,
        )
    }

    fn expiry(&self) -> Option<finstack_core::dates::Date> {
        Some(self.premium.end)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        Some(self.premium.start)
    }
}

// Implement CurveDependencies for DV01 calculator.
// In Constituents mode, include per-constituent credit curves so that DV01/BucketedDV01
// correctly bump all credit curves, not just the index-level one.
impl crate::instruments::common_impl::traits::CurveDependencies for CDSIndex {
    fn curve_dependencies(
        &self,
    ) -> finstack_core::Result<crate::instruments::common_impl::traits::InstrumentCurves> {
        let mut builder = crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.premium.discount_curve_id.clone())
            .credit(self.protection.credit_curve_id.clone());

        if self.pricing == IndexPricing::Constituents {
            for constituent in &self.constituents {
                if !constituent.defaulted {
                    builder = builder.credit(constituent.credit.credit_curve_id.clone());
                }
            }
        }

        builder.build()
    }
}
