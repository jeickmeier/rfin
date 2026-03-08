//! Instrument-level margin metrics.
//!
//! Provides metrics calculators for initial margin (IM) and variation margin (VM)
//! at the individual instrument level. These metrics integrate with the standard
//! metrics framework and can be computed alongside other instrument metrics.
//!
//! # Available Metrics
//!
//! - [`InitialMarginMetric`]: Calculates IM based on instrument's margin spec
//! - [`VariationMarginMetric`]: Calculates VM based on MTM exposure
//! - [`TotalMarginMetric`]: Combined IM + VM requirement
//!
//! # Example
//!
//! ```rust,no_run
//! use finstack_valuations::margin::metrics::{InitialMarginMetric, VariationMarginMetric};
//! use finstack_valuations::instruments::rates::irs::InterestRateSwap;
//! use finstack_core::market_data::context::MarketContext;
//! use time::macros::date;
//!
//! # fn main() -> finstack_core::Result<()> {
//! let swap = InterestRateSwap::example()?;
//! let market = MarketContext::new();
//! let as_of = date!(2025-01-01);
//!
//! // Calculate initial margin
//! let im_metric = InitialMarginMetric::new();
//! let im = im_metric.calculate(&swap, &market, as_of)?;
//! # let _ = im;
//!
//! // Calculate variation margin
//! let vm_metric = VariationMarginMetric::new();
//! let vm = vm_metric.calculate(&swap, &market, as_of)?;
//! # let _ = vm;
//! # Ok(())
//! # }
//! ```

use crate::margin::calculators::{
    ClearingHouseImCalculator, HaircutImCalculator, ImCalculator, ImResult,
    InternalModelImCalculator, ScheduleImCalculator, SimmCalculator, VmCalculator, VmResult,
};
use crate::margin::traits::{InstrumentMarginResult, Marginable, SimmSensitivities};
use crate::margin::types::{ClearingStatus, ImMethodology, OtcMarginSpec};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;
use tracing::{debug, warn};

/// Initial margin metric calculator.
///
/// Calculates the initial margin requirement for a marginable instrument
/// based on its margin specification. Dispatches to the appropriate
/// IM calculator (SIMM, Schedule, Haircut, or CCP).
#[derive(Debug, Clone, Default)]
pub struct InitialMarginMetric {
    /// Override SIMM calculator (uses default if None)
    simm_calculator: Option<SimmCalculator>,
    /// Override schedule calculator (uses BCBS standard if None)
    schedule_calculator: Option<ScheduleImCalculator>,
    /// Override CCP calculator (uses CCP lookup if None)
    clearing_calculator: Option<ClearingHouseImCalculator>,
    /// Override internal model calculator (uses default if None)
    internal_model_calculator: Option<InternalModelImCalculator>,
}

impl InitialMarginMetric {
    /// Create a new IM metric calculator with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom SIMM calculator.
    #[must_use]
    pub fn with_simm(mut self, simm: SimmCalculator) -> Self {
        self.simm_calculator = Some(simm);
        self
    }

    /// Create with custom schedule calculator.
    #[must_use]
    pub fn with_schedule(mut self, schedule: ScheduleImCalculator) -> Self {
        self.schedule_calculator = Some(schedule);
        self
    }

    /// Create with custom clearing house calculator.
    #[must_use]
    pub fn with_clearing_house(mut self, clearing: ClearingHouseImCalculator) -> Self {
        self.clearing_calculator = Some(clearing);
        self
    }

    /// Create with custom internal model calculator.
    #[must_use]
    pub fn with_internal_model(mut self, internal: InternalModelImCalculator) -> Self {
        self.internal_model_calculator = Some(internal);
        self
    }

    /// Calculate initial margin for an instrument.
    pub fn calculate<I: Marginable>(
        &self,
        instrument: &I,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Check if instrument has margin spec
        let margin_spec = instrument.margin_spec();

        if let Some(spec) = margin_spec {
            debug!(instrument = instrument.id(), methodology = %spec.im_methodology, "IM dispatch: OTC margin spec");
            self.calculate_otc_im(instrument, spec, market, as_of)
        } else if let Some(repo_spec) = instrument.repo_margin_spec() {
            debug!(instrument = instrument.id(), "IM dispatch: repo haircut");
            let haircut_calc = HaircutImCalculator::new(
                repo_spec.eligible_substitutes.clone().unwrap_or_default(),
            );
            haircut_calc.calculate(instrument, market, as_of)
        } else {
            warn!(
                instrument = instrument.id(),
                "No margin spec; returning zero IM"
            );
            let currency = instrument
                .mtm_for_vm(market, as_of)
                .map(|m| m.currency())
                .unwrap_or(Currency::USD);
            Ok(ImResult::simple(
                Money::new(0.0, currency),
                ImMethodology::Schedule,
                as_of,
                0,
            ))
        }
    }

