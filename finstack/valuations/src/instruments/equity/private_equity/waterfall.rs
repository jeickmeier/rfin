//! Equity waterfall engine for private equity distribution calculations.
//!
//! This module implements the core waterfall allocation logic used in private
//! equity funds, including return of capital, preferred IRR hurdles, catch-up
//! provisions, promote splits, and clawback mechanisms.

use finstack_core::config::{results_meta, FinstackConfig, ResultsMeta};
use finstack_core::dates::{Date, DayCount};
use finstack_core::math::root_finding::brent;
use finstack_core::money::Money;
use finstack_core::prelude::*;
use finstack_core::F;
use indexmap::IndexMap;
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Waterfall allocation style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum WaterfallStyle {
    /// European style: aggregate all events at fund level
    European,
    /// American style: allocate per deal, then aggregate
    American,
}

impl Default for WaterfallStyle {
    fn default() -> Self {
        Self::European
    }
}

/// Catch-up mode for GP profit sharing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CatchUpMode {
    /// Full catch-up: GP gets 100% until target split is reached
    Full,
    /// Partial catch-up: GP gets configured percentage
    Partial,
}

impl Default for CatchUpMode {
    fn default() -> Self {
        Self::Full
    }
}

/// Hurdle types for waterfall tiers.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Hurdle {
    /// IRR-based hurdle (annual rate)
    Irr { rate: F },
    // Future: Moic { multiple: F } - can be added without breaking serde
}

/// Individual tranche in the waterfall.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Tranche {
    /// Return LP capital contributions before any profit sharing
    ReturnOfCapital,
    /// Preferred return to LPs at specified IRR
    PreferredIrr { irr: F },
    /// Catch-up allocation to GP
    CatchUp { gp_share: F },
    /// Promote tier with hurdle and LP/GP split
    PromoteTier {
        hurdle: Hurdle,
        lp_share: F,
        gp_share: F,
    },
}

/// Clawback settlement trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ClawbackSettle {
    /// Settle at fund termination
    FundEnd,
    /// Settle periodically (quarterly/annually)
    Periodic,
}

/// Clawback specification for GP carry reconciliation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ClawbackSpec {
    /// Whether clawback is enabled
    pub enable: bool,
    /// Optional percentage of GP carry held back until settlement
    pub holdback_pct: Option<F>,
    /// When to settle clawback
    pub settle_on: ClawbackSettle,
}

impl Default for ClawbackSpec {
    fn default() -> Self {
        Self {
            enable: false,
            holdback_pct: None,
            settle_on: ClawbackSettle::FundEnd,
        }
    }
}

/// Complete waterfall specification.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WaterfallSpec {
    /// Allocation style (European vs American)
    pub style: WaterfallStyle,
    /// Ordered sequence of waterfall tranches
    pub tranches: Vec<Tranche>,
    /// Optional clawback specification
    #[serde(default)]
    pub clawback: Option<ClawbackSpec>,
    /// Day count basis for IRR calculations
    #[serde(default = "default_irr_basis")]
    pub irr_basis: DayCount,
    /// Catch-up mode
    #[serde(default)]
    pub catchup_mode: CatchUpMode,
}

fn default_irr_basis() -> DayCount {
    DayCount::Act365F
}

impl WaterfallSpec {
    /// Create a new waterfall specification builder.
    pub fn builder() -> WaterfallSpecBuilder {
        WaterfallSpecBuilder::new()
    }

