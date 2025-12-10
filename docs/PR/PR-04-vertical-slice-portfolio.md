# PR-04: Vertical Slice #2 – Portfolio

## Summary

Implement the second vertical slice focused on **Portfolio** functionality, including entity and position management, portfolio valuation views, attribute-based grouping, scenario impact visualization, and early LLM safety modes on portfolio editors.

## Background & Motivation

- Implements **Phase 4: Vertical Slice #2 – Portfolio** from `UI_KIT_ROADMAP.md`.
- Follows the Portfolio domain designs from `UI_KIT_DOMAINS.md` (Section B: Portfolio).
- Builds directly on top of Valuations work from PR-03 and the GenUI foundation from PR-02.

## Scope

### In Scope

- Entity & position views:
  - `EntityTreeView` for entity hierarchy.
  - `PositionGrid` with virtual scrolling.
  - `TradeEntryForm` for manual trade input.
- Portfolio valuation and metrics:
  - `PortfolioSummaryPanel`.
  - `PortfolioMetricsPanel`.
  - Basic `InstrumentRiskTable` for per-position DV01/CS01/Greeks.
- Attribute-based grouping and pivots:
  - `TagPivotGrid` for grouping by rating, sector, strategy, etc.
- Scenario & what-if views at portfolio level:
  - `PortfolioScenarioImpactView` (integrates with Scenarios domain later, but with basic stub flows here).
- Initial LLM-safe interaction patterns for portfolio editors (`viewer | editor | llm-assisted`).

### Out of Scope

- Full optimization surfaces and efficient frontier views (covered in PR-08).
- Margin aggregation and attribution breadth beyond the initial summary (future breadth expansion).

## Design & Implementation Details

### 1. Domain Structure

- Under `src/domains/portfolio/` add:
  - `components/`
    - `EntityTreeView.tsx`.
    - `PositionGrid.tsx`.
    - `PortfolioSummaryPanel.tsx`.
    - `PortfolioMetricsPanel.tsx`.
    - `TagPivotGrid.tsx`.
    - `PortfolioScenarioImpactView.tsx` (initial version).
    - `TradeEntryForm.tsx`.
  - `hooks/`
    - `usePortfolio.ts` – wraps WASM portfolio valuation APIs.
  - `schemas/`
    - Zod schemas for `Portfolio`, `Entity`, `Position`, `PositionUnit`, `PortfolioValuationResult`, etc., generated via `ts-rs`/`specta` and wrapped.

### 2. Entity & Position Management

- `EntityTreeView`:
  - Displays entities as a tree with lazy loading for large hierarchies.
  - Integrates with `PositionGrid` selection and filters.
- `PositionGrid`:
  - Uses `VirtualDataTable` for performance.
  - Shows positions, key instrument identifiers, and summary metrics.
  - Row selection triggers detailed views (instrument panels from Valuations domain).
- `TradeEntryForm`:
  - Uses descriptors and primitives to build forms for `Position` and `PositionUnit`.
  - Validates inputs via Zod and React Hook Form.

### 3. Portfolio Valuation & Metrics

- `usePortfolio` hook:
  - Encapsulates calls to WASM functions like `value_portfolio` and metrics computation.
  - Uses handle pattern where possible (portfolio loaded once, valuations updated via deltas).
- `PortfolioSummaryPanel`:
  - Presents total PV, base-currency aggregates, and high-level risk metrics.
  - Displays FX policy and numeric mode metadata using `MetadataPanel` from `components/common/`.
- `PortfolioMetricsPanel`:
  - Shows aggregated DV01/CS01/Greeks by portfolio/entity.
  - Complemented by `InstrumentRiskTable` for per-position details.

### 4. Attribute-Based Grouping & Pivots

- `TagPivotGrid`:
  - Uses grouping and aggregation helpers from portfolio Rust crate (`grouping.rs`).
  - UI for selecting grouping keys (e.g., `asset_class`, `rating`, `strategy`).
  - Displays aggregated PV and risk metrics for each group.
- Integrate with GenUI:
  - Allow `TagPivotGrid` to be a registered component with schema-defined props specifying grouping fields and metrics.

### 5. Scenario & What-If Views

- `PortfolioScenarioImpactView`:
  - Initial implementation that can display pre/post deltas for PV and a small set of metrics.
  - Consumes a simplified scenario application API from the unified worker (to be extended in PR-08).
  - Lays out impact by entity and top-level tags.

### 6. LLM-Safe Component Modes

- For portfolio editing components (e.g., `TradeEntryForm`, `TagPivotGrid` when editing tags):
  - Expose `mode: 'viewer' | 'editor' | 'llm-assisted'` prop.
  - In `llm-assisted` mode, require explicit user confirmation before persisting changes.
  - Use safe defaults and input bounds for LLM-initiated actions.

### 7. Testing & Validation

- Unit tests:
  - Portfolio hooks calling mocked WASM APIs.
  - `TagPivotGrid` grouping and aggregation logic.
- Integration tests (Playwright or equivalent):
  - Load an example portfolio with a small number of positions.
  - Confirm valuations and groupings update when editing positions or tags.
- Snapshot tests:
  - GenUI dashboards containing portfolio components are validated against `DashboardDefinitionSchema` and rendered without runtime errors.

## Dependencies

- Requires PR-03 for instruments and valuations infrastructure.
- Assumes portfolio-related WASM bindings and Rust types are exported and available via the schema pipeline.

## Acceptance Criteria

- Example portfolio page allows users to:
  - Browse entities and positions.
  - View aggregated portfolio PV and risk metrics.
  - Group positions by tags (rating, sector, strategy).
- Portfolio components are registered in `ComponentRegistry` and usable via GenUI dashboards.
- LLM-assisted mode on `TradeEntryForm` and other editors demonstrates confirmation flows in example UIs.
