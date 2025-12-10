# Finstack UI Kit: Development Guidelines, Accessibility & Theming

## 3.7 Development Guidelines

1. **WASM Separation:** UI components never import `finstack-wasm` directly. They use `hooks/` which manage Suspense, Error Boundaries, and async loading.
2. **Strict Typing:** Props must use `finstack` types (e.g., `Currency` enum), not strings.
3. **Metric Integrity:** Never format numbers manually. Use the `RoundingContext` from the WASM engine and `AmountDisplay` components.
4. **Error Handling:** Wrap all "Organisms" in Error Boundaries to gracefully handle WASM panics (e.g., "Curve missing for GBP").
5. **State Serialization:** All component state must be JSON-serializable for LLM snapshots.

---

## 3.7.1 Accessibility Requirements

- **Keyboard Navigation:** All interactive elements must be keyboard accessible.
- **ARIA Labels:** Financial data tables must have proper ARIA labels for screen readers.
- **Focus Management:** Modal dialogs and drawers must trap focus appropriately.
- **High Contrast:** Support high-contrast mode for trading floor environments.
- **Announcements:** Real-time value changes should be announced to screen readers.

Example helper:

```typescript
// utils/accessibility.ts
export function announceToScreenReader(message: string) {
  const announcement = document.createElement('div');
  announcement.setAttribute('aria-live', 'polite');
  announcement.setAttribute('aria-atomic', 'true');
  announcement.className = 'sr-only';
  announcement.textContent = message;
  document.body.appendChild(announcement);
  setTimeout(() => announcement.remove(), 1000);
}
```

---

## 3.7.2 Internationalization

- **Number Formatting:** Use `Intl.NumberFormat` with `RoundingContext` settings.
- **Currency Display:** ISO-4217 codes with localized symbols.
- **Date Formatting:** Business calendar + locale-aware display.

```typescript
// utils/formatting.ts
export function formatAmount(
  value: number,
  currency: Currency,
  locale: string = navigator.language
): string {
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: currency.code,
    minimumFractionDigits: currency.decimals,
    maximumFractionDigits: currency.decimals,
  }).format(value);
}
``>

---

## 3.7.3 Theming

Support both light and dark themes with CSS variables:

```css
/* themes/financial.css */
:root {
  /* Light theme */
  --color-positive: #16a34a;
  --color-negative: #dc2626;
  --color-neutral: #6b7280;
  
  /* Risk heatmap colors */
  --heatmap-low: #10b981;
  --heatmap-mid: #f59e0b;
  --heatmap-high: #ef4444;
}

[data-theme="dark"] {
  --color-positive: #22c55e;
  --color-negative: #f87171;
  --color-neutral: #9ca3af;
}
```

Use design tokens consistently across charts, tables, and text to provide a cohesive visual language.