    /// Validate the waterfall specification.
    pub fn validate(&self) -> finstack_core::Result<()> {
        if self.tranches.is_empty() {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }

        // Validate promote tier splits sum to 1.0
        for tranche in &self.tranches {
            if let Tranche::PromoteTier {
                lp_share, gp_share, ..
            } = tranche
            {
                let sum = lp_share + gp_share;
                if (sum - 1.0).abs() > 1e-6 {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
                if !lp_share.is_finite() || !gp_share.is_finite() {
                    return Err(finstack_core::error::InputError::Invalid.into());
                }
                if *lp_share < 0.0 || *gp_share < 0.0 {
                    return Err(finstack_core::error::InputError::NegativeValue.into());
                }
            }
        }

        Ok(())
    }
}

/// Builder for waterfall specifications.
pub struct WaterfallSpecBuilder {
    style: WaterfallStyle,
    tranches: Vec<Tranche>,
    clawback: Option<ClawbackSpec>,
    irr_basis: DayCount,
    catchup_mode: CatchUpMode,
}

impl Default for WaterfallSpecBuilder {
    fn default() -> Self {
        Self {
            style: WaterfallStyle::default(),
            tranches: Vec::new(),
            clawback: None,
            irr_basis: default_irr_basis(),
            catchup_mode: CatchUpMode::default(),
        }
    }
}

impl WaterfallSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: WaterfallStyle) -> Self {
        self.style = style;
        self
    }

    pub fn irr_basis(mut self, basis: DayCount) -> Self {
        self.irr_basis = basis;
        self
    }

    pub fn catchup_mode(mut self, mode: CatchUpMode) -> Self {
        self.catchup_mode = mode;
        self
    }

    pub fn return_of_capital(mut self) -> Self {
        self.tranches.push(Tranche::ReturnOfCapital);
        self
    }

    pub fn preferred_irr(mut self, irr: F) -> Self {
        self.tranches.push(Tranche::PreferredIrr { irr });
        self
    }

    pub fn catchup(mut self, gp_share: F) -> Self {
        self.tranches.push(Tranche::CatchUp { gp_share });
        self
    }

    pub fn promote_tier(mut self, hurdle_irr: F, lp_share: F, gp_share: F) -> Self {
        self.tranches.push(Tranche::PromoteTier {
            hurdle: Hurdle::Irr { rate: hurdle_irr },
            lp_share,
            gp_share,
        });
        self
    }

    pub fn clawback(mut self, spec: ClawbackSpec) -> Self {
        self.clawback = Some(spec);
        self
    }

    pub fn build(self) -> finstack_core::Result<WaterfallSpec> {
        let spec = WaterfallSpec {
            style: self.style,
            tranches: self.tranches,
            clawback: self.clawback,
            irr_basis: self.irr_basis,
            catchup_mode: self.catchup_mode,
        };
        spec.validate()?;
        Ok(spec)
    }
}

/// Type of fund event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FundEventKind {
    /// Capital contribution from LP
    Contribution,
    /// Distribution to LP
    Distribution,
    /// Sale proceeds from investment
    Proceeds,
}

/// Single fund cash flow event.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct FundEvent {
    /// Date of the event
    pub date: Date,
    /// Amount (positive for all event types, sign determined by kind)
    pub amount: Money,
    /// Type of event
    pub kind: FundEventKind,
    /// Deal identifier for American-style waterfalls
    pub deal_id: Option<String>,
}

impl FundEvent {
    /// Create a capital contribution event.
    pub fn contribution(date: Date, amount: Money) -> Self {
        Self {
            date,
            amount,
            kind: FundEventKind::Contribution,
            deal_id: None,
        }
    }

    /// Create a distribution event.
    pub fn distribution(date: Date, amount: Money) -> Self {
        Self {
            date,
            amount,
            kind: FundEventKind::Distribution,
            deal_id: None,
        }
    }

    /// Create a proceeds event with deal ID.
    pub fn proceeds(date: Date, amount: Money, deal_id: impl Into<String>) -> Self {
        Self {
            date,
            amount,
            kind: FundEventKind::Proceeds,
            deal_id: Some(deal_id.into()),
        }
    }

    /// Set the deal ID for American-style waterfalls.
    pub fn with_deal_id(mut self, deal_id: impl Into<String>) -> Self {
        self.deal_id = Some(deal_id.into());
        self
    }

    /// Get the signed amount for IRR calculations.
    /// Contributions are negative (outflows), distributions/proceeds are positive (inflows).
    pub fn signed_amount(&self) -> Money {
        match self.kind {
            FundEventKind::Contribution => {
                Money::new(-self.amount.amount(), self.amount.currency())
            }
            FundEventKind::Distribution | FundEventKind::Proceeds => self.amount,
        }
    }
}

/// Single row in the allocation ledger.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AllocationRow {
    /// Date of allocation
    pub date: Date,
    /// Period key (for grouping)
    pub period_key: Option<String>,
    /// Deal ID for American-style waterfalls
    pub deal_id: Option<String>,
    /// Tranche name/description
    pub tranche: String,
    /// Amount allocated to LP
    pub to_lp: Money,
    /// Amount allocated to GP
    pub to_gp: Money,
    /// LP unreturned capital balance after allocation
    pub lp_unreturned: Money,
    /// GP cumulative carry after allocation
    pub gp_carry_cum: Money,
    /// LP IRR to date (if calculable)
    pub lp_irr_to_date: Option<F>,
    /// Optional note/description
    pub note: Option<String>,
}

