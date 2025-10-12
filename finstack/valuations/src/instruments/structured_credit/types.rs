//! Unified structured credit instrument (ABS, CLO, CMBS, RMBS).
//!
//! This module consolidates four nearly-identical instrument types into a single
//! clean implementation using composition for deal-specific differences.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::constants::DECIMAL_TO_PERCENT;
use crate::instruments::common::structured_credit::{
    AssetPool, CoverageTests, CreditFactors, DealType, DefaultBehavior, DefaultModelSpec,
    MarketConditions, PrepaymentBehavior, PrepaymentModelSpec, RecoveryBehavior,
    RecoveryModelSpec, StructuredCreditWaterfall, TrancheStructure, TrancheCashflowResult,
    TrancheValuation, TrancheValuationExt,
};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};

use std::any::Any;
use std::sync::Arc;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Instrument-specific fields that differ across deal types.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum InstrumentSpecificFields {
    /// Asset-Backed Security specific fields
    Abs {
        servicer_id: Option<String>,
        trustee_id: Option<String>,
        abs_speed: Option<f64>,
        cdr_annual: Option<f64>,
    },
    /// Collateralized Loan Obligation specific fields
    Clo {
        manager_id: Option<String>,
        servicer_id: Option<String>,
        cpr_annual: Option<f64>,
        cdr_annual: Option<f64>,
        recovery_rate: Option<f64>,
        recovery_lag_months: Option<u32>,
        reinvestment_price: Option<f64>,
    },
    /// Commercial Mortgage-Backed Security specific fields
    Cmbs {
        master_servicer_id: Option<String>,
        special_servicer_id: Option<String>,
        open_cpr: Option<f64>,
        cdr_annual: Option<f64>,
    },
    /// Residential Mortgage-Backed Security specific fields
    Rmbs {
        servicer_id: Option<String>,
        master_servicer_id: Option<String>,
        psa_speed: f64,
        sda_speed: f64,
    },
}

impl Default for InstrumentSpecificFields {
    fn default() -> Self {
        InstrumentSpecificFields::Abs {
            servicer_id: None,
            trustee_id: None,
            abs_speed: None,
            cdr_annual: None,
        }
    }
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
    pub waterfall: StructuredCreditWaterfall,

    /// Coverage tests and monitoring
    pub coverage_tests: CoverageTests,

    /// Key dates
    pub closing_date: Date,
    pub first_payment_date: Date,
    pub reinvestment_end_date: Option<Date>,
    pub legal_maturity: Date,

    /// Payment frequency for the structure
    pub payment_frequency: Frequency,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,

    /// Prepayment model specification
    #[cfg_attr(feature = "serde", serde(default = "StructuredCredit::default_prepayment_spec"))]
    pub prepayment_spec: PrepaymentModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    prepayment_model_cache: once_cell::sync::OnceCell<Arc<dyn PrepaymentBehavior>>,

    /// Default model specification
    #[cfg_attr(feature = "serde", serde(default = "StructuredCredit::default_default_spec"))]
    pub default_spec: DefaultModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    default_model_cache: once_cell::sync::OnceCell<Arc<dyn DefaultBehavior>>,

    /// Recovery model specification
    #[cfg_attr(feature = "serde", serde(default = "StructuredCredit::default_recovery_spec"))]
    pub recovery_spec: RecoveryModelSpec,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[builder(skip)]
    recovery_model_cache: once_cell::sync::OnceCell<Arc<dyn RecoveryBehavior>>,

    /// Market conditions impacting behavior
    pub market_conditions: MarketConditions,

    /// Credit factors impacting default behavior
    pub credit_factors: CreditFactors,

    /// Instrument-specific fields
    #[cfg_attr(feature = "serde", serde(default))]
    pub specific: InstrumentSpecificFields,
}

