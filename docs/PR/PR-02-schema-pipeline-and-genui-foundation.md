# PR-02: Schema Pipeline & GenUI Foundation

## Summary

Introduce the Rust→TypeScript→Zod schema pipeline, define `DashboardDefinitionSchema v1`, and implement the first version of the GenUI foundation: `ComponentRegistry`, `DynamicRenderer`, mutation actions, and basic LLM-facing schema generation utilities.

## Background & Motivation

- Implements **Phase 2: Schema Pipeline & GenUI Foundation** from `UI_KIT_ROADMAP.md`.
- Leverages the schema-first design in `UI_KIT_DESIGN.md` and `UI_KIT_GENUI_AND_SCHEMAS.md`.
- Critical for enabling AI-native usage where LLMs generate JSON view definitions and mutation actions validated against Zod schemas.

## Scope

### In Scope

- Integrate `ts-rs` or `specta` in relevant Rust crates (`finstack-wasm` submodules) to auto-generate TypeScript types for engine-facing structures.
- Create a `packages/finstack-ui/src/schemas/generated/` directory populated from Rust via `ts-rs`/`specta`.
- Build Zod wrappers around generated TS types for runtime validation, including schema versioning for LLM-facing types.
- Define and implement `DashboardDefinitionSchema v1` and supporting schemas:
  - `LayoutTemplateSchema` (Single, TwoColumn, Grid, TabSet, Report).
  - `ComponentInstanceSchema` including `mode: 'viewer' | 'editor' | 'llm-assisted'`.
  - `DataBindingsSchema` and `BindingPathSchema` (Data Binding DSL).
- Implement `ComponentRegistry` with typed component registration, props schemas, and LLM metadata.
- Implement `DynamicRenderer` that consumes a `DashboardDefinition` and renders registered components safely.
- Implement initial mutation action schemas and reducers for GenUI dashboard editing.
- Provide `toLLMContext()` helpers and `schemaGenerator.ts` to produce OpenAI function schemas from Zod.

### Out of Scope

