//! Real estate asset valuation types and logic.

use crate::impl_instrument_base;
use crate::instruments::common_impl::traits::{
    Attributes, CurveDependencies, Instrument, InstrumentCurves,
};
use crate::pricer::InstrumentType;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, DayCountCtx};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_core::Error as CoreError;

/// Broad property classification for reporting / tagging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealEstatePropertyType {
    /// Office (CBD/suburban, single-tenant or multi-tenant).
    Office,
    /// Multifamily / residential rental.
    Multifamily,
    /// Retail (strip, mall, big box, high street).
    Retail,
    /// Industrial (warehouse, logistics, manufacturing).
    Industrial,
    /// Hospitality (hotel, resort).
    Hospitality,
    /// Mixed-use (multiple property types).
    MixedUse,
    /// Other / uncategorized.
    Other,
}

/// Valuation method for a real estate asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealEstateValuationMethod {
    /// Discounted cashflow using an explicit NOI schedule and discount rate.
    Dcf,
    /// Direct capitalization using a stabilized NOI and cap rate.
    DirectCap,
}

/// Real estate asset valuation instrument.
///
/// Supports DCF (explicit NOI schedule) and direct capitalization valuation.
#[derive(
    Clone, Debug, finstack_valuations_macros::FinancialBuilder, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields)]
pub struct RealEstateAsset {
    /// Unique instrument identifier.
    pub id: InstrumentId,
    /// Currency for valuation.
    pub currency: Currency,
    /// Valuation date (base date for discounting).
    pub valuation_date: Date,
    /// Valuation method (DCF or DirectCap).
    pub valuation_method: RealEstateValuationMethod,
    /// Optional property type classification (for reporting).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_type: Option<RealEstatePropertyType>,
    /// Net operating income schedule (date, amount).
    pub noi_schedule: Vec<(Date, f64)>,
    /// Capital expenditure schedule (date, amount). Values are treated as **positive outflows**.
    ///
    /// When present, cashflows are valued as `NOI - CapEx` (unlevered net cash flow).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capex_schedule: Option<Vec<(Date, f64)>>,
    /// Discount rate for DCF (annualized).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<f64>,
    /// Capitalization rate for direct cap (annualized).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_rate: Option<f64>,
    /// Optional stabilized NOI override for direct cap.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_noi: Option<f64>,
    /// Optional terminal cap rate for DCF (uses last NOI).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_cap_rate: Option<f64>,
    /// Optional terminal growth rate used to project `NOI_{N+1}` for exit valuation.
    ///
    /// Market convention for exit-cap terminal value is \(TV = NOI_{N+1} / cap\_rate\_exit\).
    /// When not provided, defaults to 0 (uses last NOI as-is).
    /// Validation range is \([-100\%, 20\%]\) to guard against configuration errors.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_growth_rate: Option<f64>,
    /// Optional sale/exit date that truncates the DCF horizon.
    ///
    /// When set, DCF only values unlevered flows up to and including `sale_date`.
    /// Terminal proceeds (if configured) are realized on `sale_date`.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sale_date: Option<Date>,
    /// Optional explicit gross sale price (terminal proceeds), before disposition costs.
    ///
    /// When set, this takes precedence over `terminal_cap_rate` for terminal proceeds.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sale_price: Option<Money>,
    /// Optional purchase price (useful for IRR / cap rate metrics).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purchase_price: Option<Money>,
    /// Optional one-time acquisition cost deducted at `as_of` in DCF valuation.
    ///
    /// This is intended for closing costs, fees, and other transaction costs.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_cost: Option<f64>,
    /// Optional detailed acquisition cost line items (positive outflows) deducted at `as_of`.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquisition_costs: Vec<Money>,
    /// Optional disposition cost percentage applied to terminal value.
    ///
    /// A value of `0.02` represents 2% selling costs. Must be in \([0, 1)\).
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition_cost_pct: Option<f64>,
    /// Optional detailed disposition cost line items (positive outflows) deducted from terminal proceeds.
    #[builder(default)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disposition_costs: Vec<Money>,
    /// Optional appraisal override value.
    #[builder(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appraisal_value: Option<Money>,
    /// Day count convention for year fractions.
    pub day_count: DayCount,
    /// Discount curve identifier (for risk attribution).
    pub discount_curve_id: CurveId,
    /// Attributes for tagging and scenarios.
    #[builder(default)]
    #[serde(default)]
    #[builder(default)]
    pub pricing_overrides: crate::instruments::PricingOverrides,
    /// Attributes for scenario selection and tagging
    pub attributes: Attributes,
}

