# PR-09: Testing, Parity & Documentation

## Summary

Establish comprehensive testing and documentation coverage for the UI Kit, including unit, integration, visual regression, and golden parity tests, as well as Storybook-based component documentation and API references.

## Background & Motivation

- Implements **Phase 9: Testing & Documentation** from `UI_KIT_ROADMAP.md` and `UI_KIT_TESTING.md`.
- Ensures >80% test coverage, schema parity checks, LLM dashboard snapshots, and performance budgets.

## Scope

### In Scope

- Unit tests (Vitest + React Testing Library) for components and hooks across domains.
- Integration tests (Playwright or similar) for critical end-to-end flows.
- Visual regression tests via Storybook + Chromatic (or equivalent) for key components.
- Golden parity tests ensuring numeric equivalence with Rust engine outputs.
- API documentation and Storybook stories for all public components.

### Out of Scope

- New functional features beyond what is necessary to support testing and documentation.

## Design & Implementation Details

### 1. Unit Testing

- Expand unit test coverage per `UI_KIT_TESTING.md`:
  - Components: primitives, charts, tables, domain panels.
  - Hooks: `useFinstack`, `useValuation`, `useStatement`, `usePortfolio`, etc.
  - Edge cases: negative values, missing market data, panic handling, and loading/error states.
- Organize tests under `__tests__/` and co-located `*.test.tsx` files.

### 2. Integration Testing

- Configure Playwright test suite with example routes:
  - Calibration flows (discount/forward curves, vol surfaces).
  - Basic portfolio valuation and grouping.
  - Statements viewing and editing (including corkscrew tracing).
  - Scenario creation and execution.
- Ensure WASM initialization and error boundaries are verified in browser context.

### 3. Visual Regression Testing

- Set up Storybook for `packages/finstack-ui`:
  - Stories for primitives, charts, tables, domain components, and complex dashboards.
  - Chromatic (or alternative) integration for snapshot-based visual regression.
- Configure responsive viewports and themes (light/dark) in stories.

### 4. Golden Parity Tests

- Import golden JSON fixtures from `finstack` Rust tests (or mirror them into JS fixtures) for:
  - Bond and swap pricing.
  - Portfolio valuations.
  - Statements metrics and scenarios.
- Implement Vitest tests that:
  - Call WASM bindings directly from JS.
  - Assert numeric parity to decimal precision.

### 5. Schema Parity Tests

- For each Rust type with TS/Zod counterparts:
  - Validate that Rust-emitted JSON passes Zod validation.
  - Generate JSON from Zod fixtures and validate on Rust side (where appropriate) via CI scripts.
- Add CI checks that fail on schema mismatches.

### 6. LLM Dashboard Snapshots

- Maintain a library of canonical dashboard JSONs under `fixtures/dashboards/`:
  - Basic Rates, Portfolio, Statements, Scenarios, and Analysis examples.
- Snapshot tests:
  - Validate JSON against `DashboardDefinitionSchema`.
  - Render with `DynamicRenderer` and capture structural snapshots.

### 7. Documentation & API References

- Generate and maintain API docs for:
  - Public React components, hooks, and schemas.
  - GenUI functions and mutation actions.
- Use Storybook as live documentation with annotated stories.
- Ensure README and higher-level docs in `finstack-wasm/docs` link to UI Kit documentation.

## Dependencies

- Requires prior functional PRs (PR-01 through PR-08) to provide the feature surface to test.
- Requires CI infrastructure to run browser tests and publish Storybook artifacts.

## Acceptance Criteria

- Test coverage >80% across UI Kit codebase.
- Golden parity tests pass for key numerical paths.
- Storybook runs locally and is published in CI, with basic visual regression enabled.
- CI fails on schema parity or golden parity regressions.