    /// Calculate IM for OTC derivatives based on margin spec.
    fn calculate_otc_im<I: Marginable>(
        &self,
        instrument: &I,
        spec: &OtcMarginSpec,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        let mut im_result = match spec.im_methodology {
            ImMethodology::Simm => {
                let simm = self.simm_calculator.clone().unwrap_or_default();

                // Get sensitivities from instrument
                let sensitivities = instrument.simm_sensitivities(market, as_of)?;

                // Calculate SIMM
                self.calculate_simm_from_sensitivities(&simm, &sensitivities, spec, as_of)
            }
            ImMethodology::Schedule => {
                let schedule = match self.schedule_calculator.clone() {
                    Some(calc) => calc,
                    None => ScheduleImCalculator::bcbs_standard()?,
                };

                schedule.calculate(instrument, market, as_of)
            }
            ImMethodology::Haircut => {
                let haircut_calc = HaircutImCalculator::new(spec.csa.eligible_collateral.clone());
                haircut_calc.calculate(instrument, market, as_of)
            }
            ImMethodology::ClearingHouse => {
                let calc = self.clearing_calculator.clone().unwrap_or_else(|| {
                    spec.ccp()
                        .map(|ccp| ClearingHouseImCalculator::for_ccp(ccp, instrument.key()))
                        .unwrap_or_else(|| ClearingHouseImCalculator::generic_var(0.99, 250))
                });
                calc.calculate(instrument, market, as_of)
            }
            ImMethodology::InternalModel => {
                let calc = self.internal_model_calculator.clone().unwrap_or_default();
                calc.calculate(instrument, market, as_of)
            }
        }?;

        Self::apply_im_call_terms(&mut im_result, spec);
        Ok(im_result)
    }

    fn apply_im_call_terms(im_result: &mut ImResult, spec: &OtcMarginSpec) {
        if let Some(im_params) = &spec.csa.im_params {
            let net_required = (im_result.amount.amount() - im_params.threshold.amount()).max(0.0);
            if net_required <= im_params.mta.amount() {
                im_result.amount = Money::new(0.0, im_result.amount.currency());
                im_result.breakdown.clear();
            } else {
                im_result.amount = Money::new(net_required, im_result.amount.currency());
            }
        }
    }

    /// Calculate SIMM from pre-computed sensitivities.
    fn calculate_simm_from_sensitivities(
        &self,
        simm: &SimmCalculator,
        sensitivities: &SimmSensitivities,
        spec: &OtcMarginSpec,
        as_of: Date,
    ) -> Result<ImResult> {
        let currency = spec.csa.base_currency;
        let (total_im, breakdown) = simm.calculate_from_sensitivities(sensitivities, currency);

        // Get MPOR from spec
        let mpor_days = spec
            .csa
            .im_params
            .as_ref()
            .map(|p| p.mpor_days)
            .unwrap_or(10);

        Ok(ImResult::with_breakdown(
            Money::new(total_im, currency),
            ImMethodology::Simm,
            as_of,
            mpor_days,
            breakdown,
        ))
    }
}

/// Variation margin metric calculator.
///
/// Calculates the variation margin requirement for a marginable instrument
/// based on its mark-to-market exposure and CSA parameters.
#[derive(Debug, Clone, Default)]
pub struct VariationMarginMetric {
    /// Currently posted collateral (for calculating net requirement)
    posted_collateral: Option<Money>,
}

impl VariationMarginMetric {
    /// Create a new VM metric calculator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the currently posted collateral for net calculation.
    #[must_use]
    pub fn with_posted(mut self, posted: Money) -> Self {
        self.posted_collateral = Some(posted);
        self
    }

