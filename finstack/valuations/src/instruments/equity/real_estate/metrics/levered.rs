//! Metrics for [`LeveredRealEstateEquity`].

use crate::instruments::common_impl::traits::Instrument;
use crate::instruments::equity::real_estate::LeveredRealEstateEquity;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::cashflow::InternalRateOfReturn;
use finstack_core::Error as CoreError;

/// Levered equity IRR (XIRR-style) from the levered equity cashflow schedule.
#[derive(Debug, Default)]
pub struct LeveredIrr;

impl MetricCalculator for LeveredIrr {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let inst = context
            .instrument
            .as_any()
            .downcast_ref::<LeveredRealEstateEquity>()
            .ok_or_else(|| CoreError::Validation("LeveredIrr: instrument type mismatch".into()))?;

        let flows = inst.equity_cashflows(&context.curves, context.as_of)?;
        flows
            .as_slice()
            .irr_with_daycount(inst.irr_day_count(), None)
    }
}

/// Equity multiple (MOIC-like): total inflows / total outflows (absolute).
#[derive(Debug, Default)]
pub struct EquityMultiple;

impl MetricCalculator for EquityMultiple {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let inst = context
            .instrument
            .as_any()
            .downcast_ref::<LeveredRealEstateEquity>()
            .ok_or_else(|| {
                CoreError::Validation("EquityMultiple: instrument type mismatch".into())
            })?;

        let flows = inst.equity_cashflows(&context.curves, context.as_of)?;
        let mut inflows = 0.0;
        let mut outflows = 0.0;
        for (_d, a) in flows {
            if a >= 0.0 {
                inflows += a;
            } else {
                outflows += -a;
            }
        }
        if outflows <= 0.0 {
            return Err(CoreError::Validation(
                "EquityMultiple: total outflows must be positive".into(),
            ));
        }
        Ok(inflows / outflows)
    }
}

/// Loan-to-value at `as_of`: financing PV / asset PV.
///
/// Uses present values (not face/outstanding) to keep the metric consistent with
/// the library's valuation approach.
#[derive(Debug, Default)]
pub struct LoanToValue;

impl MetricCalculator for LoanToValue {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let inst = context
            .instrument
            .as_any()
            .downcast_ref::<LeveredRealEstateEquity>()
            .ok_or_else(|| CoreError::Validation("LoanToValue: type mismatch".into()))?;

        let asset_pv = inst.asset.value(&context.curves, context.as_of)?;
        let denom = asset_pv.amount();
        if denom <= 0.0 {
            return Err(CoreError::Validation(
                "LoanToValue: asset PV must be positive".into(),
            ));
        }
        let mut financing_pv = 0.0;
        for inst_json in &inst.financing {
            let boxed = inst_json.clone().into_boxed()?;
            let pv = boxed.value(&context.curves, context.as_of)?;
            financing_pv += pv.amount();
        }
        Ok(financing_pv.abs() / denom)
    }
}

/// Minimum DSCR over the NOI dates in the asset schedule: NOI / (cash interest + fees + principal).
///
/// This is a simplified DSCR proxy computed on NOI dates. It is intended for screening and
/// covenant-like reporting, not legal covenant calculation.
#[derive(Debug, Default)]
pub struct DscrMin;

impl MetricCalculator for DscrMin {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let inst = context
            .instrument
            .as_any()
            .downcast_ref::<LeveredRealEstateEquity>()
            .ok_or_else(|| CoreError::Validation("DscrMin: type mismatch".into()))?;

        let as_of = context.as_of;
        let exit = inst.clone().resolve_exit_date(as_of)?;

        let noi = inst.asset.noi_flows(as_of)?;
        if noi.is_empty() {
            return Err(CoreError::Validation("DscrMin: missing NOI flows".into()));
        }

        let schedules = inst.financing_schedules_supported(&context.curves, as_of)?;

        let mut min_dscr = f64::INFINITY;
        let mut prev = as_of;
        for (d, noi_amt) in noi {
            if d > exit {
                break;
            }
            let mut debt_service = 0.0;
            for sched in &schedules {
                debt_service += sched
                    .flows
                    .iter()
                    .filter(|cf| cf.date > prev && cf.date <= d)
                    .filter(|cf| {
                        // Exclude borrower funding legs (negative Notional from lender perspective).
                        if matches!(cf.kind, finstack_core::cashflow::CFKind::Notional)
                            && cf.amount.amount() < 0.0
                        {
                            return false;
                        }
                        // Exclude PIK.
                        !matches!(cf.kind, finstack_core::cashflow::CFKind::PIK)
                    })
                    // Borrower debt service is negative of lender inflows. We want a positive service amount.
                    .map(|cf| cf.amount.amount().abs())
                    .sum::<f64>();
            }

            if debt_service > 0.0 {
                min_dscr = min_dscr.min(noi_amt / debt_service);
            }
            prev = d;
        }

        if !min_dscr.is_finite() {
            return Err(CoreError::Validation(
                "DscrMin: could not compute (no debt service)".into(),
            ));
        }
        Ok(min_dscr)
    }
}

/// Debt payoff at exit (absolute amount).
#[derive(Debug, Default)]
pub struct DebtPayoffAtExit;

impl MetricCalculator for DebtPayoffAtExit {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let inst = context
            .instrument
            .as_any()
            .downcast_ref::<LeveredRealEstateEquity>()
            .ok_or_else(|| CoreError::Validation("DebtPayoffAtExit: type mismatch".into()))?;

        let payoff = inst.financing_payoff_at_exit(&context.curves, context.as_of)?;
        Ok(payoff.amount())
    }
}
