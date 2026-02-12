//! Real estate operating statement templates.
//!
//! These helpers provide a consistent NOI / NCF buildup pattern for property-level models.

use crate::builder::{ModelBuilder, Ready};
use crate::error::{Error, Result};
use crate::types::AmountOrScalar;
use finstack_core::dates::PeriodId;

fn sum_expr(nodes: &[&str]) -> Result<String> {
    if nodes.is_empty() {
        return Err(Error::build("Expected at least one node".to_string()));
    }
    Ok(nodes.join(" + "))
}

fn sum_expr_or_zero(nodes: &[&str]) -> String {
    if nodes.is_empty() {
        "0".to_string()
    } else {
        nodes.join(" + ")
    }
}

/// Add a standard NOI buildup:
/// `total_revenue = sum(revenue_nodes)`
/// `total_expenses = sum(expense_nodes)`
/// `noi = total_revenue - total_expenses`
pub fn add_noi_buildup(
    builder: ModelBuilder<Ready>,
    total_revenue_node: &str,
    revenue_nodes: &[&str],
    total_expenses_node: &str,
    expense_nodes: &[&str],
    noi_node: &str,
) -> Result<ModelBuilder<Ready>> {
    let total_rev_expr = sum_expr(revenue_nodes)?;
    let total_exp_expr = sum_expr(expense_nodes)?;

    let builder = builder
        .compute(total_revenue_node, &total_rev_expr)?
        .compute(total_expenses_node, &total_exp_expr)?
        .compute(
            noi_node,
            format!("{total_revenue_node} - {total_expenses_node}"),
        )?;

    Ok(builder)
}

/// Add a standard NCF (net cash flow) buildup:
/// `ncf = noi - sum(capex_nodes)`
pub fn add_ncf_buildup(
    builder: ModelBuilder<Ready>,
    noi_node: &str,
    capex_nodes: &[&str],
    ncf_node: &str,
) -> Result<ModelBuilder<Ready>> {
    if capex_nodes.is_empty() {
        return builder.compute(ncf_node, noi_node);
    }
    let capex_expr = sum_expr(capex_nodes)?;
    builder.compute(ncf_node, format!("{noi_node} - ({capex_expr})"))
}

/// Simple lease-level rent schedule spec for rent-roll style revenue generation.
///
/// Values are per-model-period amounts (i.e., if the model is quarterly, `base_rent` is per quarter).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseSpec {
    /// Node id to store this lease's rent revenue series.
    pub node_id: String,
    /// First period (inclusive) when the lease is active.
    pub start: PeriodId,
    /// Last period (inclusive) when the lease is active. `None` means through model end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<PeriodId>,
    /// Base rent per period at `start`.
    pub base_rent: f64,
    /// Growth rate applied per period after `start` (e.g., 0.03 for +3% per period).
    #[serde(default)]
    pub growth_rate: f64,
    /// Number of periods of free rent starting at `start`.
    #[serde(default)]
    pub free_rent_periods: u32,
    /// Occupancy factor \(\in [0,1]\) applied to rent (useful for probability/vacancy).
    #[serde(default = "default_occupancy")]
    pub occupancy: f64,
}

fn default_occupancy() -> f64 {
    1.0
}

/// Add a minimal rent roll by generating lease rent series and summing into a total rent node.
///
/// - Creates one **value** node per lease (`LeaseSpec::node_id`) with an explicit per-period series.
/// - Creates a **calculated** node `total_rent_node = sum(lease_nodes)`.
///
/// This intentionally stays simple (no reimbursements, % rent, downtime, TI/LC). It’s meant to
/// be a foundation for more market-standard templates.
pub fn add_rent_roll_rental_revenue(
    mut builder: ModelBuilder<Ready>,
    leases: &[LeaseSpec],
    total_rent_node: &str,
) -> Result<ModelBuilder<Ready>> {
    if leases.is_empty() {
        return Err(Error::build(
            "add_rent_roll_rental_revenue: expected at least one lease",
        ));
    }

    // Build each lease's explicit series from builder periods.
    for lease in leases {
        if lease.node_id.trim().is_empty() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue: lease node_id cannot be empty",
            ));
        }
        if !lease.base_rent.is_finite() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue: base_rent must be finite",
            ));
        }
        if !lease.growth_rate.is_finite() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue: growth_rate must be finite",
            ));
        }
        if !(0.0..=1.0).contains(&lease.occupancy) {
            return Err(Error::build(
                "add_rent_roll_rental_revenue: occupancy must be in [0, 1]",
            ));
        }

        let mut values: Vec<(PeriodId, AmountOrScalar)> = Vec::with_capacity(builder.periods.len());
        let mut periods_since_start: u32 = 0;

        for p in &builder.periods {
            let pid = p.id;
            let active = pid >= lease.start && lease.end.is_none_or(|e| pid <= e);
            let rent = if active {
                let rent_before_free =
                    lease.base_rent * (1.0 + lease.growth_rate).powi(periods_since_start as i32);
                let rent_after_free = if periods_since_start < lease.free_rent_periods {
                    0.0
                } else {
                    rent_before_free
                };
                periods_since_start = periods_since_start.saturating_add(1);
                rent_after_free * lease.occupancy
            } else {
                0.0
            };

            values.push((pid, AmountOrScalar::scalar(rent)));
        }

        builder = builder.value(lease.node_id.clone(), &values);
    }

    let lease_nodes = leases
        .iter()
        .map(|l| l.node_id.as_str())
        .collect::<Vec<_>>();
    let total_expr = sum_expr(&lease_nodes)?;
    builder.compute(total_rent_node, &total_expr)
}

