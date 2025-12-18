use crate::calibration::api::schema::BaseCorrelationParams;
use crate::calibration::quotes::CreditQuote;
use crate::calibration::quotes::InstrumentConventions;
use crate::calibration::solver::BootstrapTarget;
use crate::instruments::cds_tranche::pricer::CDSTranchePricer;
use crate::instruments::cds_tranche::{CdsTranche, TrancheSide};
use crate::instruments::common::traits::Attributes;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{BaseCorrelationCurve, CreditIndexData};
use finstack_core::money::Money;
use finstack_core::Result;
use std::cell::RefCell;
use std::sync::Arc;

/// A prepared tranche quote paired with a pre-built tranche instrument.
#[derive(Clone)]
pub struct PreparedTrancheQuote {
    /// Original tranche quote payload.
    pub quote: Arc<CreditQuote>,
    /// Pre-built tranche instrument used for pricing.
    pub tranche: Arc<CdsTranche>,
    /// Tranche notional (cached for residual normalization).
    pub notional: f64,
    /// Target upfront fraction (cached; upfront_pct / 100).
    pub target_upfront_frac: f64,
    /// Normalized detachment point in percent (used as the knot-time for bootstrapping).
    pub detachment_pct: f64,
}

/// Bootstrapper for base correlation curves from CDS tranche quotes.
///
/// Implements sequential bootstrapping of base correlation curves using
/// CDS tranche quotes with different detachment points. The bootstrapper
/// creates synthetic tranches and prices them to solve for correlation
/// values that match market quotes.
pub struct BaseCorrelationBootstrapper {
    /// Parameters defining the base correlation curve structure (index, settlement, etc).
    pub params: BaseCorrelationParams,
    /// Baseline market context containing discount curves and credit index components.
    pub base_context: MarketContext,
    /// Optional reusable context for sequential solvers to reduce memory pressure.
    reuse_context: Option<RefCell<MarketContext>>,
}

impl BaseCorrelationBootstrapper {
    /// Creates a new base correlation bootstrapper.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters defining the base correlation curve structure
    /// * `base_context` - Market context containing discount curves and index data
    ///
    /// # Returns
    ///
    /// A new `BaseCorrelationBootstrapper` instance ready for calibration.
    pub fn new(params: BaseCorrelationParams, base_context: MarketContext, use_parallel: bool) -> Self {
        let reuse_context = if use_parallel {
            None
        } else {
            Some(RefCell::new(base_context.clone()))
        };
        Self {
            params,
            base_context,
            reuse_context,
        }
    }

    /// Pre-build tranche instruments once per quote so residual evaluation does not allocate.
    pub fn prepare_quotes(&self, quotes: Vec<CreditQuote>) -> Result<Vec<PreparedTrancheQuote>> {
        quotes
            .into_iter()
            .map(|q| {
                // Validate and pre-build tranche once.
                let (attach_pct, detach_pct, maturity, upfront_pct, running_spread_bp) = match &q {
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

                let quote_conventions = match &q {
                    CreditQuote::CDSTranche { conventions, .. } => conventions,
                    _ => unreachable!("validated above"),
                };

                let schedule_conventions = self.resolve_schedule_conventions(quote_conventions)?;
                let tranche = self.create_synthetic_tranche(
                    attach_pct,
                    detach_pct,
                    maturity,
                    running_spread_bp,
                    schedule_conventions,
                )?;
                let notional = tranche.notional.amount();
                if !notional.is_finite() || notional <= 0.0 {
                    return Err(finstack_core::Error::Validation(format!(
                        "Invalid tranche notional: {}",
                        notional
                    )));
                }
                let target_upfront_frac = upfront_pct / 100.0;
                let detachment_pct = Self::normalize_pct(detach_pct);

                Ok(PreparedTrancheQuote {
                    quote: Arc::new(q),
                    tranche: Arc::new(tranche),
                    notional,
                    target_upfront_frac,
                    detachment_pct,
                })
            })
            .collect()
    }

    fn with_temp_context<F, T>(&self, curve: &BaseCorrelationCurve, op: F) -> Result<T>
    where
        F: FnOnce(&MarketContext) -> Result<T>,
    {
        if let Some(ctx_cell) = &self.reuse_context {
            let mut ctx = ctx_cell.borrow_mut();

            let original_index_data = self.base_context.credit_index_ref(&self.params.index_id)?;
            let updated_index_data = CreditIndexData {
                base_correlation_curve: Arc::new(curve.clone()),
                ..original_index_data.clone()
            };
            ctx.insert_credit_index_mut(&self.params.index_id, updated_index_data);
            op(&ctx)
        } else {
            let original_index_data = self.base_context.credit_index_ref(&self.params.index_id)?;
            let updated_index_data = CreditIndexData {
                base_correlation_curve: Arc::new(curve.clone()),
                ..original_index_data.clone()
            };
            let temp_context = self
                .base_context
                .clone()
                .insert_credit_index(&self.params.index_id, updated_index_data);
            op(&temp_context)
        }
    }

    /// Create a synthetic CDS tranche instrument for calibration.
    ///
    /// The tranche is used as the calibration instrument whose PV is reconciled
    /// against market quotes.
    fn create_synthetic_tranche(
        &self,
        attach_pct: f64,
        detach_pct: f64,
        maturity: Date,
        running_spread_bp: f64,
        schedule_conventions: (Tenor, DayCount, BusinessDayConvention, Option<String>),
    ) -> Result<CdsTranche> {
        let (payment_frequency, day_count, business_day_convention, calendar_id) =
            schedule_conventions;
        CdsTranche::builder()
            .id("CALIB_TRANCHE".into())
            .index_name(self.params.index_id.clone())
            .series(self.params.series)
            .attach_pct(Self::normalize_pct(attach_pct))
            .detach_pct(Self::normalize_pct(detach_pct))
            .notional(Money::new(self.params.notional, self.params.currency))
            .maturity(maturity)
            .running_coupon_bp(running_spread_bp)
            .payment_frequency(payment_frequency)
            .day_count(day_count)
            .business_day_convention(business_day_convention)
            .calendar_id_opt(calendar_id)
            .discount_curve_id(self.params.discount_curve_id.clone())
            .credit_index_id(finstack_core::types::CurveId::new(
                self.params.index_id.clone(),
            ))
            .side(TrancheSide::SellProtection)
            .effective_date_opt(None)
            .accumulated_loss(0.0)
            .standard_imm_dates(self.params.use_imm_dates)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| finstack_core::Error::Validation(e.to_string()))
    }

