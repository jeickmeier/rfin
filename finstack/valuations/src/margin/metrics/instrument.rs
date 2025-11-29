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
//! ```rust,ignore
//! use finstack_valuations::margin::metrics::{InitialMarginMetric, VariationMarginMetric};
//! use finstack_valuations::instruments::irs::InterestRateSwap;
//!
//! let swap = InterestRateSwap::example()?;
//! let market = MarketContext::builder().build();
//!
//! // Calculate initial margin
//! let im_metric = InitialMarginMetric::new();
//! let im = im_metric.calculate(&swap, &market, as_of)?;
//!
//! // Calculate variation margin
//! let vm_metric = VariationMarginMetric::new();
//! let vm = vm_metric.calculate(&swap, &market, as_of)?;
//! ```

use crate::margin::calculators::{
    HaircutImCalculator, ImCalculator, ImResult, ScheduleImCalculator, SimmCalculator,
    VmCalculator, VmResult,
};
use crate::margin::traits::{InstrumentMarginResult, Marginable, SimmSensitivities};
use crate::margin::types::{ClearingStatus, ImMethodology, OtcMarginSpec};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::Result;

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

    /// Calculate initial margin for an instrument.
    pub fn calculate<I: Marginable>(
        &self,
        instrument: &I,
        market: &MarketContext,
        as_of: Date,
    ) -> Result<ImResult> {
        // Check if instrument has margin spec
        let margin_spec = instrument.margin_spec();

        // Determine methodology and calculate
        if let Some(spec) = margin_spec {
            self.calculate_otc_im(instrument, spec, market, as_of)
        } else if let Some(repo_spec) = instrument.repo_margin_spec() {
            // For repos, use haircut-based IM
            let haircut_calc =
                HaircutImCalculator::new(repo_spec.eligible_substitutes.clone().unwrap_or_default());
            haircut_calc.calculate(instrument, market, as_of)
        } else {
            // No margin spec - return zero IM
            // Try to get MTM to determine currency, otherwise default to USD
            let currency = instrument
                .mtm_for_vm(market, as_of)
                .map(|m| m.currency())
                .unwrap_or(Currency::USD);
            Ok(ImResult::simple(
                Money::new(0.0, currency),
                ImMethodology::Schedule, // Default methodology for no-margin case
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
        match spec.im_methodology {
            ImMethodology::Simm => {
                let simm = self
                    .simm_calculator
                    .clone()
                    .unwrap_or_default();

                // Get sensitivities from instrument
                let sensitivities = instrument.simm_sensitivities(market, as_of)?;

                // Calculate SIMM
                self.calculate_simm_from_sensitivities(&simm, &sensitivities, spec, as_of)
            }
            ImMethodology::Schedule => {
                let schedule = self
                    .schedule_calculator
                    .clone()
                    .unwrap_or_else(ScheduleImCalculator::bcbs_standard);

                schedule.calculate(instrument, market, as_of)
            }
            ImMethodology::Haircut => {
                let haircut_calc = HaircutImCalculator::new(
                    spec.csa.eligible_collateral.clone(),
                );
                haircut_calc.calculate(instrument, market, as_of)
            }
            ImMethodology::ClearingHouse => {
                // For cleared trades, we'd call the CCP calculator
                // For now, fall back to schedule
                let schedule = ScheduleImCalculator::bcbs_standard();
                schedule.calculate(instrument, market, as_of)
            }
            ImMethodology::InternalModel => {
                // Internal model not implemented - fall back to schedule
                let schedule = ScheduleImCalculator::bcbs_standard();
                schedule.calculate(instrument, market, as_of)
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
        let (mut total_im, mut breakdown) =
            simm.calculate_from_sensitivities(sensitivities, currency);

        // Apply IM threshold if any
        if let Some(im_params) = &spec.csa.im_params {
            if total_im < im_params.threshold.amount() {
                total_im = 0.0;
                breakdown.clear();
            }
        }

        // Get MPOR from spec
        let mpor_days = spec.csa.im_params.as_ref().map(|p| p.mpor_days).unwrap_or(10);

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
mod tests {
    use super::*;

    #[test]
    fn test_initial_margin_metric_default() {
        let metric = InitialMarginMetric::new();
        assert!(metric.simm_calculator.is_none());
        assert!(metric.schedule_calculator.is_none());
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
}
