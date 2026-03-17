//! Internal model IM calculator.
//!
//! Stub implementation for bank internal models (VaR/ES-based).

use crate::calculators::traits::{ImCalculator, ImResult};
use crate::traits::Marginable;
use crate::types::ImMethodology;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_core::Result;
use std::sync::Arc;

/// Internal model input source for VaR/ES outputs.
pub trait InternalModelInputSource: Send + Sync {
    /// Return an internal model IM amount when available.
    fn initial_margin(
        &self,
        instrument: &dyn Marginable,
        context: &MarketContext,
        as_of: Date,
    ) -> Option<Money>;

    /// Optional MPOR override for internal model calculations.
    fn mpor_days(&self) -> Option<u32> {
        None
    }

    /// Optional label for the model used.
    fn model_name(&self) -> Option<String> {
        None
    }
}

/// Internal model IM calculator.
#[derive(Clone)]
pub struct InternalModelImCalculator {
    /// Margin period of risk (days)
    pub mpor_days: u32,
    /// Conservative fallback rate applied to notional proxy
    pub conservative_rate: f64,
    /// Optional external internal model input source
    pub input_source: Option<Arc<dyn InternalModelInputSource>>,
}

impl std::fmt::Debug for InternalModelImCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalModelImCalculator")
            .field("mpor_days", &self.mpor_days)
            .field("conservative_rate", &self.conservative_rate)
            .field(
                "input_source",
                &self
                    .input_source
                    .as_ref()
                    .map(|_| "<dyn InternalModelInputSource>"),
            )
            .finish()
    }
}

impl Default for InternalModelImCalculator {
    fn default() -> Self {
        Self {
            mpor_days: 10,
            conservative_rate: 0.05,
            input_source: None,
        }
    }
}

impl InternalModelImCalculator {
    /// Create a new internal model calculator with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach an internal model input source.
    #[must_use]
    pub fn with_input_source(mut self, source: Arc<dyn InternalModelInputSource>) -> Self {
        self.input_source = Some(source);
        self
    }

    /// Override the conservative fallback rate.
    #[must_use]
    pub fn with_conservative_rate(mut self, rate: f64) -> Self {
        self.conservative_rate = rate;
        self
    }

    /// Override the MPOR in days.
    #[must_use]
    pub fn with_mpor_days(mut self, days: u32) -> Self {
        self.mpor_days = days;
        self
    }

    /// Calculate IM using conservative estimate.
    pub fn calculate_conservative(&self, notional: Money) -> Money {
        Money::new(notional.amount().abs(), notional.currency()) * self.conservative_rate
    }
}

impl ImCalculator for InternalModelImCalculator {
    fn calculate(
        &self,
        instrument: &dyn Marginable,
        context: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Use PV as proxy for notional
        let mtm = instrument.mtm_for_vm(context, as_of)?;
        let currency = mtm.currency();
        let notional = Money::new(mtm.amount().abs(), currency);

        let mut im_amount = self.calculate_conservative(notional);
        let mut mpor_days = self.mpor_days;
        let mut label = "internal_model".to_string();

        if let Some(source) = &self.input_source {
            if let Some(amount) = source.initial_margin(instrument, context, as_of) {
                im_amount = amount;
            }
            if let Some(override_mpor) = source.mpor_days() {
                mpor_days = override_mpor;
            }
            if let Some(name) = source.model_name() {
                label = name;
            }
        }

        let mut breakdown = HashMap::default();
        breakdown.insert(label, im_amount);

        Ok(ImResult::with_breakdown(
            im_amount,
            ImMethodology::InternalModel,
            as_of,
            mpor_days,
            breakdown,
        ))
    }

    fn methodology(&self) -> ImMethodology {
        ImMethodology::InternalModel
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestInstrument {
        id: String,
        value: Money,
    }

    impl TestInstrument {
        fn new(value: Money) -> Self {
            Self {
                id: "TEST-INST".to_string(),
                value,
            }
        }
    }

    impl Marginable for TestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn margin_spec(&self) -> Option<&crate::OtcMarginSpec> {
            None
        }

        fn netting_set_id(&self) -> Option<crate::NettingSetId> {
            None
        }

        fn simm_sensitivities(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> Result<crate::SimmSensitivities> {
            Ok(crate::SimmSensitivities::new(self.value.currency()))
        }

        fn mtm_for_vm(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }
    }

    #[derive(Debug)]
    struct TestInputSource {
        amount: Money,
    }

    impl InternalModelInputSource for TestInputSource {
        fn initial_margin(
            &self,
            _instrument: &dyn Marginable,
            _context: &MarketContext,
            _as_of: Date,
        ) -> Option<Money> {
            Some(self.amount)
        }

        fn model_name(&self) -> Option<String> {
            Some("internal_var".to_string())
        }
    }

    #[test]
    fn conservative_internal_model_calc() {
        let calc = InternalModelImCalculator::default();
        let notional = Money::new(100_000_000.0, Currency::USD);
        let im = calc.calculate_conservative(notional);
        assert_eq!(im.amount(), 5_000_000.0);
    }

    #[test]
    fn input_source_overrides_amount() {
        let calc =
            InternalModelImCalculator::default().with_input_source(Arc::new(TestInputSource {
                amount: Money::new(2_500_000.0, Currency::USD),
            }));
        let instrument = TestInstrument::new(Money::new(10_000_000.0, Currency::USD));
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");
        let result = calc.calculate(&instrument, &market, as_of).expect("im");

        assert_eq!(result.amount.amount(), 2_500_000.0);
        assert!(result.breakdown.contains_key("internal_var"));
    }
}
