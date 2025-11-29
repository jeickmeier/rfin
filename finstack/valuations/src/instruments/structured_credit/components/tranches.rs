//! Tranche structures for structured credit instruments.

// InterestSpec removed with loan; retain coupon for metadata only
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::money::Money;
#[cfg(test)]
use finstack_core::types::CurveId;
use finstack_core::types::InstrumentId;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::enums::{CreditRating, TrancheSeniority, TriggerConsequence};

/// Tranche behavioral types (simplified to standard only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TrancheBehaviorType {
    /// Standard bond (pays interest and principal)
    Standard,
}

#[cfg(feature = "serde")]
fn default_behavior_type() -> TrancheBehaviorType {
    TrancheBehaviorType::Standard
}

/// Coverage test trigger specification
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CoverageTrigger {
    /// Trigger threshold level (e.g., 1.20 for 120% OC)
    pub trigger_level: f64,
    /// Higher level required to cure breach
    pub cure_level: Option<f64>,
    /// Date when breach occurred (if any)
    pub breach_date: Option<Date>,
    /// What happens when triggered
    pub consequence: TriggerConsequence,
}

impl CoverageTrigger {
    /// Create a new coverage trigger
    pub fn new(trigger_level: f64, consequence: TriggerConsequence) -> Self {
        Self {
            trigger_level,
            cure_level: None,
            breach_date: None,
            consequence,
        }
    }

    /// With cure level (typically higher than trigger)
    pub fn with_cure_level(mut self, cure_level: f64) -> Self {
        self.cure_level = Some(cure_level);
        self
    }

    /// Check if currently breached
    pub fn is_breached(&self, current_level: f64) -> bool {
        current_level < self.trigger_level
    }

    /// Check if breach is cured
    pub fn is_cured(&self, current_level: f64) -> bool {
        if let Some(cure) = self.cure_level {
            current_level >= cure
        } else {
            current_level >= self.trigger_level
        }
    }
}

/// Credit enhancement mechanisms for a tranche
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CreditEnhancement {
    /// Subordination amount (sum of junior tranches)
    pub subordination: Money,
    /// Overcollateralization amount
    pub overcollateralization: Money,
    /// Reserve account balance
    pub reserve_account: Money,
    /// Available excess spread
    pub excess_spread: f64,
    /// Cash trap/turbo active
    pub cash_trap_active: bool,
}

impl Default for CreditEnhancement {
    fn default() -> Self {
        Self {
            subordination: Money::new(0.0, finstack_core::currency::Currency::USD),
            overcollateralization: Money::new(0.0, finstack_core::currency::Currency::USD),
            reserve_account: Money::new(0.0, finstack_core::currency::Currency::USD),
            excess_spread: 0.0,
            cash_trap_active: false,
        }
    }
}

/// Tranche coupon specification
///
/// Supports fixed and floating rate coupons used in standard structured credit instruments.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TrancheCoupon {
    /// Fixed rate coupon (rate as decimal, e.g., 0.05 for 5%)
    Fixed {
        /// Fixed interest rate as decimal (e.g., 0.05 for 5%)
        rate: f64,
    },

    /// Floating rate coupon using canonical FloatingRateSpec.
    ///
    /// Uses the standard floating rate specification with all rates in basis points.
    Floating(crate::cashflow::builder::FloatingRateSpec),
}

impl TrancheCoupon {
    /// Get current rate for a given date (without index lookup)
    ///
    /// For Fixed: returns the fixed rate
    /// For Floating: returns just the spread component (use current_rate_with_index for full rate)
    pub fn current_rate(&self, _date: Date) -> f64 {
        match self {
            TrancheCoupon::Fixed { rate } => *rate,
            TrancheCoupon::Floating(spec) => spec.spread_bp / 10_000.0,
        }
    }

