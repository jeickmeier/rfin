//! Agency CMO types.
//!
//! Collateralized Mortgage Obligations (CMOs) are structured products that
//! redistribute the cashflows from underlying MBS pools into tranches with
//! different risk/return profiles.

use crate::instruments::agency_mbs_passthrough::{AgencyMbsPassthrough, AgencyProgram};
use crate::instruments::common::traits::Attributes;
use crate::instruments::PricingOverrides;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

/// CMO tranche type enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CmoTrancheType {
    /// Sequential pay - receives principal in order
    Sequential,
    /// PAC (Planned Amortization Class) - protected by support
    Pac,
    /// Support/Companion - absorbs prepayment variability
    Support,
    /// Interest-Only strip
    InterestOnly,
    /// Principal-Only strip
    PrincipalOnly,
}

impl std::fmt::Display for CmoTrancheType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmoTrancheType::Sequential => write!(f, "SEQ"),
            CmoTrancheType::Pac => write!(f, "PAC"),
            CmoTrancheType::Support => write!(f, "SUP"),
            CmoTrancheType::InterestOnly => write!(f, "IO"),
            CmoTrancheType::PrincipalOnly => write!(f, "PO"),
        }
    }
}

/// PAC collar boundaries.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PacCollar {
    /// Lower PSA bound
    pub lower_psa: f64,
    /// Upper PSA bound
    pub upper_psa: f64,
}

impl PacCollar {
    /// Create a standard PAC collar.
    pub fn new(lower_psa: f64, upper_psa: f64) -> Self {
        Self {
            lower_psa,
            upper_psa,
        }
    }

    /// Standard 100-300 PSA collar.
    pub fn standard() -> Self {
        Self::new(1.0, 3.0)
    }
}

/// CMO tranche definition.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CmoTranche {
    /// Tranche identifier (e.g., "A", "B", "IO")
    pub id: String,
    /// Tranche type
    pub tranche_type: CmoTrancheType,
    /// Original face amount
    pub original_face: Money,
    /// Current face amount
    pub current_face: Money,
    /// Coupon rate (0.0 for PO)
    pub coupon: f64,
    /// Payment priority (1 = highest for sequential)
    pub priority: u32,
    /// PAC collar (if PAC tranche)
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub pac_collar: Option<PacCollar>,
}

impl CmoTranche {
    /// Create a sequential tranche.
    pub fn sequential(id: &str, face: Money, coupon: f64, priority: u32) -> Self {
        Self {
            id: id.to_string(),
            tranche_type: CmoTrancheType::Sequential,
            original_face: face,
            current_face: face,
            coupon,
            priority,
            pac_collar: None,
        }
    }

    /// Create a PAC tranche.
    pub fn pac(id: &str, face: Money, coupon: f64, priority: u32, collar: PacCollar) -> Self {
        Self {
            id: id.to_string(),
            tranche_type: CmoTrancheType::Pac,
            original_face: face,
            current_face: face,
            coupon,
            priority,
            pac_collar: Some(collar),
        }
    }

    /// Create a support tranche.
    pub fn support(id: &str, face: Money, coupon: f64, priority: u32) -> Self {
        Self {
            id: id.to_string(),
            tranche_type: CmoTrancheType::Support,
            original_face: face,
            current_face: face,
            coupon,
            priority,
            pac_collar: None,
        }
    }

    /// Create an IO strip.
    pub fn io_strip(id: &str, notional: Money, coupon: f64) -> Self {
        Self {
            id: id.to_string(),
            tranche_type: CmoTrancheType::InterestOnly,
            original_face: notional,
            current_face: notional,
            coupon,
            priority: 0, // IO gets interest before principal allocation
            pac_collar: None,
        }
    }

    /// Create a PO strip.
    pub fn po_strip(id: &str, face: Money) -> Self {
        Self {
            id: id.to_string(),
            tranche_type: CmoTrancheType::PrincipalOnly,
            original_face: face,
            current_face: face,
            coupon: 0.0,
            priority: 0,
            pac_collar: None,
        }
    }

    /// Get current factor.
    pub fn factor(&self) -> f64 {
        self.current_face.amount() / self.original_face.amount()
    }

    /// Check if tranche is interest-bearing.
    pub fn is_interest_bearing(&self) -> bool {
        self.coupon > 0.0 && self.tranche_type != CmoTrancheType::PrincipalOnly
    }

