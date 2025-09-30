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

The example application demonstrates:

1. **Period Plan Example**
   - Generates fiscal quarters (2024 Q1-Q4)
   - Shows period IDs, start/end dates, and actual/forecast status
   - Demonstrates period DSL parsing

2. **Market Data Example**
   - Creates a USD discount curve (OIS)
   - Builds an FX matrix with USD/EUR rates
   - Interpolates CPI time series data
   - Stores and retrieves equity spot prices
   - Shows proper WASM memory management

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
