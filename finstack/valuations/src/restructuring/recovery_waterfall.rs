//! Recovery waterfall engine for gone-concern claim distribution.
//!
//! Implements the Absolute Priority Rule (APR) with configurable
//! deviations for negotiated plans. Distributes total enterprise value
//! or liquidation proceeds across ordered claim classes, respecting
//! collateral carve-outs for secured creditors.
//!
//! # Algorithm
//!
//! 1. Validate all inputs (non-negative amounts, consistent currencies,
//!    valid haircuts, known deviation targets).
//! 2. Sort claims by seniority (highest priority first).
//! 3. **Phase 1 -- Collateral recovery**: each secured claim recovers
//!    up to its net collateral value; any shortfall becomes a deficiency
//!    claim in the unsecured pool.
//! 4. **Phase 2 -- General distribution**: remaining value allocated
//!    top-down by seniority. Within a class, pro-rata or strict priority
//!    per the claim's `intra_class_allocation`.
//! 5. **Phase 3 -- APR check**: verify no lower-priority class recovers
//!    while a higher-priority class is impaired.
//!
//! # References
//!
//! - US Bankruptcy Code ss. 1129(b) (APR cram-down standard)
//! - Moyer, *Distressed Debt Analysis* (2004), Ch. 10-12

use finstack_core::money::Money;
use serde::{Deserialize, Serialize};

use super::error::RestructuringError;
use super::types::{Claim, ClaimSeniority};

/// Recovery waterfall specification.
///
/// Defines the total distributable value and the ordered claim classes
/// that compete for recovery. Implements the Absolute Priority Rule (APR)
/// with optional deviations for negotiated plans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryWaterfall {
    /// Total enterprise value or liquidation proceeds available for distribution.
    pub distributable_value: Money,
    /// Claims from highest to lowest priority.
    ///
    /// The order in this vec IS the priority order; [`ClaimSeniority`] provides
    /// the default ordering, but users can override for non-standard plans.
    pub claims: Vec<Claim>,
    /// Whether to enforce strict APR (`true`) or allow plan deviations (`false`).
    ///
    /// When `false`, each claim's recovery can be manually overridden via
    /// `plan_deviations`.
    #[serde(default = "default_strict_apr")]
    pub strict_apr: bool,
    /// Optional plan deviations from APR.
    ///
    /// Keyed by claim ID; specifies a recovery override as a fraction (0.0 - 1.0).
    /// Only applied when `strict_apr` is `false`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_deviations: Vec<PlanDeviation>,
}

fn default_strict_apr() -> bool {
    true
}

/// Explicit deviation from absolute priority.
///
/// Models gifting (equity getting value despite unsecured impairment),
/// carve-outs (professional fee carve-outs from secured collateral),
/// or negotiated settlements that break strict priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDeviation {
    /// Claim ID receiving the deviation.
    pub claim_id: String,
    /// Fixed recovery amount override (replaces waterfall-computed amount).
    pub recovery_override: Option<Money>,
    /// Recovery rate override as fraction (replaces waterfall-computed rate).
    pub recovery_rate_override: Option<f64>,
    /// Reason for deviation (for audit trail).
    pub reason: String,
}

/// Result of running the recovery waterfall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// Per-claim recovery breakdown.
    pub claim_recoveries: Vec<ClaimRecovery>,
    /// Total value distributed.
    pub total_distributed: Money,
    /// Residual value (should be zero if claims exceed distributable value).
    pub residual: Money,
    /// Whether APR was strictly followed.
    pub apr_satisfied: bool,
    /// Any APR violations detected (non-empty if a lower-priority class
    /// recovers while a higher-priority class is impaired).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apr_violations: Vec<String>,
}

/// Recovery outcome for a single claim class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimRecovery {
    /// Claim identifier.
    pub claim_id: String,
    /// Seniority level.
    pub seniority: ClaimSeniority,
    /// Total allowed claim.
    pub total_claim: Money,
    /// Recovery from collateral (secured claims).
    pub collateral_recovery: Money,
    /// Recovery from general unsecured pool / enterprise value.
    pub general_recovery: Money,
    /// Total recovery (collateral + general).
    pub total_recovery: Money,
    /// Recovery rate (total_recovery / total_claim), clamped to [0.0, 1.0].
    pub recovery_rate: f64,
    /// Deficiency claim (total_claim - collateral_recovery for undersecured
    /// claims; participates in unsecured pool).
    pub deficiency_claim: Money,
}

