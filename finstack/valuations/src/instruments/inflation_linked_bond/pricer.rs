use crate::instruments::inflation_linked_bond::InflationLinkedBond;
use crate::instruments::common::GenericDiscountingPricer;

/// Simple type alias for the inflation linked bond pricer
pub type SimpleInflationLinkedBondDiscountingPricer = GenericDiscountingPricer<InflationLinkedBond>;
