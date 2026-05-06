use super::engine::{AodInputs, CDSPricer, CouponPeriod};
use super::helpers::{df_asof_to, sp_cond_to};
use crate::constants::{numerical, BASIS_POINTS_PER_UNIT, ONE_BASIS_POINT};
use crate::instruments::common_impl::helpers::year_fraction;
use crate::instruments::credit_derivatives::cds::{CreditDefaultSwap, PayReceive};
use finstack_core::dates::{adjust, next_cds_date, Date};
#[cfg(test)]
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, HazardCurve};
use finstack_core::money::Money;
use finstack_core::{Error, Result};

impl CDSPricer {
    /// Generate the canonical CDS payment schedule.
    ///
    /// CDS pricing uses the ISDA IMM-20 schedule with business-day adjustment
    /// per the instrument calendar.
    #[must_use = "schedule generation is pure computation"]
    pub(crate) fn generate_schedule(
        &self,
        cds: &CreditDefaultSwap,
        _as_of: Date,
    ) -> Result<Vec<Date>> {
        self.generate_isda_schedule(cds)
    }

    /// Generate ISDA standard coupon dates (20th of Mar/Jun/Sep/Dec).
    ///
    /// Payment dates are adjusted using the CDS calendar and business day
    /// convention (Modified Following per ISDA 2014 standard). If no calendar
    /// is specified, dates are returned unadjusted.
    pub(crate) fn generate_isda_schedule(&self, cds: &CreditDefaultSwap) -> Result<Vec<Date>> {
        let mut schedule = vec![cds.premium.start];
        let mut current = cds.premium.start;

        // Resolve calendar for business day adjustment
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        while current < cds.premium.end {
            current = next_cds_date(current);
            if current <= cds.premium.end {
                // Apply business day adjustment if calendar is available
                let adjusted = if let Some(cal) = calendar {
                    adjust(current, cds.premium.bdc, cal).unwrap_or(current)
                } else {
                    current
                };
                schedule.push(adjusted);
            }
        }

        // Handle maturity date - ensure it's in the schedule
        let maturity_adjusted = if let Some(cal) = calendar {
            adjust(cds.premium.end, cds.premium.bdc, cal).unwrap_or(cds.premium.end)
        } else {
            cds.premium.end
        };

        if schedule.last() != Some(&maturity_adjusted) {
            schedule.push(maturity_adjusted);
        }

        Ok(schedule)
    }

    pub(super) fn coupon_periods(
        &self,
        cds: &CreditDefaultSwap,
        as_of: Date,
    ) -> Result<Vec<CouponPeriod>> {
        self.generate_isda_coupon_periods(cds, as_of)
    }

    fn generate_isda_coupon_periods(
        &self,
        cds: &CreditDefaultSwap,
        _as_of: Date,
    ) -> Result<Vec<CouponPeriod>> {
        if cds.uses_adjusted_premium_accrual_dates() {
            // Degenerate schedules (start >= end) have no future premium
            // cashflows. Returning an empty list mirrors the unadjusted
            // ISDA path and avoids spurious one-day phantom periods that
            // can appear when the maturity is on a holiday and gets
            // business-day-adjusted forward.
            if cds.premium.start >= cds.premium.end {
                return Ok(Vec::new());
            }
            let schedule = self.generate_isda_schedule(cds)?;
            return Ok(schedule
                .windows(2)
                .enumerate()
                .map(|window| CouponPeriod {
                    accrual_start: window.1[0],
                    accrual_end: window.1[1],
                    payment_date: window.1[1],
                    is_final: window.0 + 2 == schedule.len(),
                })
                .collect());
        }

        let mut accrual_dates = vec![cds.premium.start];
        let mut current = cds.premium.start;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);

        while current < cds.premium.end {
            current = next_cds_date(current);
            if current <= cds.premium.end {
                accrual_dates.push(current);
            }
        }
        if accrual_dates.last() != Some(&cds.premium.end) {
            accrual_dates.push(cds.premium.end);
        }