/// Rent step that resets the base rent starting at `start` (inclusive).
///
/// The lease `growth_rate` then applies from this step forward until the next step.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RentStepSpec {
    /// Period (inclusive) when this rent level becomes effective.
    pub start: PeriodId,
    /// Rent per model period starting at `start`.
    pub rent: f64,
}

/// Free rent (concession) window that zeros out rent for `periods` starting at `start`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeRentWindowSpec {
    /// Period (inclusive) when free rent starts.
    pub start: PeriodId,
    /// Number of model periods of free rent.
    pub periods: u32,
}

/// Renewal specification for a lease.
///
/// This is modeled in an **expected value** sense via `probability`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenewalSpec {
    /// Downtime (no rent) after the initial term ends.
    #[serde(default)]
    pub downtime_periods: u32,
    /// Renewal term length in model periods.
    pub term_periods: u32,
    /// Probability of renewal \(\in [0,1]\).
    pub probability: f64,
    /// Rent multiplier applied to the last contractual rent of the initial term.
    ///
    /// Example: `1.05` means renewal starts at +5% vs prior rent level.
    #[serde(default = "default_rent_factor")]
    pub rent_factor: f64,
    /// Free rent periods at renewal start.
    #[serde(default)]
    pub free_rent_periods: u32,
}

fn default_rent_factor() -> f64 {
    1.0
}

/// Convention that determines how `growth_rate` compounds in a lease.
///
/// - `PerPeriod` (default): `growth_rate` is applied every model period.
/// - `AnnualEscalator`: `growth_rate` is applied once per **lease-start anniversary**,
///   measured in model periods (i.e., every `periods_per_year()` periods from the segment
///   start). Within the same lease year rent is flat; the bump resets at each rent step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseGrowthConvention {
    /// Growth rate compounds every model period.
    #[default]
    PerPeriod,
    /// Growth rate compounds annually on the lease-start anniversary.
    AnnualEscalator,
}

/// Richer lease spec for rent roll generation:
/// - rent steps (explicit bumps)
/// - arbitrary free-rent windows
/// - optional renewal with downtime + probability
///
/// All amounts are per-model-period (quarterly model => per quarter).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseSpecV2 {
    /// Base id used to derive node ids:
    /// `{node_id}.pgi`, `{node_id}.free_rent`, `{node_id}.vacancy_loss`, `{node_id}.effective_rent`.
    pub node_id: String,
    /// First period (inclusive) when the lease is active.
    pub start: PeriodId,
    /// Last period (inclusive) of the initial term. `None` means through model end (no renewal modeling).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<PeriodId>,
    /// Base rent per period at `start`.
    pub base_rent: f64,
    /// Growth rate applied per period or annually (depending on `growth_convention`)
    /// within a rent segment (between steps).
    #[serde(default)]
    pub growth_rate: f64,
    /// Convention for compounding `growth_rate`.
    #[serde(default)]
    pub growth_convention: LeaseGrowthConvention,
    /// Rent steps that reset rent levels at their start periods.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rent_steps: Vec<RentStepSpec>,
    /// Number of free rent periods from `start`.
    #[serde(default)]
    pub free_rent_periods: u32,
    /// Additional free rent windows (beyond the initial `free_rent_periods`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub free_rent_windows: Vec<FreeRentWindowSpec>,
    /// Occupancy factor \(\in [0,1]\) applied to non-free contractual rent.
    #[serde(default = "default_occupancy")]
    pub occupancy: f64,
    /// Optional renewal modeling after `end`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renewal: Option<RenewalSpec>,
}

