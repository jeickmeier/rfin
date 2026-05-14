use super::config::{
    CDSTranchePricer, CDSTranchePricerConfig, ProjectedDiscountedRow, ProjectionInputs,
};
use crate::cashflow::builder::{CashFlowMeta, CashFlowSchedule};
use crate::cashflow::primitives::{CFKind, CashFlow};
use crate::constants::BASIS_POINTS_PER_UNIT;
use crate::correlation::copula::{
    Copula, CopulaSpec, GaussianCopula, MultiFactorCopula, RandomFactorLoadingCopula,
    StudentTCopula,
};
use crate::instruments::credit_derivatives::cds_tranche::{CDSTranche, TrancheSide};
use finstack_core::dates::{CalendarRegistry, Date, DateExt, HolidayCalendar};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::math::{standard_normal_inv_cdf, student_t_inv_cdf, GaussHermiteQuadrature};
use finstack_core::money::Money;
use finstack_core::Result;

impl CDSTranchePricer {
    #[inline]
    pub(super) fn select_quadrature(&self) -> Result<&GaussHermiteQuadrature> {
        Ok(self.quadrature_cache.get_or_init(|| {
            crate::correlation::copula::select_quadrature(self.params.quadrature_order)
        }))
    }

    /// Return the cached copula instance, building it on first call.
    ///
    /// The copula is determined entirely by `self.params.copula_spec` at
    /// pricer-construction time, so a single instance can be reused across
    /// every EL/integrand evaluation for the lifetime of this pricer.
    pub(super) fn copula(&self) -> &dyn Copula {
        self.copula_cache
            .get_or_init(|| match &self.params.copula_spec {
                CopulaSpec::Gaussian => Box::new(GaussianCopula::with_quadrature_order(
                    self.params.quadrature_order,
                )),
                CopulaSpec::StudentT { degrees_of_freedom } => {
                    Box::new(StudentTCopula::with_quadrature_order(
                        *degrees_of_freedom,
                        self.params.quadrature_order,
                    ))
                }
                CopulaSpec::RandomFactorLoading { loading_volatility } => {
                    Box::new(RandomFactorLoadingCopula::with_quadrature_order(
                        *loading_volatility,
                        self.params.quadrature_order,
                    ))
                }
                CopulaSpec::MultiFactor { num_factors } => {
                    Box::new(MultiFactorCopula::new(*num_factors))
                }
            })
            .as_ref()
    }

    pub(super) fn default_threshold_for_copula(&self, default_prob: f64) -> f64 {
        let eps = self.params.probability_clip;
        let p = default_prob.max(eps).min(1.0 - eps);
        match &self.params.copula_spec {
            CopulaSpec::StudentT { degrees_of_freedom } => {
                student_t_inv_cdf(p, *degrees_of_freedom)
            }
            _ => standard_normal_inv_cdf(p),
        }
    }

    pub(super) fn conditional_default_prob_copula(
        &self,
        copula: &dyn Copula,
        default_threshold: f64,
        factor_realization: &[f64],
        correlation: f64,
    ) -> f64 {
        copula
            .conditional_default_prob(default_threshold, factor_realization, correlation)
            .clamp(0.0, 1.0)
    }
    /// Create a new Gaussian Copula model with default parameters.
    pub fn new() -> Self {
        Self {
            params: CDSTranchePricerConfig::default(),
            copula_cache: std::sync::OnceLock::new(),
            quadrature_cache: std::sync::OnceLock::new(),
        }
    }

    /// Create a new model with custom parameters.
    pub fn with_params(params: CDSTranchePricerConfig) -> Self {
        Self {
            params,
            copula_cache: std::sync::OnceLock::new(),
            quadrature_cache: std::sync::OnceLock::new(),
        }
    }

