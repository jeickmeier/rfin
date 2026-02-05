# PR-11: Enterprise Row-Level Permissioning & Configurable Workflow

## Summary

Add enterprise-grade **row-level permissioning** and a **configurable approval workflow** to `finstack-io` so a single shared database can safely serve many users across an asset management firm.

Key additions:
- Role/group based visibility for instruments, portfolios, market data, statements, scenarios, and registries.
- Owner-only private drafts and "what-if" objects that can later be published to wider groups.
- A configurable workflow with human and system final states: `VERIFIED` and `SYSTEM_VERIFIED`.
- Support for both one-step and two-step verification, with configurable "no self-approval" rules by resource and by org scope (role/group).
- Works consistently across SQLite, Postgres, and Turso (SQLite-compatible), with optional Postgres RLS for defense-in-depth.

## Background & Motivation

`finstack-io` currently persists domain objects (instruments, market contexts, portfolios, scenarios, statement models, metric registries, time series) into shared tables. For broad enterprise adoption, we must prevent accidental or unauthorized access across desks, teams, and functions while still enabling collaboration and governed publication.

We also need a controlled promotion path:
- Humans create and iterate privately.
- Content is submitted for review and verification.
- Verified content is visible to the intended audience.
- System-to-system ingestions can be treated as final with clear provenance (`SYSTEM_VERIFIED`) while still allowing later human edits that re-enter governance.

## Goals

- Enforce row-level visibility based on role and group membership.
- Enable private drafts (single-user) for test trades, draft statements, and scenarios.
- Provide a configurable workflow engine usable across resource types.
- Support system ingestion that can bypass review and land as `SYSTEM_VERIFIED`.
- Keep behavior consistent across all supported backends.
- Provide an audit trail for changes, approvals, and system ingestions.

## Non-Goals (For This PRD)

- Implement SSO/OIDC/LDAP integration end-to-end (we store stable user identities and accept external IDs).
- Build a full admin UI (CLI or API is sufficient initially).
- Provide field-level redaction within payload JSON (row-level only).
- Implement point-level permissions on every time-series point (permission is at the series or market-context level).

## Personas

- Portfolio Manager: builds portfolios, runs scenarios, shares to their desk.
- Risk Manager: reads portfolios and market data within a scope; reviews pending changes.
- Operations/Control: verifies statements and reference data; may require two-step approval.
- Data Engineer / Integration: loads instruments and market data from upstream systems; expects `SYSTEM_VERIFIED`.
- Compliance/Audit: reviews approvals, provenance, and access history.
- Power User (Research): creates private what-ifs and selectively publishes.

## Definitions

- Resource: a persisted domain object (instrument, portfolio snapshot, market context snapshot, scenario, statement model, metric registry, time-series series metadata).
- Actor: a user or a system principal performing an action.
- Principal: a user, role, or group used for entitlements.
- Visibility: who can read a verified resource (role/group/public/private), plus optional explicit shares.
- Workflow Policy: a configured set of states, transitions, and constraints used for a given resource change.
- Change Proposal: a draft/pending/checking version of a resource that may be applied to the verified store when approved.

## Scope

### In Scope

- Row-level access model:
  - Owner-based access for drafts.
  - Verified visibility by role and group.
  - Explicit sharing exceptions by user/role/group.
- Workflow engine:
  - Configurable states and transitions per policy.
  - Default states include `DRAFT`, `PENDING`, `CHECKING`, `VERIFIED`, `SYSTEM_VERIFIED`.
  - Support both one-step and two-step flows.
  - Support distinct-actor constraints (no self-approval) configurable by resource and by role/group scope.
- System ingestion:
  - Allow creation directly into `SYSTEM_VERIFIED` (bypass `PENDING`).
  - Persist provenance (source system, run id, ingest timestamp).
- Audit and history:
  - Persistent log of workflow transitions and applied changes.
- Backend support:
  - SQLite, Postgres, Turso all supported with identical semantics.
  - Optional Postgres RLS as an additional guardrail.

### Out of Scope

- UI for policy design and access administration.
- Complex segregation-of-duties frameworks beyond distinct-actor checks.
- End-user notifications and workflow task queues (can be layered on later).

## Requirements

### R1: Default Deny and Resource Visibility

- Resources are not visible to non-owners until they are in a final state (`VERIFIED` or `SYSTEM_VERIFIED`) and the actor is entitled by:
  - Visibility scope (role or group), or
  - Explicit share grants, or
  - Admin/auditor override.
- Visibility can be defined per resource using:
  - `PRIVATE` (owner only)
  - `ROLE` (all users with role)
  - `GROUP` (all users in group)
  - `PUBLIC` (all authenticated users, if enabled)

### R2: Drafts and Private What-Ifs

- Users can create resources as private drafts (owner-only) for experiments.
- Drafts are writable only by the owner.
- Drafts can be promoted into a review state without changing the last verified version.

### R3: Change Proposals (Editing Verified Data)

