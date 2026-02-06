//! Governance types and helpers for row-level permissions and workflow.

use crate::{Error, Result};
use std::cmp::Ordering;
use std::env;
use std::sync::OnceLock;

use crate::store::GovernanceStore;
use crate::{
    LookbackStore, MarketContextSnapshot, PortfolioSnapshot, SeriesKey, SeriesKind, Store,
    StoreHandle, TimeSeriesStore,
};
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_portfolio::PortfolioSpec;
use finstack_scenarios::ScenarioSpec;
use finstack_statements::registry::MetricRegistry;
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::InstrumentJson;
use serde_json::Value;
use time::OffsetDateTime;

/// Kind of actor performing a change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorKind {
    /// Human user.
    User,
    /// System principal (service account, ingestion pipeline).
    System,
}

impl ActorKind {
    /// Stable string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ActorKind::User => "USER",
            ActorKind::System => "SYSTEM",
        }
    }

    /// Parse an actor kind from storage.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "USER" => Ok(Self::User),
            "SYSTEM" => Ok(Self::System),
            _ => Err(Error::Invariant(format!("Unknown actor_kind: {value}"))),
        }
    }
}

/// Context for the acting principal.
#[derive(Clone, Debug)]
pub struct ActorContext {
    /// Actor kind.
    pub kind: ActorKind,
    /// Stable identifier for the actor (user id or system principal id).
    pub actor_id: String,
    /// Optional user id when a system actor acts on behalf of a user.
    pub assume_user_id: Option<String>,
}

impl ActorContext {
    /// Create a human actor context.
    pub fn user(actor_id: impl Into<String>) -> Self {
        Self {
            kind: ActorKind::User,
            actor_id: actor_id.into(),
            assume_user_id: None,
        }
    }

    /// Create a system actor context.
    pub fn system(actor_id: impl Into<String>) -> Self {
        Self {
            kind: ActorKind::System,
            actor_id: actor_id.into(),
            assume_user_id: None,
        }
    }
}

/// Visibility scope for a resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisibilityScope {
    /// Owner only.
    Private,
    /// Users with a role.
    Role,
    /// Users in a group.
    Group,
    /// All authenticated users.
    Public,
}

impl VisibilityScope {
    /// Stable string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            VisibilityScope::Private => "PRIVATE",
            VisibilityScope::Role => "ROLE",
            VisibilityScope::Group => "GROUP",
            VisibilityScope::Public => "PUBLIC",
        }
    }

    /// Parse a visibility scope from storage.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "PRIVATE" => Ok(Self::Private),
            "ROLE" => Ok(Self::Role),
            "GROUP" => Ok(Self::Group),
            "PUBLIC" => Ok(Self::Public),
            _ => Err(Error::Invariant(format!(
                "Unknown visibility_scope: {value}"
            ))),
        }
    }
}

/// Share type for explicit grants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShareType {
    /// User principal.
    User,
    /// Role.
    Role,
    /// Group.
    Group,
    /// Role within a specific group.
    RoleInGroup,
}

impl ShareType {
    /// Stable string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ShareType::User => "USER",
            ShareType::Role => "ROLE",
            ShareType::Group => "GROUP",
            ShareType::RoleInGroup => "ROLE_IN_GROUP",
        }
    }

    /// Parse a share type from storage.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "USER" => Ok(Self::User),
            "ROLE" => Ok(Self::Role),
            "GROUP" => Ok(Self::Group),
            "ROLE_IN_GROUP" => Ok(Self::RoleInGroup),
            _ => Err(Error::Invariant(format!("Unknown share_type: {value}"))),
        }
    }
}

/// Permission levels for explicit shares.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SharePermission {
    /// Read-only.
    Read,
    /// Read/write.
    Write,
    /// Administrative control.
    Admin,
}

impl SharePermission {
    /// Stable string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            SharePermission::Read => "READ",
            SharePermission::Write => "WRITE",
            SharePermission::Admin => "ADMIN",
        }
    }

    /// Parse permission from storage.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "READ" => Ok(Self::Read),
            "WRITE" => Ok(Self::Write),
            "ADMIN" => Ok(Self::Admin),
            _ => Err(Error::Invariant(format!(
                "Unknown share permission: {value}"
            ))),
        }
    }

    /// Returns true if this permission satisfies the required level.
    #[must_use]
    pub fn allows(self, required: SharePermission) -> bool {
        self >= required
    }
}

/// Change kind for proposals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeKind {
    /// Create new resource.
    Create,
    /// Edit existing resource.
    Edit,
    /// System ingestion.
    Ingest,
}

impl ChangeKind {
    /// Stable string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeKind::Create => "CREATE",
            ChangeKind::Edit => "EDIT",
            ChangeKind::Ingest => "INGEST",
        }
    }

    /// Parse change kind from storage.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "CREATE" => Ok(Self::Create),
            "EDIT" => Ok(Self::Edit),
            "INGEST" => Ok(Self::Ingest),
            _ => Err(Error::Invariant(format!("Unknown change_kind: {value}"))),
        }
    }
}

/// Default workflow state names.
pub mod workflow_states {
    /// Draft state.
    pub const DRAFT: &str = "DRAFT";
    /// Pending state.
    pub const PENDING: &str = "PENDING";
    /// Checking state.
    pub const CHECKING: &str = "CHECKING";
    /// Verified state.
    pub const VERIFIED: &str = "VERIFIED";
    /// System-verified state.
    pub const SYSTEM_VERIFIED: &str = "SYSTEM_VERIFIED";
}

/// Auth role membership (optionally scoped to a group).
#[derive(Clone, Debug)]
pub struct UserRole {
    /// Role id.
    pub role_id: String,
    /// Optional group id for group-scoped roles.
    pub group_id: Option<String>,
}

/// Resource entity metadata for authorization.
#[derive(Clone, Debug)]
pub struct ResourceEntity {
    /// Resource type.
    pub resource_type: String,
    /// Resource id.
    pub resource_id: String,
    /// Owner user id.
    pub owner_user_id: String,
    /// Visibility scope.
    pub visibility_scope: VisibilityScope,
    /// Visibility id (role or group).
    pub visibility_id: Option<String>,
}

/// Explicit share grant.
#[derive(Clone, Debug)]
pub struct ResourceShare {
    /// Resource type.
    pub resource_type: String,
    /// Resource id.
    pub resource_id: String,
    /// Share type.
    pub share_type: ShareType,
    /// Share id.
    pub share_id: String,
    /// Optional share scope id (for role-in-group).
    pub share_scope_id: Option<String>,
    /// Permission level.
    pub permission: SharePermission,
}

/// Workflow policy binding.
#[derive(Clone, Debug)]
pub struct WorkflowBinding {
    /// Binding id.
    pub id: String,
    /// Resource type.
    pub resource_type: String,
    /// Optional visibility scope.
    pub visibility_scope: Option<String>,
    /// Optional visibility id.
    pub visibility_id: Option<String>,
    /// Optional change kind.
    pub change_kind: Option<String>,
    /// Optional base verified source.
    pub base_verified_source: Option<String>,
    /// Policy id.
    pub policy_id: String,
    /// Priority (higher wins).
    pub priority: i32,
}

/// Workflow transition definition.
#[derive(Clone, Debug)]
pub struct WorkflowTransition {
    /// Transition id.
    pub id: String,
    /// Policy id.
    pub policy_id: String,
    /// From state.
    pub from_state: String,
    /// To state.
    pub to_state: String,
    /// Required role id.
    pub required_role_id: Option<String>,
    /// Required group id.
    pub required_group_id: Option<String>,
    /// Allow owner to perform transition.
    pub allow_owner: bool,
    /// Allow system actor.
    pub allow_system_actor: bool,
    /// Require verifier not owner.
    pub require_verifier_not_owner: bool,
    /// Require verifier not submitter.
    pub require_verifier_not_submitter: bool,
    /// Require distinct from last transition actor.
    pub require_distinct_from_last_actor: bool,
}

