# TDD-11: Enterprise Permissioning & Configurable Workflow (finstack-io)

## Overview

This document describes a backend-agnostic implementation of:
- Row-level access control (RBAC + ownership + explicit sharing).
- Configurable workflows for creating and editing persisted resources.
- Support for system-to-system ingestion via a final `SYSTEM_VERIFIED` state.

Target backends:
- SQLite (embedded) / Turso (SQLite-compatible)
- Postgres (scale-out)

Constraint: behavior must be identical across backends. Postgres may additionally use RLS for defense-in-depth, but the application-layer authorization must remain correct.

## Design Summary

1. **Separate "verified storage" from "change proposals".**
   - Existing resource tables remain the source of truth for the latest verified/system-verified content.
   - New `resource_changes` table stores drafts and proposals, including pending/checking and final states as an immutable history of proposed payloads.
   - Editing a verified resource creates a change proposal without disrupting consumers of the last verified row.

2. **Centralize entitlements in small indexed tables.**
   - `resource_entities` defines owner and default visibility (role/group/public/private) per logical resource.
   - `resource_shares` provides explicit exceptions (user/role/group grants).
   - `user_roles` and `user_groups` define actor membership.

3. **Workflow is data-driven.**
   - `workflow_policies`, `workflow_states`, `workflow_transitions`, and `workflow_bindings` define allowed paths and separation-of-duties constraints.
   - Policies can differ by resource type, target visibility (role/group), change kind (create vs edit), and provenance (system vs human).

4. **System ingestion is first-class.**
   - System actors can bypass review and produce `SYSTEM_VERIFIED` proposals that are applied immediately to the verified store.
   - Provenance is stored on the change proposal and in the workflow event log.

## Resource Model

### Resource Types

The following resource types are in scope (names are suggested canonical strings):
- `instrument`
- `portfolio` (snapshots keyed by `as_of`)
- `market_context` (snapshots keyed by `as_of`)
- `scenario`
- `statement_model`
- `metric_registry`
- `series_meta` (time-series series-level metadata)

The existing per-type tables continue to store verified content, using their current keys:
- `instruments(id)`
- `portfolios(id, as_of)`
- `market_contexts(id, as_of)`
- `scenarios(id)`
- `statement_models(id)`
- `metric_registries(id)` where `id = namespace`
- `series_meta(namespace, kind, series_id)`

### Logical Resource Identity

Authorization and sharing is keyed by `(resource_type, resource_id)` where:
- For `instrument`, `resource_id = instrument_id`
- For `portfolio`, `resource_id = portfolio_id` (covers all `as_of` snapshots)
- For `market_context`, `resource_id = market_id` (covers all `as_of` snapshots)
- For `scenario`, `resource_id = scenario_id`
- For `statement_model`, `resource_id = model_id`
- For `metric_registry`, `resource_id = namespace`
- For `series_meta`, `resource_id = "{namespace}:{kind}:{series_id}"` (canonical encoding)

Change proposals additionally identify a "row key" for snapshot resources via `resource_key2`:
- For `portfolio` and `market_context`, `resource_key2 = as_of` (ISO date string)
- For other resource types, `resource_key2 = ""`

## Authorization Model

### Visibility Scopes

`resource_entities.visibility_scope` is one of:
- `PRIVATE` (owner only)
- `ROLE` (users with the role in `visibility_id`)
- `GROUP` (users in the group in `visibility_id`)
- `PUBLIC` (all authenticated users, if enabled)

`resource_entities.visibility_id` is required for `ROLE` and `GROUP`.

### Explicit Sharing

`resource_shares` grants per-resource exceptions. Each share has:
- `share_type`: `USER`, `ROLE`, `GROUP`, `ROLE_IN_GROUP`
- `share_id`: user_id or role_id or group_id
- `share_scope_id`: optional group_id for `ROLE_IN_GROUP`
- `permission`: `READ`, `WRITE`, `ADMIN`

### Actor Context

All governed operations require an `ActorContext`:
- `actor_kind`: `USER` or `SYSTEM`
- `actor_id`: stable identifier (user id or system service account id)
- `assume_user_id`: optional for system actors acting on behalf of a user (audit only)

Membership resolution uses DB tables (`user_roles`, `user_groups`) and may be cached per request.

### Permission Evaluation (Read)

An actor can read a verified resource if any condition holds:
- Actor is the `owner_user_id`.
- Resource `visibility_scope` allows the actor via role/group/public.
- `resource_shares` grants `READ|WRITE|ADMIN` to any of:
  - the actor user
  - any actor roles
  - any actor groups
  - any actor role-in-group bindings (if used)
