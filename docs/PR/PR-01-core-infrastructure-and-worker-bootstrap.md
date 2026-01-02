# PR-01: Core Infrastructure & Unified Worker Bootstrap

## Summary

Establish the `finstack-ui` library with React/TypeScript, Tailwind + Shadcn UI, testing/tooling, and the initial WASM integration layer. Implement the unified Finstack engine worker, panic handling, `FinstackProvider`, and core hooks (`useFinstack`, `useFinstackEngine`) plus the initial set of financial input primitives required by later phases.

## Background & Motivation

- Implements **Phase 1: Core Infrastructure** from `UI_KIT_ROADMAP.md`.
- Aligns with architecture and stack choices in `UI_KIT_ARCHITECTURE.md` and `UI_KIT_DESIGN.md` (React 19, TS strict, Vite, Zustand, TanStack, Tailwind, Shadcn).
- Lays the foundation for later GenUI work, domain components, and LLM integrations by providing a reliable WASM bridge and common UI primitives.

## Scope

### In Scope

- Create `finstack-ui` in the monorepo with:
  - Vite library config (React + TS strict mode).
  - Tailwind CSS + `clsx` + `tailwind-merge` + Shadcn base primitives.
  - Basic `src` layout matching `UI_KIT_ARCHITECTURE.md` (components, hooks, store, workers, schemas, engine, utils).
- Implement unified Finstack engine worker bootstrap with panic handling hooks (per `UI_KIT_HOOKS_AND_WORKERS.md` and `UI_KIT_ADRS.md` ADR-001/002/010).
- Implement `FinstackProvider` context and initial hooks:
  - `useFinstack` (configuration, market context, rounding context, loading/error state).
  - `useFinstackEngine` (typed RPC to unified worker).
- Implement worker pool singleton and WASM initialization singleton (`ensureWasmInit`, `canInitWasm`).
- Implement first-pass **financial primitives** required in later phases:
  - `AmountDisplay` (string-only, no JS math, respects rounding context).
  - `AmountInput`.
  - `CurrencySelect`.
  - `TenorInput`.
  - `DatePicker` (business-day-aware shell, with hooks to plug calendars later).
- Set up test infrastructure:
  - Vitest + React Testing Library for unit tests.
  - Basic test scaffolding for hooks and primitives.

### Out of Scope

- Full domain-specific components (Valuations, Portfolio, Statements, Market, Scenarios) beyond what is needed to test the infrastructure.
- GenUI schema pipeline, ComponentRegistry, DynamicRenderer, or LLM integration (handled in PR-02 and PR-05).
- Advanced performance tuning and virtualization thresholds (initial hooks only; details in PR-06).

## Design & Implementation Details

### 1. Project Structure & Tooling

- Create `finstack-ui/package.json` configured as a Vite library (ESM + type declarations) targeting React 19.
- Enable TypeScript strict mode and path aliases for internal modules (`@finstack-ui/components`, `@finstack-ui/hooks`, etc.).
- Configure Tailwind with a design-token-aware config consistent with `UI_KIT_DEVELOPMENT_AND_A11Y.md` and `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`.
- Import Shadcn-generated primitives into `src/components/ui/` and wrap where necessary for consistent theming.

### 2. Directory Layout

- Implement the high-level layout from `UI_KIT_ARCHITECTURE.md`:

  - `src/components/primitives/` – `AmountDisplay`, `AmountInput`, `CurrencySelect`, `TenorInput`, `DatePicker`.
  - `src/components/ui/` – Shadcn primitives (Button, Card, etc.).
  - `src/hooks/` – hooks layer entrypoints (`useFinstack`, `useFinstackEngine`).
  - `src/workers/` – unified engine worker implementation (initial skeleton).
  - `src/store/` – placeholder for Zustand store types (Engine/UI separation introduced later).
  - `src/utils/` – error handling helpers, accessibility helpers, and WASM init utilities.

### 3. WASM Initialization & Panic Handling

