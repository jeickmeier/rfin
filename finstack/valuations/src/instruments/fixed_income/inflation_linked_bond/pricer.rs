use crate::instruments::common_impl::GenericInstrumentPricer;
use crate::instruments::fixed_income::inflation_linked_bond::InflationLinkedBond;

/// Simple type alias for the inflation linked bond pricer.
///
/// # Deprecated
///
/// Use `GenericInstrumentPricer::<InflationLinkedBond>::discounting(InstrumentType::InflationLinkedBond)` directly.
#[deprecated(
    since = "0.5.0",
    note = "Use `GenericInstrumentPricer::<InflationLinkedBond>::discounting(InstrumentType::InflationLinkedBond)` directly"
)]
pub type SimpleInflationLinkedBondDiscountingPricer = GenericInstrumentPricer<InflationLinkedBond>;

#[allow(deprecated)]
impl Default for SimpleInflationLinkedBondDiscountingPricer {
    fn default() -> Self {
        Self::discounting(crate::pricer::InstrumentType::InflationLinkedBond)
    }
}