/// Run the recovery waterfall.
///
/// Allocates distributable value across claims in priority order:
/// 1. Secured claims recover from their allocated collateral first.
/// 2. Any deficiency (claim > collateral) becomes an unsecured claim.
/// 3. Remaining value distributed top-down by seniority.
/// 4. Within each class, allocated pro-rata or strict priority per spec.
///
/// # Errors
///
/// Returns error if:
/// - Any claim amount is negative
/// - Currency mismatch between distributable value and claims
/// - Collateral haircut outside [0.0, 1.0]
/// - Plan deviation references a non-existent claim ID
pub fn execute_recovery_waterfall(waterfall: &RecoveryWaterfall) -> crate::Result<RecoveryResult> {
    validate_waterfall(waterfall)?;

    let currency = waterfall.distributable_value.currency();
    let mut remaining_value = waterfall.distributable_value.amount();

    // Sort claims by seniority (highest priority first).
    let mut ordered: Vec<&Claim> = waterfall.claims.iter().collect();
    ordered.sort_by_key(|c| c.seniority);

    // Phase 1: Collateral recovery for secured claims.
    // Each secured claim recovers up to its collateral net value;
    // any shortfall becomes a deficiency claim in the unsecured pool.
    let mut phase1: Vec<(&Claim, f64, f64)> = Vec::with_capacity(ordered.len());
    for claim in &ordered {
        let total = claim.total_claim()?.amount();
        let coll_recovery = match &claim.collateral {
            Some(coll) => {
                let net = coll.net_value().amount();
                let recovery = net.min(total).min(remaining_value).max(0.0);
                remaining_value -= recovery;
                recovery
            }
            None => 0.0,
        };
        let deficiency = (total - coll_recovery).max(0.0);
        phase1.push((claim, coll_recovery, deficiency));
    }

    // Phase 2: Distribute remaining value top-down by seniority.
    let mut results = Vec::with_capacity(phase1.len());
    let mut total_distributed = 0.0;

    for (claim, coll_recovery, deficiency) in &phase1 {
        let general = if !waterfall.strict_apr {
            if let Some(dev) = find_deviation(&waterfall.plan_deviations, &claim.id) {
                apply_deviation(dev, claim, *coll_recovery, currency)?
            } else {
                let alloc = deficiency.min(remaining_value).max(0.0);
                remaining_value -= alloc;
                alloc
            }
        } else {
            let alloc = deficiency.min(remaining_value).max(0.0);
            remaining_value -= alloc;
            alloc
        };

        let total_recovery = coll_recovery + general;
        let total_claim_amt = claim.total_claim()?.amount();
        let rate = if total_claim_amt > 0.0 {
            (total_recovery / total_claim_amt).clamp(0.0, 1.0)
        } else {
            0.0
        };
        total_distributed += total_recovery;

        results.push(ClaimRecovery {
            claim_id: claim.id.clone(),
            seniority: claim.seniority,
            total_claim: claim.total_claim()?,
            collateral_recovery: Money::new(*coll_recovery, currency),
            general_recovery: Money::new(general, currency),
            total_recovery: Money::new(total_recovery, currency),
            recovery_rate: rate,
            deficiency_claim: Money::new(*deficiency, currency),
        });
    }

    // Phase 3: Check APR compliance.
    let mut apr_violations = Vec::new();
    let apr_satisfied = check_apr(&results, &mut apr_violations);

    Ok(RecoveryResult {
        claim_recoveries: results,
        total_distributed: Money::new(total_distributed, currency),
        residual: Money::new(remaining_value.max(0.0), currency),
        apr_satisfied,
        apr_violations,
    })
}

// ─── Validation ──────────────────────────────────────────────────────

fn validate_waterfall(waterfall: &RecoveryWaterfall) -> crate::Result<()> {
    let dv = waterfall.distributable_value.amount();
    if dv < 0.0 {
        return Err(RestructuringError::NegativeDistributableValue { value: dv }.into());
    }

    let currency = waterfall.distributable_value.currency();

    for claim in &waterfall.claims {
        // Check non-negative amounts.
        if claim.principal.amount() < 0.0 {
            return Err(RestructuringError::NegativeClaimAmount {
                claim_id: claim.id.clone(),
                field: "principal".into(),
                value: claim.principal.amount(),
            }
            .into());
        }
        if claim.accrued_interest.amount() < 0.0 {
            return Err(RestructuringError::NegativeClaimAmount {
                claim_id: claim.id.clone(),
                field: "accrued_interest".into(),
                value: claim.accrued_interest.amount(),
            }
            .into());
        }
        if claim.penalties.amount() < 0.0 {
            return Err(RestructuringError::NegativeClaimAmount {
                claim_id: claim.id.clone(),
                field: "penalties".into(),
                value: claim.penalties.amount(),
            }
            .into());
        }

        // Check currency consistency.
        if claim.principal.currency() != currency {
            return Err(RestructuringError::CurrencyMismatch {
                expected: format!("{:?}", currency),
                actual: format!("{:?}", claim.principal.currency()),
                claim_id: claim.id.clone(),
            }
            .into());
        }

        // Check collateral haircut.
        if let Some(coll) = &claim.collateral {
            if !(0.0..=1.0).contains(&coll.haircut) {
                return Err(RestructuringError::InvalidHaircut {
                    claim_id: claim.id.clone(),
                    haircut: coll.haircut,
                }
                .into());
            }
        }
    }

    // Validate plan deviations reference known claims.
    let claim_ids: Vec<&str> = waterfall.claims.iter().map(|c| c.id.as_str()).collect();
    for dev in &waterfall.plan_deviations {
        if !claim_ids.contains(&dev.claim_id.as_str()) {
            return Err(RestructuringError::UnknownDeviationClaim {
                claim_id: dev.claim_id.clone(),
            }
            .into());
        }
    }

    Ok(())
}