/// Workflow state definition.
#[derive(Clone, Debug)]
pub struct WorkflowState {
    /// Policy id.
    pub policy_id: String,
    /// State key.
    pub state_key: String,
    /// Whether this is a final state.
    pub is_final: bool,
    /// Optional verified source.
    pub verified_source: Option<String>,
    /// System-only.
    pub system_only: bool,
    /// Category.
    pub category: String,
}

/// A change proposal row.
#[derive(Clone, Debug)]
pub struct ResourceChange {
    /// Change id.
    pub change_id: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource id.
    pub resource_id: String,
    /// Optional secondary key (e.g., as_of).
    pub resource_key2: String,
    /// Change kind.
    pub change_kind: ChangeKind,
    /// Workflow policy id.
    pub workflow_policy_id: Option<String>,
    /// Workflow state.
    pub workflow_state: String,
    /// Owner user id.
    pub owner_user_id: String,
    /// Created by kind.
    pub created_by_kind: ActorKind,
    /// Created by id.
    pub created_by_id: String,
    /// Submitted timestamp (ISO string).
    pub submitted_at: Option<String>,
    /// Applied timestamp (ISO string).
    pub applied_at: Option<String>,
    /// Base etag.
    pub base_etag: Option<String>,
    /// Ingestion source.
    pub ingestion_source: Option<String>,
    /// Ingestion run id.
    pub ingestion_run_id: Option<String>,
    /// Resource payload as JSON.
    ///
    /// Must use the same serialization format as the corresponding `put_*`
    /// method for the resource type. See
    /// [`GovernanceStore::apply_change_to_verified`](crate::store::GovernanceStore::apply_change_to_verified)
    /// for the expected types per resource.
    pub payload: serde_json::Value,
    /// Metadata.
    pub meta: serde_json::Value,
}

/// Input for inserting a change proposal.
#[derive(Clone, Debug)]
pub struct ResourceChangeInsert {
    /// Change id.
    pub change_id: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource id.
    pub resource_id: String,
    /// Optional secondary key (e.g., as_of).
    pub resource_key2: String,
    /// Change kind.
    pub change_kind: ChangeKind,
    /// Workflow policy id.
    pub workflow_policy_id: Option<String>,
    /// Workflow state.
    pub workflow_state: String,
    /// Owner user id.
    pub owner_user_id: String,
    /// Created by kind.
    pub created_by_kind: ActorKind,
    /// Created by id.
    pub created_by_id: String,
    /// Ingestion source.
    pub ingestion_source: Option<String>,
    /// Ingestion run id.
    pub ingestion_run_id: Option<String>,
    /// Resource payload as JSON.
    ///
    /// Must use the same serialization format as the corresponding `put_*`
    /// method for the resource type. See
    /// [`GovernanceStore::apply_change_to_verified`](crate::store::GovernanceStore::apply_change_to_verified)
    /// for the expected types per resource.
    pub payload: serde_json::Value,
    /// Metadata.
    pub meta: serde_json::Value,
}

/// Evaluate whether the actor has at least the required permission on a resource.
///
/// Checks in order: owner, admin override, visibility scope, then explicit shares.
#[must_use]
pub fn can_access_resource(
    entity: &ResourceEntity,
    shares: &[ResourceShare],
    actor: &ActorContext,
    roles: &[UserRole],
    groups: &[String],
    admin_roles: &[String],
    required: SharePermission,
) -> bool {
    if actor.actor_id == entity.owner_user_id {
        return true;
    }

    if is_admin(roles, admin_roles) {
        return true;
    }

    // Visibility scope grants implicit read access only.
    if required == SharePermission::Read {
        match entity.visibility_scope {
            VisibilityScope::Private => {}
            VisibilityScope::Public => return true,
            VisibilityScope::Role => {
                if let Some(role_id) = entity.visibility_id.as_deref() {
                    if roles.iter().any(|r| r.role_id == role_id) {
                        return true;
                    }
                }
            }
            VisibilityScope::Group => {
                if let Some(group_id) = entity.visibility_id.as_deref() {
                    if groups.iter().any(|g| g == group_id) {
                        return true;
                    }
                }
            }
        }
    }

    shares.iter().any(|share| {
        if !share.permission.allows(required) {
            return false;
        }
        match share.share_type {
            ShareType::User => share.share_id == actor.actor_id,
            ShareType::Role => roles.iter().any(|r| r.role_id == share.share_id),
            ShareType::Group => groups.iter().any(|g| g == &share.share_id),
            ShareType::RoleInGroup => {
                let role_match = roles.iter().any(|r| r.role_id == share.share_id);
                let group_match = share
                    .share_scope_id
                    .as_deref()
                    .map(|gid| groups.iter().any(|g| g == gid))
                    .unwrap_or(false);
                role_match && group_match
            }
        }
    })
}

/// Evaluate whether the actor can read a resource.
#[must_use]
pub fn can_read_resource(
    entity: &ResourceEntity,
    shares: &[ResourceShare],
    actor: &ActorContext,
    roles: &[UserRole],
    groups: &[String],
    admin_roles: &[String],
) -> bool {
    can_access_resource(
        entity,
        shares,
        actor,
        roles,
        groups,
        admin_roles,
        SharePermission::Read,
    )
}

/// Evaluate whether the actor can write to a resource.
#[must_use]
pub fn can_write_resource(
    entity: &ResourceEntity,
    shares: &[ResourceShare],
    actor: &ActorContext,
    roles: &[UserRole],
    groups: &[String],
    admin_roles: &[String],
) -> bool {
    can_access_resource(
        entity,
        shares,
        actor,
        roles,
        groups,
        admin_roles,
        SharePermission::Write,
    )
}

/// Evaluate whether the actor has administrative privileges.
#[must_use]
pub fn is_admin(roles: &[UserRole], admin_roles: &[String]) -> bool {
    if admin_roles.is_empty() {
        return false;
    }
    roles
        .iter()
        .any(|role| admin_roles.iter().any(|admin| admin == &role.role_id))
}

/// Select the best workflow binding given criteria.
#[must_use]
pub fn select_workflow_binding<'a>(
    bindings: &'a [WorkflowBinding],
    resource_type: &str,
    visibility_scope: Option<&str>,
    visibility_id: Option<&str>,
    change_kind: Option<&str>,
    base_verified_source: Option<&str>,
) -> Option<&'a WorkflowBinding> {
    bindings
        .iter()
        .filter(|binding| binding.resource_type == resource_type)
        .filter(|binding| match_field(binding.visibility_scope.as_deref(), visibility_scope))
        .filter(|binding| match_field(binding.visibility_id.as_deref(), visibility_id))
        .filter(|binding| match_field(binding.change_kind.as_deref(), change_kind))
        .filter(|binding| {
            match_field(
                binding.base_verified_source.as_deref(),
                base_verified_source,
            )
        })
        .max_by(|a, b| compare_binding(a, b))
}

fn match_field(binding_value: Option<&str>, target: Option<&str>) -> bool {
    match binding_value {
        Some(expected) => target.map(|v| v == expected).unwrap_or(false),
        None => true,
    }
}

fn compare_binding(a: &WorkflowBinding, b: &WorkflowBinding) -> Ordering {
    let priority = a.priority.cmp(&b.priority);
    if priority != Ordering::Equal {
        return priority;
    }
    let spec_a = specificity(a);
    let spec_b = specificity(b);
    spec_a.cmp(&spec_b)
}

fn specificity(binding: &WorkflowBinding) -> usize {
    let mut score = 0;
    if binding.visibility_scope.is_some() {
        score += 1;
    }
    if binding.visibility_id.is_some() {
        score += 1;
    }
    if binding.change_kind.is_some() {
        score += 1;
    }
    if binding.base_verified_source.is_some() {
        score += 1;
    }
    score
}

