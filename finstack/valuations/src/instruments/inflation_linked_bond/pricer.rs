use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;

/// Simple type alias for the inflation linked bond pricer
pub type SimpleInflationLinkedBondDiscountingPricer = GenericDiscountingPricer<InflationLinkedBond>;