impl RealEstateAsset {
    /// DCF valuation using **annual compounding** per real estate appraisal
    /// standards (RICS Red Book / USPAP).
    ///
    /// Real estate industry convention uses discrete annual discounting:
    /// ```text
    /// PV = NOI / (1 + r)^t
    /// ```
    /// rather than the continuous compounding (`exp(-r*t)`) used by capital
    /// markets instruments elsewhere in this library.  This is deliberate
    /// and aligns with how discount rates are quoted in property appraisals.
    fn npv_dcf(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<f64> {
        // Prefer a term-structure discount curve when available so DV01/theta behave
        // consistently with the rest of the library. Fall back to a flat appraisal-style
        // discount_rate when the curve is absent.
        let discount_curve = market.get_discount(&self.discount_curve_id).ok();

        let discount_rate = if discount_curve.is_none() {
            let r = self
                .discount_rate
                .ok_or_else(|| CoreError::Validation("Missing discount_rate for DCF".into()))?;
            if r <= -1.0 {
                return Err(CoreError::Validation(
                    "discount_rate must be greater than -100%".into(),
                ));
            }
            Some(r)
        } else {
            None
        };

        // Determine horizon date for DCF: sale_date (if set) else last NOI date.
        let horizon = if let Some(d) = self.sale_date {
            if d < as_of {
                return Err(CoreError::Validation(
                    "sale_date must be on/after as_of".into(),
                ));
            }
            d
        } else {
            self.last_noi(as_of)?.0
        };

        // Value unlevered net cash flows: NOI - CapEx (CapEx treated as positive outflow).
        let flows = self
            .future_unlevered_flows(as_of)?
            .into_iter()
            .filter(|(d, _)| *d <= horizon)
            .collect::<Vec<_>>();

        // Allow terminal-only valuation (e.g., explicit sale_price at/near as_of).
        // If there are no cashflows on/before the horizon date, we still allow pricing
        // as long as terminal proceeds are configured.
        let terminal_at_horizon = self.sale_proceeds_at(as_of, horizon)?;
        if flows.is_empty() && terminal_at_horizon.is_none() {
            return Err(CoreError::Validation(
                "No cashflows on/before horizon date and no terminal proceeds configured".into(),
            ));
        }

        // Optional transaction cost at time 0.
        let pv_acq_cost = self.acquisition_cost_total()?;

        let pv_flows: f64 = flows
            .iter()
            .map(|(date, amount)| {
                let t = self.year_fraction(as_of, *date)?;
                if let Some(curve) = &discount_curve {
                    Ok(amount * curve.df(t))
                } else if let Some(r) = discount_rate {
                    Ok(amount / (1.0 + r).powf(t))
                } else {
                    unreachable!("discount_curve and discount_rate cannot both be None");
                }
            })
            .collect::<finstack_core::Result<Vec<f64>>>()?
            .into_iter()
            .sum();

        let pv_terminal = match terminal_at_horizon {
            Some((date, amount)) => {
                let t = self.year_fraction(as_of, date)?;
                if let Some(curve) = &discount_curve {
                    amount * curve.df(t)
                } else if let Some(r) = discount_rate {
                    amount / (1.0 + r).powf(t)
                } else {
                    unreachable!("discount_curve and discount_rate cannot both be None");
                }
            }
            None => 0.0,
        };

        Ok(pv_flows + pv_terminal - pv_acq_cost)
    }

    fn npv_direct_cap(&self, as_of: Date) -> finstack_core::Result<f64> {
        let cap_rate = self
            .cap_rate
            .ok_or_else(|| CoreError::Validation("Missing cap_rate for direct cap".into()))?;
        if cap_rate <= 0.0 {
            return Err(CoreError::Validation("cap_rate must be positive".into()));
        }

        let noi = if let Some(noi) = self.stabilized_noi {
            noi
        } else {
            let flows = self.future_noi_flows(as_of)?;
            flows
                .first()
                .map(|(_, amount)| *amount)
                .ok_or_else(|| CoreError::Validation("NOI schedule is empty".into()))?
        };

        Ok(noi / cap_rate)
    }

    fn future_noi_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        let mut flows: Vec<(Date, f64)> = self
            .noi_schedule
            .iter()
            .copied()
            .filter(|(date, _)| *date >= as_of)
            .collect();
        if flows.is_empty() {
            return Err(CoreError::Validation(
                "NOI schedule must include at least one flow on/after as_of".into(),
            ));
        }
        flows.sort_by_key(|(date, _)| *date);
        Ok(flows)
    }