- Actor has an admin override role (configurable, for example `EnterpriseAdmin` or `Auditor`).

Drafts (`resource_changes` in `DRAFT`) are readable only by the owner.

### Permission Evaluation (Write)

Write permissions are required for:
- Updating `resource_entities` visibility.
- Creating edit proposals on existing resources (optional constraint).
- Applying verified changes to the verified store (always controlled by workflow, not direct write).

Base rule:
- `DRAFT` proposals are writable only by owner.
- Transitioning proposals requires workflow transition permission (see Workflow section).

## Workflow Model

### Change Kind

Each `resource_changes` row includes `change_kind`:
- `CREATE` for new resources not yet verified.
- `EDIT` for changes to existing verified resources.
- `INGEST` for system-to-system loads.

Policies may differ by `change_kind`.

### Workflow States (Defaults)

State keys are policy-defined. Default keys:
- `DRAFT`
- `PENDING`
- `CHECKING`
- `VERIFIED`
- `SYSTEM_VERIFIED`

`VERIFIED` and `SYSTEM_VERIFIED` are final. The `SYSTEM_VERIFIED` state indicates the content came directly from a file/API load and was not reviewed by a human.

### Separation of Duties (Configurable)

Distinct actor rules must be configurable per policy binding (resource type + org scope).
Supported constraints (minimum set):
- `require_verifier_not_owner`
- `require_verifier_not_submitter`
- `require_distinct_from_last_transition_actor`

These are implemented as checks at transition time using `workflow_events`.

### Policy Binding and Selection

Workflow policy selection is configurable. Recommended tables:
- `workflow_policies(id, name, resource_type, is_active, created_at, updated_at)`
- `workflow_bindings(id, resource_type, visibility_scope, visibility_id, change_kind, base_verified_source, policy_id, priority)`

Selection algorithm:
1. Determine `resource_type`, `change_kind`, and `visibility_scope/visibility_id` target.
2. Determine `base_verified_source` for edits (from existing verified row provenance, if available).
3. Choose highest `priority` binding matching non-null fields, preferring the most specific match.
4. Fall back to a default policy for the resource type.

### Transition Authorization

`workflow_transitions` defines allowed transitions:
- `policy_id`
- `from_state`
- `to_state`
- `required_role_id` (nullable)
- `required_group_id` (nullable)
- `allow_owner` boolean
- `allow_system_actor` boolean
- distinct actor constraints (booleans)

On transition request:
1. Load the `resource_changes` row.
2. Load the transition definition for `(policy_id, from_state, to_state)`.
3. Verify the actor is allowed by role/group membership or `allow_owner` / `allow_system_actor`.
4. Verify distinct actor constraints using `workflow_events`.
5. Write a `workflow_events` row.
6. Update `resource_changes.workflow_state`.
7. If `to_state` is final, apply the change to the verified store in the same transaction.

### Applying Final Changes

On finalization (`VERIFIED` or `SYSTEM_VERIFIED`):
- Upsert the payload into the target verified table for the resource type.
- For snapshot resources, `resource_key2` is used as `as_of`.
- Record `applied_at` on the change proposal.

Important: applying is an atomic operation with the transition update and workflow event insert, using a transaction on all backends.

### Editing Verified and SYSTEM_VERIFIED Resources

Edit flow:
1. Actor creates an `EDIT` change proposal with `DRAFT` state and desired visibility target (if changing).
2. Consumers continue to read the verified row from the existing table.
3. Actor submits to `PENDING`.
4. Review/verify per policy.
5. On finalization, apply to verified store.

This guarantees no disruption to consumers.

System-verified edits:
- Policy bindings may require stronger checks for editing `SYSTEM_VERIFIED` content (for example, two-step with data steward roles).

## Database Schema (New Tables)

All tables are created via `finstack/io/src/sql/schema/*` and discovered by the existing migration system.

### Identity and Membership

- `auth_users(id TEXT PRIMARY KEY, external_id TEXT NULL, name TEXT NULL, status TEXT NOT NULL, created_at, updated_at)`
- `auth_roles(id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at, updated_at)`
- `auth_groups(id TEXT PRIMARY KEY, name TEXT NOT NULL, created_at, updated_at)`
- `auth_user_roles(user_id TEXT, role_id TEXT, group_id TEXT NULL, created_at, PRIMARY KEY(user_id, role_id, group_id))`
- `auth_user_groups(user_id TEXT, group_id TEXT, created_at, PRIMARY KEY(user_id, group_id))`

