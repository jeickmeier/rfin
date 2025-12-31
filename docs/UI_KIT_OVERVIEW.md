# Finstack UI Kit: Overview & Product Requirements

## 1. Executive Summary

The **Finstack UI Kit** is a specialized React component library designed to serve as the "visual frontend" for the `finstack-wasm` financial engine. It solves a unique architectural challenge: **bridging strict, deterministic financial computation (Rust/WASM) with dynamic, probabilistic orchestration (LLMs/AI Agents).**

This library will empower developers and AI agents to build high-performance financial applications—including pricing dashboards, risk reports, and interactive financial statement models—where the UI state is a serializable artifact that both humans and LLMs can read, write, and reason about.

---

## 2. Product Requirements (PRD)

### 2.1 Product Vision

To create a "Lego set" for financial engineering that is:

1. **Mathematically Correct:** Enforces the same precision, rounding, and currency safety as the core Rust engine.
2. **AI-Native:** Designed from the ground up to be controlled by LLMs via structured JSON, allowing agents to "render" answers (e.g., *"Here is the risk heatmap you asked for"*).
3. **High Performance:** Capable of rendering large cashflow trees and volatility surfaces without main-thread blocking.
4. **Accessible:** Full keyboard navigation, screen reader support, and high-contrast modes for professional trading environments.

### 2.2 Target Audience

1. **Financial App Developers:** Building internal tools for trading desks, risk management, or FP&A.
2. **Quants/Analysts:** Using "Notebook-like" interfaces to interactively explore models.
3. **AI Agents:** LLMs that need a standard output format to visualize complex financial data instead of just text.

### 2.3 Core Capabilities

#### A. The Financial Primitives (Foundation)

- **Strict Inputs:** Specialized form controls for Currency (ISO-4217), Tenors (1M, 10Y), Dates (Business Day adjustment), and Rates (Bps vs %).
- **Precision Display:** `AmountDisplay` components that respect the global `finstack` `RoundingContext`.

#### B. The Visualization Layer (Components)

- **Market Data:** Interactive Yield Curves (Zero/Forward), Volatility Surfaces (3D/Heatmap).
- **Valuations:** Cashflow Waterfalls, Risk/Greeks Heatmaps, PnL Attribution Waterfalls.
- **Statements:** "Corkscrew" financial models, Balance Sheet projections, Forecast Editors.
- **Portfolio:** Position grids, Book hierarchy trees.

#### C. The GenUI Bridge (Orchestration)

- **Dynamic Renderer:** A system that accepts a JSON "View Definition" and renders the corresponding interactive component tree.
- **State Serialization:** Ability to snapshot the entire UI context (Market + Portfolio + User Edits) into JSON for LLM analysis.
- **Scenario Orchestrator:** High-level control plane that composes and applies scenarios (via the `scenarios` crate) across Market, Valuations, and Statements, with deterministic reports for each run.

---

## 5. Success Criteria

### 5.1 Functional

1. **Numeric Parity:** All displayed values match Rust engine output exactly (string transport, no JS float math).
2. **Schema Sync:** 100% of Rust types have auto-generated TypeScript counterparts via `ts-rs`/`specta`.
3. **Error Recovery:** WASM panics are caught and surfaced via Error Boundaries without crashing the tab.
4. **Schema Versioning:** All LLM-facing schemas include `schemaVersion` field with migration support.

### 5.2 Performance

5. **60fps Rendering:** Smooth scrolling on 10,000-row virtualized tables.
6. **Handle Pattern:** Market context serialization occurs ≤1 time per session (delta updates thereafter).
7. **Interactive Editing:** Curve drag operations complete in <16ms (single frame budget).
8. **Lazy Loading:** ECharts/3D components load on-demand, not in initial bundle.

### 5.3 LLM Integration

9. **Zero Hallucination:** LLM never generates numeric values; all data comes from WASM engine.
10. **Context Efficiency:** LLM context payloads < 4KB (semantic summaries, not raw data).
11. **Schema Validation:** LLM-generated JSON validates against dynamic Zod schemas with < 5% rejection rate.
12. **Mutation Actions:** LLMs use granular actions (add/remove/update) not full dashboard redefines.
13. **Safe Modes:** All sensitive components support `viewer | editor | llm-assisted` modes.

### 5.4 Quality

14. **Accessibility:** WCAG 2.1 AA compliance.
15. **Bundle Size:** < 300KB core gzipped, < 500KB with pro features (excluding WASM).
16. **Test Coverage:** > 80% line coverage, all golden tests passing.
17. **Schema Parity Tests:** Every Rust ↔ TS ↔ Zod type validated in CI.
18. **LLM Dashboard Snapshots:** Canonical dashboard outputs maintained as regression tests.
