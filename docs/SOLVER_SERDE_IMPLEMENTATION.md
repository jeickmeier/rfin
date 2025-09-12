# Mathematical Solver Serialization Implementation

## PR #6: Add Serde Support for Math Solvers

### Overview
Successfully implemented serialization support for mathematical solver configurations, enabling reproducible numerical computations across sessions and systems.

### Implementation Summary

#### 1. Solver Configurations (`math/solver.rs`)
Added conditional serde derives to all solver structures:
- **NewtonSolver**: Serializes tolerance, max_iterations, and fd_step
- **BrentSolver**: Serializes tolerance, max_iterations, bracket_expansion, and initial_bracket_size
- **HybridSolver**: Serializes both internal Newton and Brent solver configurations

#### 2. Integration Configurations (`math/integration.rs`)
Implemented custom serialization for `GaussHermiteQuadrature`:
- **Challenge**: The struct contains `&'static [F]` fields that cannot be directly serialized
- **Solution**: Serialize only the quadrature order (5, 7, or 10) and reconstruct the static data on deserialization
- **Benefits**: Minimal serialized size while maintaining full functionality

#### 3. Random Number Generator (`math/random.rs`)
Added serde support to `SimpleRng`:
- Serializes the internal state (u64)
- Enables checkpointing of Monte Carlo simulations
- Allows reproducible random number sequences

### Key Features

1. **Conditional Compilation**: All serde derives use `#[cfg_attr(feature = "serde", ...)]` to avoid unnecessary dependencies
2. **Custom Serialization**: Smart handling of static data in GaussHermiteQuadrature
3. **Full Test Coverage**: Comprehensive tests for serialization/deserialization roundtrips
4. **Functional Equivalence**: Tests verify that deserialized objects produce identical results

### Testing
Created comprehensive test suite in `finstack/core/tests/test_solver_serde.rs`:
- Tests serialization/deserialization for all solver types
- Verifies functional equivalence of deserialized objects
- Tests RNG state preservation across serialization
- All 7 tests passing

### Example Usage

```rust
// Serialize a Newton solver configuration
let solver = NewtonSolver::new()
    .with_tolerance(1e-10)
    .with_max_iterations(100);
let json = serde_json::to_string(&solver)?;

// Deserialize and use
let solver2: NewtonSolver = serde_json::from_str(&json)?;
let root = solver2.solve(f, initial_guess)?;
```

```rust
// Serialize quadrature (only stores order)
let quad = GaussHermiteQuadrature::order_7();
let json = serde_json::to_string(&quad)?; // {"order":7}

// Deserialize reconstructs full quadrature
let quad2: GaussHermiteQuadrature = serde_json::from_str(&json)?;
let integral = quad2.integrate(f);
```

### Benefits

1. **Reproducibility**: Save and restore exact solver configurations
2. **Configuration Management**: Store solver settings in configuration files
3. **Distributed Computing**: Share configurations across systems
4. **Checkpointing**: Save computation state for long-running processes
5. **Testing**: Create golden test files with specific configurations

### Files Modified

- `finstack/core/src/math/solver.rs` - Added serde derives to solver structs
- `finstack/core/src/math/integration.rs` - Custom serde implementation for GaussHermiteQuadrature
- `finstack/core/src/math/random.rs` - Added serde derive to SimpleRng

### Files Created

- `finstack/core/tests/test_solver_serde.rs` - Comprehensive test suite
- `examples/python/solver_serialization_example.py` - Python example demonstrating usage

### Verification

- ✅ All tests passing (7/7)
- ✅ No clippy warnings
- ✅ No compiler warnings
- ✅ Example runs successfully
- ✅ Maintains backward compatibility (serde feature is optional)

### Next Steps

This implementation enables:
- Saving calibration solver configurations for reproducible results
- Checkpointing long-running Monte Carlo simulations
- Configuration management for different solver strategies
- Integration with configuration files and databases