/// Complete allocation ledger with metadata.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct AllocationLedger {
    /// Allocation rows
    pub rows: Vec<AllocationRow>,
    /// Result metadata
    pub meta: ResultsMeta,
}

impl AllocationLedger {
    /// Extract LP-only cashflows for NPV calculation.
    pub fn lp_cashflows(&self) -> Vec<(Date, Money)> {
        let mut flows = Vec::new();
        let mut lp_flows_by_date: IndexMap<Date, Money> = IndexMap::new();

        for row in &self.rows {
            let existing = lp_flows_by_date
                .get(&row.date)
                .copied()
                .unwrap_or_else(|| Money::new(0.0, row.to_lp.currency()));

            // Add LP allocation (may be positive or negative)
            if let Ok(new_amount) = existing + row.to_lp {
                lp_flows_by_date.insert(row.date, new_amount);
            }
        }

        for (date, amount) in lp_flows_by_date {
            if amount.amount().abs() > 1e-6 {
                // Only include non-zero flows
                flows.push((date, amount));
            }
        }

        flows
    }

    /// Export allocation ledger as structured data for DataFrame creation.
    /// Returns column names and data vectors suitable for external DataFrame libraries.
    pub fn to_tabular_data(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let column_names = vec![
            "date",
            "period_key",
            "deal_id",
            "tranche",
            "to_lp",
            "to_gp",
            "lp_unreturned",
            "gp_carry_cum",
            "lp_irr_to_date",
            "note",
        ];

        let mut rows = Vec::new();
        for row in &self.rows {
            let mut row_data = Vec::new();
            row_data.push(row.date.to_string());
            row_data.push(row.period_key.as_deref().unwrap_or("").to_string());
            row_data.push(row.deal_id.as_deref().unwrap_or("").to_string());
            row_data.push(row.tranche.clone());
            row_data.push(row.to_lp.amount().to_string());
            row_data.push(row.to_gp.amount().to_string());
            row_data.push(row.lp_unreturned.amount().to_string());
            row_data.push(row.gp_carry_cum.amount().to_string());
            row_data.push(
                row.lp_irr_to_date
                    .map_or("".to_string(), |irr| format!("{:.6}", irr)),
            );
            row_data.push(row.note.as_deref().unwrap_or("").to_string());
            rows.push(row_data);
        }

        (column_names, rows)
    }

    /// Export as JSON string for external analysis.
    pub fn to_json(&self) -> finstack_core::Result<String> {
        serde_json::to_string_pretty(self).map_err(|_| finstack_core::Error::Internal)
    }
}

/// Equity waterfall calculation engine.
pub struct EquityWaterfallEngine<'a> {
    spec: &'a WaterfallSpec,
}

impl<'a> EquityWaterfallEngine<'a> {
    /// Create a new waterfall engine.
    pub fn new(spec: &'a WaterfallSpec) -> Self {
        Self { spec }
    }

    /// Run the waterfall allocation on the given events.
    pub fn run(&self, events: &[FundEvent]) -> finstack_core::Result<AllocationLedger> {
        // Validate and sort events
        let mut sorted_events = events.to_vec();
        sorted_events.sort_by(|a, b| a.date.cmp(&b.date));

        let mut ledger_rows = Vec::new();

        match self.spec.style {
            WaterfallStyle::European => {
                self.run_european(&sorted_events, &mut ledger_rows)?;
            }
            WaterfallStyle::American => {
                self.run_american(&sorted_events, &mut ledger_rows)?;
            }
        }

        // Apply clawback if specified
        if let Some(clawback_spec) = &self.spec.clawback {
            if clawback_spec.enable {
                self.apply_clawback(&sorted_events, &mut ledger_rows, clawback_spec)?;
            }
        }

        let config = FinstackConfig::default();
        let meta = results_meta(&config);

        Ok(AllocationLedger {
            rows: ledger_rows,
            meta,
        })
    }