/// Standard output node ids for a rent roll.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RentRollOutputNodes {
    /// Total contractual rent (PGI) from all leases.
    pub rent_pgi_node: String,
    /// Total free rent concessions.
    pub free_rent_node: String,
    /// Total vacancy loss (includes occupancy and renewal probability effects).
    pub vacancy_loss_node: String,
    /// Total effective rent (EGI rent component): `rent_pgi - free_rent - vacancy_loss`.
    pub rent_effective_node: String,
}

impl Default for RentRollOutputNodes {
    fn default() -> Self {
        Self {
            rent_pgi_node: "rent_pgi".into(),
            free_rent_node: "free_rent".into(),
            vacancy_loss_node: "vacancy_loss".into(),
            rent_effective_node: "rent_effective".into(),
        }
    }
}

fn find_period_idx(periods: &[finstack_core::dates::Period], id: PeriodId) -> Result<usize> {
    periods
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| Error::build(format!("Unknown period id '{id}'")))
}

fn apply_free_window(is_free: &mut [bool], start_idx: usize, len: u32) -> Result<()> {
    if len == 0 {
        return Ok(());
    }
    let end = start_idx
        .checked_add(len as usize)
        .ok_or_else(|| Error::build("free rent window overflow".to_string()))?;
    for i in start_idx..end.min(is_free.len()) {
        is_free[i] = true;
    }
    Ok(())
}

