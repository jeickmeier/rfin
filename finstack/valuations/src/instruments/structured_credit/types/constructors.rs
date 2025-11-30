//! Constructor methods for StructuredCredit instruments.
//!
//! This module provides deal-type specific constructors that apply
//! appropriate defaults for ABS, CLO, CMBS, and RMBS instruments.

use super::{
    AllocationMode, AssetPool, BehaviorOverrides, CreditFactors, DealMetadata, DealType,
    DefaultModelSpec, MarketConditions, PaymentType, PrepaymentModelSpec, Recipient, RecoveryModelSpec,
    StructuredCredit, Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure, Waterfall,
    WaterfallTier,
};
use crate::instruments::structured_credit::types::setup::DefaultAssumptions;
use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use crate::instruments::common::traits::Attributes;

/// Deal-specific configuration for constructor
pub(super) struct DealConfig {
    pub first_payment_date: Date,
    pub payment_frequency: Frequency,
    pub prepayment_spec: PrepaymentModelSpec,
    pub default_spec: DefaultModelSpec,
    pub recovery_spec: RecoveryModelSpec,
    pub credit_factors: CreditFactors,
    pub deal_metadata: DealMetadata,
    pub behavior_overrides: BehaviorOverrides,
}

/// Core instrument parameters shared across constructors
pub(super) struct InstrumentParams<'a> {
    pub pool: AssetPool,
    pub tranches: TrancheStructure,
    pub legal_maturity: Date,
    pub discount_curve_id: &'a str,
}

impl StructuredCredit {
    /// Apply deal-type specific defaults to a builder.
    ///
    /// This method configures sensible defaults for payment frequency,
    /// behavioral models, and other deal-type specific parameters.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let clo = StructuredCredit::builder()
    ///     .id("MY_CLO")
    ///     .deal_type(DealType::CLO)
    ///     .apply_deal_defaults()  // Sets quarterly payment, 15% CPR, 2% CDR, etc.
    ///     .pool(pool)
    ///     .tranches(tranches)
    ///     .build()?;
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn apply_deal_defaults(
        id: impl Into<String>,
        deal_type: DealType,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: Waterfall,
        closing_date: Date,
        legal_maturity: Date,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        match deal_type {
            DealType::ABS => Self::new_abs(
                id,
                pool,
                tranches,
                waterfall,
                closing_date,
                legal_maturity,
                discount_curve_id,
            ),
            DealType::CLO => Self::new_clo(
                id,
                pool,
                tranches,
                waterfall,
                closing_date,
                legal_maturity,
                discount_curve_id,
            ),
            DealType::CMBS => Self::new_cmbs(
                id,
                pool,
                tranches,
                waterfall,
                closing_date,
                legal_maturity,
                discount_curve_id,
            ),
            DealType::RMBS => Self::new_rmbs(
                id,
                pool,
                tranches,
                waterfall,
                closing_date,
                legal_maturity,
                discount_curve_id,
            ),
            _ => Self::new_abs(
                id,
                pool,
                tranches,
                waterfall,
                closing_date,
                legal_maturity,
                discount_curve_id,
            ), // Default to ABS
        }
    }

    /// Create a canonical example CLO structured credit deal with minimal components.
    ///
    /// This method is intended for testing, documentation examples, and quick prototyping.
    /// It creates a fully valid CLO deal with a single senior tranche and basic waterfall.
    ///
    /// # Panics
    ///
    /// Panics if the hard-coded example dates (2024-01-01, 2034-01-01) are invalid.
    /// This should never happen barring library bugs in the `time` crate.
    #[must_use]
    pub fn example() -> Self {
        use finstack_core::currency::Currency;
        use time::Month;
        // Build a minimal pool (empty assets for example purposes)
        let pool = AssetPool::new("POOL-1", DealType::CLO, Currency::USD);
        // Build a simple tranche structure with single tranche 0-100%
        let tranche = Tranche::new(
            "CLONOTES-A",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(100_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.06 },
            Date::from_calendar_date(2034, Month::January, 1).expect("Valid example date"),
        )
        .expect("Tranche build should not fail");
        let tranches = TrancheStructure::new(vec![tranche]).expect("TrancheStructure should build");
        // Build a simple 2-tier waterfall: pay interest then principal to the tranche
        let waterfall = Waterfall::new(Currency::USD)
            .add_tier(
                WaterfallTier::new("Tier1-Interest", 1, PaymentType::Interest)
                    .allocation_mode(AllocationMode::Sequential)
                    .add_recipient(Recipient::tranche_interest("A-INT", "CLONOTES-A")),
            )
            .add_tier(
                WaterfallTier::new("Tier2-Principal", 2, PaymentType::Principal)
                    .allocation_mode(AllocationMode::Sequential)
                    .add_recipient(Recipient::tranche_principal("A-PRIN", "CLONOTES-A", None)),
            );
        let closing =
            Date::from_calendar_date(2024, Month::January, 1).expect("Valid example date");
        let legal = Date::from_calendar_date(2034, Month::January, 1).expect("Valid example date");
        StructuredCredit::new_clo(
            "CLO-EXAMPLE",
            pool,
            tranches,
            waterfall,
            closing,
            legal,
            "USD-OIS",
        )
    }