    /// Run European-style waterfall (fund-level aggregation).
    fn run_european(
        &self,
        events: &[FundEvent],
        ledger_rows: &mut Vec<AllocationRow>,
    ) -> finstack_core::Result<()> {
        // Aggregate all events at fund level
        let mut lp_unreturned = 0.0;
        let mut gp_carry_cum = 0.0;
        let currency = events
            .first()
            .ok_or(finstack_core::error::InputError::TooFewPoints)?
            .amount
            .currency();

        for event in events {
            if event.kind == FundEventKind::Distribution || event.kind == FundEventKind::Proceeds {
                // Run waterfall allocation
                let lp_distributed_so_far: F = ledger_rows.iter().map(|r| r.to_lp.amount()).sum();
                let allocations = self.allocate_distribution(AllocationParams {
                    total_amount: event.amount,
                    initial_lp_unreturned: lp_unreturned,
                    initial_gp_carry: gp_carry_cum,
                    lp_distributed_cum_before: lp_distributed_so_far,
                    all_events: events,
                    allocation_date: event.date,
                    currency,
                })?;

                for alloc in allocations {
                    lp_unreturned = alloc.lp_unreturned.amount();
                    gp_carry_cum = alloc.gp_carry_cum.amount();
                    ledger_rows.push(alloc);
                }
            } else if event.kind == FundEventKind::Contribution {
                // Track LP capital
                lp_unreturned += event.amount.amount();
            }
        }

        Ok(())
    }

    /// Run American-style waterfall (deal-by-deal).
    fn run_american(
        &self,
        events: &[FundEvent],
        ledger_rows: &mut Vec<AllocationRow>,
    ) -> finstack_core::Result<()> {
        // Group events by deal_id
        let mut deals: HashMap<String, Vec<&FundEvent>> = HashMap::new();
        let mut fund_contributions = Vec::new();

        for event in events {
            match (&event.deal_id, event.kind) {
                (Some(deal_id), FundEventKind::Proceeds) => {
                    deals.entry(deal_id.clone()).or_default().push(event);
                }
                (_, FundEventKind::Contribution) => {
                    fund_contributions.push(event);
                }
                _ => {
                    // Fund-level distributions go to fund level
                    fund_contributions.push(event);
                }
            }
        }

        // Process each deal separately
        let mut total_lp_unreturned = fund_contributions
            .iter()
            .filter(|e| e.kind == FundEventKind::Contribution)
            .map(|e| e.amount.amount())
            .sum::<F>();

        let mut total_gp_carry = 0.0;
        let currency = events
            .first()
            .ok_or(finstack_core::error::InputError::TooFewPoints)?
            .amount
            .currency();

        for (deal_id, deal_events) in deals {
            // For each deal, allocate proceeds through the waterfall
            for event in deal_events {
                let lp_distributed_so_far: F = ledger_rows.iter().map(|r| r.to_lp.amount()).sum();
                let allocations = self.allocate_distribution(AllocationParams {
                    total_amount: event.amount,
                    initial_lp_unreturned: total_lp_unreturned,
                    initial_gp_carry: total_gp_carry,
                    lp_distributed_cum_before: lp_distributed_so_far,
                    all_events: events,
                    allocation_date: event.date,
                    currency,
                })?;

                for mut alloc in allocations {
                    alloc.deal_id = Some(deal_id.clone());
                    total_lp_unreturned = alloc.lp_unreturned.amount();
                    total_gp_carry = alloc.gp_carry_cum.amount();
                    ledger_rows.push(alloc);
                }
            }
        }

        Ok(())
    }

