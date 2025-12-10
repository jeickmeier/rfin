# PR-06: Editors, Draft Mode & History

## Summary

Implement cross-domain editor infrastructure and **Draft Mode** for safe editing of market data, trades, statements, and scenarios. Provide undo/redo history and responsive editing behavior using React 19 features like `useDeferredValue`.

## Background & Motivation

- Implements **Phase 6: Editors & Draft Mode** from `UI_KIT_ROADMAP.md`.
- Builds on state separation and schemas from `UI_KIT_DESIGN.md` and GenUI patterns from PR-02.
- Provides a safe, LLM-aware editing experience across all domains with clear draft vs committed state.

## Scope

### In Scope

- Implement `useDraftStore` and history management in Zustand:
  - Maintain separate draft and committed versions of engine/UI state.
  - Track history stack for undo/redo.
- Introduce `EditableGrid` for quote entry and `TradeEntryForm` enhancements.
- Implement `ConstraintEditor` for portfolio optimization constraints.
- Add undo/redo middleware for editor actions.
- Use `useDeferredValue` and related React features to keep editing responsive under heavy computation.

### Out of Scope

- Full optimization execution and efficient frontier visualization (handled in PR-08).
- Detailed UX flows for every individual editor (focus on shared infrastructure and 2–3 representative editors).

## Design & Implementation Details

### 1. Draft Store & History

- Implement a Zustand store in `src/store/draftStore.ts`:
  - `draftEngine` and `draftUI` mirroring `EngineStateSchema` and `UIStateSchema`.
  - `committedEngine` and `committedUI` for last-saved state.
  - History stack of actions or snapshots (bounded for memory safety).
- Provide actions:
  - `applyDraftUpdate(patch)` – applies JSON-serializable patches.
  - `commitDraft()` – promotes draft state to committed and pushes to history.
  - `revertDraft()` – resets draft to last committed state.
  - `undo()` / `redo()` – navigate the history stack.

### 2. `useDraftStore` Hook

- Provide a hook that exposes:
  - Current draft state slices for each domain (market, portfolio, statements, scenarios).
  - Action creators for applying updates and committing them.
  - Flags for dirty/clean state per domain.
- Ensure JSON-serializability and stable shapes to preserve LLM interoperability.

### 3. Editors & Grids

- `EditableGrid` (under `components/tables/EditableGrid.tsx`):
  - Built on top of `VirtualDataTable` with cell-level editing.
  - Zod-based validation for each column (quotes, constraints, etc.).
  - Integrates with `useDraftStore` so edits are draft-only until committed.
- Enhance `TradeEntryForm` (from PR-04):
  - Draft-mode awareness and history integration.
  - Clear visual cues for draft vs committed changes.
- Implement `ConstraintEditor` in `domains/portfolio/optimization/`:
  - Editor UI for optimization constraints (e.g., position limits, sector caps).
  - Uses descriptors and Zod schemas to define constraint types and bounds.

### 4. React 19 Responsiveness

- Use `useDeferredValue` and/or `startTransition` for expensive recomputations triggered by editor changes:
  - For example, while editing a large quote grid, schedule recomputations in a non-blocking way.
- Ensure editing stays responsive even when background valuations or statement recalculations are triggered.

### 5. LLM-Safe Modes & Draft Mode

- Align Draft Mode with component `mode` prop (ADR-011):
  - In `llm-assisted` mode, LLM-suggested changes always land in draft and require explicit human commit.
  - History metadata tracks action source (human vs LLM) for auditability.
- Add simple UI around editors showing whether the user is editing draft or committed state and which actor initiated changes.

### 6. Testing & Validation

- Unit tests:
  - Draft store behavior (apply, commit, undo, redo) with JSON-serializable snapshots.
  - `EditableGrid` validation and integration with draft state.
- Integration tests:
  - Example flow where user edits quotes, reviews results, and commits changes with ability to undo.
- Performance tests:
  - Basic checks that editing remains responsive (no frame jank) for medium-sized grids.

## Dependencies

- Requires PR-02 for state schemas and GenUI.
- Builds on valuation, portfolio, and statements examples from PR-03–PR-05.

## Acceptance Criteria

- Users can edit quotes or trades in a draft state, see immediate UI feedback, and choose when to commit.
- Undo/redo works reliably for at least quote edits and trade entries.
- LLM-assisted flows demonstrate that suggestions remain draft until confirmed.
