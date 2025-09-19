use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

use crate::cashflow::traits::CashflowProvider;
use crate::instruments::discountable::Discountable;

use super::super::types::Bond;

pub fn price(bond: &Bond, context: &MarketContext, as_of: Date) -> Result<Money> {
    let flows = bond.build_schedule(context, as_of)?;
    let disc = context
        .get::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            bond.disc_id.as_str(),
        )?;
    flows.npv(&*disc, as_of, bond.dc)
}