    /// Allocate a single distribution through the waterfall.
    fn allocate_distribution(
        &self,
        params: AllocationParams,
    ) -> finstack_core::Result<Vec<AllocationRow>> {
        let mut remaining_amount = params.total_amount.amount();
        let mut lp_unreturned = params.initial_lp_unreturned;
        let mut gp_carry_cum = params.initial_gp_carry;
        let mut allocations = Vec::new();

        // Track LP allocated within this distribution call (prior tranches)
        let mut lp_allocated_in_call_so_far: F = 0.0;

        // Precompute holdback percent (0.0 if none or clawback disabled)
        let holdback_pct: F = (match &self.spec.clawback {
            Some(c) if c.enable => c.holdback_pct.unwrap_or(0.0),
            _ => 0.0,
        })
        .clamp(0.0, 1.0);

        for (idx, tranche) in self.spec.tranches.iter().enumerate() {
            if remaining_amount <= 1e-6 {
                break;
            }

            let (to_lp, to_gp_paid, tranche_name, gp_carry_cum_after) = match tranche {
                Tranche::ReturnOfCapital => {
                    let allocation = remaining_amount.min(lp_unreturned);
                    lp_unreturned -= allocation;
                    remaining_amount -= allocation;
                    lp_allocated_in_call_so_far += allocation;
                    (
                        allocation,
                        0.0,
                        "Return of Capital".to_string(),
                        gp_carry_cum,
                    )
                }

                Tranche::PreferredIrr { irr } => {
                    // Calculate required amount to achieve target IRR
                    let required = self.calculate_preferred_amount(
                        *irr,
                        params.all_events,
                        params.allocation_date,
                        lp_unreturned,
                    )?;
                    let allocation = remaining_amount.min(required);
                    remaining_amount -= allocation;
                    lp_allocated_in_call_so_far += allocation;
                    (
                        allocation,
                        0.0,
                        format!("Preferred Return {:.1}%", irr * 100.0),
                        gp_carry_cum,
                    )
                }

                Tranche::CatchUp { gp_share } => {
                    // Determine target cumulative GP share from the next promote tier if available
                    let mut target_gp_share: F = *gp_share; // fallback
                    for next in self.spec.tranches.iter().skip(idx + 1) {
                        if let Tranche::PromoteTier { gp_share, .. } = next {
                            target_gp_share = *gp_share;
                            break;
                        }
                    }

                    // Compute contributions up to current date
                    let total_contributions_to_date: F = params
                        .all_events
                        .iter()
                        .filter(|e| e.kind == FundEventKind::Contribution && e.date <= params.allocation_date)
                        .map(|e| e.amount.amount())
                        .sum();

                    // Profit to date before this catch-up tranche (gross basis)
                    let mut profit_excl =
                        (params.lp_distributed_cum_before + lp_allocated_in_call_so_far + gp_carry_cum)
                            - total_contributions_to_date;
                    if !profit_excl.is_finite() || profit_excl.is_sign_negative() {
                        profit_excl = 0.0;
                    }

                    let needed_gp_gross = if target_gp_share >= 1.0 - 1e-12 {
                        remaining_amount // degenerate; give remaining in full-mode
                    } else {
                        ((target_gp_share * profit_excl) - gp_carry_cum) / (1.0 - target_gp_share)
                    };

                    let needed_gp_gross = needed_gp_gross.max(0.0);
                    let to_gp_gross = match self.spec.catchup_mode {
                        CatchUpMode::Full => needed_gp_gross.min(remaining_amount),
                        CatchUpMode::Partial => (remaining_amount * gp_share).min(needed_gp_gross),
                    };

                    let to_gp_paid = to_gp_gross * (1.0 - holdback_pct);
                    gp_carry_cum += to_gp_gross;
                    remaining_amount -= to_gp_gross;
                    (
                        0.0,
                        to_gp_paid,
                        "Catch-Up".to_string(),
                        gp_carry_cum,
                    )
                }

                Tranche::PromoteTier {
                    lp_share,
                    gp_share,
                    hurdle,
                } => {
                    let to_lp = remaining_amount * lp_share;
                    let to_gp_gross = remaining_amount * gp_share;
                    let to_gp_paid = to_gp_gross * (1.0 - holdback_pct);
                    gp_carry_cum += to_gp_gross;

                    let tranche_name = match hurdle {
                        Hurdle::Irr { rate } => format!(
                            "Promote {:.1}%+ ({}%/{}%)",
                            rate * 100.0,
                            lp_share * 100.0,
                            gp_share * 100.0
                        ),
                    };

                    remaining_amount = 0.0; // Allocate all remaining
                    lp_allocated_in_call_so_far += to_lp;
                    (to_lp, to_gp_paid, tranche_name, gp_carry_cum)
                }
            };

            // Calculate current LP IRR if we have enough data
            let lp_irr_to_date = self.calculate_lp_irr_to_date(params.all_events, params.allocation_date);

            allocations.push(AllocationRow {
                date: params.allocation_date,
                period_key: None, // TODO: Add period support if needed
                deal_id: None,    // Set by caller for American style
                tranche: tranche_name,
                to_lp: Money::new(to_lp, currency),
                to_gp: Money::new(to_gp_paid, currency),
                lp_unreturned: Money::new(lp_unreturned, params.currency),
                gp_carry_cum: Money::new(gp_carry_cum_after, params.currency),
                lp_irr_to_date,
                note: None,
            });
        }

        Ok(allocations)
    }

