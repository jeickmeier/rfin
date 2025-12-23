//! CDS Tranche specific parameters.

use finstack_core::{dates::Date, money::Money};

/// CDS Tranche specific parameters.
///
/// Groups parameters specific to CDS tranches.
#[derive(Clone, Debug)]
pub struct CDSTrancheParams {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe")
    pub index_name: String,
    /// Index series
    pub series: u16,
    /// Attachment point as percentage
    pub attach_pct: f64,
    /// Detachment point as percentage
    pub detach_pct: f64,
    /// Notional amount
    pub notional: Money,
    /// Maturity date
    pub maturity: Date,
    /// Running coupon in basis points
    pub running_coupon_bp: f64,
    /// Accumulated realized loss as fraction of original portfolio notional [0.0, 1.0]
    pub accumulated_loss: f64,
}

impl CDSTrancheParams {
    /// Create new CDS tranche parameters
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        index_name: impl Into<String>,
        series: u16,
        attach_pct: f64,
        detach_pct: f64,
        notional: Money,
        maturity: Date,
        running_coupon_bp: f64,
    ) -> Self {
        Self {
            index_name: index_name.into(),
            series,
            attach_pct,
            detach_pct,
            notional,
            maturity,
            running_coupon_bp,
            accumulated_loss: 0.0,
        }
    }

    /// Set the accumulated loss
    pub fn with_accumulated_loss(mut self, loss: f64) -> Self {
        self.accumulated_loss = loss;
        self
    }

    /// Create equity tranche parameters (0-3% typically)
    pub fn equity_tranche(
        index_name: impl Into<String>,
        series: u16,
        notional: Money,
        maturity: Date,
        running_coupon_bp: f64,
    ) -> Self {
        Self::new(
            index_name,
            series,
            0.0,
            0.03,
            notional,
            maturity,
            running_coupon_bp,
        )
    }

    /// Create mezzanine tranche parameters (3-7% typically)
    pub fn mezzanine_tranche(
        index_name: impl Into<String>,
        series: u16,
        notional: Money,
        maturity: Date,
        running_coupon_bp: f64,
    ) -> Self {
        Self::new(
            index_name,
            series,
            0.03,
            0.07,
            notional,
            maturity,
            running_coupon_bp,
        )
    }
}
