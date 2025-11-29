use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};

/// Generic discounting pricer for Equity Total Return Swaps.
pub type SimpleEquityTrsDiscountingPricer = GenericDiscountingPricer<EquityTotalReturnSwap>;

/// Generic discounting pricer for Fixed Income Index Total Return Swaps.
pub type SimpleFIIndexTrsDiscountingPricer = GenericDiscountingPricer<FIIndexTotalReturnSwap>;
