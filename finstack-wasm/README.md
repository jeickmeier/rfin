# Finstack WASM Bindings

WebAssembly bindings for the Finstack financial computation library.

## Building

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web targets
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs
```

## Usage

### Web Browser

```javascript
import init, { Date, Money, Currency } from './pkg/finstack_wasm.js';

async function run() {
    await init();
    
    // Create a date
    const date = new Date(2024, 1, 1);
    
    // Create currency and money
    const usd = new Currency("USD");
    const amount = new Money(100.0, usd);
    
    console.log(amount.amount); // 100.0
    console.log(amount.currency.code); // "USD"
}

run();
```

### Node.js

```javascript
const { Date, Money, Currency } = require('./pkg-node/finstack_wasm.js');

// Use the same API as above
const usd = new Currency("USD");
const amount = new Money(100.0, usd);
```

## Testing

```bash
wasm-pack test --chrome --firefox --headless
```