/// Add a richer rent roll that outputs standard PGI/EGI nodes and per-lease detail nodes.
///
/// Per lease (base id = `LeaseSpecV2::node_id`), creates **value** nodes:
/// - `{id}.pgi`: contractual rent before concessions/vacancy
/// - `{id}.free_rent`: concession amount (rent forgiven)
/// - `{id}.vacancy_loss`: vacancy loss amount
/// - `{id}.effective_rent`: effective rent after concessions/vacancy
///
/// Totals (via `RentRollOutputNodes`) are **calculated** nodes:
/// - `rent_pgi_node = sum({id}.pgi)`
/// - `free_rent_node = sum({id}.free_rent)`
/// - `vacancy_loss_node = sum({id}.vacancy_loss)`
/// - `rent_effective_node = rent_pgi_node - free_rent_node - vacancy_loss_node`
pub fn add_rent_roll_rental_revenue_v2(
    mut builder: ModelBuilder<Ready>,
    leases: &[LeaseSpecV2],
    nodes: &RentRollOutputNodes,
) -> Result<ModelBuilder<Ready>> {
    if leases.is_empty() {
        return Err(Error::build(
            "add_rent_roll_rental_revenue_v2: expected at least one lease",
        ));
    }

    // Periods-per-year for annual escalator calculation.
    let ppy = builder
        .periods
        .first()
        .map(|p| p.id.periods_per_year() as usize)
        .unwrap_or(4); // defensive fallback; periods is non-empty after builder.periods()

    for lease in leases {
        if lease.node_id.trim().is_empty() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue_v2: lease node_id cannot be empty",
            ));
        }
        if !lease.base_rent.is_finite() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue_v2: base_rent must be finite",
            ));
        }
        if !lease.growth_rate.is_finite() {
            return Err(Error::build(
                "add_rent_roll_rental_revenue_v2: growth_rate must be finite",
            ));
        }
        if !(0.0..=1.0).contains(&lease.occupancy) {
            return Err(Error::build(
                "add_rent_roll_rental_revenue_v2: occupancy must be in [0, 1]",
            ));
        }

        let start_idx = find_period_idx(&builder.periods, lease.start)?;
        let end_idx = if let Some(e) = lease.end {
            find_period_idx(&builder.periods, e)?
        } else {
            builder.periods.len().saturating_sub(1)
        };
        if end_idx < start_idx {
            return Err(Error::build(
                "add_rent_roll_rental_revenue_v2: end must be >= start",
            ));
        }

        // Build free-rent mask across all model periods.
        let mut is_free = vec![false; builder.periods.len()];
        apply_free_window(&mut is_free, start_idx, lease.free_rent_periods)?;
        for w in &lease.free_rent_windows {
            if w.periods == 0 {
                continue;
            }
            let w_start = find_period_idx(&builder.periods, w.start)?;
            apply_free_window(&mut is_free, w_start, w.periods)?;
        }

        let (renewal_start_idx, renewal_end_idx, renewal_prob, renewal_free_periods) =
            if let (Some(end_pid), Some(r)) = (lease.end, lease.renewal.as_ref()) {
                if !r.probability.is_finite() || !(0.0..=1.0).contains(&r.probability) {
                    return Err(Error::build(
                        "add_rent_roll_rental_revenue_v2: renewal.probability must be in [0, 1]",
                    ));
                }
                if !r.rent_factor.is_finite() || r.rent_factor <= 0.0 {
                    return Err(Error::build(
                        "add_rent_roll_rental_revenue_v2: renewal.rent_factor must be positive",
                    ));
                }
                let _ = end_pid; // already validated via end_idx
                let start = end_idx + 1 + r.downtime_periods as usize;
                let end = start + r.term_periods as usize;
                (
                    Some(start),
                    Some(end.min(builder.periods.len())),
                    r.probability,
                    r.free_rent_periods,
                )
            } else {
                (None, None, 1.0, 0)
            };

        // Helper to compute contractual rent for an idx within a phase using rent steps.
        let mut step_points: Vec<(usize, f64)> = Vec::new();
        step_points.push((start_idx, lease.base_rent));
        for s in &lease.rent_steps {
            if !s.rent.is_finite() {
                return Err(Error::build(
                    "add_rent_roll_rental_revenue_v2: rent_steps rent must be finite",
                ));
            }
            let idx = find_period_idx(&builder.periods, s.start)?;
            step_points.push((idx, s.rent));
        }
        step_points.sort_by_key(|(i, _)| *i);

        let rent_at = |idx: usize, phase_start: usize, phase_base_rent: f64| -> f64 {
            // Find last step <= idx within the same phase.
            let mut base_idx = phase_start;
            let mut base_rent = phase_base_rent;
            for (si, sr) in &step_points {
                if *si < phase_start {
                    continue;
                }
                if *si <= idx {
                    base_idx = *si;
                    base_rent = *sr;
                } else {
                    break;
                }
            }
            let periods_elapsed = idx.saturating_sub(base_idx);
            let n = match lease.growth_convention {
                LeaseGrowthConvention::PerPeriod => periods_elapsed as i32,
                LeaseGrowthConvention::AnnualEscalator => (periods_elapsed / ppy) as i32,
            };
            base_rent * (1.0 + lease.growth_rate).powi(n)
        };

        // Compute contractual rent at end of initial phase (for renewal base).
        let last_initial_contractual = rent_at(end_idx, start_idx, lease.base_rent);
        let renewal_base_rent = lease
            .renewal
            .as_ref()
            .map(|r| last_initial_contractual * r.rent_factor);

        // Apply renewal free rent window (if any).
        if let Some(r_start) = renewal_start_idx {
            apply_free_window(&mut is_free, r_start, renewal_free_periods)?;
        }

        // Generate per-period series.
        let mut pgi_vals = Vec::with_capacity(builder.periods.len());
        let mut free_vals = Vec::with_capacity(builder.periods.len());
        let mut vac_vals = Vec::with_capacity(builder.periods.len());
        let mut vac_physical_vals = Vec::with_capacity(builder.periods.len());
        let mut renewal_loss_vals = Vec::with_capacity(builder.periods.len());
        let mut eff_vals = Vec::with_capacity(builder.periods.len());

        for (i, p) in builder.periods.iter().enumerate() {
            let pid = p.id;

            let (contractual, occupancy, renewal_p) = if i >= start_idx && i <= end_idx {
                (rent_at(i, start_idx, lease.base_rent), lease.occupancy, 1.0)
            } else if let (Some(r_start), Some(r_end), Some(r_base)) =
                (renewal_start_idx, renewal_end_idx, renewal_base_rent)
            {
                if i >= r_start && i < r_end {
                    let contractual = rent_at(i, r_start, r_base);
                    (contractual, lease.occupancy, renewal_prob)
                } else {
                    (0.0, 0.0, 1.0)
                }
            } else {
                (0.0, 0.0, 1.0)
            };

            let contractual = if contractual.is_finite() {
                contractual
            } else {
                0.0
            };
            let is_free_here = contractual != 0.0 && is_free.get(i).copied().unwrap_or(false);
            let free = if is_free_here { contractual } else { 0.0 };
            let net_after_free = contractual - free;

            // Expected-value decomposition:
            // - physical vacancy loss is conditional on renewal happening: net * p * (1 - occ)
            // - renewal probability loss is net * (1 - p)
            // Total loss matches the existing behavior: net - net * (p * occ)
            let eff = net_after_free * occupancy * renewal_p;
            let vac_physical = net_after_free * renewal_p * (1.0 - occupancy);
            let renewal_loss = net_after_free * (1.0 - renewal_p);
            let vac = vac_physical + renewal_loss;

            pgi_vals.push((pid, AmountOrScalar::scalar(contractual)));
            free_vals.push((pid, AmountOrScalar::scalar(free)));
            vac_vals.push((pid, AmountOrScalar::scalar(vac)));
            vac_physical_vals.push((pid, AmountOrScalar::scalar(vac_physical)));
            renewal_loss_vals.push((pid, AmountOrScalar::scalar(renewal_loss)));
            eff_vals.push((pid, AmountOrScalar::scalar(eff)));
        }

        let base = lease.node_id.as_str();
        builder = builder
            .value(format!("{base}.pgi"), &pgi_vals)
            .value(format!("{base}.free_rent"), &free_vals)
            .value(format!("{base}.vacancy_loss"), &vac_vals)
            // Additional transparency nodes (do not change totals behavior):
            .value(format!("{base}.vacancy_loss_physical"), &vac_physical_vals)
            .value(format!("{base}.renewal_prob_loss"), &renewal_loss_vals)
            .value(format!("{base}.effective_rent"), &eff_vals);
    }

    let pgi_nodes = leases
        .iter()
        .map(|l| format!("{}.pgi", l.node_id))
        .collect::<Vec<_>>();
    let free_nodes = leases
        .iter()
        .map(|l| format!("{}.free_rent", l.node_id))
        .collect::<Vec<_>>();
    let vac_nodes = leases
        .iter()
        .map(|l| format!("{}.vacancy_loss", l.node_id))
        .collect::<Vec<_>>();
    let vac_physical_nodes = leases
        .iter()
        .map(|l| format!("{}.vacancy_loss_physical", l.node_id))
        .collect::<Vec<_>>();
    let renewal_loss_nodes = leases
        .iter()
        .map(|l| format!("{}.renewal_prob_loss", l.node_id))
        .collect::<Vec<_>>();

    let pgi_refs = pgi_nodes.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let free_refs = free_nodes.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let vac_refs = vac_nodes.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let vac_physical_refs = vac_physical_nodes
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();
    let renewal_loss_refs = renewal_loss_nodes
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>();

    let pgi_expr = sum_expr(&pgi_refs)?;
    let free_expr = sum_expr(&free_refs)?;
    let vac_expr = sum_expr(&vac_refs)?;
    let vac_physical_expr = sum_expr(&vac_physical_refs)?;
    let renewal_loss_expr = sum_expr(&renewal_loss_refs)?;

    builder = builder
        .compute(&nodes.rent_pgi_node, &pgi_expr)?
        .compute(&nodes.free_rent_node, &free_expr)?
        .compute(&nodes.vacancy_loss_node, &vac_expr)?
        // Extra totals (fixed ids) for more underwriting transparency.
        .compute("vacancy_loss_physical", &vac_physical_expr)?
        .compute("renewal_prob_loss", &renewal_loss_expr)?
        .compute(
            &nodes.rent_effective_node,
            format!(
                "{} - {} - {}",
                nodes.rent_pgi_node, nodes.free_rent_node, nodes.vacancy_loss_node
            ),
        )?;

    Ok(builder)
}