    fn future_unlevered_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        let mut noi = self.future_noi_flows(as_of)?;
        let mut capex: Vec<(Date, f64)> = self
            .capex_schedule
            .as_ref()
            .map(|v| {
                v.iter()
                    .copied()
                    .filter(|(d, _)| *d >= as_of)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        capex.sort_by_key(|(d, _)| *d);

        // Merge NOI and CapEx by date.
        // Convention: CapEx amounts are positive outflows and reduce net cashflow.
        noi.extend(capex.into_iter().map(|(d, a)| (d, -a)));
        noi.sort_by_key(|(d, _)| *d);

        // Coalesce same-date entries.
        let mut merged: Vec<(Date, f64)> = Vec::with_capacity(noi.len());
        for (d, a) in noi {
            if let Some((last_d, last_a)) = merged.last_mut() {
                if *last_d == d {
                    *last_a += a;
                    continue;
                }
            }
            merged.push((d, a));
        }
        Ok(merged)
    }

    pub(crate) fn acquisition_cost_total(&self) -> finstack_core::Result<f64> {
        let mut total = self.acquisition_cost.unwrap_or(0.0);
        for m in &self.acquisition_costs {
            if m.currency() != self.currency {
                return Err(CoreError::Validation(
                    "acquisition_costs currency must match instrument currency".into(),
                ));
            }
            total += m.amount();
        }
        Ok(total)
    }

    pub(crate) fn disposition_cost_total(&self) -> finstack_core::Result<f64> {
        let mut total = 0.0;
        for m in &self.disposition_costs {
            if m.currency() != self.currency {
                return Err(CoreError::Validation(
                    "disposition_costs currency must match instrument currency".into(),
                ));
            }
            total += m.amount();
        }
        Ok(total)
    }

    /// Compute net sale proceeds (undiscounted) realized at `exit_date`, if configured.
    ///
    /// Precedence:
    /// - If `sale_price` is set: use it as the gross proceeds.
    /// - Else if `terminal_cap_rate` is set: use exit-cap convention `TV = NOI_{N+1} / cap_rate_exit`
    ///   with NOI taken as the last schedule entry on/before `exit_date`.
    ///
    /// Then apply:
    /// - `disposition_cost_pct` (pct of gross proceeds), and
    /// - `disposition_costs` (dollar line items).
    pub(crate) fn sale_proceeds_at(
        &self,
        as_of: Date,
        exit_date: Date,
    ) -> finstack_core::Result<Option<(Date, f64)>> {
        if exit_date < as_of {
            return Err(CoreError::Validation(
                "exit_date must be on/after as_of".into(),
            ));
        }

        let gross = if let Some(px) = self.sale_price {
            if px.currency() != self.currency {
                return Err(CoreError::Validation(
                    "sale_price currency must match instrument currency".into(),
                ));
            }
            px.amount()
        } else if let Some(cap_rate) = self.terminal_cap_rate {
            if cap_rate <= 0.0 {
                return Err(CoreError::Validation(
                    "terminal_cap_rate must be positive".into(),
                ));
            }
            let terminal_noi_n = self
                .future_noi_flows(as_of)?
                .iter()
                .copied()
                .filter(|(d, _)| *d <= exit_date)
                .next_back()
                .map(|(_, a)| a)
                .ok_or_else(|| {
                    CoreError::Validation("No NOI on/before exit_date for terminal value".into())
                })?;
            let g = self.terminal_growth_rate.unwrap_or(0.0);
            if !(-1.0..=0.20).contains(&g) {
                return Err(CoreError::Validation(format!(
                    "terminal_growth_rate must be in [-100%, 20%], got {g}"
                )));
            }
            let terminal_noi_n1 = terminal_noi_n * (1.0 + g);
            terminal_noi_n1 / cap_rate
        } else {
            return Ok(None);
        };

        let mut net = gross;
        if let Some(pct) = self.disposition_cost_pct {
            if !(0.0..1.0).contains(&pct) {
                return Err(CoreError::Validation(
                    "disposition_cost_pct must be in [0, 1)".into(),
                ));
            }
            net *= 1.0 - pct;
        }
        net -= self.disposition_cost_total()?;

        Ok(Some((exit_date, net)))
    }

    fn year_fraction(&self, start: Date, end: Date) -> finstack_core::Result<f64> {
        self.day_count
            .year_fraction(start, end, DayCountCtx::default())
    }

    /// First future NOI amount on/after `as_of`.
    pub(crate) fn first_noi(&self, as_of: Date) -> finstack_core::Result<f64> {
        self.future_noi_flows(as_of)?
            .first()
            .map(|(_, a)| *a)
            .ok_or_else(|| CoreError::Validation("NOI schedule is empty".into()))
    }

    /// Last future NOI `(date, amount)` on/after `as_of`.
    pub(crate) fn last_noi(&self, as_of: Date) -> finstack_core::Result<(Date, f64)> {
        self.future_noi_flows(as_of)?
            .last()
            .copied()
            .ok_or_else(|| CoreError::Validation("NOI schedule is empty".into()))
    }

    /// Unlevered net cash flows (NOI - CapEx) on/after `as_of`.
    pub(crate) fn unlevered_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        self.future_unlevered_flows(as_of)
    }