        let mut periods = Vec::with_capacity(accrual_dates.len().saturating_sub(1));
        for (idx, window) in accrual_dates.windows(2).enumerate() {
            let payment_date = if let Some(cal) = calendar {
                adjust(window[1], cds.premium.bdc, cal).unwrap_or(window[1])
            } else {
                window[1]
            };
            periods.push(CouponPeriod {
                accrual_start: window[0],
                accrual_end: window[1],
                payment_date,
                is_final: idx + 2 == accrual_dates.len(),
            });
        }
        Ok(periods)
    }

    pub(super) fn coupon_accrual(
        &self,
        cds: &CreditDefaultSwap,
        period: &CouponPeriod,
    ) -> Result<f64> {
        // QuantLib `Actual360(true)` parity: when explicitly requested,
        // every Act/360 accrual period is inclusive of its end date. This
        // is opt-in via `cds_act360_include_last_day` for QuantLib parity
        // tests; production CDS pricing uses the standard Bloomberg CDSW
        // rule below.
        if cds
            .pricing_overrides
            .model_config
            .cds_act360_include_last_day
            && cds.premium.day_count == finstack_core::dates::DayCount::Act360
            && period.accrual_end > period.accrual_start
        {
            let days = finstack_core::dates::DayCount::calendar_days(
                period.accrual_start,
                period.accrual_end,
            ) + 1;
            return Ok((days.max(0) as f64) / 360.0);
        }
        // CDSW final-coupon convention: the final Act/360 premium period is
        // inclusive of the maturity date (one extra calendar day). The rule
        // is the canonical Bloomberg CDSW behaviour and is shared by every
        // pricer/convention that uses business-day-adjusted accrual periods
        // (`uses_adjusted_premium_accrual_dates()` — currently
        // `BloombergCdswClean` plus the explicit override flag). The CDS
        // option synthetic underlying and CDS tranche index legs both type
        // their underlying CDS as Bloomberg-clean so they pick up this rule
        // automatically.
        //
        // The `payment_date == cds.premium.end` guard avoids double-counting
        // when business-day adjustment has already pushed the final accrual
        // boundary past the unadjusted maturity (e.g. a Sunday IMM rolling
        // forward to Monday). In that case the BDA shift already accounts
        // for the extra calendar day(s) and the +1-day rule must not apply
        // on top of it.
        if cds.uses_adjusted_premium_accrual_dates()
            && cds.premium.day_count == finstack_core::dates::DayCount::Act360
            && period.is_final
            && period.payment_date == cds.premium.end
            && period.accrual_end > period.accrual_start
        {
            let days = finstack_core::dates::DayCount::calendar_days(
                period.accrual_start,
                period.accrual_end,
            ) + 1;
            return Ok((days.max(0) as f64) / 360.0);
        }

        year_fraction(
            cds.premium.day_count,
            period.accrual_start,
            period.accrual_end,
        )
    }

    pub(crate) fn premium_cashflow_accruals(
        &self,
        cds: &CreditDefaultSwap,
        as_of: Date,
    ) -> Result<Vec<(Date, f64)>> {
        self.coupon_periods(cds, as_of)?
            .into_iter()
            .map(|period| Ok((period.payment_date, self.coupon_accrual(cds, &period)?)))
            .collect()
    }

    fn clean_accrued_fraction(&self, cds: &CreditDefaultSwap, as_of: Date) -> Result<f64> {
        if as_of <= cds.premium.start || as_of >= cds.premium.end {
            return Ok(0.0);
        }

        let schedule = self.generate_schedule(cds, as_of)?;
        let mut last_coupon = cds.premium.start;
        for &coupon_date in &schedule {
            if coupon_date <= as_of {
                last_coupon = coupon_date;
            } else {
                break;
            }
        }

        if cds.premium.day_count == finstack_core::dates::DayCount::Act360 {
            let days = finstack_core::dates::DayCount::calendar_days(last_coupon, as_of) + 1;
            return Ok((days.max(0) as f64) / 360.0);
        }

        year_fraction(cds.premium.day_count, last_coupon, as_of)
    }

    fn clean_par_spread_denominator(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let full_premium_per_bp = self.premium_leg_pv_per_bp(cds, disc, surv, as_of)?;
        let accrued_per_unit_spread = self.clean_accrued_fraction(cds, as_of)?;
        Ok(full_premium_per_bp / ONE_BASIS_POINT - accrued_per_unit_spread)
    }

    /// Calculate par spread (bps) that sets NPV to zero.
    ///
    /// # ISDA Standard Par Spread Definition
    ///
    /// By default (when `par_spread_uses_full_premium = false`), this implements
    /// the **ISDA standard definition**:
    ///
    /// ```text
    /// Par Spread = Protection_PV / Risky_Annuity
    /// ```
    ///
    /// where `Risky_Annuity` is the sum of `DF(t) * SP(t) * YearFrac` across
    /// coupon periods, **excluding** accrual-on-default from the denominator.
    ///
    /// # Par Spread with Full Premium Leg
    ///
    /// When `par_spread_uses_full_premium = true`, the denominator includes the
    /// full premium leg PV (with accrual-on-default) per basis point. This matches
    /// Bloomberg CDSW-style calculations and is appropriate for distressed credits
    /// where accrual-on-default has material impact (typically 2-5 bps for hazard > 3%).
    ///
    /// # Returns
    ///
    /// Par spread in basis points (bps).
    #[must_use = "par spread calculation is pure computation"]
    pub(crate) fn par_spread(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;

        // Default behavior (par_spread_uses_full_premium = false) uses Risky Annuity only.
        // This excludes accrual-on-default from the denominator per ISDA convention.
        let denom = if self.config.par_spread_uses_full_premium {
            // Opt-in: Compute full premium PV per 1bp including AoD
            let periods = self.coupon_periods(cds, as_of)?;
            let calendar = cds
                .premium
                .calendar_id
                .as_deref()
                .and_then(finstack_core::dates::calendar::calendar_by_id);
            let mut ann = 0.0;
            for period in periods {
                let start_date = period.accrual_start;
                let end_date = period.accrual_end;
                let payment_date = period.payment_date;

                // Skip periods that have already ended before as_of
                if end_date <= as_of {
                    continue;
                }

                // Accrual uses instrument day-count
                let accrual = self.coupon_accrual(cds, &period)?;

                // Discounting uses discount curve's day-count and relative DF from as_of
                let df = df_asof_to(disc, as_of, payment_date)?;

                // Survival uses hazard curve's day-count and conditional probability
                let sp = sp_cond_to(surv, as_of, end_date)?;

                let unit_spread = 1.0;
                // coupon part per unit spread
                ann += unit_spread * accrual * sp * df;

                // AoD part per unit spread in this period.
                let aod_accrual_start = if cds.uses_clean_price() {
                    start_date.max(as_of)
                } else {
                    start_date
                };
                ann += self.accrual_on_default_dispatch(AodInputs {
                    cds,
                    spread: unit_spread,
                    accrual_start_date: aod_accrual_start,
                    start_date: start_date.max(as_of),
                    end_date,
                    settlement_delay: cds.protection.settlement_delay,
                    calendar,
                    as_of,
                    disc,
                    surv,
                })?;
            }
            ann
        } else if cds.uses_clean_price() {
            self.clean_par_spread_denominator(cds, disc, surv, as_of)?
        } else {
            // ISDA Standard: Risky Annuity (sum of DF * SP * YearFrac)
            self.risky_annuity(cds, disc, surv, as_of)?
        };

        if denom.abs() < numerical::RATE_COMPARISON_TOLERANCE {
            return Err(Error::Validation(
                "Par spread denominator is too small (risky annuity ≈ 0). \
                 This may indicate zero survival probability or expired CDS."
                    .to_string(),
            ));
        }

        // Result in Basis Points
        Ok(protection_pv.amount() / (denom * cds.notional.amount()) * BASIS_POINTS_PER_UNIT)
    }

    /// Premium leg PV per 1 bp of spread, including accrual-on-default if configured.
    ///
    /// Uses proper time-axis conventions for discounting and survival.
    #[must_use = "premium leg calculation is pure computation"]
    pub(crate) fn premium_leg_pv_per_bp(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let periods = self.coupon_periods(cds, as_of)?;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);
        let mut per_bp_pv = 0.0;
        for period in periods {
            let start_date = period.accrual_start;
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = self.coupon_accrual(cds, &period)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            per_bp_pv += ONE_BASIS_POINT * accrual * sp * df;

            if self.config.include_accrual {
                let aod_accrual_start = if cds.uses_clean_price() {
                    start_date.max(as_of)
                } else {
                    start_date
                };
                per_bp_pv += self.accrual_on_default_dispatch(AodInputs {
                    cds,
                    spread: ONE_BASIS_POINT,
                    accrual_start_date: aod_accrual_start,
                    start_date: start_date.max(as_of),
                    end_date,
                    settlement_delay: cds.protection.settlement_delay,
                    calendar,
                    as_of,
                    disc,
                    surv,
                })?;
            }
        }
        Ok(per_bp_pv)
    }

    /// Forward premium leg PV per 1 bp of spread.
    ///
    /// For a forward-starting CDS, scheduled premium cashflows use the standard
    /// CDS coupon schedule, including the full first coupon accrual when the
    /// forward start falls inside a coupon period. Accrual-on-default, however,
    /// only starts once forward protection is live; defaults before
    /// `forward_start` cancel the forward CDS rather than accruing premium.
    #[must_use = "premium leg calculation is pure computation"]
    pub(crate) fn forward_premium_leg_pv_per_bp(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
        forward_start: Date,
    ) -> Result<f64> {
        let periods = self.coupon_periods(cds, as_of)?;
        let calendar = cds
            .premium
            .calendar_id
            .as_deref()
            .and_then(finstack_core::dates::calendar::calendar_by_id);
        let mut per_bp_pv = 0.0;
        for period in periods {
            let start_date = period.accrual_start;
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            if end_date <= as_of || end_date <= forward_start {
                continue;
            }

            let accrual = self.coupon_accrual(cds, &period)?;
            let df = df_asof_to(disc, as_of, payment_date)?;
            let sp = sp_cond_to(surv, as_of, end_date)?;
            per_bp_pv += ONE_BASIS_POINT * accrual * sp * df;

            if self.config.include_accrual {
                let aod_accrual_start = if cds.uses_clean_price() {
                    start_date.max(as_of).max(forward_start)
                } else {
                    start_date.max(forward_start)
                };
                per_bp_pv += self.accrual_on_default_dispatch(AodInputs {
                    cds,
                    spread: ONE_BASIS_POINT,
                    accrual_start_date: aod_accrual_start,
                    start_date: start_date.max(as_of).max(forward_start),
                    end_date,
                    settlement_delay: cds.protection.settlement_delay,
                    calendar,
                    as_of,
                    disc,
                    surv,
                })?;
            }
        }
        Ok(per_bp_pv)
    }

    /// Risky annuity: survival-weighted duration of premium payments.
    ///
    /// This is the sum of `DF(t) × SP(t) × YearFrac` across all coupon periods.
    /// Used as the denominator in ISDA standard par spread calculation.
    ///
    /// Uses proper time-axis conventions for discounting and survival.
    #[must_use = "risky annuity calculation is pure computation"]
    pub(crate) fn risky_annuity(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let periods = self.coupon_periods(cds, as_of)?;
        let mut annuity = 0.0;
        for period in periods {
            let end_date = period.accrual_end;
            let payment_date = period.payment_date;

            // Skip periods that have already ended before as_of
            if end_date <= as_of {
                continue;
            }

            // Accrual uses instrument day-count
            let accrual = self.coupon_accrual(cds, &period)?;

            // Discounting uses discount curve's day-count and relative DF from as_of
            let df = df_asof_to(disc, as_of, payment_date)?;

            // Survival uses hazard curve's day-count and conditional probability
            let sp = sp_cond_to(surv, as_of, end_date)?;

            annuity += accrual * sp * df;
        }
        Ok(annuity)
    }

    /// Risky PV01: change in NPV for a 1bp increase in the CDS spread.
    ///
    /// Computed as `Risky Annuity × Notional / 10000`.
    #[must_use = "risky PV01 calculation is pure computation"]
    pub(crate) fn risky_pv01(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<f64> {
        let risky_annuity = self.risky_annuity(cds, disc, surv, as_of)?;
        Ok(risky_annuity * cds.notional.amount() / BASIS_POINTS_PER_UNIT)
    }

    /// Instrument NPV from the perspective of the `PayReceive` side.
    ///
    /// - **Protection buyer** (PayFixed): NPV = Protection PV − Premium PV
    /// - **Protection seller** (ReceiveFixed): NPV = Premium PV − Protection PV
    pub(crate) fn npv(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let protection_pv = self.pv_protection_leg(cds, disc, surv, as_of)?;
        let premium_pv = self.pv_premium_leg(cds, disc, surv, as_of)?;
        match cds.side {
            PayReceive::PayFixed => protection_pv.checked_sub(premium_pv),
            PayReceive::ReceiveFixed => premium_pv.checked_sub(protection_pv),
        }
    }

    /// Instrument NPV including both types of upfront payments.
    ///
    /// This method applies two types of upfront payments (if present):
    ///
    /// 1. **Dated cashflow** (`cds.upfront: Option<(Date, Money)>`):
    ///    A specific payment on a specific date, discounted from `as_of`.
    ///    - Positive amount = payment by buyer, negative = receipt by buyer
    ///    - Applied with sign convention based on trade side
    ///
    /// 2. **PV adjustment** (`cds.pricing_overrides.market_quotes.upfront_payment: Option<Money>`):
    ///    An already-discounted adjustment to the PV at `as_of`.
    ///    - Applied without further discounting
    ///    - Positive = paid by buyer (reduces buyer NPV, increases seller NPV)
    ///    - Sign convention matches the dated upfront and `Instrument::value()`
    ///
    /// Both can be set simultaneously without double-counting.
    #[cfg(test)]
    pub(crate) fn npv_with_upfront(
        &self,
        cds: &CreditDefaultSwap,
        disc: &DiscountCurve,
        surv: &HazardCurve,
        as_of: Date,
    ) -> Result<Money> {
        let mut pv = self.npv(cds, disc, surv, as_of)?;

        // 1. Handle dated cashflow upfront (discounted and signed)
        if let Some((dt, amount)) = cds.upfront {
            if dt >= as_of {
                let df = df_asof_to(disc, as_of, dt)?;
                let upfront_pv = Money::new(amount.amount() * df, cds.notional.currency());
                // Sign convention: positive upfront is paid by buyer
                pv = match cds.side {
                    PayReceive::PayFixed => pv.checked_sub(upfront_pv)?,
                    PayReceive::ReceiveFixed => pv.checked_add(upfront_pv)?,
                };
            }
        }

        // 2. Handle PV adjustment upfront (signed per side, no further discounting).
        //    Positive = paid by protection buyer, matching the dated upfront convention
        //    and Instrument::value() semantics.
        if let Some(upfront) = cds.pricing_overrides.market_quotes.upfront_payment {
            pv = match cds.side {
                PayReceive::PayFixed => pv.checked_sub(upfront)?,
                PayReceive::ReceiveFixed => pv.checked_add(upfront)?,
            };
        }

        Ok(pv)
    }

    /// Resolve curves from MarketContext and compute NPV with upfront.
    #[cfg(test)]
    pub(crate) fn npv_market(
        &self,
        cds: &CreditDefaultSwap,
        curves: &MarketContext,
        as_of: Date,
    ) -> Result<Money> {
        let disc = curves.get_discount(&cds.premium.discount_curve_id)?;
        let surv = curves.get_hazard(&cds.protection.credit_curve_id)?;
        self.npv_with_upfront(cds, disc.as_ref(), surv.as_ref(), as_of)
    }
}
