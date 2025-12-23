use finstack_core::dates::{adjust, BusinessDayConvention, Date, DateExt};
use finstack_core::Result;

use super::super::types::Bond;
use super::super::CashflowSpec;

pub(super) fn settlement_date(bond: &Bond, as_of: Date) -> Result<Date> {
    let Some(sd_u32) = bond.settlement_days else {
        return Ok(as_of);
    };

    let sd: i32 = sd_u32 as i32;
    let (calendar_id, bdc) = match &bond.cashflow_spec {
        CashflowSpec::Fixed(spec) => (spec.calendar_id.as_deref(), spec.bdc),
        CashflowSpec::Floating(spec) => (spec.rate_spec.calendar_id.as_deref(), spec.rate_spec.bdc),
        CashflowSpec::Amortizing { base, .. } => match &**base {
            CashflowSpec::Fixed(spec) => (spec.calendar_id.as_deref(), spec.bdc),
            CashflowSpec::Floating(spec) => {
                (spec.rate_spec.calendar_id.as_deref(), spec.rate_spec.bdc)
            }
            _ => (None, BusinessDayConvention::Following),
        },
    };

    if let Some(id) = calendar_id {
        if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
            let d = as_of.add_business_days(sd, cal)?;
            return adjust(d, bdc, cal);
        }
    }

    Ok(as_of.add_weekdays(sd))
}
