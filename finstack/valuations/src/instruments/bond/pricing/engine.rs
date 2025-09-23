use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
// Discountable trait not required after switching to curve day-count path

use super::super::types::Bond;

/// Bond pricing engine providing core valuation methods.
pub struct BondEngine;

impl BondEngine {
    /// Price a bond using discount curve present value calculation.
    pub fn price(bond: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
        let flows = bond.build_schedule(context, as_of)?;
        let disc = context
            .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
                bond.disc_id.as_str(),
            )?;
        // Discount using the curve's own day-count convention for time mapping.
        // Transform (date, amount) -> (df_on_date_curve(date) * amount) and sum.
        if flows.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }
        let ccy = flows[0].1.currency();
        let mut total = Money::new(0.0, ccy);
        // Settlement PV: start discounting from settlement date if provided
        let settle_date = if let Some(sd) = bond.settlement_days {
            let mut s = as_of + time::Duration::days(sd as i64);
            if let Some(id) = bond.calendar_id {
                if let Some(cal) = finstack_core::dates::calendar::calendar_by_id(id) {
                    s = finstack_core::dates::adjust(s, bond.bdc, cal)?;
                }
            }
            s
        } else {
            as_of
        };
        for (d, amt) in &flows {
            if *d <= settle_date {
                continue;
            }
            let df = finstack_core::market_data::term_structures::discount_curve::DiscountCurve::df_on_date_curve(&disc, *d);
            total = (total + (*amt * df))?;
        }
        Ok(total)
    }
}
