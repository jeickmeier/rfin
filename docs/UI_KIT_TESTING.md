# Finstack UI Kit: Testing & Quality Strategy

## 3.8 Testing Strategy

### 3.8.1 Unit Tests (Vitest + React Testing Library)

```typescript
// __tests__/components/AmountDisplay.test.tsx
import { render, screen } from '@testing-library/react';
import { AmountDisplay } from '../components/primitives/AmountDisplay';
import { Currency } from 'finstack-wasm';

// Mock WASM module
vi.mock('finstack-wasm', () => ({
  Currency: vi.fn().mockImplementation((code) => ({ code, decimals: 2 })),
}));

describe('AmountDisplay', () => {
  it('formats USD amount correctly', () => {
    render(<AmountDisplay value={1234.56} currency="USD" />);
    expect(screen.getByText('$1,234.56')).toBeInTheDocument();
  });
});
```

Focus:

- Rendering correctness (formatting, styling).
- Behavior of hooks with mocked WASM APIs.
- Edge cases (negative values, missing data, loading states).

### 3.8.2 Integration Tests (Playwright)

```typescript
// e2e/calibration.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Discount Curve Calibration', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/examples/calibration');
    // Wait for WASM to initialize
    await page.waitForSelector('[data-testid="wasm-ready"]');
  });

  test('calibrates curve from deposit quotes', async ({ page }) => {
    // Enter quotes
    await page.fill('[data-testid="quote-rate-0"]', '0.05');
    await page.fill('[data-testid="quote-tenor-0"]', '1Y');
    
    // Run calibration
    await page.click('[data-testid="calibrate-button"]');
    
    // Verify success
    await expect(page.locator('[data-testid="calibration-status"]'))
      .toHaveText('Success');
  });
});
```

Goals:

- Validate end-to-end flows (e.g., calibration, valuation, scenario execution).
- Confirm WASM initialization and error boundaries behave correctly in the browser.

### 3.8.3 Visual Regression (Storybook + Chromatic)

```typescript
// stories/CurveChart.stories.tsx
import type { Meta, StoryObj } from '@storybook/react';
import { CurveChart } from '../components/charts/CurveChart';

const meta: Meta<typeof CurveChart> = {
  component: CurveChart,
  title: 'Charts/CurveChart',
  parameters: {
    chromatic: { viewports: [320, 768, 1200] },
  },
};
```

Intent:

- Lock in visual baselines for key components.
- Catch regressions in theming, layout, and responsive behavior.

### 3.8.4 Golden Tests (Rust Parity)

```typescript
// __tests__/parity/bondPricing.test.ts
import { describe, it, expect, beforeAll } from 'vitest';
import init, { Bond, MarketContext, FinstackConfig } from 'finstack-wasm';
import goldenValues from '../../fixtures/golden/bond_pricing.json';

describe('Bond Pricing Parity', () => {
  beforeAll(async () => {
    await init();
  });

  goldenValues.testCases.forEach((testCase) => {
    it(`matches Rust output for ${testCase.name}`, () => {
      const bond = Bond.fromJSON(testCase.instrument);
      const market = MarketContext.fromJSON(testCase.market);
      const config = new FinstackConfig();
      
      const pv = bond.price(market, config);
      
      // Match to 6 decimal places (Decimal precision)
      expect(pv.amount).toBeCloseTo(testCase.expected.pv, 6);
    });
  });
});
```

Objectives:

- Ensure numeric parity between UI-layer bindings and core Rust engine.
- Prevent silent drift in pricing/metric calculations.



