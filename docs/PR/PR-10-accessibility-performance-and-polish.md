# PR-10: Accessibility, Performance & Final Polish

## Summary

Complete the UI Kit with a focused pass on accessibility, performance, bundle size, and polish. Implement accessibility audits, keyboard navigation checks, high-contrast theming, bundle optimization, and long-session stability testing.

## Background & Motivation

- Implements **Phase 10: Accessibility & Final Polish** from `UI_KIT_ROADMAP.md` and guidance from `UI_KIT_DEVELOPMENT_AND_A11Y.md` and `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`.
- Ensures the UI Kit meets professional-grade standards for trading and risk environments.

## Scope

### In Scope

- Accessibility audits using tools like axe-core.
- Keyboard navigation and screen reader checks across key components.
- High-contrast theme support and validation.
- Bundle size optimization and lazy loading for heavy dependencies.
- Memory leak and long-running session testing.

### Out of Scope

- New business features; this PR is quality-focused.

## Design & Implementation Details

### 1. Accessibility Audits

- Integrate axe-core checks into Storybook and/or Playwright tests.
- Verify:
  - All interactive elements are keyboard-accessible.
  - ARIA labels are present for financial data tables and key controls.
  - Modals and drawers correctly trap focus.
- Address all WCAG 2.1 AA issues identified during audits.

### 2. Keyboard Navigation & Screen Readers

- Systematically test keyboard navigation paths for:
  - Primitives (inputs, selects, grids).
  - Domain components (calibration views, statements viewer, portfolio grids, scenarios builder).
- Ensure screen readers (NVDA, VoiceOver) correctly announce:
  - Table headers and cells.
  - Changes in values (using helpers like `announceToScreenReader`).

### 3. Theming & High Contrast

- Finalize theme tokens in CSS (light/dark and high-contrast) as per `UI_KIT_DEVELOPMENT_AND_A11Y.md`:
  - Colors for positive/negative/neutral values.
  - Heatmap gradients for risk and attribution views.
- Validate high-contrast legibility for charts, tables, and overlays.

### 4. Performance & Bundle Size

- Apply lazy loading patterns from `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`:
  - Split heavy dependencies (ECharts, WebGL charts, advanced analysis views) into separate chunks or an optional `finstack-ui-pro` package.
  - Use `React.Suspense` and skeleton loaders.
- Implement and enforce performance budgets in CI:
  - Maximum bundle size thresholds (<300KB core, <500KB with pro features, excluding WASM).
  - Rendering time budgets for critical views (e.g., large tables, risk heatmaps).

### 5. Long-Running Sessions & Memory Leaks

- Design tests that simulate long-lived sessions:
  - Frequent recalculations, viewport changes, and scenario runs.
  - Ensure workers and subscriptions are cleaned up correctly.
- Use browser profiling tools to detect and address leaks or runaway memory growth.

## Dependencies

- Requires all functional features (PR-01 through PR-09) to be in place so audits cover real usage.

## Acceptance Criteria

- Accessibility audits show no critical WCAG 2.1 AA violations for key components.
- Keyboard-only workflows are viable across major flows (valuation, portfolio, statements, scenarios).
- Core bundle size meets defined thresholds, with heavy features lazy-loaded.
- No significant memory leaks identified in long-running usage scenarios.