    /// Compute current rate including index forward where applicable.
    ///
    /// For Fixed coupons, returns the fixed rate.
    /// For Floating coupons, uses centralized projection with floor/cap support.
    pub fn current_rate_with_index(
        &self,
        date: Date,
        context: &finstack_core::market_data::MarketContext,
    ) -> f64 {
        match self {
            TrancheCoupon::Fixed { rate } => *rate,
            TrancheCoupon::Floating(spec) => {
                // Use centralized projection
                let fwd = match context.get_forward_ref(spec.index_id.as_str()) {
                    Ok(f) => f,
                    Err(_) => return spec.spread_bp / 10_000.0, // Fallback to spread only
                };

                let tenor = fwd.tenor();
                let period_end_approx = date + time::Duration::days((tenor * 365.25) as i64);

                crate::cashflow::builder::project_floating_rate_with_curve(
                    date,
                    period_end_approx,
                    spec.spread_bp,
                    spec.gearing,
                    spec.floor_bp,
                    spec.cap_bp,
                    fwd,
                )
                .unwrap_or(spec.spread_bp / 10_000.0)
            }
        }
    }
}

/// Structured credit tranche with attachment/detachment points
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Tranche {
    /// Unique tranche identifier
    pub id: InstrumentId,

    /// Structural boundaries (as % of total capital structure)
    pub attachment_point: f64, // Lower bound (e.g., 0.0% for equity, 10% for mezz)
    /// Detachment point as percentage (upper loss bound for this tranche)
    pub detachment_point: f64, // Upper bound (e.g., 10% for equity, 15% for mezz)

    /// Behavioral classification for specialized handling
    #[cfg_attr(feature = "serde", serde(default = "default_behavior_type"))]
    pub behavior_type: TrancheBehaviorType,

    /// Tranche characteristics
    pub seniority: TrancheSeniority,
    /// Credit rating (if rated by agencies)
    pub rating: Option<CreditRating>,

    /// Size and balances
    pub original_balance: Money,
    /// Current outstanding balance (after amortization and losses)
    pub current_balance: Money,
    /// Target balance for revolving period (optional, for revolving structures)
    pub target_balance: Option<Money>, // For revolving structures

    /// Interest specification
    pub coupon: TrancheCoupon,

    /// Coverage test triggers
    pub oc_trigger: Option<CoverageTrigger>,
    /// Interest coverage trigger specification
    pub ic_trigger: Option<CoverageTrigger>,

    /// Credit enhancement details
    pub credit_enhancement: CreditEnhancement,

    /// Payment characteristics
    pub payment_frequency: Frequency,
    /// Day count convention for interest accrual
    pub day_count: DayCount,
    /// Accumulated deferred interest (if payment has been deferred)
    pub deferred_interest: Money,

    /// Structural features
    pub is_revolving: bool,
    /// Whether reinvestment of principal is permitted
    pub can_reinvest: bool,
    /// Legal final maturity date
    pub legal_maturity: Date,
    /// Expected maturity date (may be earlier than legal maturity for CLOs)
    pub expected_maturity: Option<Date>,

    /// Payment priority (1 = highest)
    pub payment_priority: u32,

    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl Tranche {
    /// Create a new tranche with required fields
    pub fn new(
        id: impl Into<String>,
        attachment_point: f64,
        detachment_point: f64,
        seniority: TrancheSeniority,
        original_balance: Money,
        coupon: TrancheCoupon,
        legal_maturity: Date,
    ) -> finstack_core::Result<Self> {
        // Validate attachment/detachment points
        if attachment_point < 0.0 || detachment_point <= attachment_point {
            return Err(finstack_core::error::InputError::Invalid.into());
        }
        if detachment_point > 100.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        Ok(Self {
            id: InstrumentId::new(id.into()),
            attachment_point,
            detachment_point,
            behavior_type: TrancheBehaviorType::Standard,
            seniority,
            rating: None,
            original_balance,
            current_balance: original_balance,
            target_balance: None,
            coupon,
            oc_trigger: None,
            ic_trigger: None,
            credit_enhancement: CreditEnhancement::default(),
            payment_frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            deferred_interest: Money::new(0.0, original_balance.currency()),
            is_revolving: false,
            can_reinvest: false,
            legal_maturity,
            expected_maturity: None,
            payment_priority: match seniority {
                TrancheSeniority::Senior => 1,
                TrancheSeniority::Mezzanine => 2,
                TrancheSeniority::Subordinated => 3,
                TrancheSeniority::Equity => 4,
            },
            attributes: Attributes::new(),
        })
    }

    /// Tranche thickness (detachment - attachment)
    pub fn thickness(&self) -> f64 {
        self.detachment_point - self.attachment_point
    }

    /// Check if tranche is first loss (attachment at 0%)
    pub fn is_first_loss(&self) -> bool {
        self.attachment_point == 0.0
    }

    /// Check if tranche is currently impaired by losses
    pub fn is_impaired(&self, cumulative_loss_pct: f64) -> bool {
        cumulative_loss_pct > self.attachment_point
    }

    /// Calculate loss allocation to this tranche
    pub fn loss_allocation(&self, cumulative_loss_pct: f64, _total_pool_balance: Money) -> Money {
        if cumulative_loss_pct <= self.attachment_point {
            // No loss to this tranche
            Money::new(0.0, self.original_balance.currency())
        } else if cumulative_loss_pct >= self.detachment_point {
            // Tranche fully impaired
            self.original_balance
        } else {
            // Partial loss
            let loss_to_tranche_pct = cumulative_loss_pct - self.attachment_point;
            let loss_rate = loss_to_tranche_pct / self.thickness();
            self.original_balance * loss_rate
        }
    }

    /// Current tranche balance after losses
    pub fn current_balance_after_losses(
        &self,
        cumulative_loss_pct: f64,
        total_pool_balance: Money,
    ) -> Money {
        let loss_amount = self.loss_allocation(cumulative_loss_pct, total_pool_balance);
        Money::new(
            (self.current_balance.amount() - loss_amount.amount()).max(0.0),
            self.current_balance.currency(),
        )
    }

    /// Builder methods for fluent construction
    pub fn with_rating(mut self, rating: CreditRating) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Add overcollateralization coverage trigger
    pub fn with_oc_trigger(mut self, trigger: CoverageTrigger) -> Self {
        self.oc_trigger = Some(trigger);
        self
    }

    /// Add interest coverage trigger
    pub fn with_ic_trigger(mut self, trigger: CoverageTrigger) -> Self {
        self.ic_trigger = Some(trigger);
        self
    }

    /// Mark tranche as revolving (enables reinvestment)
    pub fn revolving(mut self) -> Self {
        self.is_revolving = true;
        self.can_reinvest = true;
        self
    }

    /// Set expected maturity date (typically earlier than legal maturity)
    pub fn with_expected_maturity(mut self, date: Date) -> Self {
        self.expected_maturity = Some(date);
        self
    }
}

/// Builder for creating tranches with validation
pub struct TrancheBuilder {
    id: Option<String>,
    attachment_point: Option<f64>,
    detachment_point: Option<f64>,
    seniority: Option<TrancheSeniority>,
    original_balance: Option<Money>,
    coupon: Option<TrancheCoupon>,
    legal_maturity: Option<Date>,
    rating: Option<CreditRating>,
    payment_frequency: Frequency,
    day_count: DayCount,
}

impl TrancheBuilder {
    /// Create new tranche builder
    pub fn new() -> Self {
        Self {
            id: None,
            attachment_point: None,
            detachment_point: None,
            seniority: None,
            original_balance: None,
            coupon: None,
            legal_maturity: None,
            rating: None,
            payment_frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
        }
    }

    /// Set tranche ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set attachment and detachment points (as percentages)
    pub fn attachment_detachment(mut self, attachment: f64, detachment: f64) -> Self {
        self.attachment_point = Some(attachment);
        self.detachment_point = Some(detachment);
        self
    }

    /// Set tranche seniority level
    pub fn seniority(mut self, seniority: TrancheSeniority) -> Self {
        self.seniority = Some(seniority);
        self
    }

    /// Set original tranche balance
    pub fn balance(mut self, balance: Money) -> Self {
        self.original_balance = Some(balance);
        self
    }

    /// Set coupon specification (fixed or floating)
    pub fn coupon(mut self, coupon: TrancheCoupon) -> Self {
        self.coupon = Some(coupon);
        self
    }

    /// Set legal maturity date
    pub fn legal_maturity(mut self, date: Date) -> Self {
        self.legal_maturity = Some(date);
        self
    }

    /// Set credit rating
    pub fn rating(mut self, rating: CreditRating) -> Self {
        self.rating = Some(rating);
        self
    }

    /// Set payment frequency
    pub fn payment_frequency(mut self, freq: Frequency) -> Self {
        self.payment_frequency = freq;
        self
    }

    /// Set day count convention
    pub fn day_count(mut self, dc: DayCount) -> Self {
        self.day_count = dc;
        self
    }

    /// Build the tranche with validation
    pub fn build(self) -> finstack_core::Result<Tranche> {
        let id = self.id.ok_or(finstack_core::error::InputError::Invalid)?;
        let attachment_point = self
            .attachment_point
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let detachment_point = self
            .detachment_point
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let seniority = self
            .seniority
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let original_balance = self
            .original_balance
            .ok_or(finstack_core::error::InputError::Invalid)?;

        // Validate original_balance is positive
        if original_balance.amount() <= 0.0 {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        let coupon = self
            .coupon
            .ok_or(finstack_core::error::InputError::Invalid)?;
        let legal_maturity = self
            .legal_maturity
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let mut tranche = Tranche::new(
            id,
            attachment_point,
            detachment_point,
            seniority,
            original_balance,
            coupon,
            legal_maturity,
        )?;

        if let Some(rating) = self.rating {
            tranche = tranche.with_rating(rating);
        }

        tranche.payment_frequency = self.payment_frequency;
        tranche.day_count = self.day_count;

        Ok(tranche)
    }
}

impl Default for TrancheBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of tranches forming the capital structure
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TrancheStructure {
    /// Ordered tranches (typically sorted by payment priority)
    pub tranches: Vec<Tranche>,
    /// Total size of all tranches combined
    pub total_size: Money,
}

impl TrancheStructure {
    /// Create new tranche structure
    pub fn new(tranches: Vec<Tranche>) -> finstack_core::Result<Self> {
        if tranches.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }

        // Validate structure
        Self::validate_structure(&tranches)?;

        // Calculate total size
        let total_size = tranches.iter().try_fold(
            Money::new(0.0, tranches[0].original_balance.currency()),
            |acc, t| acc.checked_add(t.original_balance),
        )?;

        Ok(Self {
            tranches,
            total_size,
        })
    }

    /// Validate tranche structure for consistency
    fn validate_structure(tranches: &[Tranche]) -> finstack_core::Result<()> {
        // Validate attachment points are finite before sorting
        for tranche in tranches {
            if !tranche.attachment_point.is_finite() || !tranche.detachment_point.is_finite() {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
        }

        // Sort by attachment point for validation
        let mut sorted_tranches = tranches.to_vec();
        sorted_tranches.sort_by(|a, b| a.attachment_point.total_cmp(&b.attachment_point));

        // Check for gaps or overlaps
        let mut expected_attachment = 0.0;
        const TOLERANCE: f64 = 1e-6;

        for tranche in &sorted_tranches {
            if (tranche.attachment_point - expected_attachment).abs() > TOLERANCE {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            if tranche.detachment_point <= tranche.attachment_point {
                return Err(finstack_core::error::InputError::Invalid.into());
            }
            expected_attachment = tranche.detachment_point;
        }

        // Should reach 100%
        if (expected_attachment - 100.0).abs() > TOLERANCE {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        // Check currency consistency
        let base_currency = tranches[0].original_balance.currency();
        for tranche in tranches {
            if tranche.original_balance.currency() != base_currency {
                return Err(finstack_core::Error::CurrencyMismatch {
                    expected: base_currency,
                    actual: tranche.original_balance.currency(),
                });
            }
        }

        Ok(())
    }

    /// Get tranches by seniority
    pub fn by_seniority(&self, seniority: TrancheSeniority) -> Vec<&Tranche> {
        self.tranches
            .iter()
            .filter(|t| t.seniority == seniority)
            .collect()
    }

    /// Get tranches senior to a given tranche
    pub fn senior_to(&self, tranche_id: &str) -> Vec<&Tranche> {
        let target_tranche = self.tranches.iter().find(|t| t.id.as_str() == tranche_id);

        if let Some(target) = target_tranche {
            self.tranches
                .iter()
                .filter(|t| t.payment_priority < target.payment_priority)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get total balance of senior tranches
    pub fn senior_balance(&self, tranche_id: &str) -> Money {
        self.senior_to(tranche_id)
            .iter()
            .try_fold(Money::new(0.0, self.total_size.currency()), |acc, t| {
                acc.checked_add(t.current_balance)
            })
            .unwrap_or_else(|_| Money::new(0.0, self.total_size.currency()))
    }

    /// Calculate tranche subordination amount
    pub fn subordination_amount(&self, tranche_id: &str) -> Money {
        let target_tranche = self.tranches.iter().find(|t| t.id.as_str() == tranche_id);

        if let Some(target) = target_tranche {
            self.tranches
                .iter()
                .filter(|t| t.payment_priority > target.payment_priority)
                .try_fold(Money::new(0.0, self.total_size.currency()), |acc, t| {
                    acc.checked_add(t.current_balance)
                })
                .unwrap_or_else(|_| Money::new(0.0, self.total_size.currency()))
        } else {
            Money::new(0.0, self.total_size.currency())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use time::Month;

    fn test_date() -> Date {
        Date::from_calendar_date(2024, Month::January, 1).expect("valid date")
    }

    #[test]
    fn test_tranche_creation() {
        let tranche = Tranche::new(
            "EQUITY",
            0.0,
            10.0,
            TrancheSeniority::Equity,
            Money::new(100_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.12 },
            test_date(),
        )
        .expect("should succeed");

        assert_eq!(tranche.attachment_point, 0.0);
        assert_eq!(tranche.detachment_point, 10.0);
        assert_eq!(tranche.thickness(), 10.0);
        assert!(tranche.is_first_loss());
    }

    #[test]
    fn test_loss_allocation() {
        let tranche = Tranche::new(
            "MEZZ",
            10.0,
            15.0,
            TrancheSeniority::Mezzanine,
            Money::new(50_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.08 },
            test_date(),
        )
        .expect("should succeed");

        let pool_balance = Money::new(1_000_000_000.0, Currency::USD);

        // No loss case
        let loss = tranche.loss_allocation(5.0, pool_balance);
        assert_eq!(loss.amount(), 0.0);

        // Partial loss case (12% cumulative loss)
        let loss = tranche.loss_allocation(12.0, pool_balance);
        assert!(loss.amount() > 0.0);
        assert!(loss.amount() < tranche.original_balance.amount());

        // Full loss case (20% cumulative loss)
        let loss = tranche.loss_allocation(20.0, pool_balance);
        assert_eq!(loss.amount(), tranche.original_balance.amount());
    }

    #[test]
    fn test_tranche_structure_validation() {
        let equity = TrancheBuilder::new()
            .id("EQUITY")
            .attachment_detachment(0.0, 10.0)
            .seniority(TrancheSeniority::Equity)
            .balance(Money::new(100_000_000.0, Currency::USD))
            .coupon(TrancheCoupon::Fixed { rate: 0.12 })
            .legal_maturity(test_date())
            .build()
            .expect("should succeed");

        let senior = TrancheBuilder::new()
            .id("SENIOR")
            .attachment_detachment(10.0, 100.0)
            .seniority(TrancheSeniority::Senior)
            .balance(Money::new(900_000_000.0, Currency::USD))
            .coupon(TrancheCoupon::Floating(
                crate::cashflow::builder::FloatingRateSpec {
                    index_id: CurveId::new("SOFR-3M".to_string()),
                    spread_bp: 150.0,
                    gearing: 1.0,
                    gearing_includes_spread: true,
                    floor_bp: None,
                    cap_bp: None,
                    all_in_floor_bp: None,
                    index_cap_bp: None,
                    reset_freq: finstack_core::dates::Frequency::quarterly(),
                    reset_lag_days: 2,
                    dc: finstack_core::dates::DayCount::Act360,
                    bdc: finstack_core::dates::BusinessDayConvention::ModifiedFollowing,
                    calendar_id: None,
                    fixing_calendar_id: None,
                },
            ))
            .legal_maturity(test_date())
            .build()
            .expect("should succeed");

        let structure = TrancheStructure::new(vec![equity, senior]).expect("should succeed");
        assert_eq!(structure.tranches.len(), 2);
        assert_eq!(structure.total_size.amount(), 1_000_000_000.0);
    }

    #[test]
    fn test_coverage_trigger() {
        let trigger =
            CoverageTrigger::new(1.20, TriggerConsequence::DivertCashFlow).with_cure_level(1.25);

        // Breach scenario
        assert!(trigger.is_breached(1.15));
        assert!(!trigger.is_cured(1.22)); // Below cure level
        assert!(trigger.is_cured(1.26)); // Above cure level

        // Not breached
        assert!(!trigger.is_breached(1.25));
    }
}
