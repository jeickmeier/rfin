//! Unified structured credit instrument (ABS, CLO, CMBS, RMBS).
//!
//! This module consolidates four nearly-identical instrument types into a single
//! clean implementation using composition for deal-specific differences.

use super::components::{
    AssetPool, CreditFactors, DealType, DefaultModelSpec, MarketConditions, PrepaymentModelSpec,
    RecoveryModelSpec, TrancheCashflowResult, TrancheStructure, TrancheValuation,
    TrancheValuationExt, WaterfallEngine,
};
use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use super::config::DefaultAssumptions;
use std::any::Any;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Deal metadata (counterparties and identifiers).
///
/// This struct captures operational metadata about the deal's service providers
/// and external parties, separate from behavioral/pricing assumptions.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DealMetadata {
    /// Manager identifier (for CLO)
    pub manager_id: Option<String>,
    /// Servicer identifier (for ABS/RMBS/CMBS)
    pub servicer_id: Option<String>,
    /// Master servicer identifier (for CMBS/RMBS)
    pub master_servicer_id: Option<String>,
    /// Special servicer identifier (for CMBS)
    pub special_servicer_id: Option<String>,
    /// Trustee identifier (for ABS)
    pub trustee_id: Option<String>,
}

/// Behavioral overrides for prepayment, default, and recovery assumptions.
///
/// These fields allow instrument-level overrides of the model specifications.
/// If set, they take precedence over the model specs for specific parameters.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BehaviorOverrides {
    // Prepayment overrides
    /// Override prepayment with constant annual CPR
    pub cpr_annual: Option<f64>,
    /// Override prepayment with monthly ABS speed
    pub abs_speed: Option<f64>,
    /// Override prepayment with PSA multiplier
    pub psa_speed_multiplier: Option<f64>,

    // Default overrides
    /// Override default with constant annual CDR
    pub cdr_annual: Option<f64>,
    /// Override default with SDA multiplier
    pub sda_speed_multiplier: Option<f64>,

    // Recovery overrides
    /// Override recovery with constant rate
    pub recovery_rate: Option<f64>,
    /// Override recovery lag (months)
    pub recovery_lag_months: Option<u32>,

    // Trading overrides
    /// Reinvestment price constraint (% of par)
    pub reinvestment_price: Option<f64>,
}

/// Unified structured credit instrument representation.
///
/// This single type replaces the previous separate `Abs`, `Clo`, `Cmbs`, and `Rmbs`
/// types, consolidating ~1,400 lines of near-duplicate code into a clean, composable design.
#[derive(Clone, finstack_valuations_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StructuredCredit {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (ABS/CLO/CMBS/RMBS)
    pub deal_type: DealType,

    /// Asset pool definition
    pub pool: AssetPool,

    /// Tranche structure
    pub tranches: TrancheStructure,

    /// Waterfall distribution rules
    pub waterfall: WaterfallEngine,

    /// Key dates
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub reinvestment_end_date: Option<Date>,
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Discount curve for valuation
    pub discount_curve_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_prepayment_spec")
    )]
    pub prepayment_spec: PrepaymentModelSpec,

    /// Default model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_default_spec")
    )]
    pub default_spec: DefaultModelSpec,

    /// Recovery model specification
    #[cfg_attr(
        feature = "serde",
        serde(default = "StructuredCredit::default_recovery_spec")
    )]
    pub recovery_spec: RecoveryModelSpec,

    /// Market conditions impacting behavior
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior
    pub credit_factors: CreditFactors,

    /// Deal metadata (counterparties, identifiers)
    #[cfg_attr(feature = "serde", serde(default))]
    pub deal_metadata: DealMetadata,

    /// Behavioral assumption overrides
    #[cfg_attr(feature = "serde", serde(default))]
    pub behavior_overrides: BehaviorOverrides,

    /// Default behavioral assumptions for the deal.
    #[cfg_attr(feature = "serde", serde(default))]
    pub default_assumptions: DefaultAssumptions,
}

/// Deal-specific configuration for constructor
struct DealConfig {
    first_payment_date: Date,
    payment_frequency: Frequency,
    prepayment_spec: PrepaymentModelSpec,
    default_spec: DefaultModelSpec,
    recovery_spec: RecoveryModelSpec,
    credit_factors: CreditFactors,
    deal_metadata: DealMetadata,
    behavior_overrides: BehaviorOverrides,
}

