# Quick Reference

## The Two Documentation Systems

### rustdoc (API Reference) ← Your Rust docstrings
```bash
make doc        # Generate and open
```
Auto-generated from `///` comments in code. Shows types, methods, signatures.

### mdBook (User Guide) ← Hand-written markdown
```bash
make book-serve  # Build and serve with live reload
```
Tutorials and guides you write manually in `book/src/`.

## The Pages Are Intentionally Blank!

mdBook does NOT auto-extract from Rust code. You write content manually.

## Quick Start

1. Run: `make book-serve`
2. Edit: `book/src/valuations/bonds.md`
3. See: Changes appear in browser instantly

## What to Write

**Good for mdBook:**
- "How to price a bond step-by-step"
- "Understanding currency safety"
- "Best practices for market data"

**Already in rustdoc:**
- "What methods does Amount have?"
- "Parameters for BondPricer::price()"
- "All public APIs"

## Full Documentation

Read: `DOCUMENTATION.md` (in workspace root)
Read: `WRITING_GUIDE.md` (in book/ directory)