### Better Auth Compatibility

Better Auth provides its own core tables plus organization plugin tables. To keep the storage design compatible with Better Auth:

- Prefer Better Auth as the source of truth for user/org membership and use **schema overrides** so Better Auth writes directly into the `auth_*` table names in this design.
- Adapters or views are still possible if a deployment wants to keep Better Auth defaults, but schema overrides are the preferred path.
- The organization plugin adds `organization` and `member` tables, and optionally `organizationRole`, `team`, and `teamMember` tables. Roles can be multi-valued in `member.role`.
- Better Auth allows custom table and column names for core schema and plugin tables using `modelName`, `fields`, and `schema` config.

Recommended mapping when using Better Auth default tables:
- `auth_users` maps to Better Auth `user`.
- `auth_groups` maps to `organization` and optionally `team`.
- `auth_user_groups` maps to `member` and `teamMember`.
- `auth_roles` and `auth_user_roles` map to `organizationRole` plus `member.role` (string list).

Group identity collision avoidance:
- When storing organization or team identifiers in `visibility_id` and `resource_shares.share_id`, use a typed prefix such as `org:<id>` or `team:<id>`. This avoids ambiguity and allows the adapter to resolve membership against `organization` and `team` tables without adding new columns.

### Resource Entitlements

- `resource_entities(resource_type TEXT, resource_id TEXT, owner_user_id TEXT, visibility_scope TEXT, visibility_id TEXT NULL, created_at, updated_at, PRIMARY KEY(resource_type, resource_id))`
- `resource_shares(id TEXT PRIMARY KEY, resource_type TEXT, resource_id TEXT, share_type TEXT, share_id TEXT, share_scope_id TEXT NULL, permission TEXT, created_at, updated_at)`

### Workflow Configuration

- `workflow_policies(id TEXT PRIMARY KEY, resource_type TEXT, name TEXT, is_active BOOL, created_at, updated_at)`
- `workflow_states(policy_id TEXT, state_key TEXT, is_final BOOL, verified_source TEXT NULL, system_only BOOL, category TEXT, sort_order INT, PRIMARY KEY(policy_id, state_key))`
- `workflow_transitions(id TEXT PRIMARY KEY, policy_id TEXT, from_state TEXT, to_state TEXT, required_role_id TEXT NULL, required_group_id TEXT NULL, allow_owner BOOL, allow_system_actor BOOL, require_verifier_not_owner BOOL, require_verifier_not_submitter BOOL, require_distinct_from_last_actor BOOL, created_at, updated_at)`
- `workflow_bindings(id TEXT PRIMARY KEY, resource_type TEXT, visibility_scope TEXT NULL, visibility_id TEXT NULL, change_kind TEXT NULL, base_verified_source TEXT NULL, policy_id TEXT, priority INT, created_at, updated_at)`
- `workflow_events(id TEXT PRIMARY KEY, change_id TEXT, resource_type TEXT, resource_id TEXT, resource_key2 TEXT, from_state TEXT, to_state TEXT, actor_kind TEXT, actor_id TEXT, at_ts, note TEXT NULL)`

### Change Proposals

- `resource_changes(change_id TEXT PRIMARY KEY, resource_type TEXT, resource_id TEXT, resource_key2 TEXT NOT NULL DEFAULT '', change_kind TEXT, workflow_policy_id TEXT, workflow_state TEXT, owner_user_id TEXT, created_by_kind TEXT, created_by_id TEXT, submitted_at, applied_at, base_etag TEXT NULL, ingestion_source TEXT NULL, ingestion_run_id TEXT NULL, payload BLOB/JSONB, meta TEXT/JSONB, created_at, updated_at)`

Notes:
- `payload` uses `payload_col(Backend::Sqlite|Postgres)` like existing tables.
- `meta` uses `meta_col` for consistent JSON storage.
- `resource_key2` is a string for portability and allows one generic change table.

## Query Strategy

### Verified Reads (Resource Tables)

For governed reads, queries are augmented to require entitlement against `resource_entities` and `resource_shares`.

Portable pattern (EXISTS-based, parameterized):
- Fetch resource rows only if `can_read(resource_type, resource_id, actor)` evaluates true.
- Implement `can_read` as a SQL EXISTS subquery that checks:
  - ownership
  - visibility scope membership (`auth_user_roles`, `auth_user_groups`)
  - explicit shares (`resource_shares`)