- Implement `lib/wasmSingleton.ts` following `UI_KIT_HOOKS_AND_WORKERS.md`:
  - `ensureWasmInit(): Promise<void>` – memoized promise calling `finstack-wasm`'s `init()` exactly once.
  - `canInitWasm(): boolean` – SSR guard based on `typeof window !== 'undefined'`.
- Add a Rust-side panic hook (in `finstack-wasm`) that converts panics to structured errors consumable by JS, following `UI_KIT_HOOKS_AND_WORKERS.md` and `UI_KIT_ADRS.md`.
- Ensure the unified worker calls `init()` once and shares initialized state across requests.

### 4. Unified Engine Worker (Bootstrap)

- Create `src/workers/finstackEngine.ts` as a Comlink-exposed worker implementing an initial subset of the API described in `UI_KIT_GENUI_AND_SCHEMAS.md` and `UI_KIT_HOOKS_AND_WORKERS.md`:
  - `initialize()` – sets up `MarketContext` and `FinstackConfig` instances.
  - `loadMarket(marketJson: string): Promise<string>` – returns a handle ID (e.g., `market-main`).
  - `priceInstrument(instrumentJson: string): Promise<ValuationResult>` – minimal valuation capability to support smoke tests.
- Store engine state (market, config) in worker memory, not in the main thread, laying groundwork for the **Handle Pattern** described in `UI_KIT_GENUI_AND_SCHEMAS.md`.
- Implement minimal error normalization so React components can display friendly error messages.

### 5. `FinstackProvider` & Hooks

- Implement `FinstackProvider` context (per `UI_KIT_HOOKS_AND_WORKERS.md`):
  - Manages WASM readiness (via `ensureWasmInit` and worker `initialize`).
  - Exposes `isReady`, `isLoading`, `error`, `config`, `market`, `setMarket`, and `roundingContext`.
  - Guards against SSR by no-op behavior when `canInitWasm() === false`.
  - Integrates with React Suspense for pending WASM initialization and error boundaries for panics.
- Implement `useFinstack` hook:
  - Reads from `FinstackProvider` context.
  - Throws helpful errors when used outside provider.
- Implement `useFinstackEngine` hook:
  - Wraps Comlink worker reference.
  - Ensures `initialize()` has been called.
  - Provides type-safe API surface to the worker.

### 6. Financial Primitives (String Transport)

- Implement primitives according to `UI_KIT_DESIGN.md`, `UI_KIT_DOMAINS.md`, and ADR-002:
  - `AmountDisplay`
    - Accepts **string values** (no JS `number` arithmetic).
    - Delegates formatting to a helper that respects `RoundingContext` and locale.
  - `AmountInput`
    - Emits string values compatible with Rust-side decimal expectations.
    - Composes with `CurrencySelect` to produce a stable `MoneyTransport` object where needed.
  - `CurrencySelect`
    - Uses `Currency` enum from `finstack-wasm` once schema bindings are available, initially stubbed with ISO-4217 codes.
  - `TenorInput` and `DatePicker`
    - Emit structured values compatible with WASM types (`Tenor`, `DateSpec`), with a placeholder calendar until full calendar support is wired.

### 7. Testing & Quality

- Add unit tests using Vitest + React Testing Library:
  - `AmountDisplay` renders correctly formatted text for simple currencies.
  - `FinstackProvider` exposes `isReady` after WASM initialization.
  - `useFinstackEngine` returns a usable worker API and passes through a simple `priceInstrument` call (using mocked or lightweight WASM bindings).
- Wire tests into the repo-wide `make test-rust` / JS test commands as appropriate.
- Ensure basic linting (ESLint + TypeScript) is configured for `finstack-ui`.

## Dependencies

- Depends on existing `finstack-wasm` bindings and build configuration.
- Must not break existing WASM exports; any Rust changes are limited to panic hooks or configuration types used by the worker.

## Acceptance Criteria

- `finstack-ui` builds successfully via Vite in library mode.
- `FinstackProvider` and hooks can be used in a minimal example page to:
  - Initialize WASM.
  - Load a small `MarketContext` fixture.
  - Price a single Bond instrument via the unified worker.
- `AmountDisplay` and other primitives render and round sample values correctly using string transport.
- Unit tests for primitives and hooks pass in CI.