    /// Check if tranche receives principal.
    pub fn receives_principal(&self) -> bool {
        self.tranche_type != CmoTrancheType::InterestOnly
    }
}

/// CMO waterfall configuration.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CmoWaterfall {
    /// Tranches in the deal (ordered by priority for sequential)
    pub tranches: Vec<CmoTranche>,
    /// Whether to use pro-rata allocation within same priority
    pub pro_rata_same_priority: bool,
}

impl CmoWaterfall {
    /// Create a new waterfall with tranches.
    pub fn new(tranches: Vec<CmoTranche>) -> Self {
        Self {
            tranches,
            pro_rata_same_priority: false,
        }
    }

    /// Get tranche by ID.
    pub fn get_tranche(&self, id: &str) -> Option<&CmoTranche> {
        self.tranches.iter().find(|t| t.id == id)
    }

    /// Get mutable tranche by ID.
    pub fn get_tranche_mut(&mut self, id: &str) -> Option<&mut CmoTranche> {
        self.tranches.iter_mut().find(|t| t.id == id)
    }

    /// Get total current face across all tranches (excluding IO).
    pub fn total_current_face(&self) -> Money {
        let total: f64 = self
            .tranches
            .iter()
            .filter(|t| t.receives_principal())
            .map(|t| t.current_face.amount())
            .sum();

        let currency = self
            .tranches
            .first()
            .map(|t| t.current_face.currency())
            .unwrap_or(Currency::USD);

        Money::new(total, currency)
    }
}

/// Agency CMO instrument.
///
/// Represents a CMO deal backed by agency MBS collateral with multiple
/// tranches that receive cashflows according to waterfall rules.
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AgencyCmo {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Deal name (e.g., "FNR 2024-1")
    pub deal_name: String,
    /// Agency program
    pub agency: AgencyProgram,
    /// Issue date
    pub issue_date: Date,
    /// Waterfall configuration with tranches
    pub waterfall: CmoWaterfall,
    /// Reference tranche ID for pricing (which tranche to value)
    pub reference_tranche_id: String,
    /// Collateral pool (optional - for detailed cashflow projection)
    #[builder(optional)]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub collateral: Option<Box<AgencyMbsPassthrough>>,
    /// Collateral WAC (if no explicit collateral)
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collateral_wac: Option<f64>,
    /// Collateral WAM (if no explicit collateral)
    #[builder(optional)]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub collateral_wam: Option<u32>,
    /// Discount curve identifier.
    pub discount_curve_id: CurveId,
    /// Pricing overrides.
    #[builder(default)]
    #[cfg_attr(feature = "serde", serde(default))]
    pub pricing_overrides: PricingOverrides,
    /// Attributes for tagging and selection.
    #[builder(default)]
    pub attributes: Attributes,
}

impl AgencyCmo {
    /// Create a canonical example CMO for testing.
    pub fn example() -> Self {
        // Create sequential structure: A (front), B (middle), Z (last)
        let tranches = vec![
            CmoTranche::sequential("A", Money::new(40_000_000.0, Currency::USD), 0.04, 1),
            CmoTranche::sequential("B", Money::new(30_000_000.0, Currency::USD), 0.045, 2),
            CmoTranche::sequential("Z", Money::new(30_000_000.0, Currency::USD), 0.05, 3),
        ];

        Self::builder()
            .id(InstrumentId::new("FNR-2024-1-A"))
            .deal_name("FNR 2024-1".to_string())
            .agency(AgencyProgram::Fnma)
            .issue_date(
                Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
            )
            .waterfall(CmoWaterfall::new(tranches))
            .reference_tranche_id("A".to_string())
            .collateral_wac(0.045)
            .collateral_wam(360)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .attributes(
                Attributes::new()
                    .with_tag("cmo")
                    .with_tag("agency")
                    .with_meta("deal", "fnr-2024-1"),
            )
            .build()
            .expect("Example CMO construction should not fail")
    }

    /// Create an example PAC/Support structure.
    pub fn example_pac_support() -> Self {
        let tranches = vec![
            CmoTranche::pac(
                "PAC",
                Money::new(50_000_000.0, Currency::USD),
                0.04,
                1,
                PacCollar::standard(),
            ),
            CmoTranche::support("SUP", Money::new(50_000_000.0, Currency::USD), 0.05, 2),
        ];

        Self::builder()
            .id(InstrumentId::new("FNR-2024-2-PAC"))
            .deal_name("FNR 2024-2".to_string())
            .agency(AgencyProgram::Fnma)
            .issue_date(
                Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
            )
            .waterfall(CmoWaterfall::new(tranches))
            .reference_tranche_id("PAC".to_string())
            .collateral_wac(0.045)
            .collateral_wam(360)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("Example PAC/Support CMO construction should not fail")
    }

