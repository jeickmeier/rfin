# Quick Start Guide

Follow these steps to run the finstack-wasm examples:

## Step 1: Build the WASM Package

From the `finstack-wasm` directory:

```bash
npm run build
```

Or using wasm-pack directly:

```bash
wasm-pack build --target web --out-dir pkg
```

This creates the `pkg/` directory with the compiled WASM module and TypeScript definitions.

## Step 2: Install Example Dependencies

```bash
npm run examples:install
```

Or manually:

```bash
cd examples
npm install
```

## Step 3: Run the Development Server

```bash
npm run examples:dev
```

Or manually from the examples directory:

```bash
cd examples
npm run dev
```

This will:

- Start a Vite development server at http://localhost:3000
- Enable hot module replacement
- Open your browser automatically

## What You'll See

The example application demonstrates comprehensive date and market data functionality with feature parity to the Python bindings:

### Date & Calendar Functionality

1. **Date Construction & Properties** - Creating dates, accessing components, weekend checks, quarter/fiscal year
2. **Date Utilities** - Month arithmetic, month-end handling, leap years, epoch conversions
3. **Calendars & Business Day Adjustments** - Holiday calendars, business day checks, adjustment conventions
4. **Day Count Conventions** - Act/360, Act/365F, 30/360, Act/Act (ISDA/ISMA), BUS/252 with contexts
5. **Schedule Builder** - Monthly/quarterly/semi-annual schedules with stub rules and CDS IMM
6. **Period Plans** - Calendar and fiscal periods with actual/forecast segmentation
7. **IMM Dates & Option Expiries** - IMM dates, CDS rolls, equity option expiries
8. **Tenor Conventions** - Standard and custom tenors

### Market Data

- USD discount curve (OIS)
- FX matrix with USD/EUR rates
- CPI time series interpolation
- Equity spot prices
- Proper WASM memory management

## Troubleshooting

### "Cannot find module 'finstack-wasm'"

Make sure you've built the WASM package first (Step 1).

### Build Errors

Ensure you have Rust and wasm-pack installed:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

### Port Already in Use

If port 3000 is already in use, Vite will automatically try the next available port, or you can specify one:

```bash
npm run dev -- --port 3001
```

## Next Steps

- Explore the source code in `src/examples/`
- Add your own examples
- Check `README.md` for more detailed documentation
- Review TypeScript definitions in `../pkg/finstack_wasm.d.ts`
