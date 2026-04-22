---
name: documentation-reviewer
description: Reviews and improves documentation for public APIs. Ensures descriptions, arguments, return types, usage examples, and academic references for math/financial code are present and complete. Use when reviewing documentation quality, writing docstrings, adding API documentation, or when documentation is missing or incomplete.
---

# Documentation Reviewer

## Quick start

When reviewing documentation, produce a review with:

1. **Summary**: coverage level, what's missing.
2. **Missing documentation**: list of undocumented or poorly documented items.
3. **Action items**: specific documentation to add, with templates.

After each review cycle, write the missing documentation and re-check. Continue iterating until all public APIs are documented. If items remain undocumented, treat as incomplete.

## Documentation requirements

All public APIs must have:

| Requirement | Rust | Python |
|-------------|------|--------|
| Description | `///` doc comment | Docstring first line |
| Arguments | `# Arguments` section | `Parameters` section |
| Returns | `# Returns` section | `Returns` section |
| Examples | `# Examples` with code block | `Examples` section |
| References | `# References` for math/finance | `Sources` section |

### When references are required

Add academic/research references when the code implements:
- Pricing models (Black-Scholes, Black-76, SABR, Heston, etc.)
- Day count conventions (ISDA standards)
- Greeks calculations (analytical or numerical)
- Monte Carlo methods (path generation, variance reduction)
- Curve construction (bootstrap, interpolation)
- Risk calculations (VaR, SIMM, sensitivities)
- Any algorithm with a canonical academic source

Link to anchors in `docs/REFERENCES.md` for canonical citations.

## Rust documentation template

```rust
/// Brief one-line description.
///
/// Extended description explaining the purpose, behavior,
/// and any important details.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// Description of return value.
///
/// # Examples
///
/// ```rust
/// use my_crate::MyType;
///
/// let result = my_function(arg1, arg2);
/// assert_eq!(result, expected);
/// ```
///
/// # References
///
/// - Black (1976): see docs/REFERENCES.md#black1976
/// - Hull: Options, Futures, and Other Derivatives
pub fn my_function(param1: Type1, param2: Type2) -> ReturnType {
    // implementation
}
```

## Python documentation template

```python
def my_function(param1: Type1, param2: Type2) -> ReturnType:
    """Brief one-line description.

    Extended description explaining the purpose, behavior,
    and any important details.

    Parameters
    ----------
    param1 : Type1
        Description of first parameter.
    param2 : Type2
        Description of second parameter.

    Returns
    -------
    ReturnType
        Description of return value.

    Examples
    --------
    >>> result = my_function(arg1, arg2)
    >>> result
    expected_output

    Sources
    -------
    - Black (1976): see docs/REFERENCES.md#black1976
    """
```

## Review checklist

### Coverage check

- [ ] All `pub` functions/methods have doc comments
- [ ] All `pub` structs/enums have doc comments
- [ ] All `pub` struct fields have doc comments
- [ ] All enum variants have doc comments
- [ ] Module-level documentation exists (`//!` or module docstring)

### Quality check

- [ ] Descriptions are clear and concise
- [ ] Arguments are documented with types and purpose
- [ ] Return values are documented
- [ ] Edge cases are noted (panics, errors, None returns)
- [ ] Examples compile and run correctly
- [ ] Examples demonstrate typical usage

### Financial/math code check

- [ ] Formulas are documented or referenced
- [ ] Academic sources are cited
- [ ] Conventions are explicitly stated (day count, compounding, etc.)
- [ ] Assumptions are documented
- [ ] Numerical precision notes where relevant

## Severity rubric

- **Blocker**: Public API with no documentation at all.
- **Major**: Missing arguments, returns, or examples on public API.
- **Minor**: Missing references on financial code, unclear descriptions.
- **Nit**: Style inconsistencies, typos.

## Review workflow

1. **Identify scope**: List all public items in the file/module.
2. **Check coverage**: For each public item, verify documentation exists.
3. **Check quality**: For documented items, verify completeness.
4. **Write fixes**: Add or improve documentation for each finding.
5. **Verify**: Re-check that all items are now documented.

## Output template

```markdown
## Summary
<coverage percentage, main gaps>

## Missing documentation
### Blockers
- `function_name` - no documentation

### Majors
- `struct_name::field` - missing description
- `function_name` - missing examples

### Minors
- `pricing_function` - missing reference to Black (1976)

## Action items
- [ ] Add docs for `function_name`
- [ ] Add examples for `struct_name`
- [ ] Add reference to REFERENCES.md#black1976
```

## Additional resources

- For canonical references, see [reference.md](reference.md)
- For common documentation patterns, see [examples.md](examples.md)
- Project canonical references: `docs/REFERENCES.md`
