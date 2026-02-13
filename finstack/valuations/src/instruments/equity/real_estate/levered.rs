//! Levered real estate equity instrument.
//!
//! This instrument composes an unlevered [`RealEstateAsset`] with a financing stack
//! (e.g., term loans, bonds, convertibles) to provide:
//! - Equity value as `Asset PV - Financing PV`
//! - Levered deal-style metrics (IRR, MOIC, DSCR, LTV)

use super::types::RealEstateAsset;
use crate::cashflow::traits::CashflowProvider;
use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::{
    Attributes, CurveDependencies, Instrument, InstrumentCurves,
};
use crate::instruments::{InstrumentJson, MarketDependencies};
use crate::pricer::InstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Error as CoreError;
use std::collections::BTreeMap;

/// Levered real estate equity = unlevered asset + financing.
///
/// Value convention:
/// - `PV_equity = PV_asset - PV_financing` (financing valued from lender perspective).
///
/// Return/coverage metrics are computed from a simplified equity cashflow schedule:
/// - Initial outflow: `-(purchase_price + acquisition_cost)` at `as_of`
/// - Financing funding legs on/after `as_of` are included as equity inflows
/// - Interim equity CFs: `(NOI - CapEx) - debt_service_cash`
/// - Exit: `(sale_proceeds - financing_payoff)` at `exit_date`
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct LeveredRealEstateEquity {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Currency (must match asset currency; financing PV is validated at valuation time).
    pub currency: Currency,
    /// Underlying unlevered asset.
    pub asset: RealEstateAsset,
    /// Financing instruments (borrower liabilities), valued from lender perspective.
    ///
    /// PV convention nets these from the asset PV.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub financing: Vec<InstrumentJson>,
    /// Optional explicit exit/sale date. Defaults to the last NOI date on/after `as_of`.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_date: Option<Date>,
    /// Discount curve identifier for equity PV attribution (typically same as asset curve).
    ///
    /// This is used only for curve dependency reporting; PV is computed as `asset - financing`.
    pub discount_curve_id: CurveId,
    /// Attributes for tagging and scenarios.
    #[builder(default)]
    pub attributes: Attributes,
}