- Full breadth of engine types (focus on a minimal subset for vertical slice #1).
- Advanced migration strategies between multiple dashboard schema versions (only `schemaVersion: '1'` and a stub migration API).
- Domain-specific GenUI presets for portfolio/statements/scenarios (handled in later PRs).

## Design & Implementation Details

### 1. Rust→TypeScript Generation

- In `finstack-wasm`, annotate selected structs with `TS`/`specta` macros, following examples in `UI_KIT_GENUI_AND_SCHEMAS.md`:
  - Start with a small set of types needed for rates instruments and market context (`BondSpec`, `MarketContextWire`, etc.).
- Configure `export_to = "../../packages/finstack-ui/src/schemas/generated/"` so generated TS files land inside the UI package.
- Ensure the generated TS types align exactly with Rust serde shapes (field names, optionality, enums).

### 2. Zod Wrappers & Versioning

- Wrap generated TS types in Zod schemas located in `src/schemas/`:
  - `EngineStateSchema`, `MarketContextWireSchema`, `ValuationResultWireSchema`, etc., as outlined in `UI_KIT_DESIGN.md`.
- Implement versioned LLM-facing types with explicit `schemaVersion`:
  - `VersionedDashboardDefinition` (per `UI_KIT_DESIGN.md` and ADR-005).
  - Stub `migrateDashboard(data, fromVersion, toVersion)` with a single supported path or a `TODO` plus tests confirming rejection of unknown versions.

### 3. Engine/UI State Separation (Skeleton)

- Introduce `EngineStateSchema`, `UIStateSchema`, and `RootStateSchema` in `src/store/types.ts` (see `UI_KIT_DESIGN.md`):
  - `engine` contains versioned wire state for market, portfolios, statements, and cached results.
  - `ui` contains `activeView`, `panelState`, selections, filters.
  - No full Zustand store implementation yet; this PR only defines schemas/types for later PRs to use.
- Ensure all state schemas are JSON-serializable (no classes, Maps, or functions).

### 4. Data Binding DSL

- Implement `BindingSourceSchema`, `BindingPathSchema`, and `DataBindingsSchema` as described in `UI_KIT_DESIGN.md` and `UI_KIT_GENUI_AND_SCHEMAS.md`:
  - Sources: `market`, `portfolio`, `statements`, `scenarios`.
  - `path` validated via a regex to enforce documented grammar.
- Provide TypeScript utilities and examples for LLM documentation, demonstrating typical bindings for curves, portfolio PV, and statement KPIs.

### 5. Dashboard Definition & Layout Templates

- Implement `DashboardDefinitionSchema`:
  - `schemaVersion: '1'`.
  - `id`, `name`, `layout`, `components`, `bindings`, `userIntent`, `createdAt`, `updatedAt`.
- Implement discriminated `LayoutTemplateSchema` covering the initial layouts (Single, TwoColumn, Grid, TabSet, Report) per `UI_KIT_ARCHITECTURE.md`.
- Implement `ComponentInstanceSchema`:
  - Enforce `id` as UUID.
  - `type` (string key to be validated at runtime against the `ComponentRegistry`).
  - `props` as `z.record(z.unknown())`, to be fully validated by per-component `propsSchema` at runtime.
  - `mode` enum with default `'viewer'` to support LLM-safe modes (ADR-011).

### 6. Component Registry

- Implement `engine/ComponentRegistry.ts` as described in `UI_KIT_GENUI_AND_SCHEMAS.md`:
  - `RegisteredComponent<TProps>` interface containing `Component`, `propsSchema`, description, example props, and `allowedModes`.
  - Registration API to add entries at startup.
  - Retrieval API used by `DynamicRenderer`.
- Add initial components to the registry for use in later vertical slices (e.g., `CurveChart`, `BondPanel`, `SwapPanel`) as placeholders with stub props schemas.

### 7. Dynamic Renderer

- Implement `engine/DynamicRenderer.tsx`:
  - Accepts `dashboard: DashboardDefinition`.
  - For each `ComponentInstance`:
    - Looks up registry entry by `type`.
    - Validates `props` against entry `propsSchema`.
    - Verifies requested `mode` is in `allowedModes`.
    - Renders the component wrapped in an error boundary that is WASM-aware.
  - Composes components into layouts based on `layout.kind`.
- Add an `onError(componentId, error)` callback prop for logging and LLM feedback.

### 8. Mutation Actions & Reducers

- Define `DashboardActionSchema` in `engine/dashboardActions.ts` as a Zod discriminated union:
  - `add_component` (component instance + layout slot/position).
  - `update_component` (by `id`, partial props update).
  - `remove_component`.
  - `reorder_components`.
- Implement pure reducer functions that:
  - Take `DashboardDefinition` and an action.
  - Return a new `DashboardDefinition` with changes applied.
  - Enforce layout invariants (no orphaned component IDs, consistent layout arrays).
- These reducers are later used both by humans (via UI) and LLMs (via mutation actions).

### 9. LLM Function Schema Generation

- Implement `schemas/functionSchemas.ts` and/or `schemaGenerator.ts` using `zod-to-json-schema`:
  - Generate JSON schemas for:
    - `DashboardDefinitionSchema`.
    - `DashboardActionSchema`.
    - Selected component props schemas from the registry.
- Provide a helper to export **OpenAI function calling** definitions that can be used by agents to construct dashboards and actions.

## Dependencies

- Requires `PR-01` (Finstack UI package and basic infrastructure) to be merged first.
- Relies on `finstack-wasm` exporting selected Rust types with `ts-rs`/`specta` macros.

## Acceptance Criteria

- `ts-rs`/`specta` successfully generates TS files into `src/schemas/generated/` without manual edits.
- Zod schemas compile and can validate sample engine state and dashboard definitions.
- `DynamicRenderer` can render a simple hard-coded dashboard using 2–3 registered components.
- `DashboardAction` reducers pass unit tests for all supported actions.
- Generated OpenAI function schemas exist for at least `DashboardDefinition` and `DashboardAction` and validate against sample payloads.
