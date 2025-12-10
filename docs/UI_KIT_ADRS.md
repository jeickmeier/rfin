# Finstack UI Kit: Architecture Decision Records (ADRs)

## 6. Architecture Decision Records (ADRs)

### ADR-001: Unified Worker over Domain-Specific Workers

**Decision:** Use a single `finstackEngine` worker instead of separate `valuationWorker`, `statementWorker`, etc.

**Rationale:** 

- Market Context (5–10MB) would be duplicated across workers.
- Shared state enables cross-domain operations (e.g., scenarios affecting both market and statements).
- Single initialization point simplifies error handling.

**Trade-offs:**

- Single point of failure (mitigated by panic hooks).
- Cannot parallelize across domains (acceptable given computation profiles).

---

### ADR-002: String Transport for Monetary Values

**Decision:** All monetary values cross the WASM bridge as strings, never as JavaScript numbers.

**Rationale:**

- JavaScript floats lose precision (0.1 + 0.2 ≠ 0.3).
- Rust uses `rust_decimal` with arbitrary precision.
- Golden test parity requires exact decimal representation.

**Trade-offs:**

- Slightly higher serialization overhead.
- Cannot use JS number formatting directly (mitigated by `AmountDisplay` component).

---

### ADR-003: Canvas Overlay for Dependency Visualization

**Decision:** Use a transparent `<canvas>` overlay for corkscrew arrows instead of SVG lines between DOM elements.

**Rationale:**

- TanStack Virtual removes off-screen DOM nodes.
- Cannot draw lines between non-existent elements.
- Canvas allows math-based coordinate calculation independent of DOM.

**Trade-offs:**

- Requires manual coordinate math.
- Canvas redraws on every scroll (mitigated by `requestAnimationFrame`).

---

### ADR-004: Schema Generation Pipeline

**Decision:** Auto-generate TypeScript types from Rust using `ts-rs` or `specta`, then derive Zod schemas.

**Rationale:**

- Manual sync between Rust and TypeScript is error-prone.
- LLM integration requires accurate schemas.
- Single source of truth (Rust) reduces drift.

**Trade-offs:**

- Build-time dependency on schema generation.
- Generated code must not be manually edited.

---

### ADR-005: Schema Versioning for LLM Persistence

**Decision:** All LLM-facing schemas include a `schemaVersion` field with migration functions between versions.

**Rationale:**

- LLM-generated dashboards may be persisted and reloaded months later.
- Schema evolution is inevitable.
- Without versioning, old JSON becomes invalid after updates.

**Trade-offs:**

- Migration code must be maintained.
- Complexity increases with each version.

---

### ADR-006: Engine/UI State Separation

**Decision:** Hard-separate `EngineState` (protocol-like, versioned) from `UIState` (transient, unversioned) in the Zustand store.

**Rationale:**

- Serializing everything for LLMs or history is heavy.
- Only `EngineState + DashboardDefinition` typically needed for snapshots.
- Easier to version protocol-like engine state than transient UI bits.

**Trade-offs:**

- Two state trees to manage.
- Must ensure UI state doesn't accidentally depend on stale engine state.

---

### ADR-007: Layout Templates over Component Soup

**Decision:** LLMs choose from predefined layout templates (TwoColumn, Grid, TabSet, Report) rather than positioning arbitrary component arrays.

**Rationale:**

- Unconstrained LLM output produces noisy, inconsistent layouts.
- Templates ensure professional-looking dashboards.
- Smaller schema surface area for LLMs.

**Trade-offs:**

- Less flexibility for advanced users.
- May need to add new templates over time.

---

### ADR-008: Mutation Actions for LLM Interactions

**Decision:** LLMs send granular mutation actions (add_component, update_component, etc.) rather than full dashboard definitions.

**Rationale:**

- Smaller payloads.
- Natural undo/redo (each action = history entry).
- Easier to reason about diffs.
- LLMs make fewer mistakes with simple operations.

**Trade-offs:**

- More action types to implement and document.
- Must handle action validation and rollback.

---

### ADR-009: Generic Instrument Form System

**Decision:** Use descriptor-based `GenericInstrumentPanel` for most instruments; hand-craft custom panels only for complex cases.

**Rationale:**

- 30+ instrument panels is a maintenance nightmare.
- Most instruments have similar structure: inputs → valuation → cashflows → metrics.
- Complex instruments (Swaption, Convertible) genuinely need custom UI.

**Trade-offs:**

- Descriptors must stay in sync with Rust types.
- Generic panels may feel less polished than custom ones.

---

### ADR-010: Singleton Worker Pool

**Decision:** Use a single shared worker pool rather than spawning workers per component.

**Rationale:**

- Multiple components mounting workers = resource exhaustion.
- Worker spawn/teardown overhead for short tasks.
- Shared market context avoids memory duplication.

**Trade-offs:**

- No parallelism across domains (acceptable given computation profiles).
- Pool management complexity.

---

### ADR-011: LLM-Safe Component Modes

**Decision:** Components that modify state expose a `mode` prop: `viewer | editor | llm-assisted`. In `llm-assisted` mode, mutations require user confirmation.

**Rationale:**

- LLMs can suggest trades, scenarios, optimizations—powerful and dangerous.
- Guard rails at component level, not just API level.
- Clear UX distinction between human actions and AI suggestions.

**Trade-offs:**

- Extra UI for confirmation dialogs.
- Mode prop propagation through component trees.