    /// Internal helper to create structured credit with common fields
    pub(super) fn new_with_deal_config(
        id: impl Into<String>,
        deal_type: DealType,
        params: InstrumentParams,
        config: DealConfig,
        closing_date: Date,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type,
            pool: params.pool,
            tranches: params.tranches,
            closing_date,
            first_payment_date: config.first_payment_date,
            reinvestment_end_date: None,
            legal_maturity: params.legal_maturity,
            payment_frequency: config.payment_frequency,
            discount_curve_id: CurveId::new(params.discount_curve_id.to_string()),
            attributes: Attributes::new(),
            prepayment_spec: config.prepayment_spec,
            default_spec: config.default_spec,
            recovery_spec: config.recovery_spec,
            market_conditions: MarketConditions::default(),
            credit_factors: config.credit_factors,
            deal_metadata: config.deal_metadata,
            behavior_overrides: config.behavior_overrides,
            default_assumptions: DefaultAssumptions::default(),
            // Stochastic specs default to None (deterministic pricing)
            stochastic_prepay_spec: None,
            stochastic_default_spec: None,
            correlation_structure: None,
            // Hedge swaps default to empty
            hedge_swaps: Vec::new(),
        }
    }

    /// Create a new ABS instrument from its building blocks.
    ///
    /// Note: The waterfall parameter is accepted for backward compatibility but ignored.
    /// Waterfall is now created dynamically based on deal type.
    #[allow(unused_variables)]
    pub fn new_abs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: Waterfall,
        closing_date: Date,
        legal_maturity: Date,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        let disc_id_str = discount_curve_id.into();
        let mut inst = Self::new_with_deal_config(
            id,
            DealType::ABS,
            InstrumentParams {
                pool,
                tranches,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .expect("Valid example date"),
                payment_frequency: Frequency::monthly(),
                prepayment_spec: PrepaymentModelSpec::constant_cpr(0.18), // Auto ABS standard
                default_spec: DefaultModelSpec::constant_cdr(0.015),      // Consumer standard
                recovery_spec: RecoveryModelSpec::with_lag(0.70, 12),     // Collateral-backed
                credit_factors: CreditFactors::default(),
                deal_metadata: DealMetadata::default(),
                behavior_overrides: BehaviorOverrides::default(),
            },
            closing_date,
        );
        inst.default_assumptions = DefaultAssumptions::abs_auto_standard();
        inst
    }

    /// Create a new CLO instrument from its building blocks.
    ///
    /// Note: The waterfall parameter is accepted for backward compatibility but ignored.
    /// Waterfall is now created dynamically based on deal type.
    #[allow(unused_variables)]
    pub fn new_clo(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: Waterfall,
        closing_date: Date,
        legal_maturity: Date,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        let disc_id_str = discount_curve_id.into();
        let mut inst = Self::new_with_deal_config(
            id,
            DealType::CLO,
            InstrumentParams {
                pool,
                tranches,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1)
                    .expect("Valid example date"),
                payment_frequency: Frequency::quarterly(),
                prepayment_spec: PrepaymentModelSpec::constant_cpr(0.15),
                default_spec: DefaultModelSpec::constant_cdr(0.025), // Corporate standard
                recovery_spec: RecoveryModelSpec::with_lag(0.40, 18), // Corporate unsecured
                credit_factors: CreditFactors::default(),
                deal_metadata: DealMetadata::default(),
                behavior_overrides: BehaviorOverrides::default(),
            },
            closing_date,
        );
        inst.default_assumptions = DefaultAssumptions::clo_standard();
        inst
    }

    /// Create a new CMBS instrument from its building blocks.
    ///
    /// Note: The waterfall parameter is accepted for backward compatibility but ignored.
    /// Waterfall is now created dynamically based on deal type.
    #[allow(unused_variables)]
    pub fn new_cmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: Waterfall,
        closing_date: Date,
        legal_maturity: Date,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        let disc_id_str = discount_curve_id.into();
        let mut inst = Self::new_with_deal_config(
            id,
            DealType::CMBS,
            InstrumentParams {
                pool,
                tranches,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .expect("Valid example date"),
                payment_frequency: Frequency::monthly(),
                prepayment_spec: PrepaymentModelSpec::constant_cpr(0.10), // CMBS standard
                default_spec: DefaultModelSpec::constant_cdr(0.01),       // Commercial real estate
                recovery_spec: RecoveryModelSpec::with_lag(0.60, 24),     // Commercial collateral
                credit_factors: CreditFactors::default(),
                deal_metadata: DealMetadata::default(),
                behavior_overrides: BehaviorOverrides::default(),
            },
            closing_date,
        );
        inst.default_assumptions = DefaultAssumptions::cmbs_standard();
        inst
    }

    /// Create a new RMBS instrument from its building blocks.
    ///
    /// Note: The waterfall parameter is accepted for backward compatibility but ignored.
    /// Waterfall is now created dynamically based on deal type.
    #[allow(unused_variables)]
    pub fn new_rmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: Waterfall,
        closing_date: Date,
        legal_maturity: Date,
        discount_curve_id: impl Into<String>,
    ) -> Self {
        let disc_id_str = discount_curve_id.into();
        let mut inst = Self::new_with_deal_config(
            id,
            DealType::RMBS,
            InstrumentParams {
                pool,
                tranches,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .expect("Valid example date"),
                payment_frequency: Frequency::monthly(),
                prepayment_spec: PrepaymentModelSpec::psa(1.0), // 100% PSA
                default_spec: DefaultModelSpec::constant_cdr(0.005), // RMBS standard
                recovery_spec: RecoveryModelSpec::with_lag(0.70, 18), // Mortgage collateral
                credit_factors: CreditFactors {
                    ltv: Some(0.80),
                    ..Default::default()
                },
                deal_metadata: DealMetadata::default(),
                behavior_overrides: BehaviorOverrides::default(),
            },
            closing_date,
        );
        inst.default_assumptions = DefaultAssumptions::rmbs_standard();
        inst
    }

}
