use crate::calibration::config::CalibrationConfig;
use crate::calibration::v2::api::schema::BaseCorrelationParams;
use crate::calibration::v2::domain::quotes::CreditQuote;
use crate::calibration::v2::domain::solver::BootstrapTarget;
use crate::instruments::cds_tranche::pricer::CDSTranchePricer;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{BaseCorrelationCurve, CreditIndexData};
use finstack_core::money::Money;
use finstack_core::Result;
use std::sync::Arc;

/// Bootstrapper for base correlation curves from CDS tranche quotes.
///
/// Implements sequential bootstrapping of base correlation curves using
/// CDS tranche quotes with different detachment points. The bootstrapper
/// creates synthetic tranches and prices them to solve for correlation
/// values that match market quotes.
pub struct BaseCorrelationBootstrapper {
    params: BaseCorrelationParams,
    #[allow(dead_code)]
    config: CalibrationConfig,
    quotes: Vec<CreditQuote>, // Only Tranche quotes
    base_context: MarketContext,
}

impl BaseCorrelationBootstrapper {
    /// Creates a new base correlation bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the base correlation curve structure
    /// * `quotes` - CDS tranche quotes used for calibration
    /// * `config` - Calibration configuration settings
    /// * `base_context` - Market context containing discount curves and index data
    ///
    /// # Returns
    ///
    /// A new `BaseCorrelationBootstrapper` instance ready for calibration.
    pub fn new(
        params: BaseCorrelationParams,
        quotes: Vec<CreditQuote>,
        config: CalibrationConfig,
        base_context: MarketContext,
    ) -> Self {
        Self {
            params,
            config,
            quotes,
            base_context,
        }
    }

    /// Returns the tranche quotes used for calibration.
    ///
    /// # Returns
    ///
    /// A slice of credit quotes (tranche quotes) that will be used
    /// during the bootstrapping process.
    pub fn instruments(&self) -> &[CreditQuote] {
        &self.quotes
    }

    fn create_synthetic_tranche(
        &self,
        attach_pct: f64,
        detach_pct: f64,
        maturity: Date,
        running_spread_bp: f64,
    ) -> Result<CdsTranche> {
        CdsTranche::builder()
            .id("CALIB_TRANCHE".into())
            .index_name(self.params.index_id.clone())
            .series(self.params.series)
            .attach_pct(attach_pct)
            .detach_pct(detach_pct)
            .notional(Money::new(
                10_000_000.0,
                finstack_core::currency::Currency::USD,
            )) // TODO: Infer currency
            .maturity(maturity)
            .running_coupon_bp(running_spread_bp)
            .payment_frequency(Tenor::quarterly())
            .day_count(DayCount::Act360)
            .business_day_convention(BusinessDayConvention::Following)
            .discount_curve_id(self.params.discount_curve_id.clone())
            .credit_index_id(finstack_core::types::CurveId::new(
                self.params.index_id.clone(),
            ))
            .side(TrancheSide::SellProtection)
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))
    }
}

impl BootstrapTarget for BaseCorrelationBootstrapper {
    type Quote = CreditQuote;
    type Curve = BaseCorrelationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        match quote {
            CreditQuote::CDSTranche { detachment, .. } => Ok(*detachment),
            _ => Err(finstack_core::Error::Input(
                finstack_core::error::InputError::Invalid,
            )),
        }
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // knots are (detachment, correlation)
        let sorted_knots = knots.to_vec();
        // Assuming knots are sorted by bootstrapper

        BaseCorrelationCurve::builder(format!("{}_CORR", self.params.index_id))
            .knots(sorted_knots)
            .build()
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let (attach_pct, detach_pct, maturity, upfront_pct, running_spread_bp) = match quote {
            CreditQuote::CDSTranche {
                index,
                attachment,
                detachment,
                maturity,
                upfront_pct,
                running_spread_bp,
                ..
            } if index == &self.params.index_id => (
                *attachment,
                *detachment,
                *maturity,
                *upfront_pct,
                *running_spread_bp,
            ),
            _ => {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ))
            }
        };

        let synthetic_tranche =
            self.create_synthetic_tranche(attach_pct, detach_pct, maturity, running_spread_bp)?;
        let target_upfront = upfront_pct / 100.0 * synthetic_tranche.notional.amount();

        let pricing_model = CDSTranchePricer::new();

        // Update context with candidate curve
        let original_index_data = self.base_context.credit_index_ref(&self.params.index_id)?;
        let updated_index_data = CreditIndexData {
            base_correlation_curve: Arc::new(curve.clone()),
            ..original_index_data.clone()
        };

        let temp_context = self
            .base_context
            .clone()
            .insert_credit_index(&self.params.index_id, updated_index_data);

        let pv = pricing_model
            .price_tranche(&synthetic_tranche, &temp_context, self.params.base_date)?
            .amount();
        Ok(pv - target_upfront)
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        Ok(previous_knots.last().map(|(_, v)| *v).unwrap_or(0.3))
    }
}
