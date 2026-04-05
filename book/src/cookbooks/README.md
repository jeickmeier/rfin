# Cookbooks

Step-by-step recipes for common financial workflows. Each cookbook shows the same
operation in both Rust and Python, with WASM/TypeScript placeholders for future
expansion.

## Conventions

- Every cookbook is self-contained — copy-paste and run.
- Market data setup is explicit, never hidden behind a helper.
- Realistic financial terminology (not `foo`/`bar`).
- Each recipe links to the relevant architecture page for deeper context.

## Recipes

| Recipe | Description |
|--------|-------------|
| [Curve Building](curve-building.md) | Bootstrap discount and forward curves from market quotes |
| [Bond Pricing](bond-pricing.md) | Price a fixed-rate bond, compute DV01, CS01, z-spread |
| [Swap Pricing](swap-pricing.md) | IRS, basis swaps, cross-currency swaps |
| [Options Pricing](options-pricing.md) | Caps/floors, swaptions, equity options |
| [Credit Analysis](credit-analysis.md) | CDS pricing, hazard curve construction, CDX |
| [Portfolio Valuation](portfolio-valuation.md) | Build a portfolio, run valuation, aggregate risk |
| [Scenario Analysis](scenario-analysis.md) | Define shocks, run scenarios, compare results |
| [Statement Modeling](statement-modeling.md) | Waterfalls, covenants, forecasting |
| [Monte Carlo](monte-carlo.md) | MC engine setup, path generation, convergence |
| [P&L Attribution](pnl-attribution.md) | Daily P&L decomposition by risk factor |
| [Exotic Options](exotic-options.md) | Barriers, autocallables, cliquets, Asian options |
| [Margin & Netting](margin-netting.md) | Margin calculations, netting sets |
