use crate::instruments::common::GenericDiscountingPricer;
use crate::instruments::inflation_linked_bond::InflationLinkedBond;

/// Simple type alias for the inflation linked bond pricer
pub type SimpleInflationLinkedBondDiscountingPricer = GenericDiscountingPricer<InflationLinkedBond>;

// Auto-register InflationLinkedBond discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleInflationLinkedBondDiscountingPricer::new()),
    }
}