    /// Create an example IO/PO strip structure.
    pub fn example_io_po() -> Self {
        let tranches = vec![
            CmoTranche::io_strip("IO", Money::new(100_000_000.0, Currency::USD), 0.04),
            CmoTranche::po_strip("PO", Money::new(100_000_000.0, Currency::USD)),
        ];

        Self::builder()
            .id(InstrumentId::new("FNS-2024-1-IO"))
            .deal_name("FNS 2024-1".to_string())
            .agency(AgencyProgram::Fnma)
            .issue_date(
                Date::from_calendar_date(2024, time::Month::January, 1)
                    .expect("Valid example date"),
            )
            .waterfall(CmoWaterfall::new(tranches))
            .reference_tranche_id("IO".to_string())
            .collateral_wac(0.04)
            .collateral_wam(360)
            .discount_curve_id(CurveId::new("USD-OIS"))
            .build()
            .expect("Example IO/PO CMO construction should not fail")
    }

    /// Get the reference tranche being valued.
    pub fn reference_tranche(&self) -> Option<&CmoTranche> {
        self.waterfall.get_tranche(&self.reference_tranche_id)
    }
}

impl crate::instruments::common::traits::CurveDependencies for AgencyCmo {
    fn curve_dependencies(&self) -> crate::instruments::common::traits::InstrumentCurves {
        crate::instruments::common::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}

impl crate::instruments::common::traits::Instrument for AgencyCmo {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::AgencyCmo
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
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<finstack_core::money::Money> {
        crate::instruments::agency_cmo::pricer::price_cmo(self, market, as_of)
    }

    fn price_with_metrics(
        &self,
        market: &finstack_core::market_data::context::MarketContext,
        as_of: finstack_core::dates::Date,
        metrics: &[crate::metrics::MetricId],
    ) -> finstack_core::Result<crate::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        crate::instruments::common::helpers::build_with_metrics_dyn(
            std::sync::Arc::new(self.clone()),
            std::sync::Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
        )
    }

    fn required_discount_curves(&self) -> Vec<CurveId> {
        vec![self.discount_curve_id.clone()]
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for AgencyCmo {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmo_example() {
        let cmo = AgencyCmo::example();
        assert_eq!(cmo.agency, AgencyProgram::Fnma);
        assert_eq!(cmo.waterfall.tranches.len(), 3);
    }

    #[test]
    fn test_tranche_types() {
        let cmo = AgencyCmo::example();

        for tranche in &cmo.waterfall.tranches {
            assert_eq!(tranche.tranche_type, CmoTrancheType::Sequential);
        }
    }

    #[test]
    fn test_pac_support_structure() {
        let cmo = AgencyCmo::example_pac_support();

        let pac = cmo.waterfall.get_tranche("PAC").expect("PAC exists");
        assert_eq!(pac.tranche_type, CmoTrancheType::Pac);
        assert!(pac.pac_collar.is_some());

        let sup = cmo.waterfall.get_tranche("SUP").expect("SUP exists");
        assert_eq!(sup.tranche_type, CmoTrancheType::Support);
    }

    #[test]
    fn test_io_po_structure() {
        let cmo = AgencyCmo::example_io_po();

        let io = cmo.waterfall.get_tranche("IO").expect("IO exists");
        assert_eq!(io.tranche_type, CmoTrancheType::InterestOnly);
        assert!(io.is_interest_bearing());
        assert!(!io.receives_principal());

        let po = cmo.waterfall.get_tranche("PO").expect("PO exists");
        assert_eq!(po.tranche_type, CmoTrancheType::PrincipalOnly);
        assert!(!po.is_interest_bearing());
        assert!(po.receives_principal());
    }

    #[test]
    fn test_total_face() {
        let cmo = AgencyCmo::example();
        let total = cmo.waterfall.total_current_face();

        // 40M + 30M + 30M = 100M
        assert!((total.amount() - 100_000_000.0).abs() < 1.0);
    }

    #[test]
    fn test_reference_tranche() {
        let cmo = AgencyCmo::example();
        let ref_tranche = cmo.reference_tranche().expect("ref exists");

        assert_eq!(ref_tranche.id, "A");
    }
}
