// Using generic pricer implementation to eliminate boilerplate
pub use crate::instruments::common::GenericDiscountingPricer;

/// IRS discounting pricer using the generic implementation.
pub type SimpleIrsDiscountingPricer =
    GenericDiscountingPricer<crate::instruments::InterestRateSwap>;

impl Default for SimpleIrsDiscountingPricer {
    fn default() -> Self {
        Self::new(crate::pricer::InstrumentType::IRS)
    }
}

// Auto-register IRS discounting pricer
inventory::submit! {
    crate::pricer::PricerRegistration {
        ctor: || Box::new(SimpleIrsDiscountingPricer::default()),
    }
}