/// Core instrument parameters shared across constructors
struct InstrumentParams<'a> {
    pool: AssetPool,
    tranches: TrancheStructure,
    waterfall: WaterfallEngine,
    legal_maturity: Date,
    discount_curve_id: &'a str,
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
        waterfall: WaterfallEngine,
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

    /// Internal helper to create structured credit with common fields
    fn new_with_deal_config(
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
            waterfall: params.waterfall,
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
        }
    }

    /// Create a new ABS instrument from its building blocks.
    pub fn new_abs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: WaterfallEngine,
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
                waterfall,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .unwrap(),
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
    pub fn new_clo(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: WaterfallEngine,
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
                waterfall,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1).unwrap(),
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
    pub fn new_cmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: WaterfallEngine,
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
                waterfall,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .unwrap(),
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
    pub fn new_rmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: WaterfallEngine,
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
                waterfall,
                legal_maturity,
                discount_curve_id: &disc_id_str,
            },
            DealConfig {
                first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1)
                    .unwrap(),
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

    /// Calculate current loss percentage of the pool.
    pub fn current_loss_percentage(&self) -> f64 {
        let total_balance = self.pool.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
            / total_balance
            * DECIMAL_TO_PERCENT
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        Ok(self.pool.weighted_avg_maturity(as_of))
    }

    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::constant_cpr(0.10) // Generic 10% CPR
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::constant_cdr(0.02) // Generic 2% CDR
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::with_lag(0.40, 12) // Generic 40% recovery, 12 month lag
    }

    /// Create waterfall engine based on deal type
    fn create_waterfall_engine_internal(&self) -> WaterfallEngine {
        use super::components::{
            ManagementFeeType, PaymentCalculation, PaymentRecipient, Recipient, WaterfallEngine,
        };
        use super::config::{
            ABS_SERVICING_FEE_BPS, BASIS_POINTS_DIVISOR, CLO_SENIOR_MGMT_FEE_BPS,
            CLO_TRUSTEE_FEE_ANNUAL, CMBS_MASTER_SERVICER_FEE_BPS, RMBS_SERVICING_FEE_BPS,
        };

        let base_ccy = self.pool.base_currency();

        let fees = match self.deal_type {
            DealType::ABS => {
                vec![Recipient::new(
                    "servicing_fees",
                    PaymentRecipient::ServiceProvider("Servicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: ABS_SERVICING_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                )]
            }
            DealType::CLO => {
                vec![
                    Recipient::new(
                        "trustee_fees",
                        PaymentRecipient::ServiceProvider("Trustee".to_string()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(CLO_TRUSTEE_FEE_ANNUAL, base_ccy),
                        },
                    ),
                    Recipient::new(
                        "senior_mgmt_fee",
                        PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
                        PaymentCalculation::PercentageOfCollateral {
                            rate: CLO_SENIOR_MGMT_FEE_BPS / BASIS_POINTS_DIVISOR,
                            annualized: true,
                        },
                    ),
                ]
            }
            DealType::CMBS => {
                vec![Recipient::new(
                    "master_servicing",
                    PaymentRecipient::ServiceProvider("MasterServicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: CMBS_MASTER_SERVICER_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                )]
            }
            DealType::RMBS => {
                vec![Recipient::new(
                    "servicing_fees",
                    PaymentRecipient::ServiceProvider("Servicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: RMBS_SERVICING_FEE_BPS / BASIS_POINTS_DIVISOR,
                        annualized: true,
                    },
                )]
            }
            _ => vec![],
        };

        WaterfallEngine::standard_sequential(base_ccy, &self.tranches, fees)
    }
}

impl CashflowProvider for StructuredCredit {
    fn build_schedule(
        &self,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        use super::instrument_trait::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_tranche_cashflows(self, context, as_of)
    }
}

impl Instrument for StructuredCredit {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        crate::pricer::InstrumentType::StructuredCredit
    }

    fn as_any(&self) -> &dyn Any {
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

    fn value(&self, context: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        let disc = context.get_discount_ref(self.discount_curve_id.as_str())?;
        let flows = self.build_schedule(context, as_of)?;

        use crate::instruments::common::discountable::Discountable;
        flows.npv(disc, as_of, finstack_core::dates::DayCount::Act360)
    }

    fn price_with_metrics(
        &self,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;

        if metrics.is_empty() {
            return Ok(ValuationResult::stamped(
                self.id.as_str(),
                as_of,
                base_value,
            ));
        }

        let flows = self.build_schedule(context, as_of)?;
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            base_value,
        );
        metric_context.cashflows = Some(flows);
        metric_context.discount_curve_id = Some(self.discount_curve_id.to_owned());

        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;

        let mut result = ValuationResult::stamped(self.id.as_str(), as_of, base_value);
        for (metric_id, value) in computed_metrics {
            result.measures.insert(metric_id.to_string(), value);
        }

        Ok(result)
    }
}

impl crate::instruments::common::pricing::HasDiscountCurve for StructuredCredit {
    fn discount_curve_id(&self) -> &CurveId {
        &self.discount_curve_id
    }
}

impl super::instrument_trait::StructuredCreditInstrument for StructuredCredit {
    fn pool(&self) -> &AssetPool {
        &self.pool
    }

    fn tranches(&self) -> &TrancheStructure {
        &self.tranches
    }

    fn closing_date(&self) -> Date {
        self.closing_date
    }

    fn first_payment_date(&self) -> Date {
        self.first_payment_date
    }

    fn legal_maturity(&self) -> Date {
        self.legal_maturity
    }

    fn payment_frequency(&self) -> Frequency {
        self.payment_frequency
    }

    fn prepayment_spec(&self) -> &PrepaymentModelSpec {
        &self.prepayment_spec
    }

    fn default_spec_ref(&self) -> &DefaultModelSpec {
        &self.default_spec
    }

    fn recovery_spec_ref(&self) -> &RecoveryModelSpec {
        &self.recovery_spec
    }

    fn default_assumptions(&self) -> &DefaultAssumptions {
        &self.default_assumptions
    }

    fn market_conditions(&self) -> &MarketConditions {
        &self.market_conditions
    }

    fn credit_factors(&self) -> &CreditFactors {
        &self.credit_factors
    }

    fn create_waterfall_engine(&self) -> WaterfallEngine {
        self.create_waterfall_engine_internal()
    }

    fn prepayment_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        use super::components::annual_to_monthly;

        // Check overrides in priority order
        if let Some(abs_speed) = self.behavior_overrides.abs_speed {
            return Some(abs_speed);
        }

        if let Some(cpr) = self.behavior_overrides.cpr_annual {
            return Some(annual_to_monthly(cpr));
        }

        if let Some(psa_mult) = self.behavior_overrides.psa_speed_multiplier {
            // Inline PSA calculation
            use super::components::annual_to_monthly;
            let psa_ramp_months = 30;
            let psa_terminal_cpr = 0.06;
            let base_cpr = if seasoning <= psa_ramp_months {
                (seasoning as f64 / psa_ramp_months as f64) * psa_terminal_cpr
            } else {
                psa_terminal_cpr
            };
            let cpr = base_cpr * psa_mult;
            return Some(annual_to_monthly(cpr));
        }

        None
    }

    fn default_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        use super::components::annual_to_monthly;

        // Check overrides in priority order
        if let Some(cdr) = self.behavior_overrides.cdr_annual {
            return Some(annual_to_monthly(cdr));
        }

        if let Some(sda_mult) = self.behavior_overrides.sda_speed_multiplier {
            // Inline SDA calculation
            let peak_month = 30;
            let peak_cdr = 0.006;
            let terminal_cdr = 0.0003;

            let cdr = if seasoning <= peak_month {
                // Ramp up to peak
                (seasoning as f64 / peak_month as f64) * peak_cdr
            } else if seasoning <= 60 {
                // Decline from peak to terminal
                let months_past_peak = (seasoning - peak_month) as f64;
                let decline_period = 30.0;
                peak_cdr - (months_past_peak / decline_period) * (peak_cdr - terminal_cdr)
            } else {
                // Terminal rate
                terminal_cdr
            } * sda_mult;

            // Convert CDR to MDR
            return Some(1.0 - (1.0 - cdr).powf(1.0 / 12.0));
        }

        None
    }
}

impl TrancheValuationExt for StructuredCredit {
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<TrancheCashflowResult> {
        use super::instrument_trait::StructuredCreditInstrument;
        <Self as StructuredCreditInstrument>::generate_specific_tranche_cashflows(
            self, tranche_id, context, as_of,
        )
    }

    fn value_tranche(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        let cashflows = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let disc = context.get_discount(&self.discount_curve_id)?;

        // Pre-compute as_of discount factor for correct theta
        let disc_dc = disc.day_count();
        let t_as_of = disc_dc
            .year_fraction(
                disc.base_date(),
                as_of,
                finstack_core::dates::DayCountCtx::default(),
            )
            .unwrap_or(0.0);
        let df_as_of = disc.df(t_as_of);

        let mut pv = Money::new(0.0, self.pool.base_currency());
        for (date, amount) in &cashflows.cashflows {
            if *date > as_of {
                // Discount from as_of for correct theta
                let t_cf = disc_dc
                    .year_fraction(
                        disc.base_date(),
                        *date,
                        finstack_core::dates::DayCountCtx::default(),
                    )
                    .unwrap_or(0.0);
                let df_cf_abs = disc.df(t_cf);
                let df = if df_as_of != 0.0 {
                    df_cf_abs / df_as_of
                } else {
                    1.0
                };
                let flow_pv = Money::new(amount.amount() * df, amount.currency());
                pv = pv.checked_add(flow_pv)?;
            }
        }

        Ok(pv)
    }

    fn value_tranche_with_metrics(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<TrancheValuation> {
        use super::components::{
            calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
            calculate_tranche_z_spread,
        };

        let cashflow_result = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let pv = self.value_tranche(tranche_id, context, as_of)?;

        // Most metrics are calculated via the generic metrics registry.
        // We create a context and pass the detailed cashflow result to it.
        let mut metric_context = crate::metrics::MetricContext::new(
            std::sync::Arc::new(self.clone())
                as std::sync::Arc<dyn crate::instruments::common::traits::Instrument>,
            std::sync::Arc::new(context.clone()),
            as_of,
            pv,
        );
        metric_context.cashflows = Some(cashflow_result.cashflows.clone());
        metric_context.detailed_tranche_cashflows = Some(cashflow_result.clone());
        metric_context.discount_curve_id = Some(self.discount_curve_id.to_owned());

        let registry = crate::metrics::standard_registry();
        let computed_metrics = registry.compute(metrics, &mut metric_context)?;

        let tranche = self
            .tranches
            .tranches
            .iter()
            .find(|t| t.id.as_str() == tranche_id)
            .ok_or_else(|| {
                finstack_core::Error::from(finstack_core::error::InputError::NotFound {
                    id: format!("tranche:{}", tranche_id),
                })
            })?;

        let notional = tranche.original_balance.amount();

        let dirty_price = if notional > 0.0 {
            (pv.amount() / notional) * 100.0
        } else {
            0.0
        };

        let accrued_value = computed_metrics
            .get(&MetricId::Accrued)
            .copied()
            .unwrap_or(0.0);
        let accrued = Money::new(accrued_value, pv.currency());

        let clean_price = if notional > 0.0 {
            dirty_price - (accrued.amount() / notional) * 100.0
        } else {
            dirty_price
        };

        // Ensure WAL is calculated if requested, as it's a primary output field
        let wal = match computed_metrics.get(&MetricId::WAL) {
            Some(v) => *v,
            None => calculate_tranche_wal(&cashflow_result, as_of)?,
        };

        // Fallback calculations for metrics not handled by the registry or if not requested
        let disc = context.get_discount(&self.discount_curve_id)?;
        let modified_duration = computed_metrics
            .get(&MetricId::DurationMod)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_duration(&cashflow_result.cashflows, &disc, as_of, pv)
                    .unwrap_or(0.0)
            });

        let z_spread = computed_metrics
            .get(&MetricId::ZSpread)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_z_spread(&cashflow_result.cashflows, &disc, pv, as_of)
                    .unwrap_or(0.0)
            });

        let z_spread_decimal = z_spread / 10_000.0;
        let cs01 = computed_metrics
            .get(&MetricId::Cs01)
            .copied()
            .unwrap_or_else(|| {
                calculate_tranche_cs01(&cashflow_result.cashflows, &disc, z_spread_decimal, as_of)
                    .unwrap_or(0.0)
            });

        let ytm = computed_metrics
            .get(&MetricId::Ytm)
            .copied()
            .unwrap_or(0.05); // Default guess

        // Convert computed metrics to std::collections::HashMap for the TrancheValuation struct
        let final_metrics: std::collections::HashMap<MetricId, f64> =
            computed_metrics.into_iter().collect();

        Ok(TrancheValuation {
            tranche_id: tranche_id.to_string(),
            pv,
            clean_price,
            dirty_price,
            accrued,
            wal,
            modified_duration,
            z_spread_bps: z_spread,
            cs01,
            ytm,
            metrics: final_metrics,
        })
    }
}