/// Basis for management fee calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ManagementFeeBase {
    /// Fee is applied to EGI.
    #[default]
    Egi,
    /// Fee is applied to effective rent only (excludes other income).
    EffectiveRent,
}

/// Management fee specification.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementFeeSpec {
    /// Management fee rate as a decimal fraction (e.g., 0.03 for 3%).
    pub rate: f64,
    /// Fee base for calculation.
    #[serde(default)]
    pub base: ManagementFeeBase,
}

impl Default for ManagementFeeSpec {
    fn default() -> Self {
        Self {
            rate: 0.0,
            base: ManagementFeeBase::Egi,
        }
    }
}

/// Standard node ids for a full property operating statement template.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyTemplateNodes {
    /// Rent roll output nodes (PGI/free rent/vacancy/effective rent).
    #[serde(default)]
    pub rent_roll: RentRollOutputNodes,
    /// Total other income node id.
    #[serde(default = "default_other_income_total")]
    pub other_income_total_node: String,
    /// Effective gross income (EGI) node id.
    #[serde(default = "default_egi")]
    pub egi_node: String,
    /// Management fee node id (if configured).
    #[serde(default = "default_management_fee")]
    pub management_fee_node: String,
    /// Total operating expenses node id (includes management fee if enabled).
    #[serde(default = "default_opex_total")]
    pub opex_total_node: String,
    /// Net operating income (NOI) node id.
    #[serde(default = "default_noi")]
    pub noi_node: String,
    /// Total CapEx node id.
    #[serde(default = "default_capex_total")]
    pub capex_total_node: String,
    /// Net cash flow (NCF) node id: `noi - capex_total`.
    #[serde(default = "default_ncf")]
    pub ncf_node: String,
}