    /// Price a CDS tranche using the Gaussian Copula model.
    ///
    /// Falls back to zero PV when credit index data is not available as default behavior.
    ///
    /// # Arguments
    /// * `tranche` - The CDS tranche to price
    /// * `market_ctx` - Market data context containing curves and credit index data
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    /// The present value of the tranche
    ///
    /// # Settlement Convention
    ///
    /// Uses ISDA standard settlement:
    /// - Index CDS tranches (CDX, iTraxx): T+1 business days (Big Bang 2009)
    /// - Bespoke tranches: T+3 business days
    #[must_use = "pricing result should be used"]
    pub fn price_tranche(
        &self,
        tranche: &CDSTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        // Check if tranche is already wiped out
        if tranche.accumulated_loss >= tranche.detach_pct / 100.0 {
            return Ok(Money::new(0.0, tranche.notional.currency()));
        }

        let discount_curve = market_ctx.get_discount(tranche.discount_curve_id.as_ref())?;
        let rows = self.project_discountable_rows(tranche, market_ctx, as_of)?;

        if rows.is_empty() {
            return Ok(Money::new(0.0, tranche.notional.currency()));
        }

        let net_pv = self.discount_projected_rows(&rows, discount_curve.as_ref(), as_of)?;

        Ok(Money::new(net_pv, tranche.notional.currency()))
    }