This is implementable with sea-query and is compatible across backends.

### Draft/Proposal Reads (`resource_changes`)

Rules:
- `workflow_state = DRAFT`: owner only
- `workflow_state in (PENDING, CHECKING)`: owner + reviewers allowed by policy
- `workflow_state in final`: readable to auditors/admins; consumers should read from verified tables

### System Actor Bypass

System actor operations are allowed only when:
- The actor is a known system principal (stored in `auth_users` with type or via role).
- The workflow transition allows `allow_system_actor`.
- The policy defines `SYSTEM_VERIFIED` as final and `system_only = true`.

## Postgres RLS (Optional Defense-In-Depth)

If enabled, Postgres can enforce RLS on verified tables and on `resource_changes`.

Approach:
- Store the acting `user_id` in a session setting `finstack.user_id`.
- Define SQL functions `finstack_current_user_id()` and `finstack_can_read(resource_type, resource_id)` that consult `resource_entities`, `resource_shares`, and membership tables.
- Add RLS policies:
  - Verified tables: `USING (finstack_can_read('<type>', id_or_key))`
  - `resource_changes`: restrict `DRAFT` to owner, `PENDING/CHECKING` to reviewers, finals to admin/audit.

RLS is not available on SQLite/Turso, so application-layer checks remain mandatory.

## Migration Plan

1. Add new tables and indexes (no behavior change).
2. Backfill `resource_entities` for existing verified rows:
   - Default visibility may be `PUBLIC` for single-user deployments, but enterprise deployments should require explicit assignment.
3. Introduce governed APIs that require `ActorContext` and enforce entitlement checks.
4. Provide a feature flag or configuration mode:
   - `FINSTACK_IO_GOVERNANCE=off` uses current behavior (trusted).
   - `FINSTACK_IO_GOVERNANCE=on` requires actor context and denies reads for missing entitlements.
5. Optional: enable Postgres RLS in enterprise deployments.

## Indexing

Required indexes (minimum):
- `resource_entities(resource_type, resource_id)` primary key
- `resource_entities(owner_user_id)`
- `resource_entities(visibility_scope, visibility_id)`
- `resource_shares(resource_type, resource_id)`
- `resource_shares(share_type, share_id, share_scope_id)`
- `auth_user_roles(user_id, role_id, group_id)`
- `auth_user_groups(user_id, group_id)`
- `resource_changes(resource_type, resource_id, resource_key2)`
- `resource_changes(owner_user_id, workflow_state)`
- `resource_changes(workflow_state, updated_at)`
- `workflow_transitions(policy_id, from_state, to_state)` unique or indexed
- `workflow_events(change_id)` and `workflow_events(resource_type, resource_id)`

## API Changes (finstack-io)

Add a governed layer rather than breaking the existing `Store` trait immediately.

Recommended API surface:
- `StoreHandle::as_actor(ctx: ActorContext) -> GovernedHandle`
- `GovernedHandle` provides:
  - `get_*` and `list_*` methods that apply entitlement filters.
  - `draft_*` methods that create `resource_changes` in `DRAFT`.
  - `submit_change(change_id)` transitions `DRAFT -> PENDING`.
  - `transition_change(change_id, to_state)` enforces workflow transitions.
  - `list_changes(filters)` for owners/reviewers/auditors.
  - `set_visibility(resource_type, resource_id, scope, id)` with write/admin permission.

The existing `Store` trait remains as a low-level trusted interface used for local notebooks and single-user workflows.

## Testing Plan

Unit tests:
- Permission evaluation logic for combinations of owner/role/group/share.
- Workflow transition validation including distinct actor constraints.
- Policy binding selection logic.

Integration tests (run on SQLite and Postgres):
- Draft is owner-only.
- Verified read allowed/denied based on role/group.
- System ingestion creates `SYSTEM_VERIFIED` and applies immediately.
- Editing verified resource does not break consumer reads until approval.
- Two-step and one-step flows both function under different policy bindings.

Security tests:
- Attempt to read without `resource_entities` entitlement in governance mode (must deny).
- Attempt to self-verify when the policy requires distinct actors (must deny).

## Open Implementation Decisions

- Whether to store system principals as a dedicated column on `auth_users` or as a role assignment (recommended: role assignment plus `actor_kind=SYSTEM` in context).
- Whether to require explicit `resource_entities` rows for all verified resources (recommended: yes in governance mode).
- Whether to add "delete" workflow and soft-deletion semantics (future work).