    /// Normalize a percentage value (either 0.01 or 1.0).
    fn normalize_pct(value: f64) -> f64 {
        if (0.0..=1.0).contains(&value) {
            value * 100.0
        } else {
            value
        }
    }

    /// Validate that correlation points are monotone and within bounds [0, 1].
    fn validate_monotone_and_bounds(points: &[(f64, f64)]) -> Result<()> {
        for &(_, corr) in points {
            if !corr.is_finite() || !(0.0..=1.0).contains(&corr) {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }
        for w in points.windows(2) {
            if w[1].0 <= w[0].0 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
            if w[1].1 + 1e-12 < w[0].1 {
                return Err(finstack_core::Error::Input(
                    finstack_core::error::InputError::Invalid,
                ));
            }
        }
        Ok(())
    }

    /// Resolve schedule conventions from quote-level overrides or plan defaults.
    fn resolve_schedule_conventions(
        &self,
        conventions: &InstrumentConventions,
    ) -> Result<(Tenor, DayCount, BusinessDayConvention, Option<String>)> {
        let payment_frequency = conventions
            .payment_frequency
            .or(self.params.payment_frequency)
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Missing tranche payment frequency; set quote.conventions.payment_frequency or params.payment_frequency"
                        .to_string(),
                )
            })?;

        let day_count = conventions
            .day_count
            .or(self.params.day_count)
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Missing tranche day count; set quote.conventions.day_count or params.day_count"
                        .to_string(),
                )
            })?;

        let business_day_convention = conventions
            .business_day_convention
            .or(self.params.business_day_convention)
            .ok_or_else(|| {
                finstack_core::Error::Validation(
                    "Missing tranche business day convention; set quote.conventions.business_day_convention or params.business_day_convention"
                        .to_string(),
                )
            })?;

        let calendar_id = conventions
            .effective_payment_calendar_id()
            .map(|c| c.to_string())
            .or_else(|| self.params.calendar_id.clone());

        Ok((
            payment_frequency,
            day_count,
            business_day_convention,
            calendar_id,
        ))
    }
}

impl BootstrapTarget for BaseCorrelationBootstrapper {
    type Quote = PreparedTrancheQuote;
    type Curve = BaseCorrelationCurve;

    fn quote_time(&self, quote: &Self::Quote) -> Result<f64> {
        Ok(quote.detachment_pct)
    }

    fn build_curve(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        // knots are (detachment, correlation)
        let mut sorted_knots = knots.to_vec();

        // Market-standard bootstrap needs to be able to price an equity tranche [0,K]
        // even when only a single detachment bucket has been solved so far. The core
        // `BaseCorrelationCurve` requires at least two points, so add a temporary
        // second point with flat extension.
        if sorted_knots.len() == 1 {
            let (k, v) = sorted_knots[0];
            let bump = 10.0;
            let k2 = if k + bump <= 100.0 {
                k + bump
            } else if k >= bump {
                k - bump
            } else {
                (k + 1.0).min(100.0)
            };
            if (k2 - k).abs() > 1e-12 {
                sorted_knots.push((k2, v));
            }
        }

        sorted_knots.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .expect("f64 comparison should always be comparable")
        });
        sorted_knots.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-12);
        if sorted_knots.len() < 2 {
            return Err(finstack_core::Error::Input(
                finstack_core::error::InputError::TooFewPoints,
            ));
        }

        Self::validate_monotone_and_bounds(&sorted_knots)?;

        BaseCorrelationCurve::builder(format!("{}_CORR", self.params.index_id))
            .knots(sorted_knots)
            .build()
    }

    fn build_curve_final(&self, knots: &[(f64, f64)]) -> Result<Self::Curve> {
        let curve = self.build_curve(knots)?;
        let validation = curve.validate_arbitrage_free();
        if !validation.is_arbitrage_free {
            return Err(finstack_core::Error::Validation(format!(
                "Base correlation curve is not arbitrage-free: {:?}",
                validation.violations
            )));
        }
        Ok(curve)
    }

    fn calculate_residual(&self, curve: &Self::Curve, quote: &Self::Quote) -> Result<f64> {
        let pricing_model = CDSTranchePricer::new();

        self.with_temp_context(curve, |ctx| {
            let pv = pricing_model
                .price_tranche(quote.tranche.as_ref(), ctx, self.params.base_date)?
                .amount();
            Ok(pv / quote.notional - quote.target_upfront_frac)
        })
    }

    fn initial_guess(&self, _quote: &Self::Quote, previous_knots: &[(f64, f64)]) -> Result<f64> {
        let prev = previous_knots.last().map(|(_, v)| *v).unwrap_or(0.0);
        Ok(prev.max(0.30).clamp(0.0, 0.999))
    }
}