    #[allow(clippy::too_many_arguments)]
    struct AllocationParams<'e> {
        total_amount: Money,
        initial_lp_unreturned: F,
        initial_gp_carry: F,
        lp_distributed_cum_before: F,
        all_events: &'e [FundEvent],
        allocation_date: Date,
        currency: Currency,
    }

    /// Calculate the amount needed for preferred return using robust root finding.
    fn calculate_preferred_amount(
        &self,
        target_irr: F,
        all_events: &[FundEvent],
        current_date: Date,
        _lp_unreturned: F,
    ) -> finstack_core::Result<F> {
        // Build LP cashflow history up to current date, including contributions and prior distributions
        let mut lp_flows = Vec::new();

        for event in all_events {
            if event.date < current_date {
                lp_flows.push((event.date, event.signed_amount()));
            }
        }

        if lp_flows.is_empty() {
            return Ok(0.0); // Need at least one prior flow (contribution)
        }

        // Current IRR without additional preferred return
        let base_date = lp_flows[0].0;
        let current_irr = self.calculate_irr(&lp_flows, base_date).unwrap_or(0.0);

        if current_irr >= target_irr {
            return Ok(0.0); // Already at or above target IRR
        }

        // Use root finding to determine required additional distribution amount
        let target_function = |additional_amount: f64| -> f64 {
            if additional_amount < 0.0 {
                return f64::INFINITY;
            }

            let mut flows_with_additional = lp_flows.clone();
            flows_with_additional.push((
                current_date,
                Money::new(additional_amount, lp_flows[0].1.currency()),
            ));

            match self.calculate_irr(&flows_with_additional, base_date) {
                Ok(irr) => irr - target_irr,
                Err(_) => f64::INFINITY, // Invalid IRR
            }
        };

        // Use broader bounds - sometimes large distributions needed for high IRR targets
        let total_contributions: F = lp_flows
            .iter()
            .filter(|(_, amount)| amount.amount() < 0.0)
            .map(|(_, amount)| amount.amount().abs())
            .sum();

        let max_reasonable = total_contributions * 10.0; // Up to 10x contributions as upper bound

        match brent(target_function, 0.0, max_reasonable, 1e-6, 100) {
            Ok(amount) => Ok(amount.max(0.0)),
            Err(_) => {
                // If root finding fails, try to estimate analytically
                // For a simple case: if we have one contribution and want target IRR over time t,
                // then: target_amount = contribution * (1 + target_irr)^t
                if lp_flows.len() == 1 {
                    let contrib_amount = lp_flows[0].1.amount().abs();
                    let years = self
                        .spec
                        .irr_basis
                        .year_fraction(base_date, current_date)
                        .unwrap_or(1.0);
                    let required_total = contrib_amount * (1.0 + target_irr).powf(years);
                    let already_received = total_contributions - contrib_amount; // Net distributions so far
                    Ok((required_total - already_received).max(0.0))
                } else {
                    Ok(0.0)
                }
            }
        }
    }

    /// Calculate IRR for LP cashflows using Brent's method.
    fn calculate_irr(&self, flows: &[(Date, Money)], base_date: Date) -> finstack_core::Result<F> {
        if flows.len() < 2 {
            return Err(finstack_core::error::InputError::TooFewPoints.into());
        }

        let npv_function = |rate: f64| -> f64 {
            let mut npv = 0.0;
            for (date, amount) in flows {
                let t = self
                    .spec
                    .irr_basis
                    .year_fraction(base_date, *date)
                    .unwrap_or(0.0);
                let df = if rate.abs() < 1e-10 {
                    1.0 // Avoid division by zero for 0% rate
                } else {
                    (1.0 + rate).powf(-t)
                };
                npv += amount.amount() * df;
            }
            npv
        };

        // Use Brent's method to find IRR (rate where NPV = 0)
        brent(npv_function, -0.99, 5.0, 1e-12, 100)
            .map_err(|_| finstack_core::error::InputError::Invalid.into())
    }

    /// Calculate LP IRR to date.
    fn calculate_lp_irr_to_date(&self, all_events: &[FundEvent], as_of_date: Date) -> Option<F> {
        let lp_flows: Vec<(Date, Money)> = all_events
            .iter()
            .filter(|e| e.date <= as_of_date)
            .map(|e| (e.date, e.signed_amount()))
            .collect();

        if lp_flows.len() < 2 {
            return None;
        }

        let base_date = lp_flows[0].0;
        self.calculate_irr(&lp_flows, base_date).ok()
    }

    /// Apply clawback reconciliation.
    fn apply_clawback(
        &self,
        events: &[FundEvent],
        ledger_rows: &mut Vec<AllocationRow>,
        clawback_spec: &ClawbackSpec,
    ) -> finstack_core::Result<()> {
        // Periodic clawback not supported in this PR
        if matches!(clawback_spec.settle_on, ClawbackSettle::Periodic) {
            return Err(finstack_core::error::InputError::Invalid.into());
        }

        let last = match ledger_rows.last() {
            Some(r) => r.clone(),
            None => return Ok(()),
        };

        let currency = last.to_gp.currency();
        let settlement_date = events
            .iter()
            .map(|e| e.date)
            .max()
            .unwrap_or(last.date);

        // Compute total contributions and distributions from events (fund-level)
        let total_contributions: F = events
            .iter()
            .filter(|e| e.kind == FundEventKind::Contribution)
            .map(|e| e.amount.amount())
            .sum();
        let total_distributions: F = events
            .iter()
            .filter(|e| e.kind == FundEventKind::Distribution || e.kind == FundEventKind::Proceeds)
            .map(|e| e.amount.amount())
            .sum();
        let profit_total = (total_distributions - total_contributions).max(0.0);

        // Determine target GP share from first promote tier if present
        let target_gp_share: F = self
            .spec
            .tranches
            .iter()
            .find_map(|t| match t {
                Tranche::PromoteTier { gp_share, .. } => Some(*gp_share),
                _ => None,
            })
            .unwrap_or(0.0);

        // Allowed GP carry based on final fund profit
        let allowed_gp_total = (profit_total * target_gp_share).max(0.0);

        // Paid GP to date (net of holdback)
        let paid_gp_total: F = ledger_rows.iter().map(|r| r.to_gp.amount()).sum();

        // Difference => settlement amount for GP (positive => pay GP; negative => GP returns)
        let delta_gp: F = allowed_gp_total - paid_gp_total;

        if delta_gp.abs() <= 1e-9 {
            return Ok(()); // Nothing to settle
        }

        // Prepare settlement row
        let to_gp = Money::new(delta_gp, currency);
        let to_lp = Money::new((-delta_gp).max(0.0), currency);

        let settlement_row = AllocationRow {
            date: settlement_date,
            period_key: None,
            deal_id: None,
            tranche: "Clawback Settlement (fund_end)".to_string(),
            to_lp,
            to_gp,
            lp_unreturned: last.lp_unreturned, // unchanged
            gp_carry_cum: Money::new(allowed_gp_total, currency),
            lp_irr_to_date: self.calculate_lp_irr_to_date(events, settlement_date),
            note: Some("Clawback settlement and holdback release".to_string()),
        };

        ledger_rows.push(settlement_row);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn test_currency() -> Currency {
        Currency::USD
    }

    fn test_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap()
    }

    #[test]
    fn waterfall_spec_builder() {
        let spec = WaterfallSpec::builder()
            .style(WaterfallStyle::European)
            .return_of_capital()
            .preferred_irr(0.08)
            .catchup(1.0)
            .promote_tier(0.0, 0.8, 0.2)
            .build()
            .unwrap();

        assert_eq!(spec.style, WaterfallStyle::European);
        assert_eq!(spec.tranches.len(), 4);
        assert_eq!(spec.irr_basis, DayCount::Act365F);
    }

    #[test]
    fn fund_event_creation() {
        let contrib = FundEvent::contribution(
            test_date(2020, 1, 1),
            Money::new(1000000.0, test_currency()),
        );

        assert_eq!(contrib.kind, FundEventKind::Contribution);
        assert_eq!(contrib.signed_amount().amount(), -1000000.0);

        let distrib = FundEvent::distribution(
            test_date(2025, 1, 1),
            Money::new(1500000.0, test_currency()),
        );

        assert_eq!(distrib.kind, FundEventKind::Distribution);
        assert_eq!(distrib.signed_amount().amount(), 1500000.0);
    }

    #[test]
    fn simple_waterfall_allocation() {
        let spec = WaterfallSpec::builder()
            .return_of_capital()
            .promote_tier(0.0, 0.8, 0.2) // Simple 80/20 split
            .build()
            .unwrap();

        let events = vec![
            FundEvent::contribution(
                test_date(2020, 1, 1),
                Money::new(1000000.0, test_currency()),
            ),
            FundEvent::distribution(
                test_date(2025, 1, 1),
                Money::new(1500000.0, test_currency()),
            ),
        ];

        let engine = EquityWaterfallEngine::new(&spec);
        let ledger = engine.run(&events).unwrap();

        assert!(!ledger.rows.is_empty());

        // Should have return of capital and promote allocation
        let roc_rows: Vec<_> = ledger
            .rows
            .iter()
            .filter(|r| r.tranche.contains("Return of Capital"))
            .collect();
        assert!(!roc_rows.is_empty());

        // LP should get back their $1M capital first
        let total_lp_roc: F = roc_rows.iter().map(|r| r.to_lp.amount()).sum();
        assert!((total_lp_roc - 1000000.0).abs() < 1e-6);
    }

    #[test]
    fn ledger_to_tabular_conversion() {
        let spec = WaterfallSpec::builder()
            .return_of_capital()
            .build()
            .unwrap();

        let events = vec![
            FundEvent::contribution(
                test_date(2020, 1, 1),
                Money::new(1000000.0, test_currency()),
            ),
            FundEvent::distribution(
                test_date(2025, 1, 1),
                Money::new(1000000.0, test_currency()),
            ),
        ];

        let engine = EquityWaterfallEngine::new(&spec);
        let ledger = engine.run(&events).unwrap();
        let (columns, _rows) = ledger.to_tabular_data();

        // Check tabular structure
        assert!(columns.contains(&"date"));
        assert!(columns.contains(&"tranche"));
        assert!(columns.contains(&"to_lp"));
        assert!(columns.contains(&"to_gp"));
    }

    #[test]
    fn validate_waterfall_spec() {
        // Valid spec
        let valid_spec = WaterfallSpec::builder()
            .return_of_capital()
            .promote_tier(0.0, 0.8, 0.2)
            .build();
        assert!(valid_spec.is_ok());

        // Invalid spec - promote shares don't sum to 1.0
        let invalid_spec = WaterfallSpec {
            style: WaterfallStyle::European,
            tranches: vec![Tranche::PromoteTier {
                hurdle: Hurdle::Irr { rate: 0.0 },
                lp_share: 0.7,
                gp_share: 0.4, // 0.7 + 0.4 = 1.1 > 1.0
            }],
            clawback: None,
            irr_basis: DayCount::Act365F,
            catchup_mode: CatchUpMode::Full,
        };

        assert!(invalid_spec.validate().is_err());
    }

    #[test]
    fn catchup_precise_reaches_target_split() {
        let spec = WaterfallSpec::builder()
            .style(WaterfallStyle::European)
            .return_of_capital()
            .preferred_irr(0.08)
            .catchup(1.0)
            .promote_tier(0.0, 0.8, 0.2)
            .build()
            .unwrap();

        let events = vec![
            FundEvent::contribution(
                test_date(2020, 1, 1),
                Money::new(100.0, test_currency()),
            ),
            FundEvent::distribution(
                test_date(2024, 1, 1),
                Money::new(200.0, test_currency()),
            ),
        ];

        let engine = EquityWaterfallEngine::new(&spec);
        let ledger = engine.run(&events).unwrap();

        let total_gp: F = ledger.rows.iter().map(|r| r.to_gp.amount()).sum();
        let total_lp: F = ledger.rows.iter().map(|r| r.to_lp.amount()).sum();
        let profit = (total_lp + total_gp) - 100.0;

        // Ensure profit positive and GP share ~20%
        assert!(profit > 0.0);
        let gp_share = if profit.abs() > 1e-9 { total_gp / profit } else { 0.0 };
        assert!((gp_share - 0.20).abs() < 1e-6, "gp_share={}", gp_share);

        // Catch-up row should exist with positive GP allocation (likely ~9)
        let catchup_rows: Vec<_> = ledger
            .rows
            .iter()
            .filter(|r| r.tranche.contains("Catch-Up") && r.to_gp.amount() > 0.0)
            .collect();
        assert!(!catchup_rows.is_empty());
    }

    #[test]
    fn clawback_fund_end_overdistribution() {
        let claw = ClawbackSpec {
            enable: true,
            holdback_pct: None,
            settle_on: ClawbackSettle::FundEnd,
        };

        let spec = WaterfallSpec::builder()
            .style(WaterfallStyle::European)
            .return_of_capital()
            .promote_tier(0.0, 0.8, 0.2)
            .clawback(claw)
            .build()
            .unwrap();

        let events = vec![
            FundEvent::contribution(
                test_date(2020, 1, 1),
                Money::new(100.0, test_currency()),
            ),
            FundEvent::distribution(
                test_date(2022, 1, 1),
                Money::new(150.0, test_currency()),
            ),
            FundEvent::contribution(
                test_date(2023, 1, 1),
                Money::new(90.0, test_currency()),
            ),
        ];

        let engine = EquityWaterfallEngine::new(&spec);
        let ledger = engine.run(&events).unwrap();

        // Last row should be clawback settlement with negative GP amount
        let last = ledger.rows.last().expect("rows");
        assert!(last.tranche.contains("Clawback Settlement"));
        assert!(last.to_gp.amount() < 0.0);
        // In this scenario, first promote paid GP 10; final profit is 0 so GP should return ~10
        assert!((last.to_gp.amount() + 10.0).abs() < 1e-6);
    }
}
