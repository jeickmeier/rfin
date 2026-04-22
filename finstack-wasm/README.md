# Finstack WASM Bindings

`finstack-wasm` packages the Rust Finstack workspace for browser and Node.js
consumers via `wasm-bindgen`. The package exposes a namespaced JavaScript and
TypeScript facade that mirrors the Rust umbrella crate structure instead of
forcing consumers to import the raw `wasm-bindgen` output directly.

## Available Namespaces

The current package exposes these top-level namespaces:

- `core`
- `analytics`
- `margin`
- `monte_carlo`
- `portfolio`
- `scenarios`
- `statements`
- `statements_analytics`
- `valuations` (includes nested `valuations.correlation`)

These namespaces are assembled in `index.js` from the files under `exports/`,
while the Rust binding implementations live under `src/api/`.

## Package Layout

- `index.js`: namespaced facade for the published package.
- `index.d.ts`: package-level TypeScript declarations.
- `exports/`: per-domain JavaScript namespace shims.
- `src/api/`: Rust bindings grouped by workspace domain.
- `tests/`: wasm integration coverage for analytics, margin, Monte Carlo,
  portfolio, scenarios, statements, statements analytics, and valuations.
- `pkg/` and `pkg-node/`: generated web and Node.js build output.

## Quick Start

```javascript
import init, { core, valuations, portfolio, scenarios } from 'finstack-wasm';

await init();

const usd = new core.Currency('USD');
const amount = new core.Money(1000.0, usd);

console.log(amount.toString());
console.log(Object.keys({ valuations, portfolio, scenarios }));
```

The default export is the WASM initializer. Call it once at application startup
before using the namespaced APIs.

## Building

From the repository root:

```bash
mise run wasm-pkg
```

From `finstack-wasm/` directly:

```bash
npm run build
npm run build:node
```

## Testing And Quality Checks

Package-local commands:

```bash
npm run test
npm run test:browser
npm run lint
npm run format:check
```

Root-level equivalents:

```bash
mise run wasm-test
mise run all-lint
```

## Relationship To Rust And Python

The WASM package follows the same domain split as the Rust workspace and the
Python package in `finstack-py`. The bindings are maintained as thin
conversion-oriented wrappers, so the canonical product structure still lives in
the Rust crates.

For analytics specifically, the WASM surface is intentionally pure-function
oriented today. It does not expose the stateful Rust `Performance` panel API;
use the Python bindings for that facade, or compose the standalone analytics
functions directly in JS/TS.

## License

MIT OR Apache-2.0
