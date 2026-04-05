# Add an Instrument

This guide walks through adding a new instrument type end-to-end:
Rust struct → Instrument trait → Pricer → Python binding → tests.

## Step 1: Define the Rust Struct

Add the struct in the appropriate `finstack/valuations/instruments/` module:

```rust,no_run
/// A new derivative type.
#[derive(Debug, Clone)]
pub struct MyDerivative {
    /// Unique instrument identifier.
    id: String,
    /// Notional amount.
    notional: Money,
    /// Discount curve identifier.
    disc_id: String,
    // ... other fields
}
```

## Step 2: Add to InstrumentType Enum

Add a variant to `InstrumentType` in `finstack/valuations/src/instrument_type.rs`:

```rust,no_run
pub enum InstrumentType {
    // ... existing variants
    MyDerivative,
}
```

## Step 3: Implement the Instrument Trait

```rust,no_run
impl Instrument for MyDerivative {
    fn id(&self) -> &str { &self.id }
    fn key(&self) -> InstrumentType { InstrumentType::MyDerivative }
    // ... implement remaining methods
}
```

## Step 4: Create a Builder

Use the fluent builder pattern:

```rust,no_run
pub struct MyDerivativeBuilder {
    id: String,
    notional: Option<Money>,
    disc_id: Option<String>,
}

impl MyDerivativeBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into(), notional: None, disc_id: None }
    }

    pub fn notional(mut self, m: Money) -> Self {
        self.notional = Some(m); self
    }

    pub fn disc_id(mut self, id: impl Into<String>) -> Self {
        self.disc_id = Some(id.into()); self
    }

    pub fn build(self) -> Result<MyDerivative> {
        Ok(MyDerivative {
            id: self.id,
            notional: self.notional.ok_or(Error::MissingField("notional"))?,
            disc_id: self.disc_id.ok_or(Error::MissingField("disc_id"))?,
        })
    }
}
```

## Step 5: Implement a Pricer

See [Add a Pricer](add-pricer.md).

## Step 6: Python Binding

See [Add a Python Binding](add-python-binding.md).

## Step 7: Tests

1. **Rust unit test**: Test construction and basic pricing
2. **Rust integration test**: Test with full market context
3. **Python parity test**: Verify Python produces same results as Rust

## Checklist

- [ ] Rust struct with doc comments on all public fields
- [ ] `InstrumentType` variant added
- [ ] `Instrument` trait implemented
- [ ] Builder with validation
- [ ] Pricer registered in `standard_registry()`
- [ ] Python binding with `.pyi` stub
- [ ] WASM binding (if applicable)
- [ ] Unit tests + parity tests
- [ ] `cargo clippy` clean (`-D warnings`)
