//! CDS Tranche types, builder entrypoint, and pricing impl.

use crate::cashflow::builder::ScheduleParams;
use crate::instruments::common_impl::helpers::build_with_metrics_dyn;
use crate::instruments::common_impl::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
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
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
    pub payment_frequency: Tenor,
    /// Day count (typically Act/360)
    pub day_count: DayCount,
    /// Business day convention
    pub business_day_convention: BusinessDayConvention,
    /// Optional holiday calendar id
    pub calendar_id: Option<String>,
    /// Discount curve identifier (by quote currency)
    pub discount_curve_id: CurveId,
    /// Credit index identifier for survival/loss modeling (placeholder)
    pub credit_index_id: CurveId,
    /// Tranche side (buy/sell protection)
    pub side: TrancheSide,
    /// Optional effective date for schedule anchoring (if None, uses as_of date)
    pub effective_date: Option<Date>,
    /// Accumulated realized loss as fraction of original portfolio notional
    pub accumulated_loss: f64,
    /// Whether to enforce standard IMM dates (20th of Mar, Jun, Sep, Dec)
    pub standard_imm_dates: bool,
    /// Optional upfront payment (date, amount). Positive means paid by protection buyer.
    #[cfg_attr(feature = "serde", serde(default))]
    pub upfront: Option<(Date, Money)>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CdsTranche {
    /// Create a canonical example CDS tranche (CDX.NA.IG 0-3% equity tranche).
    pub fn example() -> Self {
        use crate::cashflow::builder::ScheduleParams;
        use finstack_core::currency::Currency;
        use time::macros::date;
        let params = super::parameters::CDSTrancheParams::equity_tranche(
            "CDX.NA.IG",
            42,
            Money::new(10_000_000.0, Currency::USD),
            date!(2029 - 12 - 20),
            100.0,
        );
        let sched = ScheduleParams {
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: "weekends_only".to_string(),
            stub: StubKind::ShortFront,
            end_of_month: false,
            payment_lag_days: 0,
        };
        CdsTranche::new(
            InstrumentId::new("CDXIG-42-0X3"),
            &params,
            &sched,
            CurveId::new("USD-OIS"),
            CurveId::new("CDX.NA.IG.HAZARD"),
            TrancheSide::BuyProtection,
        )
    }
    /// Create a new CDS tranche using parameter structs.
    ///
    /// # Panics
    ///
    /// Panics if tranche parameters are invalid:
    /// - `attach_pct` must be less than `detach_pct`
    /// - Both must be in [0, 100] (percent, not fraction)
    /// - Notional must be positive
    pub fn new(
        id: impl Into<InstrumentId>,
        tranche_params: &CDSTrancheParams,
        schedule_params: &ScheduleParams,
        discount_curve_id: impl Into<CurveId>,
        credit_index_id: impl Into<CurveId>,
        side: TrancheSide,
    ) -> Self {
        // Runtime validation (active in both debug and release builds)
        assert!(
            tranche_params.attach_pct < tranche_params.detach_pct,
            "attach_pct ({}) must be less than detach_pct ({})",
            tranche_params.attach_pct,
            tranche_params.detach_pct
        );
        assert!(
            tranche_params.attach_pct >= 0.0 && tranche_params.detach_pct <= 100.0,
            "attach_pct ({}) and detach_pct ({}) must be in [0, 100] (percent, not fraction)",
            tranche_params.attach_pct,
            tranche_params.detach_pct
        );
        assert!(
            tranche_params.notional.amount() > 0.0,
            "Tranche notional must be positive, got {}",
            tranche_params.notional.amount()
        );

        // Warn if values look like fractions (very small non-zero detachment)
        if tranche_params.detach_pct <= 1.0 && tranche_params.detach_pct > 0.0 {
            tracing::warn!(
                "detach_pct={} looks like a fraction; expected percent (e.g., 3.0 for 3%)",
                tranche_params.detach_pct
            );
        }

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
            calendar_id: Some(schedule_params.calendar_id.clone()),
            discount_curve_id: discount_curve_id.into(),
            credit_index_id: credit_index_id.into(),
            side,
            effective_date: None,
            accumulated_loss: tranche_params.accumulated_loss,
            standard_imm_dates: false,
            upfront: None,
            attributes: Attributes::new(),
        }
    }

    /// Create a standard CDS tranche with IMM dates and market conventions
    #[allow(clippy::too_many_arguments)]
    pub fn standard(
        id: impl Into<InstrumentId>,
        tranche_params: &CDSTrancheParams,
        discount_curve_id: impl Into<CurveId>,
        credit_index_id: impl Into<CurveId>,
        side: TrancheSide,
    ) -> Self {
        use crate::cashflow::builder::ScheduleParams;
        let sched = ScheduleParams {
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::Following,
            calendar_id: "weekends_only".to_string(),
            stub: StubKind::ShortFront,
            end_of_month: false,
            payment_lag_days: 0,
        };

        Self {
            id: id.into(),
            index_name: tranche_params.index_name.to_owned(),
            series: tranche_params.series,
            attach_pct: tranche_params.attach_pct,
            detach_pct: tranche_params.detach_pct,
            notional: tranche_params.notional,
            maturity: tranche_params.maturity,
            running_coupon_bp: tranche_params.running_coupon_bp,
            payment_frequency: sched.freq,
            day_count: sched.dc,
            business_day_convention: sched.bdc,
            calendar_id: None,
            discount_curve_id: discount_curve_id.into(),
            credit_index_id: credit_index_id.into(),
            side,
            effective_date: None,
            accumulated_loss: tranche_params.accumulated_loss,
            standard_imm_dates: true,
            upfront: None,
            attributes: Attributes::new(),
        }
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

    /// Calculate correlation delta (sensitivity to correlation changes)
    pub fn correlation_delta(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_correlation_delta(self, curves, as_of)
    }

    /// Calculate accrued premium on the tranche.
    ///
    /// Returns the premium accrued since the last payment date, calculated on
    /// the outstanding notional (after any realized losses).
    ///
    /// # Returns
    ///
    /// The accrued premium amount in the tranche currency. This represents:
    /// - For protection buyer: amount owed to seller
    /// - For protection seller: amount receivable from buyer
    ///
    /// # Use Cases
    ///
    /// - Dirty vs clean price calculation: `dirty_price = clean_price + accrued`
    /// - Settlement amount calculation
    /// - Mark-to-market accounting
    pub fn accrued_premium(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<f64> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_accrued_premium(self, curves, as_of)
    }

    /// Calculate detailed jump-to-default metrics including min, max, and average.
    ///
    /// For heterogeneous portfolios, provides the full distribution of JTD impacts.
    pub fn jump_to_default_detail(
        &self,
        curves: &MarketContext,
    ) -> finstack_core::Result<pricer::JumpToDefaultResult> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.calculate_jump_to_default_detail(self, curves)
    }

    /// Get the expected loss curve for diagnostic purposes.
    ///
    /// Returns (Date, EL_fraction) pairs showing cumulative expected loss over time.
    pub fn expected_loss_curve(
        &self,
        curves: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, f64)>> {
        let pricer = pricer::CDSTranchePricer::new();
        pricer.get_expected_loss_curve(self, curves, as_of)
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
        let pricer = pricer::CDSTranchePricer::new();
        pricer.price_tranche(self, curves, as_of)
    }

    fn price_with_metrics(
        &self,
        curves: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(curves, as_of)?;
        build_with_metrics_dyn(
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
        Some(self.maturity)
    }

    fn effective_start_date(&self) -> Option<finstack_core::dates::Date> {
        self.effective_date
    }
}

// Implement CurveDependencies for DV01 calculator
impl crate::instruments::common_impl::traits::CurveDependencies for CdsTranche {
    fn curve_dependencies(&self) -> crate::instruments::common_impl::traits::InstrumentCurves {
        crate::instruments::common_impl::traits::InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .credit(self.credit_index_id.clone())
            .build()
    }
}
