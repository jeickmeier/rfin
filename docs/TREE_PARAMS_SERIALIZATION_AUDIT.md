# Tree Model Parameters Serialization Audit

**Date**: 2025-11-13  
**Status**: Complete  
**Result**: No changes required

## Executive Summary

Tree model parameter and configuration types in `finstack/valuations/src/instruments/common/models/trees/` are **transient runtime structures only** and do not require `Serialize`/`Deserialize` traits at this time. They are not part of any persistent JSON schema, instrument specification, or wire format.

## Audited Types

The following types were reviewed and confirmed to be runtime-only:

### Core Tree Models
- `BinomialTree` - Runtime pricing engine (not config)
- `TrinomialTree` - Runtime pricing engine (not config)
- `MultiFactorTree` - Runtime pricing engine (not config)
- `ShortRateTree` - Runtime pricing engine (not config)
- `TwoFactorBinomialTree` - Runtime pricing engine (not config)
- `RatesCreditTree` - Runtime pricing engine (not config)

### Configuration Structures
- `TreeType` (enum) - Algorithm selector passed to constructors
- `TrinomialTreeType` (enum) - Algorithm selector passed to constructors
- `ShortRateModel` (enum) - Algorithm selector passed to constructors
- `ShortRateTreeConfig` - Config struct built in-memory within pricing helpers
- `MultiFactorConfig` - Config struct (currently placeholder, not used in production)
- `TwoFactorBinomialConfig` - Config struct built in-memory within tests
- `RatesCreditConfig` - Config struct built in-memory within bond pricing

### Generic Framework Types
- `TreeParameters` - Generic parameter holder (currently unused in production)
- `EvolutionParams` - Generic evolution spec (currently unused in production)
- `StateVariables` - Runtime HashMap for tree node state
- `NodeState` - Runtime structure passed to valuators
- `TreeGreeks` - Output structure from tree pricing
- `RecombiningInputs` - Internal engine parameters

### Supporting Types
- `BarrierSpec` - Runtime config for barrier options in trees
- `BarrierStyle` (enum) - Knock-in/knock-out selector
- `TreeBranching` (enum) - Binomial vs Trinomial selector
- `BarrierType` (enum) - Up/Down and In/Out barrier types
- `FactorType` (enum) - Multi-factor tree factor descriptor

## Usage Patterns

### Where Tree Types Are Used

1. **Bond Pricing with Embedded Options** (`bond/types.rs`, `bond/pricing/tree_pricer.rs`)
   - `ShortRateTree` and `ShortRateTreeConfig` are constructed in-memory
   - Configuration is hardcoded or derived from instrument attributes
   - Trees are calibrated to discount curves at runtime

2. **Convertible Bond Pricing** (`convertible/pricer.rs`)
   - `BinomialTree` and `TrinomialTree` instantiated with step count
   - No configuration persisted; parameters come from pricing context

3. **Unit Tests**
   - All test files create tree configs locally
   - No deserialization from JSON or external sources

### Where Tree Types Are NOT Used

1. **Instrument Specifications**
   - No instrument struct (Bond, ConvertibleBond, etc.) contains tree types as fields
   - Tree selection/configuration is implicit in pricing logic

2. **JSON Import/Export** (`json_loader.rs`)
   - `InstrumentJson` tagged union contains no tree-related variants
   - `InstrumentEnvelope` does not reference tree configurations

3. **Schemas Directory** (`finstack/valuations/schemas/`)
   - No JSON schema files reference tree parameter types
   - All 41 schema files are instrument-specific (bonds, options, swaps, etc.)

4. **Public Re-exports** (`common/mod.rs`, `instruments/mod.rs`)
   - While `BinomialTree` and `TreeType` are re-exported publicly, they are used only as API types for programmatic construction
   - No serde bounds on public API

## Decision

**No serialization support is needed** for any tree-related types at this time.

### Rationale

1. **Transient Nature**: All tree models and configs are created on-demand during pricing and discarded after use
2. **No Persistence Layer**: No use cases require saving/loading tree configurations
3. **Programmatic Construction**: Tree parameters are either hardcoded defaults or computed from market data
4. **Schema Stability**: Adding serde now would commit to a stable schema without a clear requirement

## Future Extension Pattern

If a future requirement emerges to persist tree configurations (e.g., for scenario analysis, calibration storage, or instrument attributes), the recommended approach is:

1. **Add serde support at the config layer only**:
   ```rust
   #[derive(Clone, Debug)]
   #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
   #[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
   pub struct ShortRateTreeConfig {
       pub steps: usize,
       pub model: ShortRateModel,
       pub volatility: f64,
       pub mean_reversion: Option<f64>,
   }
   ```

2. **Lock the schema with golden tests**:
   - Add roundtrip tests in the relevant `tests/` directory
   - Generate and commit JSON schema files to `finstack/valuations/schemas/`
   - Include version field if needed for evolution

3. **Keep runtime structures non-serializable**:
   - `BinomialTree`, `TrinomialTree`, etc. remain internal implementation
   - Only configuration/specification types gain serde derives

4. **Document the schema in mdBook**:
   - Add section to `book/src/valuations/` explaining tree configuration format
   - Provide examples of persisted tree configs if this becomes a feature

## Related Files

### Implementation
- `finstack/valuations/src/instruments/common/models/trees/binomial_tree.rs`
- `finstack/valuations/src/instruments/common/models/trees/trinomial_tree.rs`
- `finstack/valuations/src/instruments/common/models/trees/short_rate_tree.rs`
- `finstack/valuations/src/instruments/common/models/trees/multi_factor_tree.rs`
- `finstack/valuations/src/instruments/common/models/trees/two_factor_binomial.rs`
- `finstack/valuations/src/instruments/common/models/trees/two_factor_rates_credit.rs`
- `finstack/valuations/src/instruments/common/models/trees/tree_framework.rs`
- `finstack/valuations/src/instruments/common/models/trees/mod.rs`

### Consumers
- `finstack/valuations/src/instruments/bond/types.rs` (lines 395-448)
- `finstack/valuations/src/instruments/bond/pricing/tree_pricer.rs`
- `finstack/valuations/src/instruments/convertible/pricer.rs`
- `finstack/valuations/tests/instruments/bond/integration/tree_calibration_validation.rs`
- `finstack/valuations/tests/instruments/convertible/test_pricing_trees.rs`
- `finstack/valuations/tests/instruments/common/test_barrier_trees.rs`

### JSON Infrastructure
- `finstack/valuations/src/instruments/json_loader.rs` (InstrumentEnvelope, InstrumentJson)
- `finstack/valuations/schemas/*.json` (41 instrument schema files)

## Conclusion

This audit confirms that GAP 4 requires **no code changes**. All tree model parameters and configuration types are correctly scoped as transient runtime structures. The codebase follows best practices by keeping implementation details (tree models) separate from stable wire formats (instrument JSON specs).

If serialization becomes necessary in the future, the recommended pattern above ensures backward compatibility and schema stability.