/// Validate whether an actor can perform a transition.
pub fn validate_transition(
    transition: &WorkflowTransition,
    actor: &ActorContext,
    owner_user_id: &str,
    submitter_id: Option<&str>,
    roles: &[UserRole],
    groups: &[String],
    last_actor_id: Option<&str>,
) -> Result<()> {
    if actor.kind == ActorKind::System {
        if !transition.allow_system_actor {
            return Err(Error::PermissionDenied {
                action: "transition",
                resource_type: "workflow".to_string(),
                resource_id: transition.id.clone(),
            });
        }
    } else {
        let mut allowed = transition.allow_owner && actor.actor_id == owner_user_id;
        if let Some(role_id) = transition.required_role_id.as_deref() {
            let role_match = roles.iter().any(|role| role.role_id == role_id);
            let group_match = transition
                .required_group_id
                .as_deref()
                .map(|gid| groups.iter().any(|g| g == gid))
                .unwrap_or(true);
            if role_match && group_match {
                allowed = true;
            }
        } else if let Some(group_id) = transition.required_group_id.as_deref() {
            if groups.iter().any(|g| g == group_id) {
                allowed = true;
            }
        }

        if !allowed {
            return Err(Error::PermissionDenied {
                action: "transition",
                resource_type: "workflow".to_string(),
                resource_id: transition.id.clone(),
            });
        }
    }

    if transition.require_verifier_not_owner && actor.actor_id == owner_user_id {
        return Err(Error::PermissionDenied {
            action: "transition",
            resource_type: "workflow".to_string(),
            resource_id: transition.id.clone(),
        });
    }

    if transition.require_verifier_not_submitter {
        if let Some(submitter) = submitter_id {
            if submitter == actor.actor_id {
                return Err(Error::PermissionDenied {
                    action: "transition",
                    resource_type: "workflow".to_string(),
                    resource_id: transition.id.clone(),
                });
            }
        }
    }

    if transition.require_distinct_from_last_actor {
        if let Some(last_actor) = last_actor_id {
            if last_actor == actor.actor_id {
                return Err(Error::PermissionDenied {
                    action: "transition",
                    resource_type: "workflow".to_string(),
                    resource_id: transition.id.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Governance configuration loaded from environment.
#[derive(Clone, Debug)]
pub struct GovernanceConfig {
    /// Whether governance enforcement is enabled.
    pub enabled: bool,
    /// Role identifiers that act as admin overrides.
    pub admin_role_ids: Vec<String>,
}

impl GovernanceConfig {
    /// Load governance configuration from environment variables.
    ///
    /// Reads:
    /// - `FINSTACK_IO_GOVERNANCE` (on/off, default off)
    /// - `FINSTACK_IO_ADMIN_ROLES` (comma-separated role ids)
    pub fn from_env() -> Self {
        let enabled = env::var("FINSTACK_IO_GOVERNANCE")
            .ok()
            .map(|value| value.trim().eq_ignore_ascii_case("on"))
            .unwrap_or(false);

        let admin_role_ids = env::var("FINSTACK_IO_ADMIN_ROLES")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|part| part.trim().to_string())
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            enabled,
            admin_role_ids,
        }
    }
}

/// Canonical resource type identifiers.
pub mod resource_types {
    /// Instrument definitions.
    pub const INSTRUMENT: &str = "instrument";
    /// Portfolio specs (all as-of snapshots share the same resource id).
    pub const PORTFOLIO: &str = "portfolio";
    /// Market context snapshots.
    pub const MARKET_CONTEXT: &str = "market_context";
    /// Scenario definitions.
    pub const SCENARIO: &str = "scenario";
    /// Statement models.
    pub const STATEMENT_MODEL: &str = "statement_model";
    /// Metric registry namespace.
    pub const METRIC_REGISTRY: &str = "metric_registry";
    /// Time-series series metadata.
    pub const SERIES_META: &str = "series_meta";
}

/// Cached governance configuration, loaded from environment on first access.
static GOVERNANCE_CONFIG: OnceLock<GovernanceConfig> = OnceLock::new();

/// Validate that a change's payload can deserialize as the expected domain type.
///
/// This catches payload format mismatches early (e.g., providing a raw
/// `MarketContext` instead of `MarketContextState`) that would cause silent
/// corruption in the verified tables.
///
/// Only compiled in debug builds to avoid redundant deserialization in release.
#[cfg(debug_assertions)]
fn validate_change_payload(change: &ResourceChange) -> Result<()> {
    use finstack_core::market_data::context::MarketContextState;

    match change.resource_type.as_str() {
        resource_types::INSTRUMENT => {
            serde_json::from_value::<InstrumentJson>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for instrument '{}': {e}. \
                     Payload must be a serialized InstrumentJson.",
                    change.resource_id
                ))
            })?;
        }
        resource_types::MARKET_CONTEXT => {
            serde_json::from_value::<MarketContextState>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for market_context '{}': {e}. \
                     Payload must be a serialized MarketContextState (not MarketContext).",
                    change.resource_id
                ))
            })?;
        }
        resource_types::PORTFOLIO => {
            serde_json::from_value::<PortfolioSpec>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for portfolio '{}': {e}. \
                     Payload must be a serialized PortfolioSpec.",
                    change.resource_id
                ))
            })?;
        }
        resource_types::SCENARIO => {
            serde_json::from_value::<ScenarioSpec>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for scenario '{}': {e}. \
                     Payload must be a serialized ScenarioSpec.",
                    change.resource_id
                ))
            })?;
        }
        resource_types::STATEMENT_MODEL => {
            serde_json::from_value::<FinancialModelSpec>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for statement_model '{}': {e}. \
                     Payload must be a serialized FinancialModelSpec.",
                    change.resource_id
                ))
            })?;
        }
        resource_types::METRIC_REGISTRY => {
            serde_json::from_value::<MetricRegistry>(change.payload.clone()).map_err(|e| {
                Error::Invariant(format!(
                    "Invalid payload for metric_registry '{}': {e}. \
                     Payload must be a serialized MetricRegistry.",
                    change.resource_id
                ))
            })?;
        }
        resource_types::SERIES_META => {
            // Series meta payloads are arbitrary JSON metadata; no schema validation needed.
        }
        other => {
            return Err(Error::Invariant(format!(
                "Cannot validate payload for unknown resource_type: {other}"
            )));
        }
    }
    Ok(())
}

/// Governed API wrapper around a store handle.
#[derive(Clone, Debug)]
pub struct GovernedHandle {
    store: StoreHandle,
    actor: ActorContext,
    config: GovernanceConfig,
}

impl GovernedHandle {
    /// Create a governed handle using environment configuration.
    ///
    /// Configuration is read from environment variables once per process
    /// and cached for subsequent calls. Use [`GovernedHandle::with_config`]
    /// to override with explicit configuration (e.g., in tests).
    pub fn new(store: StoreHandle, actor: ActorContext) -> Self {
        let config = GOVERNANCE_CONFIG
            .get_or_init(GovernanceConfig::from_env)
            .clone();
        Self {
            store,
            actor,
            config,
        }
    }

    /// Create a governed handle with explicit configuration.
    pub fn with_config(store: StoreHandle, actor: ActorContext, config: GovernanceConfig) -> Self {
        Self {
            store,
            actor,
            config,
        }
    }

    /// Access the actor context.
    #[must_use]
    pub fn actor(&self) -> &ActorContext {
        &self.actor
    }

    async fn ensure_can_read(&self, resource_type: &str, resource_id: &str) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let entity = self
            .store
            .get_resource_entity(resource_type, resource_id)
            .await?
            .ok_or_else(|| Error::not_found("resource_entity", resource_id))?;

