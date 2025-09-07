//! CDS Tranche types, builder entrypoint, and pricing impl.

use crate::instruments::build_with_metrics_dyn;
use crate::instruments::traits::{Attributes, Priceable};
use crate::metrics::MetricId;
use crate::market_data::ValuationMarketContext;
use crate::results::ValuationResult;
use finstack_core::market_data::MarketContext;
use finstack_core::F;
use finstack_core::dates::{BusinessDayConvention, DayCount, Date, Frequency};
use finstack_core::money::Money;

use super::model;

/// Buyer/seller perspective for CDS tranche premium/protection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrancheSide {
    /// Buy protection on the tranche (pay running, receive protection)
    BuyProtection,
    /// Sell protection on the tranche (receive running, pay protection)
    SellProtection,
}

/// CDS Tranche instrument definition (boilerplate)
#[derive(Clone, Debug)]
pub struct CdsTranche {
    /// Unique instrument identifier
    pub id: String,
    /// Index name (e.g., "CDX.NA.IG", "CDX.NA.HY", "iTraxx EUR")
    pub index_name: String,
    /// Series number (e.g., 37)
    pub series: u16,
    /// Attachment point in percent (e.g., 0.0 for equity)
    pub attach_pct: F,
    /// Detachment point in percent (e.g., 3.0 for 0-3% tranche)
    pub detach_pct: F,
    /// Notional amount of the tranche
    pub notional: Money,
    /// Maturity date of the tranche
    pub maturity: Date,
    /// Running coupon in basis points (e.g., 100 = 1.00%)
    pub running_coupon_bp: F,
    /// Payment frequency (typically quarterly)
    pub payment_frequency: Frequency,
    /// Day count (typically Act/360)
    pub day_count: DayCount,
    /// Business day convention
    pub business_day_convention: BusinessDayConvention,
    /// Optional holiday calendar id
    pub calendar_id: Option<&'static str>,
    /// Discount curve identifier (by quote currency)
    pub disc_id: &'static str,
    /// Credit index identifier for survival/loss modeling (placeholder)
    pub credit_index_id: &'static str,
    /// Tranche side (buy/sell protection)
    pub side: TrancheSide,
    /// Optional effective date for schedule anchoring (if None, uses as_of date)
    pub effective_date: Option<Date>,
    /// Attributes for tagging and selection
    pub attributes: Attributes,
}

impl CdsTranche {
    /// Create a new CDS tranche with required fields
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        index_name: impl Into<String>,
        series: u16,
        attach_pct: F,
        detach_pct: F,
        notional: Money,
        maturity: Date,
        running_coupon_bp: F,
        payment_frequency: Frequency,
        day_count: DayCount,
        business_day_convention: BusinessDayConvention,
        calendar_id: Option<&'static str>,
        disc_id: &'static str,
        credit_index_id: &'static str,
        side: TrancheSide,
    ) -> Self {
        Self {
            id: id.into(),
            index_name: index_name.into(),
            series,
            attach_pct,
            detach_pct,
            notional,
            maturity,
            running_coupon_bp,
            payment_frequency,
            day_count,
            business_day_convention,
            calendar_id,
            disc_id,
            credit_index_id,
            side,
            effective_date: None,
            attributes: Attributes::new(),
        }
    }

    /// Builder entrypoint
    pub fn builder(
    ) -> crate::instruments::fixed_income::cds_tranche::mod_cds_tranche::CdsTrancheBuilder {
        crate::instruments::fixed_income::cds_tranche::mod_cds_tranche::CdsTrancheBuilder::new()
    }
}

impl_attributable!(CdsTranche);
impl_instrument_like!(CdsTranche, "CDSTranche");

impl Priceable for CdsTranche {
    fn value(&self, curves: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        // Try to use the Gaussian Copula model if credit index data is available
        // Otherwise, fall back to zero PV for backward compatibility

        // Convert MarketContext to ValuationMarketContext
        let val_market_ctx = ValuationMarketContext::from_core(curves.clone());

        // Check if credit index data is available
        if val_market_ctx.has_credit_index(self.credit_index_id) {
            // Use the Gaussian Copula model
            let model = model::GaussianCopulaModel::new();
            model.price_tranche(self, &val_market_ctx, as_of)
        } else {
            // Fallback to zero PV when credit index data is not available
            Ok(Money::new(0.0, self.notional.currency()))
        }
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