    /// Calculate variation margin for an instrument.
    pub fn calculate<I: Marginable>(
        &self,
        instrument: &I,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<VmResult> {
        // Get MTM exposure
        let mtm = instrument.mtm_for_vm(market, as_of)?;
        let currency = mtm.currency();

        // Check if instrument has margin spec
        if let Some(spec) = instrument.margin_spec() {
            // Use VM calculator with CSA parameters
            let vm_calc = VmCalculator::new(spec.csa.clone());
            let posted = self
                .posted_collateral
                .unwrap_or_else(|| Money::new(0.0, currency));
            vm_calc.calculate(mtm, posted, as_of)
        } else {
            // No margin spec - return MTM as the VM requirement
            Ok(VmResult {
                date: as_of,
                gross_exposure: mtm,
                net_exposure: mtm,
                delivery_amount: if mtm.amount() > 0.0 {
                    mtm
                } else {
                    Money::new(0.0, currency)
                },
                return_amount: if mtm.amount() < 0.0 {
                    Money::new(mtm.amount().abs(), currency)
                } else {
                    Money::new(0.0, currency)
                },
                settlement_date: as_of,
            })
        }
    }
}

/// Total margin metric calculator.
///
/// Calculates the combined IM + VM requirement for an instrument.
#[derive(Debug, Clone, Default)]
pub struct TotalMarginMetric {
    im_metric: InitialMarginMetric,
    vm_metric: VariationMarginMetric,
}

impl TotalMarginMetric {
    /// Create a new total margin metric calculator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set posted collateral for VM calculation.
    #[must_use]
    pub fn with_posted(mut self, posted: Money) -> Self {
        self.vm_metric = self.vm_metric.with_posted(posted);
        self
    }

