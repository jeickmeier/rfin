//! CDS Tranche types, builder entrypoint, and pricing impl.

use crate::cashflow::builder::ScheduleParams;
use crate::instruments::build_with_metrics_dyn;
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::parameters::CDSTrancheParams;
use super::pricer;

/// Buyer/seller perspective for CDS tranche premium/protection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TrancheSide {
    /// Buy protection on the tranche (pay running, receive protection)
    BuyProtection,
    /// Sell protection on the tranche (receive running, pay protection)
    SellProtection,
}

impl std::fmt::Display for TrancheSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrancheSide::BuyProtection => write!(f, "buy_protection"),
            TrancheSide::SellProtection => write!(f, "sell_protection"),
        }
    }
}

impl std::str::FromStr for TrancheSide {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "buy_protection" | "buy" => Ok(TrancheSide::BuyProtection),
            "sell_protection" | "sell" => Ok(TrancheSide::SellProtection),
            other => Err(format!("Unknown tranche side: {}", other)),
        }
    }
}

/// CDS Tranche instrument definition (boilerplate)
#[derive(Clone, Debug, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdsTranche {
    /// Unique instrument identifier
    pub id: InstrumentId,
    /// Index name (e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx EUR")
    pub index_name: String,
    /// Series number (e.g., 37)
    pub series: u16,
    /// Attachment point in percent (e.g., 0.0 for equity)
    pub attach_pct: f64,
    /// Detachment point in percent (e.g., 3.0 for 0-3% tranche)
    pub detach_pct: f64,
    /// Notional amount of the tranche
    pub notional: Money,
    /// Maturity date of the tranche
    pub maturity: Date,
    /// Running coupon in basis points (e.g., 100 = 1.00%)
    pub running_coupon_bp: f64,
    /// Payment frequency (typically quarterly)
    pub payment_frequency: Frequency,
    /// Day count (typically Act/360)
    pub day_count: DayCount,
    /// Business day convention
    pub business_day_convention: BusinessDayConvention,
    /// Optional holiday calendar id
    pub calendar_id: Option<String>,
    /// Discount curve identifier (by quote currency)
    pub disc_id: CurveId,
    /// Credit index identifier for survival/loss modeling (placeholder)
    pub credit_index_id: CurveId,
    /// Tranche side (buy/sell protection)
    pub side: TrancheSide,
    /// Optional effective date for schedule anchoring (if None, uses as_of date)
    pub effective_date: Option<Date>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CdsTranche {
    /// Create a new CDS tranche using parameter structs
    pub fn new(
        id: impl Into<InstrumentId>,
        tranche_params: &CDSTrancheParams,
        schedule_params: &ScheduleParams,
        disc_id: impl Into<CurveId>,
        credit_index_id: impl Into<CurveId>,
        side: TrancheSide,
    ) -> Self {
        Self {
            id: id.into(),
            index_name: tranche_params.index_name.to_owned(),
            series: tranche_params.series,
            attach_pct: tranche_params.attach_pct,
            detach_pct: tranche_params.detach_pct,
            notional: tranche_params.notional,
            maturity: tranche_params.maturity,
            running_coupon_bp: tranche_params.running_coupon_bp,
            payment_frequency: schedule_params.freq,
            day_count: schedule_params.dc,
            business_day_convention: schedule_params.bdc,
            calendar_id: schedule_params.calendar_id.clone(),
            disc_id: disc_id.into(),
            credit_index_id: credit_index_id.into(),
            side,
            effective_date: None,
            attributes: Attributes::new(),
        }
    }

    /// Calculate the net present value of this CDS tranche
    pub fn npv(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.price_tranche(self, curves, as_of)
    }

    /// Calculate upfront amount for the tranche
    pub fn upfront(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_upfront(self, curves, as_of)
    }

    /// Calculate spread DV01 (sensitivity to 1bp change in running coupon)
    pub fn spread_dv01(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_spread_dv01(self, curves, as_of)
    }

    /// Calculate the par spread (running coupon in basis points).
    pub fn par_spread(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_par_spread(self, curves, as_of)
    }

    /// Calculate expected loss metric
    pub fn expected_loss(&self, curves: &MarketContext) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_expected_loss(self, curves)
    }

    /// Calculate jump-to-default metric
    pub fn jump_to_default(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_jump_to_default(self, curves, as_of)
    }

    /// Calculate CS01 (sensitivity to 1bp parallel shift in credit spreads)
    pub fn cs01(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_cs01(self, curves, as_of)
    }

    /// Calculate correlation delta (sensitivity to correlation changes)
    pub fn correlation_delta(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_correlation_delta(self, curves, as_of)
    }

    // Builder now provided by derive
}

// Attributable is provided via blanket impl for all Instrument types

impl Instrument for CdsTranche {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::CDSTranche
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    // === Pricing Methods ===

    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        // Call the instrument's own NPV method
        self.npv(curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        build_with_metrics_dyn(self, curves, as_of, base_value, metrics)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for CdsTranche {
    fn discount_curve_id(&self) -> &finstack_core::types::CurveId {
        &self.disc_id
    }
}