impl StructuredCredit {
    /// Create a new ABS instrument from its building blocks.
    pub fn new_abs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::ABS,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::AssetDefault {
                asset_type: "auto".to_string(),
            },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "consumer".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "collateral".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
            specific: InstrumentSpecificFields::Abs {
                servicer_id: None,
                trustee_id: None,
                abs_speed: None,
                cdr_annual: None,
            },
        }
    }

    /// Create a new CLO instrument from its building blocks.
    pub fn new_clo(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::CLO,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::April, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::quarterly(),
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::ConstantCpr { cpr: 0.15 },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "corporate".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "corporate".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
            specific: InstrumentSpecificFields::Clo {
                manager_id: None,
                servicer_id: None,
                cpr_annual: None,
                cdr_annual: None,
                recovery_rate: None,
                recovery_lag_months: None,
                reinvestment_price: None,
            },
        }
    }

    /// Create a new CMBS instrument from its building blocks.
    pub fn new_cmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::CMBS,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::AssetDefault {
                asset_type: "cmbs".to_string(),
            },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "commercial".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "commercial".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors: CreditFactors::default(),
            specific: InstrumentSpecificFields::Cmbs {
                master_servicer_id: None,
                special_servicer_id: None,
                open_cpr: None,
                cdr_annual: None,
            },
        }
    }

    /// Create a new RMBS instrument from its building blocks.
    pub fn new_rmbs(
        id: impl Into<String>,
        pool: AssetPool,
        tranches: TrancheStructure,
        waterfall: StructuredCreditWaterfall,
        legal_maturity: Date,
        disc_id: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        let credit_factors = CreditFactors {
            ltv: Some(0.80),
            ..Default::default()
        };
        Self {
            id: InstrumentId::new(id_str),
            deal_type: DealType::RMBS,
            pool,
            tranches,
            waterfall,
            coverage_tests: CoverageTests::new(),
            closing_date: Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, time::Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
            prepayment_spec: PrepaymentModelSpec::Psa { multiplier: 1.0 },
            prepayment_model_cache: once_cell::sync::OnceCell::new(),
            default_spec: DefaultModelSpec::AssetDefault {
                asset_type: "rmbs".to_string(),
            },
            default_model_cache: once_cell::sync::OnceCell::new(),
            recovery_spec: RecoveryModelSpec::AssetDefault {
                asset_type: "mortgage".to_string(),
            },
            recovery_model_cache: once_cell::sync::OnceCell::new(),
            market_conditions: MarketConditions::default(),
            credit_factors,
            specific: InstrumentSpecificFields::Rmbs {
                servicer_id: None,
                master_servicer_id: None,
                psa_speed: 1.0,
                sda_speed: 1.0,
            },
        }
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

    // Note: tranche_cashflows() removed - use TrancheValuationExt::get_tranche_cashflows() instead

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<f64> {
        Ok(self.pool.weighted_avg_maturity(as_of))
    }

    #[cfg(feature = "serde")]
    fn default_prepayment_spec() -> PrepaymentModelSpec {
        PrepaymentModelSpec::AssetDefault {
            asset_type: "generic".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_default_spec() -> DefaultModelSpec {
        DefaultModelSpec::AssetDefault {
            asset_type: "generic".to_string(),
        }
    }

    #[cfg(feature = "serde")]
    fn default_recovery_spec() -> RecoveryModelSpec {
        RecoveryModelSpec::AssetDefault {
            asset_type: "generic".to_string(),
        }
    }

    /// Create waterfall engine based on deal type
    fn create_waterfall_engine_internal(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        use crate::instruments::common::structured_credit::{
            ManagementFeeType, PaymentCalculation, PaymentRecipient, PaymentRule, WaterfallEngine,
        };

        let base_ccy = self.pool.base_currency();

        let fees = match self.deal_type {
            DealType::ABS => {
                vec![PaymentRule::new(
                    "servicing_fees",
                    1,
                    PaymentRecipient::ServiceProvider("Servicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: 0.005, // 50 bps servicing
                        annualized: true,
                    },
                )]
            }
            DealType::CLO => {
                vec![
                    PaymentRule::new(
                        "trustee_fees",
                        1,
                        PaymentRecipient::ServiceProvider("Trustee".to_string()),
                        PaymentCalculation::FixedAmount {
                            amount: Money::new(50_000.0, base_ccy),
                        },
                    ),
                    PaymentRule::new(
                        "senior_mgmt_fee",
                        2,
                        PaymentRecipient::ManagerFee(ManagementFeeType::Senior),
                        PaymentCalculation::PercentageOfCollateral {
                            rate: 0.01,
                            annualized: true,
                        },
                    ),
                ]
            }
            DealType::CMBS => {
                vec![PaymentRule::new(
                    "master_servicing",
                    1,
                    PaymentRecipient::ServiceProvider("MasterServicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: 0.0025, // 25 bps
                        annualized: true,
                    },
                )]
            }
            DealType::RMBS => {
                vec![PaymentRule::new(
                    "servicing_fees",
                    1,
                    PaymentRecipient::ServiceProvider("Servicer".to_string()),
                    PaymentCalculation::PercentageOfCollateral {
                        rate: 0.0025, // 25 bps servicing
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
        use crate::instruments::common::structured_credit::StructuredCreditInstrument;
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
        let disc = context.get_discount_ref(self.disc_id.as_str())?;
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
            return Ok(ValuationResult::stamped(self.id.as_str(), as_of, base_value));
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
        metric_context.discount_curve_id = Some(self.disc_id.to_owned());

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
        &self.disc_id
    }
}

impl crate::instruments::common::structured_credit::StructuredCreditInstrument for StructuredCredit {
    fn pool(&self) -> &crate::instruments::common::structured_credit::AssetPool {
        &self.pool
    }

    fn tranches(&self) -> &crate::instruments::common::structured_credit::TrancheStructure {
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

    fn prepayment_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::PrepaymentBehavior> {
        self.prepayment_model_cache
            .get_or_init(|| self.prepayment_spec.to_arc())
    }

    fn default_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::DefaultBehavior> {
        self.default_model_cache
            .get_or_init(|| self.default_spec.to_arc())
    }

    fn recovery_model(
        &self,
    ) -> &Arc<dyn crate::instruments::common::structured_credit::RecoveryBehavior> {
        self.recovery_model_cache
            .get_or_init(|| self.recovery_spec.to_arc())
    }

    fn market_conditions(
        &self,
    ) -> &crate::instruments::common::structured_credit::MarketConditions {
        &self.market_conditions
    }

    fn credit_factors(&self) -> &crate::instruments::common::structured_credit::CreditFactors {
        &self.credit_factors
    }

    fn create_waterfall_engine(
        &self,
    ) -> crate::instruments::common::structured_credit::WaterfallEngine {
        self.create_waterfall_engine_internal()
    }

    fn prepayment_rate_override(&self, _pay_date: Date, seasoning: u32) -> Option<f64> {
        match &self.specific {
            InstrumentSpecificFields::Abs { abs_speed, .. } => *abs_speed,
            InstrumentSpecificFields::Clo { cpr_annual, .. } => {
                cpr_annual.map(|cpr| {
                    use crate::instruments::common::structured_credit::cpr_to_smm;
                    cpr_to_smm(cpr)
                })
            }
            InstrumentSpecificFields::Cmbs { open_cpr, .. } => {
                open_cpr.map(|cpr| {
                    use crate::instruments::common::structured_credit::cpr_to_smm;
                    cpr_to_smm(cpr)
                })
            }
            InstrumentSpecificFields::Rmbs { psa_speed, .. } => {
                if *psa_speed != 1.0 {
                    use crate::instruments::common::structured_credit::{cpr_to_smm, PSAModel};
                    let psa = PSAModel::new(*psa_speed);
                    let cpr = psa.cpr_at_month(seasoning);
                    Some(cpr_to_smm(cpr))
                } else {
                    None
                }
            }
        }
    }

    fn default_rate_override(&self, pay_date: Date, seasoning: u32) -> Option<f64> {
        match &self.specific {
            InstrumentSpecificFields::Abs { cdr_annual, .. } => {
                cdr_annual.map(|cdr| {
                    use crate::instruments::common::structured_credit::cdr_to_mdr;
                    cdr_to_mdr(cdr)
                })
            }
            InstrumentSpecificFields::Clo { cdr_annual, .. } => {
                cdr_annual.map(|cdr| {
                    use crate::instruments::common::structured_credit::cdr_to_mdr;
                    cdr_to_mdr(cdr)
                })
            }
            InstrumentSpecificFields::Cmbs { cdr_annual, .. } => {
                cdr_annual.map(|cdr| {
                    use crate::instruments::common::structured_credit::cdr_to_mdr;
                    cdr_to_mdr(cdr)
                })
            }
            InstrumentSpecificFields::Rmbs { sda_speed, .. } => {
                if *sda_speed != 1.0 {
                    use crate::instruments::common::structured_credit::{
                        DefaultBehavior, SDAModel,
                    };
                    let sda = SDAModel {
                        speed: *sda_speed,
                        ..Default::default()
                    };
                    Some(sda.default_rate(
                        pay_date,
                        self.closing_date,
                        seasoning,
                        &self.credit_factors,
                    ))
                } else {
                    None
                }
            }
        }
    }
}

impl TrancheValuationExt for StructuredCredit {
    fn get_tranche_cashflows(
        &self,
        tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<TrancheCashflowResult> {
        use crate::instruments::common::structured_credit::StructuredCreditInstrument;
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
        let disc = context.get_discount(&self.disc_id)?;

        let mut pv = Money::new(0.0, self.pool.base_currency());
        for (date, amount) in &cashflows.cashflows {
            if *date > as_of {
                let df = disc.df_on_date_curve(*date);
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
        use crate::instruments::common::structured_credit::{
            calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal,
            calculate_tranche_z_spread,
        };

        let cashflow_result = self.get_tranche_cashflows(tranche_id, context, as_of)?;
        let pv = self.value_tranche(tranche_id, context, as_of)?;

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

        let accrued = Money::new(0.0, pv.currency());
        let clean_price = dirty_price;

        let wal = calculate_tranche_wal(&cashflow_result, as_of)?;

        let disc = context.get_discount(&self.disc_id)?;
        let modified_duration =
            calculate_tranche_duration(&cashflow_result.cashflows, &disc, as_of, pv)?;

        let z_spread = calculate_tranche_z_spread(&cashflow_result.cashflows, &disc, pv, as_of)?;

        let z_spread_decimal = z_spread / 10_000.0;
        let cs01 = calculate_tranche_cs01(&cashflow_result.cashflows, &disc, z_spread_decimal, as_of)?;

        let ytm = 0.05;

        let mut metric_values = std::collections::HashMap::new();
        for metric in metrics {
            match metric {
                MetricId::WAL => metric_values.insert(MetricId::WAL, wal),
                MetricId::DurationMod => {
                    metric_values.insert(MetricId::DurationMod, modified_duration)
                }
                MetricId::ZSpread => metric_values.insert(MetricId::ZSpread, z_spread),
                MetricId::Cs01 => metric_values.insert(MetricId::Cs01, cs01),
                _ => None,
            };
        }

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
            metrics: metric_values,
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
            .field("disc_id", &self.disc_id)
            .finish()
    }
}