- Editing a `VERIFIED` or `SYSTEM_VERIFIED` resource creates a new change proposal.
- The previously verified version remains readable to entitled users while the change proposal is under review.
- Once a change proposal reaches a final state, it becomes the new verified version.

### R4: Configurable Workflow (Human and System Final States)

- Workflow is policy-driven, not hard-coded.
- Policies define:
  - Allowed states.
  - Allowed transitions.
  - Who can transition (by role, group-scoped role, or owner).
  - Optional constraints like "verifier must not be the owner" and "verifier must not be the submitter".
- Default workflow states:
  - `DRAFT` (owner-only)
  - `PENDING` (submitted for review)
  - `CHECKING` (optional intermediate, used for two-step flows)
  - `VERIFIED` (final, human approved)
  - `SYSTEM_VERIFIED` (final, system ingested)
- Policies are selectable by:
  - Resource type
  - Role/group visibility target
  - Change kind (create vs edit)
  - Ingestion source (system vs human)

### R5: One-Step and Two-Step Approval

- One-step: `DRAFT -> PENDING -> VERIFIED`
- Two-step: `DRAFT -> PENDING -> CHECKING -> VERIFIED`
- The chosen path is determined by the policy attached to the change proposal.

### R6: System-to-System Ingestion

- System actors can create or update resources directly into `SYSTEM_VERIFIED`.
- System ingestions must write provenance:
  - Source system name (e.g., "Bloomberg", "Aladdin", "InternalRefData")
  - Ingestion run identifier
  - Timestamp
- System-verified content is treated as final for visibility purposes, but still editable via change proposals.

### R7: Role/Group Configurable Separation of Duties

- Distinct-actor rules are configurable by resource and by org scope (role/group).
- Examples:
  - Some groups allow self-verification for low-risk resources.
  - Other groups require verifier != owner for statements and reference data.

### R8: Auditing

- Record workflow events:
  - Who changed state and when.
  - From/to states.
  - Change proposal identifier and the affected resource.
  - Provenance for system actions.
- Provide queryability for audit and compliance (by resource, by actor, by time range).

### R9: Cross-Backend Compatibility

- Must work on:
  - SQLite (embedded)
  - Turso (SQLite-compatible)
  - Postgres (scale-out)
- Behavior should be identical across backends.
- Postgres can optionally enforce at the DB layer with RLS, but application enforcement must remain correct.

## User Journeys

### J1: Create Private Draft, Submit for Review, Verify

1. User creates draft portfolio/instrument/scenario.
2. Only the owner can read/write while in `DRAFT`.
3. Owner submits the change proposal to `PENDING`.
4. Reviewer(s) transition to `CHECKING` if policy requires.
5. Verifier transitions to `VERIFIED`.
6. Verified version becomes visible to entitled users (role/group), plus explicit shares.

### J2: System Load Bypasses Review

1. Integration actor loads instruments and market data from upstream.
2. Each load produces a `SYSTEM_VERIFIED` change proposal that is applied immediately.
3. Entitled users can read the data and see it is system-verified with provenance.

### J3: Edit Verified Resource Without Disrupting Consumers

1. Resource is currently `VERIFIED` or `SYSTEM_VERIFIED`.
2. User creates an edit change proposal (owner-only `DRAFT`).
3. Existing verified version remains readable to consumers.
4. Edit is submitted and verified under policy (which may differ from create policy).
5. New verified version replaces the previous verified version.

## Acceptance Criteria

- A user can create a private draft resource and confirm it is not visible to other users.
- A user can submit a draft and a reviewer can transition it through the configured policy.
- A verifier can approve a change and the resource becomes visible to the intended role/group.
- A system actor can ingest directly into `SYSTEM_VERIFIED` with provenance recorded.
- Editing a verified resource does not remove the old verified version until the new change is verified.
- Distinct-actor constraints can be enabled/disabled per resource type and per group/role policy binding.
- SQLite, Postgres, and Turso exhibit the same behavior for all operations in the above flows.

## Success Metrics

- Zero cross-team data leaks in permission tests and audits.
- Median authorized read latency does not regress materially relative to current `finstack-io` (target: <= 10% overhead on key reads under typical indexing).
- Workflow event coverage: 100% of publish/verify transitions produce audit events.

## Risks & Mitigations

- Risk: Permission checks add joins and slow down reads.
  - Mitigation: Keep ACL in small indexed tables, cache actor memberships, avoid per-row JSON parsing, add covering indexes.
- Risk: SQLite/Turso lack built-in RLS.
  - Mitigation: Enforce access in the Store layer and provide secure query APIs that require actor context.
- Risk: Misconfiguration of policies can block publication.
  - Mitigation: Provide default policies, config validation tooling, and "dry-run" transition validation.

## Rollout Plan (Suggested)

- Phase 1: Add schema, policy config, and governed APIs alongside existing un-governed Store APIs.
- Phase 2: Switch enterprise deployments to governed APIs; enable Postgres RLS as defense-in-depth.
- Phase 3: Add admin tooling for role/group membership and policy binding management.
