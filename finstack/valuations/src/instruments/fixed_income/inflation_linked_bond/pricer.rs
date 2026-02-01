use crate::instruments::common_impl::GenericInstrumentPricer;
use crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond;

/// Simple type alias for the inflation linked bond pricer
pub type SimpleInflationLinkedBondDiscountingPricer = GenericInstrumentPricer<InflationLinkedBond>;

impl Default for SimpleInflationLinkedBondDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InflationLinkedBond)
    }
}
