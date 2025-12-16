use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::api::schema::HazardCurveParams;
use crate::calibration::v2::domain::quotes::CreditQuote;
use crate::calibration::v2::domain::solver::BootstrapTarget;
use crate::instruments::cds::pricer::CDSPricer;
use crate::instruments::cds::{CDSConvention, PayReceive};
use finstack_core::dates::DayCountCtx;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{HazardCurve, Seniority};
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::Result;

/// Bootstrapper for hazard curves from CDS quotes.
///
/// Implements sequential bootstrapping of hazard curves using CDS quotes
/// with different maturities. The bootstrapper derives CDS conventions from
/// currency and prices synthetic CDS instruments to solve for hazard rates
/// that match market quotes.
pub struct HazardBootstrapper {
    params: HazardCurveParams,
    #[allow(dead_code)]
    config: CalibrationConfig,
    quotes: Vec<CreditQuote>,
    // Cached derived values
    #[allow(dead_code)]
    seniority: Seniority,
    convention: CDSConvention,
    base_context: MarketContext,
}

impl HazardBootstrapper {
    /// Creates a new hazard curve bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the hazard curve structure
    /// * `quotes` - CDS quotes used for calibration
    /// * `config` - Calibration configuration settings
    /// * `base_context` - Market context containing discount curves
    ///
    /// # Returns
    ///
    /// A new `HazardBootstrapper` instance ready for calibration.
    ///
    /// # Note
    ///
    /// CDS conventions are automatically derived from the currency:
    /// - USD/CAD: ISDA North American
    /// - EUR/GBP/CHF: ISDA European
    /// - JPY/HKD/SGD/AUD/NZD: ISDA Asian
    pub fn new(
        params: HazardCurveParams,
        quotes: Vec<CreditQuote>,
        config: CalibrationConfig,
        base_context: MarketContext,
    ) -> Self {
        // Derive convention from currency (defaulting logic similar to v1)
        let convention = match params.currency {
            Currency::USD | Currency::CAD => CDSConvention::IsdaNa,
            Currency::EUR | Currency::GBP | Currency::CHF => CDSConvention::IsdaEu,
            Currency::JPY | Currency::HKD | Currency::SGD | Currency::AUD | Currency::NZD => {
                CDSConvention::IsdaAs
            }
            _ => CDSConvention::IsdaNa,
        };

        Self {
            params,
            config,
            quotes,
            seniority: Seniority::Senior, // Default to Senior for now
            convention,
            base_context,
        }
    }

    /// Returns the CDS quotes used for calibration.
    ///
    /// # Returns
    ///
    /// A slice of credit quotes (CDS quotes) that will be used
    /// during the bootstrapping process.
    pub fn instruments(&self) -> &[CreditQuote] {
        &self.quotes
    }
}

impl BootstrapTarget for HazardBootstrapper {
    type Quote = CreditQuote;
    type Curve = HazardCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        let dc = self.convention.day_count();
        let maturity = quote.maturity_date().ok_or(finstack_core::Error::Input(
            finstack_core::error::InputError::Invalid,
        ))?;
        dc.year_fraction(self.params.base_date, maturity, DayCountCtx::default())
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        HazardCurve::builder(self.params.curve_id.to_string())
            .base_date(self.params.base_date)
            .day_count(self.convention.day_count())
            .recovery_rate(self.params.recovery_rate)
            .knots(knots.to_vec())
            .par_interp(finstack_core::market_data::term_structures::ParInterp::Linear) // Could be param
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let base_date = self.params.base_date;
        let discount = self.base_context.get_discount_ref(&self.params.discount_curve_id)?;
        let pricer = CDSPricer::new();

        // Extract quote details
        let (maturity, spread_bp, upfront_pct_opt, conventions) = match quote {
            CreditQuote::CDS { maturity, spread_bp, conventions, .. } => 
                (*maturity, *spread_bp, None, conventions),
            CreditQuote::CDSUpfront { maturity, running_spread_bp, upfront_pct, conventions, .. } => 
                (*maturity, *running_spread_bp, Some(*upfront_pct), conventions),
            _ => return Err(finstack_core::Error::Input(finstack_core::error::InputError::Invalid)),
        };

        // Build instrument
        let premium_spec = crate::instruments::cds::PremiumLegSpec {
            start: base_date,
            end: maturity,
            freq: conventions.payment_frequency.unwrap_or(self.convention.frequency()),
            stub: self.convention.stub_convention(),
            bdc: conventions.business_day_convention.unwrap_or(self.convention.business_day_convention()),
            calendar_id: Some(conventions.effective_payment_calendar_id().unwrap_or(self.convention.default_calendar()).to_string()),
            dc: conventions.day_count.unwrap_or(self.convention.day_count()),
            spread_bp,
            discount_curve_id: self.params.discount_curve_id.clone(),
        };

        let protection_spec = crate::instruments::cds::ProtectionLegSpec {
            credit_curve_id: self.params.curve_id.clone(),
            recovery_rate: self.params.recovery_rate,
            settlement_delay: conventions.settlement_days.unwrap_or(self.convention.settlement_delay() as i32) as u16,
        };

        let cds = crate::instruments::cds::CreditDefaultSwapBuilder::new()
            .id("CALIB_CDS".into())
            .notional(Money::new(10_000_000.0, self.params.currency))
            .side(PayReceive::PayFixed)
            .convention(self.convention)
            .premium(premium_spec)
            .protection(protection_spec)
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))?;

        let npv = pricer.npv(&cds, discount, curve, base_date)?.amount();

        match upfront_pct_opt {
            None => Ok(npv / cds.notional.amount()),
            Some(upfront) => Ok((npv - cds.notional.amount() * upfront / 100.0) / cds.notional.amount()),
        }
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        // Last point or default
        Ok(previous_knots.last().map(|&(_, v)| v).unwrap_or(0.01))
    }
}
