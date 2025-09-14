# Immediate Action Plan - Market Context Serialization

## Current Status ✅

We have successfully:
1. Added `to_state()` and `from_state()` methods to `DiscountCurve` and `ForwardCurve`
2. Simplified the serialization code and removed ~100 lines of workarounds
3. Documented the architectural limitations clearly
4. All tests passing (no regressions)

## Immediate Next Steps (Week 1)

### Day 1-2: Setup and Planning
- [ ] Create feature branch: `feature/market-context-enum-storage`
- [ ] Set up feature flags in `Cargo.toml`
- [ ] Create new module structure:
  ```
  finstack/core/src/market_data/
    storage/
      mod.rs
      curve_storage.rs
      curve_state.rs
    context_v2/
      mod.rs
      builder.rs
      serde.rs
  ```
- [ ] Schedule design review meeting with stakeholders

### Day 3-4: Implement CurveStorage Enum
- [ ] Create `CurveStorage` enum with all curve variants
- [ ] Implement conversion methods (`as_discount()`, `as_forward()`, etc.)
- [ ] Add state conversion methods
- [ ] Write unit tests for enum operations

### Day 5: Proof of Concept
- [ ] Create minimal `MarketContextV2` with just curves
- [ ] Implement basic `insert_discount()` and `disc()` methods
- [ ] Add serialization for the PoC
- [ ] Create round-trip serialization test

## Week 2: Core Implementation

### Milestone 1: Complete CurveStorage (Days 6-8)
- [ ] Add remaining curve type support
- [ ] Implement full serialization/deserialization
- [ ] Add property-based tests
- [ ] Benchmark enum dispatch vs trait objects

### Milestone 2: MarketContextV2 (Days 9-10)
- [ ] Port all insertion methods
- [ ] Port all getter methods  
- [ ] Add builder pattern support
- [ ] Implement bump system with new storage

## Week 3: Migration Support

### Milestone 3: Compatibility Layer (Days 11-13)
- [ ] Create compatibility wrapper
- [ ] Add feature flag switching
- [ ] Write migration utilities
- [ ] Document migration path

### Milestone 4: Testing (Days 14-15)
- [ ] Port all existing tests to new system
- [ ] Add migration tests
- [ ] Performance regression tests
- [ ] Integration tests with downstream code

## Week 4: Integration and Polish

### Milestone 5: Integration (Days 16-18)
- [ ] Update valuations crate to use new API
- [ ] Update Python bindings
- [ ] Update WASM bindings
- [ ] Fix any integration issues

### Milestone 6: Documentation (Days 19-20)
- [ ] Update API documentation
- [ ] Write migration guide
- [ ] Update examples
- [ ] Create announcement for users

## Decision Points

### Week 1 Checkpoint
**Decision**: Proceed with full implementation?
- Review PoC performance
- Assess migration complexity
- Get stakeholder approval

### Week 2 Checkpoint  
**Decision**: Feature flag strategy
- Dual compilation vs runtime switch
- Migration timeline
- Deprecation schedule

### Week 3 Checkpoint
**Decision**: Release strategy
- Alpha release to select users?
- Parallel systems in production?
- Rollback plan

## Risk Register

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Performance regression | High | Low | Benchmark early and often |
| Breaking API changes | High | Medium | Compatibility layer |
| Complex migration | Medium | High | Feature flags, gradual rollout |
| Serialization format change | Medium | Low | Version field in data |

## Success Metrics

- [ ] All curve types fully serializable
- [ ] No string parsing for bumped curves
- [ ] Performance within 5% of current
- [ ] Zero breaking changes to public API
- [ ] 100% test coverage maintained
- [ ] Migration path documented and tested

## Resource Requirements

### Development
- 1 senior engineer full-time for 4 weeks
- Code review from 2 other engineers
- 20% time from architect for design reviews

### Testing
- QA engineer for integration testing (Week 3-4)
- Performance testing infrastructure
- Staging environment for parallel testing

### Documentation
- Technical writer for migration guide (Week 4)
- Update all affected documentation

## Communication Plan

### Internal
- Daily standups during implementation
- Weekly progress reports to management
- Design review meetings as needed

### External
- Announcement when feature branch ready
- Beta testing invitation (Week 3)
- Migration guide publication (Week 4)
- Deprecation notice for old system

## Quick Start Commands

```bash
# Create feature branch
git checkout -b feature/market-context-enum-storage

# Set up initial structure
mkdir -p finstack/core/src/market_data/storage
mkdir -p finstack/core/src/market_data/context_v2

# Run tests with new feature
cargo test --features new-context

# Benchmark old vs new
cargo bench --features "legacy-context new-context"

# Build with both for migration testing
cargo build --features "legacy-context new-context"
```

## Code Review Checklist

- [ ] Enum exhaustiveness in all match statements
- [ ] Proper error handling for type conversions
- [ ] No panic! in production code paths
- [ ] Serialization format backward compatible
- [ ] Performance benchmarks included
- [ ] Documentation updated
- [ ] Tests cover all edge cases
- [ ] Migration path tested

## Definition of Done

A phase is complete when:
1. All code implemented and reviewed
2. All tests passing (unit, integration, property)
3. Performance benchmarks acceptable
4. Documentation complete
5. Migration path tested
6. Stakeholder approval received

## Escalation Path

1. Technical issues → Team Lead
2. Design decisions → System Architect  
3. Timeline/resource → Project Manager
4. Breaking changes → Product Owner

## Next Meeting

**Design Review Session**
- Date: [Week 1, Day 2]
- Attendees: Dev team, Architect, Product Owner
- Agenda: Review enum design, approve approach
- Deliverable: Go/No-go decision

---

This plan provides a clear, actionable path forward. The phased approach minimizes risk while delivering value incrementally. Each week has clear milestones and decision points to ensure we stay on track.