    /// Build the projected premium/default schedule for the tranche.
    pub fn build_projected_schedule(
        &self,
        tranche: &CDSTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<CashFlowSchedule> {
        if as_of >= tranche.maturity {
            return Ok(crate::cashflow::traits::schedule_from_classified_flows(
                Vec::new(),
                tranche.day_count,
                crate::cashflow::traits::ScheduleBuildOpts {
                    notional_hint: Some(tranche.notional),
                    meta: Some(CashFlowMeta {
                        representation: crate::cashflow::builder::CashflowRepresentation::Projected,
                        calendar_ids: tranche.calendar_id.clone().into_iter().collect(),
                        facility_limit: None,
                        issue_date: tranche.contractual_effective_date(as_of),
                    }),
                    ..Default::default()
                },
            ));
        }
        let (_, valuation_date, _, _) =
            self.prepare_projection_inputs(tranche, market_ctx, as_of)?;
        let flows = self
            .project_discountable_rows(tranche, market_ctx, as_of)?
            .into_iter()
            .map(|row| row.cashflow)
            .collect();

        Ok(crate::cashflow::traits::schedule_from_classified_flows(
            flows,
            tranche.day_count,
            crate::cashflow::traits::ScheduleBuildOpts {
                notional_hint: Some(tranche.notional),
                meta: Some(CashFlowMeta {
                    representation: crate::cashflow::builder::CashflowRepresentation::Projected,
                    calendar_ids: tranche.calendar_id.clone().into_iter().collect(),
                    facility_limit: None,
                    issue_date: tranche.contractual_effective_date(valuation_date),
                }),
                ..Default::default()
            },
        ))
    }

    pub(super) fn project_discountable_rows(
        &self,
        tranche: &CDSTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Vec<ProjectedDiscountedRow>> {
        if as_of >= tranche.maturity {
            return Ok(Vec::new());
        }
        let (_index_data_arc, valuation_date, payment_dates, el_curve) =
            self.prepare_projection_inputs(tranche, market_ctx, as_of)?;
        if valuation_date >= tranche.maturity {
            return Ok(Vec::new());
        }

        let coupon = tranche.running_coupon_bp / BASIS_POINTS_PER_UNIT;
        let tranche_notional = tranche.notional.amount();
        let premium_sign = match tranche.side {
            TrancheSide::BuyProtection => -1.0,
            TrancheSide::SellProtection => 1.0,
        };
        let protection_sign = -premium_sign;

        let mut rows =
            Vec::with_capacity(payment_dates.len() * 2 + usize::from(tranche.upfront.is_some()));
        let mut prev_el_fraction = self.calculate_prior_tranche_loss(tranche);

        // Pre-compute payment times once to avoid calling years_from_base twice per iteration.
        let payment_times: Vec<f64> = payment_dates
            .iter()
            .map(|&d| self.years_from_base(_index_data_arc.as_ref(), d))
            .collect::<Result<Vec<_>>>()?;

        for (i, &payment_date) in payment_dates.iter().enumerate() {
            let el_fraction = el_curve[i].1;
            let delta_el_fraction = (el_fraction - prev_el_fraction).max(0.0);
            let outstanding_notional = tranche_notional * (1.0 - prev_el_fraction);
            let period_start = if i == 0 {
                tranche
                    .contractual_effective_date(valuation_date)
                    .unwrap_or(valuation_date)
            } else {
                payment_dates[i - 1]
            };
            let accrual_period = tranche.day_count.year_fraction(
                period_start,
                payment_date,
                finstack_core::dates::DayCountContext::default(),
            )?;
            let payment_time = payment_times[i];
            let aod_adjustment = if self.params.accrual_on_default_enabled {
                self.params.aod_allocation_fraction * tranche_notional * delta_el_fraction
            } else {
                0.0
            };
            let premium_amount = coupon * accrual_period * (outstanding_notional - aod_adjustment);

            if premium_amount.abs() > f64::EPSILON {
                rows.push(ProjectedDiscountedRow {
                    cashflow: CashFlow {
                        date: payment_date,
                        reset_date: None,
                        amount: Money::new(
                            premium_amount * premium_sign,
                            tranche.notional.currency(),
                        ),
                        kind: CFKind::Fixed,
                        accrual_factor: accrual_period,
                        rate: Some(coupon),
                    },
                    discount_time: Some(payment_time),
                });
            }

            let default_amount = tranche_notional * delta_el_fraction;
            if default_amount.abs() > f64::EPSILON {
                rows.push(ProjectedDiscountedRow {
                    cashflow: CashFlow {
                        date: payment_date,
                        reset_date: None,
                        amount: Money::new(
                            default_amount * protection_sign,
                            tranche.notional.currency(),
                        ),
                        kind: CFKind::DefaultedNotional,
                        accrual_factor: 0.0,
                        rate: None,
                    },
                    discount_time: Some(if self.params.mid_period_protection {
                        let prior_time = if i == 0 { 0.0 } else { payment_times[i - 1] };
                        0.5 * (prior_time + payment_time)
                    } else {
                        payment_time
                    }),
                });
            }

            prev_el_fraction = el_fraction;
        }

        if let Some((date, amount)) = tranche.upfront.filter(|(date, _)| *date >= as_of) {
            rows.push(ProjectedDiscountedRow {
                cashflow: CashFlow {
                    date,
                    reset_date: None,
                    amount: Money::new(amount.amount() * premium_sign, amount.currency()),
                    kind: CFKind::Fee,
                    accrual_factor: 0.0,
                    rate: None,
                },
                discount_time: None,
            });
        }

        Ok(rows)
    }

    pub(super) fn discount_projected_rows(
        &self,
        rows: &[ProjectedDiscountedRow],
        discount_curve: &dyn Discounting,
        as_of: Date,
    ) -> Result<f64> {
        let mut pv = 0.0;
        for row in rows {
            let df = match row.discount_time {
                Some(t) => discount_curve.df(t),
                None => discount_curve.df_between_dates(as_of, row.cashflow.date)?,
            };
            pv += row.cashflow.amount.amount() * df;
        }
        Ok(pv)
    }

    pub(super) fn prepare_projection_inputs(
        &self,
        tranche: &CDSTranche,
        market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<ProjectionInputs> {
        let index_data_arc = market_ctx
            .get_credit_index(&tranche.credit_index_id)
            .map_err(|_| {
                finstack_core::Error::Input(finstack_core::InputError::NotFound {
                    id: format!(
                        "Credit index '{}' required for tranche '{}' pricing",
                        tranche.credit_index_id, tranche.id
                    ),
                })
            })?;
        let valuation_date = self.calculate_settlement_date(tranche, market_ctx, as_of)?;
        let payment_dates = self.generate_payment_schedule(tranche, valuation_date)?;
        let el_curve = if payment_dates.is_empty() || valuation_date >= tranche.maturity {
            Vec::new()
        } else {
            self.build_el_curve(tranche, index_data_arc.as_ref(), &payment_dates)?
        };
        Ok((index_data_arc, valuation_date, payment_dates, el_curve))
    }

    /// Calculate the settlement date based on ISDA conventions.
    ///
    /// - If effective_date is set, uses as_of directly (explicit settlement)
    /// - For index tranches (CDX, iTraxx): T+1 business days
    /// - For bespoke tranches: T+3 business days
    ///
    /// Uses business day calendars when available via the tranche's `calendar_id`.
    /// Falls back to weekend-only logic when no calendar is specified.
    pub(super) fn calculate_settlement_date(
        &self,
        tranche: &CDSTranche,
        _market_ctx: &MarketContext,
        as_of: Date,
    ) -> Result<Date> {
        // If effective date is explicitly set, use as_of directly
        if tranche.effective_date.is_some() {
            return Ok(as_of);
        }

        // Determine settlement lag based on index type
        let is_standard_index = tranche.index_name.starts_with("CDX")
            || tranche.index_name.starts_with("iTraxx")
            || tranche.index_name.starts_with("ITRAXX");
        let settlement_lag = if is_standard_index {
            self.params.index_settlement_lag
        } else {
            self.params.bespoke_settlement_lag
        };

        // Use calendar if available, otherwise fall back to weekday-only adjustment
        let calendar: Option<&dyn HolidayCalendar> = tranche
            .calendar_id
            .as_deref()
            .and_then(|id| CalendarRegistry::global().resolve_str(id));

        if let Some(cal) = calendar {
            as_of.add_business_days(settlement_lag, cal)
        } else {
            Ok(as_of.add_weekdays(settlement_lag))
        }
    }

    /// Calculate effective attachment/detachment points given accumulated losses.
    ///
    /// Returns (effective_attach, effective_detach, survival_factor)
    /// where survival_factor is (1 - L).
    ///
    /// # Invariants
    ///
    /// - Accumulated loss is in [0, 1]
    /// - Attachment <= Detachment (after percentage conversion)
    /// - Results are always in [0, 1]
    pub(super) fn calculate_effective_structure(&self, tranche: &CDSTranche) -> (f64, f64, f64) {
        let l = tranche.accumulated_loss;
        let attach = tranche.attach_pct / 100.0;
        let detach = tranche.detach_pct / 100.0;

        // Debug assertions for invariants
        debug_assert!(
            (0.0..=1.0).contains(&l),
            "accumulated_loss {} must be in [0, 1]",
            l
        );
        debug_assert!(
            attach <= detach,
            "attach {} must be <= detach {}",
            attach,
            detach
        );
        debug_assert!(
            (0.0..=1.0).contains(&attach),
            "attach {} must be in [0, 1]",
            attach
        );
        debug_assert!(
            (0.0..=1.0).contains(&detach),
            "detach {} must be in [0, 1]",
            detach
        );

        if l >= 1.0 - 1e-9 {
            return (0.0, 0.0, 0.0);
        }

        let survival_factor = 1.0 - l;

        let eff_attach = (attach - l).max(0.0) / survival_factor;
        let eff_detach = (detach - l).max(0.0) / survival_factor;

        // Clamp to [0, 1] (eff_detach can be > 1 theoretically if L is huge but we check L >= D before)
        let result = (
            eff_attach.clamp(0.0, 1.0),
            eff_detach.clamp(0.0, 1.0),
            survival_factor,
        );

        // Post-condition assertions
        debug_assert!(
            result.0 <= result.1,
            "effective attach {} > effective detach {}",
            result.0,
            result.1
        );

        result
    }
}