        let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
        let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
        let shares = self
            .store
            .list_resource_shares(resource_type, resource_id)
            .await?;

        if can_read_resource(
            &entity,
            &shares,
            &self.actor,
            &roles,
            &groups,
            &self.config.admin_role_ids,
        ) {
            Ok(())
        } else {
            Err(Error::PermissionDenied {
                action: "read",
                resource_type: resource_type.to_string(),
                resource_id: resource_id.to_string(),
            })
        }
    }

    /// Filter a list of resource entities to only those the current actor can read.
    ///
    /// Uses a single batch query for shares instead of per-entity queries.
    async fn filter_readable_ids(
        &self,
        resource_type: &str,
        entities: Vec<ResourceEntity>,
    ) -> Result<Vec<String>> {
        let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
        let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
        let shares_map = self.store.list_all_resource_shares(resource_type).await?;
        let empty_shares = Vec::new();
        let mut out = Vec::new();
        for entity in entities {
            let shares = shares_map.get(&entity.resource_id).unwrap_or(&empty_shares);
            if can_read_resource(
                &entity,
                shares,
                &self.actor,
                &roles,
                &groups,
                &self.config.admin_role_ids,
            ) {
                out.push(entity.resource_id);
            }
        }
        Ok(out)
    }

    /// Load an instrument definition (governed).
    pub async fn get_instrument(&self, instrument_id: &str) -> Result<Option<InstrumentJson>> {
        self.ensure_can_read(resource_types::INSTRUMENT, instrument_id)
            .await?;
        self.store.get_instrument(instrument_id).await
    }

    /// List instrument ids (governed).
    pub async fn list_instruments(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return self.store.list_instruments().await;
        }
        let entities = self
            .store
            .list_resource_entities(resource_types::INSTRUMENT)
            .await?;
        self.filter_readable_ids(resource_types::INSTRUMENT, entities)
            .await
    }

    /// Load a market context snapshot (governed).
    pub async fn get_market_context(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContext>> {
        self.ensure_can_read(resource_types::MARKET_CONTEXT, market_id)
            .await?;
        self.store.get_market_context(market_id, as_of).await
    }

    /// Load a portfolio spec snapshot (governed).
    pub async fn get_portfolio_spec(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSpec>> {
        self.ensure_can_read(resource_types::PORTFOLIO, portfolio_id)
            .await?;
        self.store.get_portfolio_spec(portfolio_id, as_of).await
    }

    /// Load a scenario specification (governed).
    pub async fn get_scenario(&self, scenario_id: &str) -> Result<Option<ScenarioSpec>> {
        self.ensure_can_read(resource_types::SCENARIO, scenario_id)
            .await?;
        self.store.get_scenario(scenario_id).await
    }

    /// List scenario ids (governed).
    pub async fn list_scenarios(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return self.store.list_scenarios().await;
        }
        let entities = self
            .store
            .list_resource_entities(resource_types::SCENARIO)
            .await?;
        self.filter_readable_ids(resource_types::SCENARIO, entities)
            .await
    }

    /// Load a statement model specification (governed).
    pub async fn get_statement_model(&self, model_id: &str) -> Result<Option<FinancialModelSpec>> {
        self.ensure_can_read(resource_types::STATEMENT_MODEL, model_id)
            .await?;
        self.store.get_statement_model(model_id).await
    }

    /// List statement model ids (governed).
    pub async fn list_statement_models(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return self.store.list_statement_models().await;
        }
        let entities = self
            .store
            .list_resource_entities(resource_types::STATEMENT_MODEL)
            .await?;
        self.filter_readable_ids(resource_types::STATEMENT_MODEL, entities)
            .await
    }

    /// Load a metric registry (governed).
    pub async fn get_metric_registry(&self, namespace: &str) -> Result<Option<MetricRegistry>> {
        self.ensure_can_read(resource_types::METRIC_REGISTRY, namespace)
            .await?;
        self.store.get_metric_registry(namespace).await
    }

    /// List metric registry namespaces (governed).
    pub async fn list_metric_registries(&self) -> Result<Vec<String>> {
        if !self.config.enabled {
            return self.store.list_metric_registries().await;
        }
        let entities = self
            .store
            .list_resource_entities(resource_types::METRIC_REGISTRY)
            .await?;
        self.filter_readable_ids(resource_types::METRIC_REGISTRY, entities)
            .await
    }

    /// Load series metadata (governed).
    pub async fn get_series_meta(&self, key: &SeriesKey) -> Result<Option<serde_json::Value>> {
        let resource_id = series_resource_id(key);
        self.ensure_can_read(resource_types::SERIES_META, &resource_id)
            .await?;
        self.store.get_series_meta(key).await
    }

    /// List series ids (governed).
    pub async fn list_series(&self, namespace: &str, kind: SeriesKind) -> Result<Vec<String>> {
        if !self.config.enabled {
            return self.store.list_series(namespace, kind).await;
        }

        let series_ids = self.store.list_series(namespace, kind).await?;
        let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
        let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
        let entities = self
            .store
            .list_resource_entities(resource_types::SERIES_META)
            .await?;
        let shares_map = self
            .store
            .list_all_resource_shares(resource_types::SERIES_META)
            .await?;
        let empty_shares = Vec::new();
        // Build a lookup for resource entities by resource_id.
        let entity_map: std::collections::HashMap<&str, &ResourceEntity> = entities
            .iter()
            .map(|e| (e.resource_id.as_str(), e))
            .collect();
        let mut out = Vec::new();
        for series_id in series_ids {
            let resource_id = format!("{}:{}:{}", namespace, kind.as_str(), series_id);
            if let Some(entity) = entity_map.get(resource_id.as_str()) {
                let shares = shares_map.get(&resource_id).unwrap_or(&empty_shares);
                if can_read_resource(
                    entity,
                    shares,
                    &self.actor,
                    &roles,
                    &groups,
                    &self.config.admin_role_ids,
                ) {
                    out.push(series_id);
                }
            }
        }
        Ok(out)
    }

    /// List market context snapshots (governed).
    pub async fn list_market_contexts(
        &self,
        market_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<MarketContextSnapshot>> {
        self.ensure_can_read(resource_types::MARKET_CONTEXT, market_id)
            .await?;
        self.store.list_market_contexts(market_id, start, end).await
    }

    /// List portfolio snapshots (governed).
    pub async fn list_portfolios(
        &self,
        portfolio_id: &str,
        start: Date,
        end: Date,
    ) -> Result<Vec<PortfolioSnapshot>> {
        self.ensure_can_read(resource_types::PORTFOLIO, portfolio_id)
            .await?;
        self.store.list_portfolios(portfolio_id, start, end).await
    }

    /// Get the latest market context snapshot (governed).
    pub async fn latest_market_context_on_or_before(
        &self,
        market_id: &str,
        as_of: Date,
    ) -> Result<Option<MarketContextSnapshot>> {
        self.ensure_can_read(resource_types::MARKET_CONTEXT, market_id)
            .await?;
        self.store
            .latest_market_context_on_or_before(market_id, as_of)
            .await
    }

    /// Get the latest portfolio snapshot (governed).
    pub async fn latest_portfolio_on_or_before(
        &self,
        portfolio_id: &str,
        as_of: Date,
    ) -> Result<Option<PortfolioSnapshot>> {
        self.ensure_can_read(resource_types::PORTFOLIO, portfolio_id)
            .await?;
        self.store
            .latest_portfolio_on_or_before(portfolio_id, as_of)
            .await
    }

    /// Upsert the visibility settings for a resource.
    pub async fn set_visibility(
        &self,
        resource_type: &str,
        resource_id: &str,
        visibility_scope: VisibilityScope,
        visibility_id: Option<&str>,
    ) -> Result<()> {
        let existing = self
            .store
            .get_resource_entity(resource_type, resource_id)
            .await?;

        if let Some(entity) = existing.as_ref() {
            let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
            let is_admin = is_admin(&roles, &self.config.admin_role_ids);
            if entity.owner_user_id != self.actor.actor_id && !is_admin {
                return Err(Error::PermissionDenied {
                    action: "set_visibility",
                    resource_type: resource_type.to_string(),
                    resource_id: resource_id.to_string(),
                });
            }
        }

        let owner_user_id = existing
            .as_ref()
            .map(|entity| entity.owner_user_id.clone())
            .unwrap_or_else(|| self.actor.actor_id.clone());

        let entity = ResourceEntity {
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            owner_user_id,
            visibility_scope,
            visibility_id: visibility_id.map(|value| value.to_string()),
        };

        self.store.upsert_resource_entity(&entity).await
    }

    /// Create a draft change proposal.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_draft_change(
        &self,
        resource_type: &str,
        resource_id: &str,
        resource_key2: Option<&str>,
        change_kind: ChangeKind,
        payload: Value,
        meta: Option<Value>,
        visibility_scope: Option<VisibilityScope>,
        visibility_id: Option<&str>,
    ) -> Result<String> {
        if self.actor.kind != ActorKind::User {
            return Err(Error::PermissionDenied {
                action: "create_draft",
                resource_type: resource_type.to_string(),
                resource_id: resource_id.to_string(),
            });
        }
        if change_kind == ChangeKind::Ingest {
            return Err(Error::PermissionDenied {
                action: "create_draft",
                resource_type: resource_type.to_string(),
                resource_id: resource_id.to_string(),
            });
        }

        let existing = self
            .store
            .get_resource_entity(resource_type, resource_id)
            .await?;

        if let Some(entity) = existing.as_ref() {
            let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
            let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
            let shares = self
                .store
                .list_resource_shares(resource_type, resource_id)
                .await?;
            if !can_write_resource(
                entity,
                &shares,
                &self.actor,
                &roles,
                &groups,
                &self.config.admin_role_ids,
            ) {
                return Err(Error::PermissionDenied {
                    action: "create_draft",
                    resource_type: resource_type.to_string(),
                    resource_id: resource_id.to_string(),
                });
            }
            if change_kind == ChangeKind::Create {
                return Err(Error::Invariant(format!(
                    "Resource already exists for create: {resource_type}/{resource_id}"
                )));
            }
        } else if change_kind == ChangeKind::Edit {
            return Err(Error::not_found(
                "resource",
                format!("{resource_type}/{resource_id}"),
            ));
        } else {
            let scope = visibility_scope.unwrap_or(VisibilityScope::Private);
            self.set_visibility(resource_type, resource_id, scope, visibility_id)
                .await?;
        }

        let change_id = generate_id("chg", &self.actor);
        let insert = ResourceChangeInsert {
            change_id: change_id.clone(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            resource_key2: resource_key2.unwrap_or("").to_string(),
            change_kind,
            workflow_policy_id: None,
            workflow_state: workflow_states::DRAFT.to_string(),
            owner_user_id: self.actor.actor_id.clone(),
            created_by_kind: self.actor.kind,
            created_by_id: self.actor.actor_id.clone(),
            ingestion_source: None,
            ingestion_run_id: None,
            payload,
            meta: meta.unwrap_or_else(|| serde_json::json!({})),
        };

        self.store.insert_resource_change(&insert).await?;
        Ok(change_id)
    }

    /// Submit a draft change for review (DRAFT -> PENDING).
    pub async fn submit_change(&self, change_id: &str) -> Result<()> {
        let change = self
            .store
            .get_resource_change(change_id)
            .await?
            .ok_or_else(|| Error::not_found("resource_change", change_id))?;

        if change.workflow_state != workflow_states::DRAFT {
            return Err(Error::Invariant(format!(
                "Change is not in DRAFT: {change_id}"
            )));
        }

        if change.owner_user_id != self.actor.actor_id {
            return Err(Error::PermissionDenied {
                action: "submit_change",
                resource_type: change.resource_type.clone(),
                resource_id: change.resource_id.clone(),
            });
        }

        let entity = self
            .store
            .get_resource_entity(&change.resource_type, &change.resource_id)
            .await?
            .ok_or_else(|| Error::not_found("resource_entity", change.resource_id.clone()))?;

        let bindings = self
            .store
            .list_workflow_bindings(&change.resource_type)
            .await?;

        let base_state = self
            .store
            .latest_verified_state(&change.resource_type, &change.resource_id)
            .await?;
        let base_verified_source = match base_state.as_deref() {
            Some(workflow_states::SYSTEM_VERIFIED) => Some("SYSTEM"),
            Some(workflow_states::VERIFIED) => Some("HUMAN"),
            _ => None,
        };

        let binding = select_workflow_binding(
            &bindings,
            &change.resource_type,
            Some(entity.visibility_scope.as_str()),
            entity.visibility_id.as_deref(),
            Some(change.change_kind.as_str()),
            base_verified_source,
        )
        .ok_or_else(|| Error::Invariant("No workflow binding found".to_string()))?;

        let transition = self
            .store
            .get_workflow_transition(
                &binding.policy_id,
                &change.workflow_state,
                workflow_states::PENDING,
            )
            .await?
            .ok_or_else(|| Error::Invariant("Missing workflow transition".to_string()))?;

        let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
        let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
        let last_actor = self.store.last_workflow_event_actor(change_id).await?;
        validate_transition(
            &transition,
            &self.actor,
            &change.owner_user_id,
            Some(&change.created_by_id),
            &roles,
            &groups,
            last_actor.as_deref(),
        )?;

        let submitted_at = now_timestamp_string()?;
        self.store
            .update_resource_change_state(
                change_id,
                workflow_states::PENDING,
                Some(&binding.policy_id),
                Some(&submitted_at),
                None,
            )
            .await?;

        let event_id = generate_id("evt", &self.actor);
        self.store
            .insert_workflow_event(
                &event_id,
                change_id,
                &change.resource_type,
                &change.resource_id,
                &change.resource_key2,
                &change.workflow_state,
                workflow_states::PENDING,
                self.actor.kind,
                &self.actor.actor_id,
                None,
            )
            .await?;

        Ok(())
    }

    /// Transition a change to another workflow state.
    ///
    /// # Consistency Warning
    ///
    /// When the target state is final, this method performs three operations
    /// (apply to verified tables, update change state, insert audit event)
    /// **without transactional isolation**. If the process crashes between
    /// steps, the verified table may contain data while the change record
    /// and audit trail are incomplete.
    ///
    // TODO: wrap the three-step finalization in a backend transaction to
    // guarantee atomicity. This requires extending GovernanceStore with a
    // transactional finalize method.
    pub async fn transition_change(&self, change_id: &str, to_state: &str) -> Result<()> {
        let change = self
            .store
            .get_resource_change(change_id)
            .await?
            .ok_or_else(|| Error::not_found("resource_change", change_id))?;

        if change.workflow_state == to_state {
            return Err(Error::Invariant(format!(
                "Change already in state: {to_state}"
            )));
        }

        let policy_id = change
            .workflow_policy_id
            .as_ref()
            .ok_or_else(|| Error::Invariant("Missing workflow policy id".to_string()))?;

        let transition = self
            .store
            .get_workflow_transition(policy_id, &change.workflow_state, to_state)
            .await?
            .ok_or_else(|| Error::Invariant("Missing workflow transition".to_string()))?;

        let target_state = self
            .store
            .get_workflow_state(policy_id, to_state)
            .await?
            .ok_or_else(|| Error::Invariant("Missing workflow state".to_string()))?;

        if target_state.system_only && self.actor.kind != ActorKind::System {
            return Err(Error::PermissionDenied {
                action: "transition_change",
                resource_type: change.resource_type.clone(),
                resource_id: change.resource_id.clone(),
            });
        }

        let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
        let groups = self.store.list_user_groups(&self.actor.actor_id).await?;
        let last_actor = self.store.last_workflow_event_actor(change_id).await?;
        validate_transition(
            &transition,
            &self.actor,
            &change.owner_user_id,
            Some(&change.created_by_id),
            &roles,
            &groups,
            last_actor.as_deref(),
        )?;

        let mut applied_at: Option<String> = None;
        if target_state.is_final {
            #[cfg(debug_assertions)]
            validate_change_payload(&change)?;
            self.store.apply_change_to_verified(&change).await?;
            applied_at = Some(now_timestamp_string()?);
        }

        self.store
            .update_resource_change_state(
                change_id,
                to_state,
                Some(policy_id),
                None,
                applied_at.as_deref(),
            )
            .await?;

        let event_id = generate_id("evt", &self.actor);
        self.store
            .insert_workflow_event(
                &event_id,
                change_id,
                &change.resource_type,
                &change.resource_id,
                &change.resource_key2,
                &change.workflow_state,
                to_state,
                self.actor.kind,
                &self.actor.actor_id,
                None,
            )
            .await?;

        Ok(())
    }

    /// Ingest a change as a system actor, bypassing review into SYSTEM_VERIFIED.
    #[allow(clippy::too_many_arguments)]
    pub async fn ingest_system_change(
        &self,
        resource_type: &str,
        resource_id: &str,
        resource_key2: Option<&str>,
        payload: Value,
        meta: Option<Value>,
        visibility_scope: Option<VisibilityScope>,
        visibility_id: Option<&str>,
    ) -> Result<String> {
        if self.actor.kind != ActorKind::System {
            return Err(Error::PermissionDenied {
                action: "system_ingest",
                resource_type: resource_type.to_string(),
                resource_id: resource_id.to_string(),
            });
        }

        let scope = visibility_scope.unwrap_or(VisibilityScope::Private);
        self.set_visibility(resource_type, resource_id, scope, visibility_id)
            .await?;

        let change_id = generate_id("chg", &self.actor);
        let bindings = self.store.list_workflow_bindings(resource_type).await?;
        let binding = select_workflow_binding(
            &bindings,
            resource_type,
            Some(scope.as_str()),
            visibility_id,
            Some(ChangeKind::Ingest.as_str()),
            Some("SYSTEM"),
        )
        .ok_or_else(|| Error::Invariant("No workflow binding found".to_string()))?;

        let insert = ResourceChangeInsert {
            change_id: change_id.clone(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            resource_key2: resource_key2.unwrap_or("").to_string(),
            change_kind: ChangeKind::Ingest,
            workflow_policy_id: Some(binding.policy_id.clone()),
            workflow_state: workflow_states::DRAFT.to_string(),
            owner_user_id: self
                .actor
                .assume_user_id
                .clone()
                .unwrap_or_else(|| self.actor.actor_id.clone()),
            created_by_kind: self.actor.kind,
            created_by_id: self.actor.actor_id.clone(),
            ingestion_source: Some("system".to_string()),
            ingestion_run_id: None,
            payload,
            meta: meta.unwrap_or_else(|| serde_json::json!({})),
        };

        self.store.insert_resource_change(&insert).await?;
        self.transition_change(&change_id, workflow_states::SYSTEM_VERIFIED)
            .await?;
        Ok(change_id)
    }

    /// Get a change proposal by id.
    ///
    /// Only the change owner or an admin can view a change. When governance is
    /// disabled the check is skipped and all changes are readable.
    pub async fn get_change(&self, change_id: &str) -> Result<Option<ResourceChange>> {
        let change = self.store.get_resource_change(change_id).await?;
        if let Some(ref c) = change {
            if self.config.enabled && c.owner_user_id != self.actor.actor_id {
                let roles = self.store.list_user_roles(&self.actor.actor_id).await?;
                if !is_admin(&roles, &self.config.admin_role_ids) {
                    return Err(Error::PermissionDenied {
                        action: "get_change",
                        resource_type: c.resource_type.clone(),
                        resource_id: c.resource_id.clone(),
                    });
                }
            }
        }
        Ok(change)
    }

    /// List change proposals owned by the current actor.
    pub async fn list_changes_for_owner(&self) -> Result<Vec<ResourceChange>> {
        self.store
            .list_resource_changes_for_owner(&self.actor.actor_id)
            .await
    }
}

fn series_resource_id(key: &SeriesKey) -> String {
    format!("{}:{}:{}", key.namespace, key.kind.as_str(), key.series_id)
}

/// Parse a composite series resource id of the form `namespace:kind:series_id`.
pub(crate) fn parse_series_resource_id(resource_id: &str) -> Result<(String, String, String)> {
    let mut parts = resource_id.splitn(3, ':');
    let namespace = parts.next().unwrap_or_default().to_string();
    let kind = parts.next().unwrap_or_default().to_string();
    let series_id = parts.next().unwrap_or_default().to_string();
    if namespace.is_empty() || kind.is_empty() || series_id.is_empty() {
        return Err(Error::Invariant(format!(
            "Invalid series resource id: {resource_id}"
        )));
    }
    Ok((namespace, kind, series_id))
}

/// Generate a unique, collision-resistant ID with a prefix.
///
/// Format: `{prefix}-{actor_id}-{timestamp_nanos}-{random}`.
/// The random suffix prevents collisions when multiple IDs are generated
/// within the same nanosecond by the same actor.
fn generate_id(prefix: &str, actor: &ActorContext) -> String {
    let nanos = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let random: u32 = random_u32();
    format!("{prefix}-{}-{nanos}-{random:08x}", actor.actor_id)
}

/// Simple random u32 using thread-local state seeded from system time.
///
/// This is not cryptographically secure but sufficient for collision avoidance
/// in ID generation. We avoid pulling in a full `rand` or `uuid` crate dependency.
fn random_u32() -> u32 {
    use std::cell::Cell;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    thread_local! {
        static STATE: Cell<u64> = Cell::new({
            let mut h = DefaultHasher::new();
            std::thread::current().id().hash(&mut h);
            OffsetDateTime::now_utc().unix_timestamp_nanos().hash(&mut h);
            h.finish()
        });
    }

    STATE.with(|cell| {
        // xorshift64 step
        let mut s = cell.get();
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        cell.set(s);
        s as u32
    })
}

fn now_timestamp_string() -> Result<String> {
    crate::helpers::format_timestamp_key(OffsetDateTime::now_utc())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn user_actor(id: &str) -> ActorContext {
        ActorContext::user(id)
    }

    fn system_actor(id: &str) -> ActorContext {
        ActorContext::system(id)
    }

    fn entity(owner: &str, scope: VisibilityScope, vis_id: Option<&str>) -> ResourceEntity {
        ResourceEntity {
            resource_type: "instrument".to_string(),
            resource_id: "inst-1".to_string(),
            owner_user_id: owner.to_string(),
            visibility_scope: scope,
            visibility_id: vis_id.map(|s| s.to_string()),
        }
    }

    fn share(
        share_type: ShareType,
        share_id: &str,
        scope_id: Option<&str>,
        perm: SharePermission,
    ) -> ResourceShare {
        ResourceShare {
            resource_type: "instrument".to_string(),
            resource_id: "inst-1".to_string(),
            share_type,
            share_id: share_id.to_string(),
            share_scope_id: scope_id.map(|s| s.to_string()),
            permission: perm,
        }
    }

    fn role(role_id: &str, group_id: Option<&str>) -> UserRole {
        UserRole {
            role_id: role_id.to_string(),
            group_id: group_id.map(|s| s.to_string()),
        }
    }

    // -----------------------------------------------------------------------
    // is_admin
    // -----------------------------------------------------------------------

    #[test]
    fn is_admin_empty_admin_roles_returns_false() {
        assert!(!is_admin(&[role("editor", None)], &[]));
    }

    #[test]
    fn is_admin_matching_role() {
        let admin_roles = vec!["admin".to_string()];
        assert!(is_admin(&[role("admin", None)], &admin_roles));
    }

    #[test]
    fn is_admin_no_matching_role() {
        let admin_roles = vec!["admin".to_string()];
        assert!(!is_admin(&[role("editor", None)], &admin_roles));
    }

    #[test]
    fn is_admin_empty_user_roles() {
        let admin_roles = vec!["admin".to_string()];
        assert!(!is_admin(&[], &admin_roles));
    }

    // -----------------------------------------------------------------------
    // can_access_resource (read)
    // -----------------------------------------------------------------------

    #[test]
    fn owner_can_always_read() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("alice");
        assert!(can_read_resource(&e, &[], &actor, &[], &[], &[]));
    }

    #[test]
    fn non_owner_cannot_read_private() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        assert!(!can_read_resource(&e, &[], &actor, &[], &[], &[]));
    }

    #[test]
    fn admin_can_read_private() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let admin_roles = vec!["admin".to_string()];
        let roles = vec![role("admin", None)];
        assert!(can_read_resource(
            &e,
            &[],
            &actor,
            &roles,
            &[],
            &admin_roles
        ));
    }

    #[test]
    fn public_scope_grants_read_to_anyone() {
        let e = entity("alice", VisibilityScope::Public, None);
        let actor = user_actor("eve");
        assert!(can_read_resource(&e, &[], &actor, &[], &[], &[]));
    }

    #[test]
    fn public_scope_does_not_grant_write() {
        let e = entity("alice", VisibilityScope::Public, None);
        let actor = user_actor("eve");
        assert!(!can_write_resource(&e, &[], &actor, &[], &[], &[]));
    }

    #[test]
    fn role_scope_grants_read_to_matching_role() {
        let e = entity("alice", VisibilityScope::Role, Some("analyst"));
        let actor = user_actor("bob");
        let roles = vec![role("analyst", None)];
        assert!(can_read_resource(&e, &[], &actor, &roles, &[], &[]));
    }

    #[test]
    fn role_scope_does_not_grant_write() {
        let e = entity("alice", VisibilityScope::Role, Some("analyst"));
        let actor = user_actor("bob");
        let roles = vec![role("analyst", None)];
        assert!(!can_write_resource(&e, &[], &actor, &roles, &[], &[]));
    }

    #[test]
    fn group_scope_grants_read_to_member() {
        let e = entity("alice", VisibilityScope::Group, Some("team-a"));
        let actor = user_actor("bob");
        let groups = vec!["team-a".to_string()];
        assert!(can_read_resource(&e, &[], &actor, &[], &groups, &[]));
    }

    // -----------------------------------------------------------------------
    // can_access_resource (shares)
    // -----------------------------------------------------------------------

    #[test]
    fn user_share_read_grants_read() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let shares = vec![share(ShareType::User, "bob", None, SharePermission::Read)];
        assert!(can_read_resource(&e, &shares, &actor, &[], &[], &[]));
    }

    #[test]
    fn user_share_read_does_not_grant_write() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let shares = vec![share(ShareType::User, "bob", None, SharePermission::Read)];
        assert!(!can_write_resource(&e, &shares, &actor, &[], &[], &[]));
    }

    #[test]
    fn user_share_write_grants_read_and_write() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let shares = vec![share(ShareType::User, "bob", None, SharePermission::Write)];
        assert!(can_read_resource(&e, &shares, &actor, &[], &[], &[]));
        assert!(can_write_resource(&e, &shares, &actor, &[], &[], &[]));
    }

    #[test]
    fn admin_share_grants_write() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let shares = vec![share(ShareType::User, "bob", None, SharePermission::Admin)];
        assert!(can_write_resource(&e, &shares, &actor, &[], &[], &[]));
    }

    #[test]
    fn role_share_grants_access() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let roles = vec![role("editor", None)];
        let shares = vec![share(
            ShareType::Role,
            "editor",
            None,
            SharePermission::Write,
        )];
        assert!(can_write_resource(&e, &shares, &actor, &roles, &[], &[]));
    }

    #[test]
    fn group_share_grants_access() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let groups = vec!["team-a".to_string()];
        let shares = vec![share(
            ShareType::Group,
            "team-a",
            None,
            SharePermission::Read,
        )];
        assert!(can_read_resource(&e, &shares, &actor, &[], &groups, &[]));
    }

    #[test]
    fn role_in_group_share_requires_both() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let roles = vec![role("editor", Some("team-a"))];
        let groups = vec!["team-a".to_string()];
        let shares = vec![share(
            ShareType::RoleInGroup,
            "editor",
            Some("team-a"),
            SharePermission::Write,
        )];
        // Has both role and group
        assert!(can_write_resource(
            &e,
            &shares,
            &actor,
            &roles,
            &groups,
            &[]
        ));
    }

    #[test]
    fn role_in_group_share_fails_without_group() {
        let e = entity("alice", VisibilityScope::Private, None);
        let actor = user_actor("bob");
        let roles = vec![role("editor", None)];
        let shares = vec![share(
            ShareType::RoleInGroup,
            "editor",
            Some("team-a"),
            SharePermission::Write,
        )];
        // Has role but not group
        assert!(!can_write_resource(&e, &shares, &actor, &roles, &[], &[]));
    }

    // -----------------------------------------------------------------------
    // select_workflow_binding
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn binding(
        id: &str,
        resource_type: &str,
        vis_scope: Option<&str>,
        vis_id: Option<&str>,
        change_kind: Option<&str>,
        base_source: Option<&str>,
        policy_id: &str,
        priority: i32,
    ) -> WorkflowBinding {
        WorkflowBinding {
            id: id.to_string(),
            resource_type: resource_type.to_string(),
            visibility_scope: vis_scope.map(|s| s.to_string()),
            visibility_id: vis_id.map(|s| s.to_string()),
            change_kind: change_kind.map(|s| s.to_string()),
            base_verified_source: base_source.map(|s| s.to_string()),
            policy_id: policy_id.to_string(),
            priority,
        }
    }

    #[test]
    fn select_binding_returns_none_when_empty() {
        let result = select_workflow_binding(&[], "instrument", None, None, None, None);
        assert!(result.is_none());
    }

    #[test]
    fn select_binding_matches_by_resource_type() {
        let bindings = vec![binding("b1", "instrument", None, None, None, None, "p1", 0)];
        let result = select_workflow_binding(&bindings, "instrument", None, None, None, None)
            .expect("should match binding");
        assert_eq!(result.id, "b1");
    }

    #[test]
    fn select_binding_no_match_different_type() {
        let bindings = vec![binding("b1", "instrument", None, None, None, None, "p1", 0)];
        let result = select_workflow_binding(&bindings, "portfolio", None, None, None, None);
        assert!(result.is_none());
    }

    #[test]
    fn select_binding_higher_priority_wins() {
        let bindings = vec![
            binding("b1", "instrument", None, None, None, None, "p-low", 1),
            binding("b2", "instrument", None, None, None, None, "p-high", 10),
        ];
        let result = select_workflow_binding(&bindings, "instrument", None, None, None, None)
            .expect("should match binding");
        assert_eq!(result.id, "b2");
    }

    #[test]
    fn select_binding_more_specific_wins_at_same_priority() {
        let bindings = vec![
            binding("generic", "instrument", None, None, None, None, "p1", 0),
            binding(
                "specific",
                "instrument",
                Some("PRIVATE"),
                None,
                Some("CREATE"),
                None,
                "p2",
                0,
            ),
        ];
        let result = select_workflow_binding(
            &bindings,
            "instrument",
            Some("PRIVATE"),
            None,
            Some("CREATE"),
            None,
        )
        .expect("should match specific binding");
        assert_eq!(result.id, "specific");
    }

    #[test]
    fn select_binding_specific_field_must_match() {
        // Binding requires CREATE but we ask for EDIT -- should not match
        let bindings = vec![binding(
            "b1",
            "instrument",
            None,
            None,
            Some("CREATE"),
            None,
            "p1",
            0,
        )];
        let result =
            select_workflow_binding(&bindings, "instrument", None, None, Some("EDIT"), None);
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // validate_transition
    // -----------------------------------------------------------------------

    fn transition(
        allow_owner: bool,
        required_role: Option<&str>,
        required_group: Option<&str>,
        allow_system: bool,
        verifier_not_owner: bool,
        verifier_not_submitter: bool,
        distinct_from_last: bool,
    ) -> WorkflowTransition {
        WorkflowTransition {
            id: "t1".to_string(),
            policy_id: "p1".to_string(),
            from_state: "PENDING".to_string(),
            to_state: "VERIFIED".to_string(),
            required_role_id: required_role.map(|s| s.to_string()),
            required_group_id: required_group.map(|s| s.to_string()),
            allow_owner,
            allow_system_actor: allow_system,
            require_verifier_not_owner: verifier_not_owner,
            require_verifier_not_submitter: verifier_not_submitter,
            require_distinct_from_last_actor: distinct_from_last,
        }
    }

    #[test]
    fn system_actor_allowed_when_allow_system_true() {
        let t = transition(false, None, None, true, false, false, false);
        let actor = system_actor("svc-1");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], None).is_ok());
    }

    #[test]
    fn system_actor_denied_when_allow_system_false() {
        let t = transition(false, None, None, false, false, false, false);
        let actor = system_actor("svc-1");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], None).is_err());
    }

    #[test]
    fn owner_allowed_when_allow_owner_true() {
        let t = transition(true, None, None, false, false, false, false);
        let actor = user_actor("alice");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], None).is_ok());
    }

    #[test]
    fn non_owner_denied_when_only_owner_allowed() {
        let t = transition(true, None, None, false, false, false, false);
        let actor = user_actor("bob");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], None).is_err());
    }

    #[test]
    fn role_grants_access() {
        let t = transition(false, Some("reviewer"), None, false, false, false, false);
        let actor = user_actor("bob");
        let roles = vec![role("reviewer", None)];
        assert!(validate_transition(&t, &actor, "alice", None, &roles, &[], None).is_ok());
    }

    #[test]
    fn role_and_group_required() {
        let t = transition(
            false,
            Some("reviewer"),
            Some("team-a"),
            false,
            false,
            false,
            false,
        );
        let actor = user_actor("bob");
        let roles = vec![role("reviewer", Some("team-a"))];
        let groups = vec!["team-a".to_string()];
        assert!(validate_transition(&t, &actor, "alice", None, &roles, &groups, None).is_ok());
    }

    #[test]
    fn verifier_not_owner_blocks_owner() {
        let t = transition(true, None, None, false, true, false, false);
        let actor = user_actor("alice");
        // allow_owner=true but require_verifier_not_owner=true blocks the owner
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], None).is_err());
    }

    #[test]
    fn verifier_not_submitter_blocks_submitter() {
        let t = transition(true, None, None, false, false, true, false);
        let actor = user_actor("alice");
        assert!(validate_transition(&t, &actor, "owner", Some("alice"), &[], &[], None).is_err());
    }

    #[test]
    fn distinct_from_last_actor_blocks_repeat() {
        let t = transition(true, None, None, false, false, false, true);
        let actor = user_actor("alice");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], Some("alice")).is_err());
    }

    #[test]
    fn distinct_from_last_actor_ok_with_different() {
        let t = transition(true, None, None, false, false, false, true);
        let actor = user_actor("alice");
        assert!(validate_transition(&t, &actor, "alice", None, &[], &[], Some("bob")).is_ok());
    }

    // -----------------------------------------------------------------------
    // parse_series_resource_id
    // -----------------------------------------------------------------------

    #[test]
    fn parse_series_resource_id_valid() {
        let (ns, kind, id) =
            parse_series_resource_id("market:quote:AAPL").expect("should parse valid id");
        assert_eq!(ns, "market");
        assert_eq!(kind, "quote");
        assert_eq!(id, "AAPL");
    }

    #[test]
    fn parse_series_resource_id_colons_in_id() {
        let (ns, kind, id) =
            parse_series_resource_id("ns:metric:a:b:c").expect("should parse id with colons");
        assert_eq!(ns, "ns");
        assert_eq!(kind, "metric");
        assert_eq!(id, "a:b:c");
    }

    #[test]
    fn parse_series_resource_id_missing_parts() {
        assert!(parse_series_resource_id("only_one").is_err());
        assert!(parse_series_resource_id("two:parts").is_err());
        assert!(parse_series_resource_id("").is_err());
    }

    // -----------------------------------------------------------------------
    // generate_id
    // -----------------------------------------------------------------------

    #[test]
    fn generate_id_uniqueness() {
        let actor = user_actor("alice");
        let id1 = generate_id("chg", &actor);
        let id2 = generate_id("chg", &actor);
        assert_ne!(id1, id2, "consecutive IDs should differ");
    }

    #[test]
    fn generate_id_has_prefix() {
        let actor = user_actor("alice");
        let id = generate_id("evt", &actor);
        assert!(
            id.starts_with("evt-alice-"),
            "id should start with prefix-actor: {id}"
        );
    }

    // -----------------------------------------------------------------------
    // SharePermission ordering
    // -----------------------------------------------------------------------

    #[test]
    fn permission_allows_hierarchy() {
        assert!(SharePermission::Read.allows(SharePermission::Read));
        assert!(!SharePermission::Read.allows(SharePermission::Write));
        assert!(!SharePermission::Read.allows(SharePermission::Admin));
        assert!(SharePermission::Write.allows(SharePermission::Read));
        assert!(SharePermission::Write.allows(SharePermission::Write));
        assert!(!SharePermission::Write.allows(SharePermission::Admin));
        assert!(SharePermission::Admin.allows(SharePermission::Read));
        assert!(SharePermission::Admin.allows(SharePermission::Write));
        assert!(SharePermission::Admin.allows(SharePermission::Admin));
    }

    // -----------------------------------------------------------------------
    // Enum parsing roundtrips
    // -----------------------------------------------------------------------

    #[test]
    fn actor_kind_roundtrip() {
        for kind in [ActorKind::User, ActorKind::System] {
            assert_eq!(ActorKind::parse(kind.as_str()).expect("should parse"), kind);
        }
        assert!(ActorKind::parse("UNKNOWN").is_err());
    }

    #[test]
    fn visibility_scope_roundtrip() {
        for scope in [
            VisibilityScope::Private,
            VisibilityScope::Role,
            VisibilityScope::Group,
            VisibilityScope::Public,
        ] {
            assert_eq!(
                VisibilityScope::parse(scope.as_str()).expect("should parse"),
                scope
            );
        }
        assert!(VisibilityScope::parse("UNKNOWN").is_err());
    }

    #[test]
    fn share_type_roundtrip() {
        for st in [
            ShareType::User,
            ShareType::Role,
            ShareType::Group,
            ShareType::RoleInGroup,
        ] {
            assert_eq!(ShareType::parse(st.as_str()).expect("should parse"), st);
        }
        assert!(ShareType::parse("UNKNOWN").is_err());
    }

    #[test]
    fn change_kind_roundtrip() {
        for ck in [ChangeKind::Create, ChangeKind::Edit, ChangeKind::Ingest] {
            assert_eq!(ChangeKind::parse(ck.as_str()).expect("should parse"), ck);
        }
        assert!(ChangeKind::parse("UNKNOWN").is_err());
    }
}