fn default_other_income_total() -> String {
    "other_income_total".into()
}
fn default_egi() -> String {
    "egi".into()
}
fn default_management_fee() -> String {
    "management_fee".into()
}
fn default_opex_total() -> String {
    "opex_total".into()
}
fn default_noi() -> String {
    "noi".into()
}
fn default_capex_total() -> String {
    "capex_total".into()
}
fn default_ncf() -> String {
    "ncf".into()
}

impl Default for PropertyTemplateNodes {
    fn default() -> Self {
        Self {
            rent_roll: RentRollOutputNodes::default(),
            other_income_total_node: default_other_income_total(),
            egi_node: default_egi(),
            management_fee_node: default_management_fee(),
            opex_total_node: default_opex_total(),
            noi_node: default_noi(),
            capex_total_node: default_capex_total(),
            ncf_node: default_ncf(),
        }
    }
}

/// Full property operating statement template:
/// - rent roll (PGI/free rent/vacancy/effective rent)
/// - EGI = effective rent + other income
/// - OpEx total (optionally adds management fee)
/// - NOI = EGI - OpEx
/// - CapEx total
/// - NCF = NOI - CapEx
pub fn add_property_operating_statement(
    mut builder: ModelBuilder<Ready>,
    leases: &[LeaseSpecV2],
    other_income_nodes: &[&str],
    opex_nodes: &[&str],
    capex_nodes: &[&str],
    management_fee: Option<ManagementFeeSpec>,
    nodes: &PropertyTemplateNodes,
) -> Result<ModelBuilder<Ready>> {
    builder = add_rent_roll_rental_revenue_v2(builder, leases, &nodes.rent_roll)?;

    // Other income total (optional).
    let other_income_expr = sum_expr_or_zero(other_income_nodes);
    builder = builder.compute(&nodes.other_income_total_node, other_income_expr)?;

    // EGI.
    builder = builder.compute(
        &nodes.egi_node,
        format!(
            "{} + {}",
            nodes.rent_roll.rent_effective_node, nodes.other_income_total_node
        ),
    )?;

    // Optional management fee.
    let mut opex_all: Vec<&str> = opex_nodes.to_vec();
    if let Some(spec) = management_fee {
        if !spec.rate.is_finite() || spec.rate < 0.0 {
            return Err(Error::build(
                "add_property_operating_statement: management_fee.rate must be finite and >= 0",
            ));
        }

        let base_expr = match spec.base {
            ManagementFeeBase::Egi => nodes.egi_node.as_str(),
            ManagementFeeBase::EffectiveRent => nodes.rent_roll.rent_effective_node.as_str(),
        };
        builder = builder.compute(
            &nodes.management_fee_node,
            format!("{base_expr} * {}", spec.rate),
        )?;
        opex_all.push(nodes.management_fee_node.as_str());
    }

    // OpEx total.
    let opex_expr = sum_expr_or_zero(&opex_all);
    builder = builder.compute(&nodes.opex_total_node, opex_expr)?;

    // NOI.
    builder = builder.compute(
        &nodes.noi_node,
        format!("{} - {}", nodes.egi_node, nodes.opex_total_node),
    )?;

    // CapEx total and NCF.
    let capex_expr = sum_expr_or_zero(capex_nodes);
    builder = builder.compute(&nodes.capex_total_node, capex_expr)?;
    builder = builder.compute(
        &nodes.ncf_node,
        format!("{} - {}", nodes.noi_node, nodes.capex_total_node),
    )?;

    Ok(builder)
}