impl LeveredRealEstateEquity {
    fn validate_currency(&self) -> finstack_core::Result<()> {
        if self.asset.currency != self.currency {
            return Err(CoreError::Validation(
                "asset currency must match levered equity currency".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn resolve_exit_date(&self, as_of: Date) -> finstack_core::Result<Date> {
        if let Some(d) = self.exit_date {
            return Ok(d);
        }
        // Default: last NOI flow date on/after as_of.
        let flows = self.asset.unlevered_flows(as_of)?;
        flows
            .last()
            .map(|(d, _)| *d)
            .ok_or_else(|| CoreError::Validation("Missing cashflows for exit date".into()))
    }

    fn asset_sale_proceeds_at(&self, as_of: Date, exit: Date) -> finstack_core::Result<f64> {
        let Some((_d, proceeds)) = self.asset.sale_proceeds_at(as_of, exit)? else {
            return Err(CoreError::Validation(
                "sale_price or terminal_cap_rate is required to compute sale proceeds".into(),
            ));
        };
        Ok(proceeds)
    }

    pub(crate) fn financing_schedules_supported(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<crate::cashflow::builder::CashFlowSchedule>> {
        let mut schedules = Vec::with_capacity(self.financing.len());
        for inst in &self.financing {
            let sched = match inst {
                InstrumentJson::TermLoan(i) => i.build_full_schedule(market, as_of)?,
                InstrumentJson::Bond(i) => i.build_full_schedule(market, as_of)?,
                InstrumentJson::RevolvingCredit(i) => i.build_full_schedule(market, as_of)?,
                InstrumentJson::Repo(i) => i.build_full_schedule(market, as_of)?,
                _ => {
                    return Err(CoreError::Validation(
                        "Unsupported financing instrument for cashflow-based metrics (supported: term_loan, bond, revolving_credit, repo)".into(),
                    ));
                }
            };
            schedules.push(sched);
        }
        Ok(schedules)
    }

    fn outstanding_before(out_path: &[(Date, Money)], target: Date, currency: Currency) -> Money {
        let mut last = Money::new(0.0, currency);
        for (d, amt) in out_path {
            if *d < target {
                last = *amt;
            } else {
                break;
            }
        }
        last
    }

    /// Build a dated equity cashflow schedule for levered return metrics.
    pub fn equity_cashflows(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Vec<(Date, f64)>> {
        self.validate_currency()?;
        let exit = self.resolve_exit_date(as_of)?;

        let purchase = self
            .asset
            .purchase_price
            .ok_or_else(|| CoreError::Validation("purchase_price is required".into()))?;
        if purchase.currency() != self.currency {
            return Err(CoreError::Validation(
                "purchase_price currency must match instrument currency".into(),
            ));
        }

        let mut flows: BTreeMap<Date, f64> = BTreeMap::new();

        // Equity purchase (outflow) at as_of.
        let acq_cost = self.asset.acquisition_cost_total()?;
        *flows.entry(as_of).or_insert(0.0) += -(purchase.amount() + acq_cost);

        // Asset interim unlevered flows (NOI - CapEx).
        for (d, a) in self.asset.unlevered_flows(as_of)? {
            if d <= exit {
                *flows.entry(d).or_insert(0.0) += a;
            }
        }

        let financing_schedules = self.financing_schedules_supported(market, as_of)?;

        // Financing borrower flows derived from lender schedules.
        for sched in &financing_schedules {
            for cf in &sched.flows {
                if cf.date < as_of || cf.date > exit {
                    continue;
                }
                // Borrower perspective is negative of lender flows.
                // Exclude PIK flows (non-cash).
                if matches!(cf.kind, finstack_core::cashflow::CFKind::PIK) {
                    continue;
                }
                // If we repay at exit via explicit payoff, exclude principal legs on the exit date
                // to avoid double-counting.
                let is_principal = matches!(
                    cf.kind,
                    finstack_core::cashflow::CFKind::Notional
                        | finstack_core::cashflow::CFKind::Amortization
                );
                if cf.date == exit && is_principal {
                    continue;
                }
                *flows.entry(cf.date).or_insert(0.0) += -cf.amount.amount();
            }
        }

        // Terminal sale proceeds and financing payoff.
        let sale = self.asset_sale_proceeds_at(as_of, exit)?;
        *flows.entry(exit).or_insert(0.0) += sale;

        let mut payoff_amt = 0.0;
        for sched in &financing_schedules {
            let out_path = sched.outstanding_by_date()?;
            let payoff = Self::outstanding_before(&out_path, exit, self.currency);
            payoff_amt += payoff.amount().abs();
        }
        *flows.entry(exit).or_insert(0.0) += -payoff_amt;

        Ok(flows.into_iter().collect())
    }

    /// Convenience: compute financing payoff amount at exit (absolute amount).
    pub fn financing_payoff_at_exit(
        &self,
        market: &MarketContext,
        as_of: Date,
    ) -> finstack_core::Result<Money> {
        self.validate_currency()?;
        let exit = self.resolve_exit_date(as_of)?;
        let mut payoff_amt = 0.0;
        for sched in self.financing_schedules_supported(market, as_of)? {
            let out_path = sched.outstanding_by_date()?;
            let payoff = Self::outstanding_before(&out_path, exit, self.currency);
            payoff_amt += payoff.amount().abs();
        }
        Ok(Money::new(payoff_amt, self.currency))
    }

    pub(crate) fn irr_day_count(&self) -> DayCount {
        self.asset.day_count
    }
}

impl Instrument for LeveredRealEstateEquity {
    impl_instrument_base!(InstrumentType::LeveredRealEstateEquity);

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        self.validate_currency()?;
        let asset_pv = self.asset.value(market, as_of)?;
        if asset_pv.currency() != self.currency {
            return Err(CoreError::Validation("asset PV currency mismatch".into()));
        }
        let mut financing_pv = 0.0;
        for inst in &self.financing {
            let boxed = inst.clone().into_boxed()?;
            let pv = boxed.value(market, as_of)?;
            if pv.currency() != self.currency {
                return Err(CoreError::Validation(
                    "financing PV currency mismatch".into(),
                ));
            }
            financing_pv += pv.amount();
        }
        Ok(Money::new(asset_pv.amount() - financing_pv, self.currency))
    }

    fn effective_start_date(&self) -> Option<Date> {
        None
    }
}

impl CurveDependencies for LeveredRealEstateEquity {
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
        let mut deps = MarketDependencies::from_curve_dependencies(&self.asset)?;
        for inst in &self.financing {
            deps.merge(MarketDependencies::from_instrument_json(inst)?);
        }
        let mut curves = deps.curves;

        // Also include a top-level discount curve id for attribution if provided.
        if !curves
            .discount_curves
            .iter()
            .any(|c| c == &self.discount_curve_id)
        {
            curves.discount_curves.push(self.discount_curve_id.clone());
        }

        Ok(curves)
    }
}
