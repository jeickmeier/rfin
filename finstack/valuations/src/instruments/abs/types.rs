//! Asset-Backed Security (ABS) instrument built on the shared structured credit core.

use crate::cashflow::traits::{CashflowProvider, DatedFlows};
use crate::instruments::common::structured_credit::{
    AssetPool, AssetType, CoverageTests, DealType, StructuredCreditWaterfall, TrancheStructure,
};
use crate::instruments::common::traits::{Attributes, Instrument};
use crate::metrics::MetricId;
use crate::results::ValuationResult;
use finstack_core::dates::utils::add_months;
use finstack_core::dates::{Date, Frequency};
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::F;
use std::any::Any;
use time::Month;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Primary ABS instrument representation.
#[derive(Debug, Clone, finstack_macros::FinancialBuilder)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Abs {
    /// Unique instrument identifier
    pub id: InstrumentId,

    /// Deal classification (always `DealType::ABS`)
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

    /// Servicer/administrator information
    pub servicer_id: Option<String>,
    pub trustee_id: Option<String>,

    /// Discount curve for valuation
    pub disc_id: CurveId,

    /// Attributes for scenario selection
    pub attributes: Attributes,
}

impl Abs {
    /// Create a new ABS instrument from its building blocks.
    pub fn new(
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
            closing_date: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            first_payment_date: Date::from_calendar_date(2025, Month::February, 1).unwrap(),
            reinvestment_end_date: None,
            legal_maturity,
            payment_frequency: Frequency::monthly(),
            servicer_id: None,
            trustee_id: None,
            disc_id: CurveId::new(disc_id.into()),
            attributes: Attributes::new(),
        }
    }

    /// Calculate current loss percentage of the pool.
    pub fn current_loss_percentage(&self) -> F {
        let total_balance = self.pool.total_balance().amount();
        if total_balance == 0.0 {
            return 0.0;
        }

        (self.pool.cumulative_defaults.amount() - self.pool.cumulative_recoveries.amount())
            / total_balance
            * 100.0
    }

    /// Get cashflows for a specific tranche.
    pub fn tranche_cashflows(
        &self,
        _tranche_id: &str,
        context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // This would be implemented to extract tranche-specific flows
        // from the overall structure cashflows
        let _all_flows = self.build_schedule(context, as_of)?;

        // Placeholder: return empty flows for now
        Ok(Vec::new())
    }

    /// Calculate expected life of the structure.
    pub fn expected_life(&self, as_of: Date) -> finstack_core::Result<F> {
        // Simplified calculation based on pool WAL
        Ok(self.pool.weighted_avg_life(as_of))
    }
}

impl CashflowProvider for Abs {
    fn build_schedule(
        &self,
        _context: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<DatedFlows> {
        // 1. Get pool cashflows by aggregating individual asset cashflows
        let mut pool_flows = Vec::new();

        for asset in &self.pool.assets {
            // Get cashflows based on asset type
            match &asset.asset_type {
                AssetType::Loan { .. } => {
                    // Use existing loan cashflow generation if available
                    // For now, simplified approach
                    let monthly_interest = asset.balance.amount() * asset.rate / 12.0;
                    let interest_payment = Money::new(monthly_interest, asset.balance.currency());

                    // Generate monthly payments (simplified)
                    let mut payment_date = as_of;
                    for _ in 0..60 {
                        payment_date = add_months(payment_date, 1);
                        if payment_date <= asset.maturity {
                            pool_flows.push((payment_date, interest_payment));
                        }
                    }

                    // Principal at maturity (simplified)
                    if asset.maturity > as_of {
                        pool_flows.push((asset.maturity, asset.balance));
                    }
                }
                AssetType::Mortgage { .. } => {
                    // Mortgages typically pay monthly
                    let monthly_interest = asset.balance.amount() * asset.rate / 12.0;
                    let interest_payment = Money::new(monthly_interest, asset.balance.currency());
                    pool_flows.push((asset.maturity, interest_payment));
                    pool_flows.push((asset.maturity, asset.balance));
                }
                _ => {
                    // Generic asset - simplified cashflow
                    pool_flows.push((asset.maturity, asset.balance));
                }
            }
        }

        // 2. Sort pool flows by date
        pool_flows.sort_by_key(|(date, _)| *date);

        // 3. Apply pool behavior (prepayments/defaults) - simplified
        // In a full implementation, this would use the loan/receivable simulation framework

        // 4. Run through waterfall to get tranche-specific flows
        // For now, return aggregated pool flows

        Ok(pool_flows)
    }
}

impl Instrument for Abs {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn key(&self) -> crate::pricer::InstrumentType {
        <Self as crate::instruments::common::traits::InstrumentKind>::TYPE
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
        _metrics: &[MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        let base_value = self.value(context, as_of)?;
        Ok(ValuationResult::stamped(
            self.id.as_str(),
            as_of,
            base_value,
        ))
    }
}

impl crate::instruments::common::traits::InstrumentKind for Abs {
    const TYPE: crate::pricer::InstrumentType = crate::pricer::InstrumentType::ABS;
}

impl crate::instruments::common::HasDiscountCurve for Abs {
    fn discount_curve_id(&self) -> &CurveId {
        &self.disc_id
    }
}