impl core::fmt::Debug for StructuredCredit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StructuredCredit")
            .field("id", &self.id)
            .field("deal_type", &self.deal_type)
            .field("closing_date", &self.closing_date)
            .field("first_payment_date", &self.first_payment_date)
            .field("legal_maturity", &self.legal_maturity)
            .field("payment_frequency", &self.payment_frequency)
            .field("discount_curve_id", &self.discount_curve_id)
            .finish()
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::super::components::{Tranche, TrancheCoupon, TrancheSeniority, TrancheStructure};
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::Date;
    use finstack_core::money::Money;
    use time::Month;

    #[test]
    fn test_structured_credit_json_roundtrip() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

        let tranche = Tranche::new(
            "EQUITY",
            0.0,
            100.0,
            TrancheSeniority::Equity,
            Money::new(1_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.12 },
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![tranche]).unwrap();
        let waterfall = WaterfallEngine::new(Currency::USD);

        let original = StructuredCredit::new_clo(
            "TEST_CLO",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize from JSON
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify key fields match
        assert_eq!(original.id.as_str(), deserialized.id.as_str());
        assert_eq!(original.deal_type, deserialized.deal_type);
        assert_eq!(original.prepayment_spec, deserialized.prepayment_spec);
        assert_eq!(original.default_spec, deserialized.default_spec);
        assert_eq!(original.recovery_spec, deserialized.recovery_spec);
    }

    #[test]
    fn test_behavior_overrides_serialization() {
        let pool = AssetPool::new("TEST_POOL", DealType::RMBS, Currency::USD);

        let tranche = Tranche::new(
            "AAA",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(10_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2035, Month::January, 1).unwrap(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![tranche]).unwrap();
        let waterfall = WaterfallEngine::new(Currency::USD);

        let mut rmbs = StructuredCredit::new_rmbs(
            "TEST_RMBS",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).unwrap(),
            Date::from_calendar_date(2035, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        // Set behavior overrides
        rmbs.behavior_overrides.psa_speed_multiplier = Some(1.5);
        rmbs.behavior_overrides.cdr_annual = Some(0.01);

        // Serialize
        let json = serde_json::to_string(&rmbs).expect("Failed to serialize");

        // Deserialize
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify overrides are preserved
        assert_eq!(
            deserialized.behavior_overrides.psa_speed_multiplier,
            Some(1.5)
        );
        assert_eq!(deserialized.behavior_overrides.cdr_annual, Some(0.01));
    }

    #[test]
    fn test_deal_metadata_serialization() {
        let pool = AssetPool::new("TEST_POOL", DealType::CLO, Currency::USD);

        let tranche = Tranche::new(
            "AAA",
            0.0,
            100.0,
            TrancheSeniority::Senior,
            Money::new(10_000_000.0, Currency::USD),
            TrancheCoupon::Fixed { rate: 0.05 },
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        )
        .unwrap();

        let tranches = TrancheStructure::new(vec![tranche]).unwrap();
        let waterfall = WaterfallEngine::new(Currency::USD);

        let mut clo = StructuredCredit::new_clo(
            "TEST_CLO",
            pool,
            tranches,
            waterfall,
            Date::from_calendar_date(2024, Month::January, 1).unwrap(),
            Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            "USD-OIS",
        );

        // Set deal metadata
        clo.deal_metadata.manager_id = Some("Apollo".to_string());
        clo.deal_metadata.servicer_id = Some("BNY Mellon".to_string());

        // Serialize
        let json = serde_json::to_string(&clo).expect("Failed to serialize");

        // Deserialize
        let deserialized: StructuredCredit =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify metadata is preserved
        assert_eq!(
            deserialized.deal_metadata.manager_id,
            Some("Apollo".to_string())
        );
        assert_eq!(
            deserialized.deal_metadata.servicer_id,
            Some("BNY Mellon".to_string())
        );
    }
}