    /// NOI cash flows on/after `as_of`.
    pub(crate) fn noi_flows(&self, as_of: Date) -> finstack_core::Result<Vec<(Date, f64)>> {
        self.future_noi_flows(as_of)
    }

    /// Compute net sale proceeds at the terminal date (undiscounted), if configured.
    ///
    /// Uses the exit-cap convention `TV = NOI_{N+1} / cap_rate_exit`, optionally applying
    /// `disposition_cost_pct`.
    pub(crate) fn terminal_sale_proceeds(
        &self,
        as_of: Date,
    ) -> finstack_core::Result<Option<(Date, f64)>> {
        let terminal_date = self.sale_date.unwrap_or(self.last_noi(as_of)?.0);
        self.sale_proceeds_at(as_of, terminal_date)
    }
}

impl Instrument for RealEstateAsset {
    impl_instrument_base!(InstrumentType::RealEstateAsset);

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        if let Some(appraisal) = &self.appraisal_value {
            if appraisal.currency() != self.currency {
                return Err(CoreError::Validation(format!(
                    "Appraisal currency {} does not match instrument currency {}",
                    appraisal.currency(),
                    self.currency
                )));
            }
            return Ok(*appraisal);
        }

        let value = match self.valuation_method {
            RealEstateValuationMethod::Dcf => self.npv_dcf(market, as_of)?,
            RealEstateValuationMethod::DirectCap => self.npv_direct_cap(as_of)?,
        };

        Ok(finstack_core::money::Money::new(value, self.currency))
    }

    fn effective_start_date(&self) -> Option<Date> {
        None
    }

    fn scenario_overrides_mut(
        &mut self,
    ) -> Option<&mut crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&mut self.pricing_overrides)
    }

    fn scenario_overrides(
        &self,
    ) -> Option<&crate::instruments::pricing_overrides::PricingOverrides> {
        Some(&self.pricing_overrides)
    }
}
impl CurveDependencies for RealEstateAsset {
    fn curve_dependencies(&self) -> finstack_core::Result<InstrumentCurves> {
        InstrumentCurves::builder()
            .discount(self.discount_curve_id.clone())
            .build()
    }
}