    /// Calculate total margin requirement.
    pub fn calculate<I: Marginable>(
        &self,
        instrument: &I,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<InstrumentMarginResult> {
        let im_result = self.im_metric.calculate(instrument, market, as_of)?;
        let vm_result = self.vm_metric.calculate(instrument, market, as_of)?;

        let currency = im_result.amount.currency();

        // Total = IM + positive VM (delivery amount)
        let total = Money::new(
            im_result.amount.amount() + vm_result.delivery_amount.amount(),
            currency,
        );

        // Get netting set
        let netting_set = instrument.netting_set_id();

        // Get clearing status
        let is_cleared = instrument
            .margin_spec()
            .map(|s| matches!(s.clearing_status, ClearingStatus::Cleared { .. }))
            .unwrap_or(false);

        // Get sensitivities if SIMM was used
        let sensitivities = if im_result.methodology == ImMethodology::Simm {
            instrument.simm_sensitivities(market, as_of).ok()
        } else {
            None
        };

        Ok(InstrumentMarginResult {
            instrument_id: instrument.id().to_string(),
            as_of,
            initial_margin: im_result.amount,
            variation_margin: vm_result.delivery_amount,
            total_margin: total,
            im_methodology: im_result.methodology,
            is_cleared,
            netting_set,
            sensitivities,
        })
    }
}

/// Batch calculate margin for multiple instruments.
///
/// Efficiently calculates margin requirements for a collection of instruments.
pub fn calculate_instrument_margins<'a, I: Marginable + 'a>(
    instruments: impl Iterator<Item = &'a I>,
    market: &MarketContext,
    as_of: Date,
) -> Vec<Result<InstrumentMarginResult>> {
    let metric = TotalMarginMetric::new();
    instruments
        .map(|inst| metric.calculate(inst, market, as_of))
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_margin_metric_default() {
        let metric = InitialMarginMetric::new();
        assert!(metric.simm_calculator.is_none());
        assert!(metric.schedule_calculator.is_none());
        assert!(metric.clearing_calculator.is_none());
        assert!(metric.internal_model_calculator.is_none());
    }

    #[test]
    fn test_variation_margin_metric_default() {
        let metric = VariationMarginMetric::new();
        assert!(metric.posted_collateral.is_none());
    }

    #[test]
    fn test_total_margin_metric_default() {
        let metric = TotalMarginMetric::new();
        // Just verify construction
        assert!(metric.vm_metric.posted_collateral.is_none());
    }

    #[test]
    fn test_with_posted_collateral() {
        let posted = Money::new(1_000_000.0, Currency::USD);
        let metric = VariationMarginMetric::new().with_posted(posted);
        assert_eq!(metric.posted_collateral, Some(posted));
    }

    #[derive(Clone)]
    struct TestInstrument {
        id: String,
        value: Money,
        margin_spec: Option<OtcMarginSpec>,
        sensitivities: Option<SimmSensitivities>,
        attributes: crate::instruments::common_impl::traits::Attributes,
    }

    impl TestInstrument {
        fn new(value: Money, margin_spec: Option<OtcMarginSpec>) -> Self {
            Self {
                id: "TEST-INST".to_string(),
                value,
                margin_spec,
                sensitivities: None,
                attributes: crate::instruments::common_impl::traits::Attributes::default(),
            }
        }

        fn with_sensitivities(mut self, sensitivities: SimmSensitivities) -> Self {
            self.sensitivities = Some(sensitivities);
            self
        }
    }

    impl crate::instruments::common_impl::traits::Instrument for TestInstrument {
        fn id(&self) -> &str {
            &self.id
        }

        fn key(&self) -> crate::pricer::InstrumentType {
            crate::pricer::InstrumentType::IRS
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn attributes(&self) -> &crate::instruments::common_impl::traits::Attributes {
            &self.attributes
        }

        fn attributes_mut(&mut self) -> &mut crate::instruments::common_impl::traits::Attributes {
            &mut self.attributes
        }

        fn clone_box(&self) -> Box<dyn crate::instruments::common_impl::traits::Instrument> {
            Box::new(self.clone())
        }

        fn value(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }

        fn price_with_metrics(
            &self,
            _market: &MarketContext,
            as_of: Date,
            _metrics: &[crate::metrics::MetricId],
        ) -> Result<crate::results::ValuationResult> {
            Ok(crate::results::ValuationResult::stamped(
                &self.id, as_of, self.value,
            ))
        }
    }

    impl Marginable for TestInstrument {
        fn margin_spec(&self) -> Option<&OtcMarginSpec> {
            self.margin_spec.as_ref()
        }

        fn netting_set_id(&self) -> Option<crate::margin::traits::NettingSetId> {
            None
        }

        fn simm_sensitivities(
            &self,
            _market: &MarketContext,
            _as_of: Date,
        ) -> Result<SimmSensitivities> {
            Ok(self
                .sensitivities
                .clone()
                .unwrap_or_else(|| SimmSensitivities::new(self.value.currency())))
        }

        fn mtm_for_vm(&self, _market: &MarketContext, _as_of: Date) -> Result<Money> {
            Ok(self.value)
        }
    }

    #[test]
    fn uses_clearing_house_calculator_for_cleared_spec() {
        let spec = OtcMarginSpec::cleared("LCH", Currency::USD).expect("registry should load");
        let instrument = TestInstrument::new(Money::new(100_000_000.0, Currency::USD), Some(spec));
        let metric = InitialMarginMetric::new();
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");

        let im = metric.calculate(&instrument, &market, as_of).expect("im");
        assert_eq!(im.methodology, ImMethodology::ClearingHouse);
        assert_eq!(im.amount.amount(), 2_000_000.0);
    }

    #[test]
    fn uses_internal_model_for_internal_model_spec() {
        let mut spec = OtcMarginSpec::usd_bilateral().expect("registry should load");
        spec.im_methodology = ImMethodology::InternalModel;
        if let Some(im_params) = spec.csa.im_params.as_mut() {
            im_params.threshold = Money::new(0.0, Currency::USD);
            im_params.mta = Money::new(0.0, Currency::USD);
        }
        let instrument = TestInstrument::new(Money::new(20_000_000.0, Currency::USD), Some(spec));
        let metric = InitialMarginMetric::new();
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");

        let im = metric.calculate(&instrument, &market, as_of).expect("im");
        assert_eq!(im.methodology, ImMethodology::InternalModel);
        assert_eq!(im.amount.amount(), 1_000_000.0);
    }

    #[test]
    fn simm_im_subtracts_threshold_after_breach() {
        let mut spec = OtcMarginSpec::usd_bilateral().expect("registry should load");
        spec.im_methodology = ImMethodology::Simm;
        let mut sensitivities = SimmSensitivities::new(Currency::USD);
        sensitivities.add_ir_delta(Currency::USD, "5Y", 50_000.0);
        sensitivities.add_equity_delta("AAPL", 100_000.0);
        sensitivities.add_fx_delta(Currency::EUR, 80_000.0);

        let calc = SimmCalculator::new(crate::margin::calculators::im::simm::SimmVersion::V2_6)
            .expect("registry should load");
        let (gross_im, _) = calc.calculate_from_sensitivities(&sensitivities, Currency::USD);
        assert!(gross_im > 0.0, "test setup must produce non-zero SIMM IM");

        let threshold = gross_im / 2.0;
        if let Some(im_params) = spec.csa.im_params.as_mut() {
            im_params.threshold = Money::new(threshold, Currency::USD);
            im_params.mta = Money::new(0.0, Currency::USD);
        }

        let instrument = TestInstrument::new(Money::new(1_000_000.0, Currency::USD), Some(spec))
            .with_sensitivities(sensitivities);
        let metric = InitialMarginMetric::new();
        let market = MarketContext::new();
        let as_of = Date::from_calendar_date(2024, time::Month::January, 1).expect("valid date");

        let im = metric.calculate(&instrument, &market, as_of).expect("im");
        let expected = gross_im - threshold;
        assert!(
            (im.amount.amount() - expected).abs() < 1e-9,
            "IM should subtract breached threshold from gross IM: expected {expected}, got {}",
            im.amount.amount()
        );
    }
}