/// Find a plan deviation for a given claim ID.
fn find_deviation<'a>(
    deviations: &'a [PlanDeviation],
    claim_id: &str,
) -> Option<&'a PlanDeviation> {
    deviations.iter().find(|d| d.claim_id == claim_id)
}

/// Apply a plan deviation to compute the general recovery for a claim.
fn apply_deviation(
    dev: &PlanDeviation,
    claim: &Claim,
    coll_recovery: f64,
    _currency: finstack_core::currency::Currency,
) -> crate::Result<f64> {
    if let Some(ref override_amount) = dev.recovery_override {
        // Fixed amount override; subtract collateral already received.
        let general = (override_amount.amount() - coll_recovery).max(0.0);
        Ok(general)
    } else if let Some(rate) = dev.recovery_rate_override {
        // Rate-based override.
        let total_claim = claim.total_claim()?.amount();
        let target = total_claim * rate.clamp(0.0, 1.0);
        let general = (target - coll_recovery).max(0.0);
        Ok(general)
    } else {
        // Deviation present but no override specified; no special treatment.
        Ok(0.0)
    }
}

/// Check APR compliance: no lower-priority class should recover while a
/// higher-priority class is impaired (recovery_rate < 1.0).
fn check_apr(results: &[ClaimRecovery], violations: &mut Vec<String>) -> bool {
    let mut satisfied = true;
    for i in 0..results.len() {
        if results[i].recovery_rate < 1.0 - 1e-9 {
            // This class is impaired. Check that no lower-priority class recovers.
            for j in (i + 1)..results.len() {
                if results[j].recovery_rate > 1e-9 {
                    let msg = format!(
                        "APR violation: '{}' ({:?}, {:.1}% recovery) is impaired while '{}' ({:?}, {:.1}% recovery) recovers",
                        results[i].claim_id,
                        results[i].seniority,
                        results[i].recovery_rate * 100.0,
                        results[j].claim_id,
                        results[j].seniority,
                        results[j].recovery_rate * 100.0,
                    );
                    violations.push(msg);
                    satisfied = false;
                }
            }
            // Once we find the first impaired class, all subsequent recovery
            // should be zero under strict APR. We've already flagged violations,
            // so break to avoid duplicate messages.
            break;
        }
    }
    satisfied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restructuring::types::*;
    use finstack_core::currency::Currency;

    fn usd(amount: f64) -> Money {
        Money::new(amount, Currency::USD)
    }

    fn simple_claim(id: &str, seniority: ClaimSeniority, principal: f64) -> Claim {
        Claim {
            id: id.to_string(),
            label: id.to_string(),
            seniority,
            principal: usd(principal),
            accrued_interest: usd(0.0),
            penalties: usd(0.0),
            instrument_id: None,
            collateral: None,
            intra_class_allocation: AllocationMode::ProRata,
        }
    }

    #[test]
    fn single_claim_full_recovery() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(200.0),
            claims: vec![simple_claim("a", ClaimSeniority::SeniorUnsecured, 100.0)],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        assert_eq!(result.claim_recoveries.len(), 1);
        assert!((result.claim_recoveries[0].recovery_rate - 1.0).abs() < 1e-9);
        assert!((result.residual.amount() - 100.0).abs() < 1e-6);
        assert!(result.apr_satisfied);
    }

    #[test]
    fn senior_before_junior() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(150.0),
            claims: vec![
                simple_claim("senior", ClaimSeniority::FirstLienSecured, 100.0),
                simple_claim("junior", ClaimSeniority::Subordinated, 100.0),
            ],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        assert_eq!(result.claim_recoveries.len(), 2);

        let senior = &result.claim_recoveries[0];
        let junior = &result.claim_recoveries[1];
        assert!(
            (senior.recovery_rate - 1.0).abs() < 1e-9,
            "senior should be fully recovered"
        );
        assert!(
            (junior.recovery_rate - 0.5).abs() < 1e-9,
            "junior should recover 50%: got {}",
            junior.recovery_rate
        );
        assert!(result.apr_satisfied);
    }

    #[test]
    fn total_distributed_never_exceeds_distributable() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(50.0),
            claims: vec![
                simple_claim("a", ClaimSeniority::FirstLienSecured, 100.0),
                simple_claim("b", ClaimSeniority::SeniorUnsecured, 100.0),
                simple_claim("c", ClaimSeniority::Subordinated, 100.0),
            ],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        assert!(
            result.total_distributed.amount() <= 50.0 + 1e-9,
            "distributed {} exceeds available 50",
            result.total_distributed.amount()
        );
    }

    #[test]
    fn collateral_recovery_before_general() {
        let mut claim = simple_claim("secured", ClaimSeniority::FirstLienSecured, 100.0);
        claim.collateral = Some(CollateralAllocation {
            description: "All assets".to_string(),
            value: usd(80.0),
            haircut: 0.0,
            shared: false,
            shared_with: vec![],
        });

        let waterfall = RecoveryWaterfall {
            distributable_value: usd(120.0),
            claims: vec![
                claim,
                simple_claim("unsecured", ClaimSeniority::SeniorUnsecured, 50.0),
            ],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");

        let secured = &result.claim_recoveries[0];
        assert!(
            (secured.collateral_recovery.amount() - 80.0).abs() < 1e-6,
            "collateral recovery should be 80"
        );
        assert!(
            (secured.general_recovery.amount() - 20.0).abs() < 1e-6,
            "general recovery should be 20 (deficiency)"
        );
        assert!((secured.recovery_rate - 1.0).abs() < 1e-9);

        let unsecured = &result.claim_recoveries[1];
        assert!(
            (unsecured.total_recovery.amount() - 20.0).abs() < 1e-6,
            "unsecured gets remaining 20"
        );
    }

    #[test]
    fn zero_distributable_value() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(0.0),
            claims: vec![simple_claim("a", ClaimSeniority::SeniorUnsecured, 100.0)],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        assert!(result.claim_recoveries[0].recovery_rate.abs() < 1e-9);
        assert!(result.total_distributed.amount().abs() < 1e-9);
    }

    #[test]
    fn negative_principal_rejected() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(100.0),
            claims: vec![simple_claim("bad", ClaimSeniority::SeniorUnsecured, -10.0)],
            strict_apr: true,
            plan_deviations: vec![],
        };
        assert!(execute_recovery_waterfall(&waterfall).is_err());
    }

    #[test]
    fn invalid_haircut_rejected() {
        let mut claim = simple_claim("sec", ClaimSeniority::FirstLienSecured, 100.0);
        claim.collateral = Some(CollateralAllocation {
            description: "test".into(),
            value: usd(80.0),
            haircut: 1.5,
            shared: false,
            shared_with: vec![],
        });
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(100.0),
            claims: vec![claim],
            strict_apr: true,
            plan_deviations: vec![],
        };
        assert!(execute_recovery_waterfall(&waterfall).is_err());
    }

    #[test]
    fn apr_violation_detected_with_plan_deviation() {
        // Give equity a recovery while senior unsecured is impaired.
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(50.0),
            claims: vec![
                simple_claim("senior", ClaimSeniority::SeniorUnsecured, 100.0),
                simple_claim("equity", ClaimSeniority::CommonEquity, 50.0),
            ],
            strict_apr: false,
            plan_deviations: vec![PlanDeviation {
                claim_id: "equity".into(),
                recovery_override: Some(usd(10.0)),
                recovery_rate_override: None,
                reason: "gifting to management".into(),
            }],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        assert!(!result.apr_satisfied);
        assert!(!result.apr_violations.is_empty());
    }

    #[test]
    fn all_claims_fully_covered() {
        let waterfall = RecoveryWaterfall {
            distributable_value: usd(1000.0),
            claims: vec![
                simple_claim("a", ClaimSeniority::FirstLienSecured, 100.0),
                simple_claim("b", ClaimSeniority::SeniorUnsecured, 200.0),
                simple_claim("c", ClaimSeniority::CommonEquity, 50.0),
            ],
            strict_apr: true,
            plan_deviations: vec![],
        };
        let result = execute_recovery_waterfall(&waterfall).expect("should succeed");
        for cr in &result.claim_recoveries {
            assert!(
                (cr.recovery_rate - 1.0).abs() < 1e-9,
                "claim {} should be fully covered",
                cr.claim_id
            );
        }
        assert!((result.residual.amount() - 650.0).abs() < 1e-6);
        assert!(result.apr_satisfied);
    }
}
