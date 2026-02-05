# PR-11 Sub-PR Implementation Plan

This plan breaks the PRD/TDD into staged sub-PRs with clear scope and acceptance checks.
Preference: **Better Auth schema overrides** are used so Better Auth writes directly into the `auth_*` tables defined in the TDD.

## PR-11.1: Schema Foundation (Auth + Entitlements + Workflow Tables)

Scope: Add new tables and indexes only. No runtime behavior changes.

Tasks:
1. Add auth tables aligned with Better Auth schema overrides:
   - `auth_users`, `auth_roles`, `auth_groups`, `auth_user_roles`, `auth_user_groups`.
2. Add entitlement tables:
   - `resource_entities`, `resource_shares`.
3. Add workflow configuration tables:
   - `workflow_policies`, `workflow_states`, `workflow_transitions`, `workflow_bindings`, `workflow_events`.
4. Add change proposal table:
   - `resource_changes`.
5. Add indexes per the TDD.
6. Register all tables in `finstack/io/src/sql/schema/mod.rs` and bump migrations.

Acceptance:
1. Migrations run on SQLite, Postgres, and Turso.
2. No existing API behavior changes.

Primary files:
- `finstack/io/src/sql/schema/*.rs`
- `finstack/io/src/sql/migrations.rs`

## PR-11.2: Better Auth Schema Overrides (Config + Docs)

Scope: Document and validate Better Auth integration using schema overrides (preferred).

Tasks:
1. Add Better Auth schema override example (core + org plugin) to the TDD.
2. Add a short integration note to `finstack/io/README.md` pointing to the TDD.
3. Validate field compatibility: `user` -> `auth_users`, `organization` -> `auth_groups`, `member` -> `auth_user_groups`, `organizationRole`/`member.role` -> `auth_roles`/`auth_user_roles`.
4. Define a typed ID convention for org/team references (e.g., `org:<id>`, `team:<id>`).

Acceptance:
1. TDD contains a clear, copyable Better Auth config example.
2. No code changes required in `finstack-io` to resolve memberships when schema overrides are used.

Primary files:
- `docs/TDD/TDD-11-enterprise-permissioning-and-workflow.md`
- `finstack/io/README.md`

## PR-11.3: Governance Core (Actor Context + Entitlement Checks)

Scope: Add authorization primitives and governed read APIs.

Tasks:
1. Add `ActorContext` and `ActorKind`.
2. Implement `can_read` / `can_write` helpers and membership resolution.
3. Implement governed `get_*` and `list_*` for verified resources.
4. Add unit tests for owner/role/group/share/admin rules.

Acceptance:
1. Unit tests cover common entitlement scenarios.
2. Existing `Store` trait remains unchanged.

Primary files:
- `finstack/io/src/store.rs`
- `finstack/io/src/config.rs`
- `finstack/io/src/providers/*`

## PR-11.4: Workflow Engine (Policies, Transitions, Events)

Scope: Policy selection, transition validation, and event logging (no apply yet).

Tasks:
1. Implement policy selection based on resource type, visibility scope/id, change kind, base verified source.
2. Implement transition checks: role/group/owner/system.
3. Implement distinct-actor constraints.
4. Write workflow event records.

Acceptance:
1. Unit tests for policy selection.
2. Unit tests for transitions and distinct-actor constraints.

Primary files:
- `finstack/io/src/workflow/*.rs` (new)
- `finstack/io/src/providers/*`

## PR-11.5: Change Proposals (Draft, Submit, Apply)

Scope: Create and manage `resource_changes`, apply final results.

Tasks:
1. Implement `create_draft_change`, `submit_change`, `transition_change`.
2. Apply `VERIFIED` / `SYSTEM_VERIFIED` changes to verified tables atomically.
3. Ensure edits do not replace verified data until finalization.
4. Persist ingestion provenance on changes.

Acceptance:
1. Integration tests for draft -> submit -> verify -> apply.
2. Verified rows remain unchanged until finalization.

Primary files:
- `finstack/io/src/sql/statements.rs`
- `finstack/io/src/providers/*`

## PR-11.6: System Ingestion Path

Scope: Allow system actors to bypass review to `SYSTEM_VERIFIED`.

Tasks:
1. Add `ActorKind::SYSTEM` handling and allow `SYSTEM_VERIFIED` transitions for system actors.
2. Allow direct creation into final state for system ingestions.
3. Ensure provenance is recorded in `resource_changes` and `workflow_events`.

Acceptance:
1. Integration test: system actor writes `SYSTEM_VERIFIED`, data is visible to entitled users.

Primary files:
- `finstack/io/src/workflow/*.rs`
- `finstack/io/src/providers/*`

## PR-11.7: Governed Store Surface + Feature Flag

Scope: Introduce governed API surface and toggle.

Tasks:
1. Add `StoreHandle::as_actor(ctx)` -> `GovernedHandle`.
2. Provide governed `get_*`, `list_*`, and change proposal APIs.
3. Add `FINSTACK_IO_GOVERNANCE=on/off` feature flag.

Acceptance:
1. Existing behavior is unchanged when governance is off.
2. Governance on denies access without entitlements.

Primary files:
- `finstack/io/src/config.rs`
- `finstack/io/src/store.rs`

## PR-11.8: Postgres RLS (Optional Defense-in-Depth)

Scope: Optional RLS for Postgres deployments.

Tasks:
1. Add SQL functions for `current_user_id` and `can_read`.
2. Add RLS policies for verified tables and `resource_changes`.
3. Add config toggle to enable RLS.

Acceptance:
1. RLS enabled blocks unauthorized reads.
2. RLS disabled preserves current behavior.

Primary files:
- `finstack/io/src/sql/migrations.rs`
- `finstack/io/src/providers/postgres/*`

## PR-11.9: Documentation + Examples

Scope: Update documentation and examples.

Tasks:
1. Add governance overview to `finstack/io/README.md`.
2. Add example in `finstack-py/examples/scripts/io` for draft -> verify and system ingest.
3. Document default policies and how to customize them.

Acceptance:
1. Documentation explains workflows and permissions clearly.
2. Example runs end-to-end in SQLite.

Primary files:
- `finstack/io/README.md`
- `finstack-py/examples/scripts/io/*`
