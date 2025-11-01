# Finstack Documentation Book

This directory contains the **mdBook-based user guide** for Finstack.

## Important: mdBook vs rustdoc

**This is NOT auto-generated from your Rust docstrings.**

Finstack has **two documentation systems**:

1. **📚 API Reference (rustdoc)** - Auto-generated from `///` docstrings in Rust code
   - Run: `make doc`
   - For: Technical API reference

2. **📖 User Guide (mdBook)** - Hand-written markdown tutorials and guides
   - Run: `make book-serve`
   - For: Learning, tutorials, conceptual explanations

See `WRITING_GUIDE.md` for how to write effective mdBook content.

## Quick Start

### Build the book

```bash
make book-build
```

The built book will be in `book/book/` directory.

### Serve with live reload

```bash
make book-serve
```

This will start a local server at `http://localhost:3000` with live reload enabled. Any changes to markdown files will automatically rebuild and refresh the browser.

### Watch mode

```bash
make book-watch
```

Watches for changes and rebuilds the book without serving.

### Clean build artifacts

```bash
make book-clean
```

## Directory Structure

```
book/
├── book.toml           # mdBook configuration
├── src/                # Markdown source files
│   ├── SUMMARY.md      # Table of contents
│   ├── introduction.md # Introduction page
│   ├── getting-started/
│   ├── core/
│   ├── statements/
│   ├── valuations/
│   ├── scenarios/
│   ├── portfolio/
│   ├── io/
│   ├── bindings/
│   ├── advanced/
│   └── developer/
└── book/               # Generated output (gitignored)
```

## Writing Documentation

### Markdown Files

All documentation is written in Markdown. mdBook supports:

- Standard Markdown syntax
- Syntax highlighting for code blocks
- Link checking
- Automatic table of contents
- Full-text search

### Adding a New Page

1. Create a new markdown file in the appropriate directory under `src/`
2. Add an entry to `src/SUMMARY.md` to make it appear in the navigation
3. The book will automatically rebuild if you're using `make book-serve` or `make book-watch`

### Code Blocks

For Rust code examples:

\`\`\`rust
use finstack::core::Amount;

let amount = Amount::from_str("100.50 USD")?;
\`\`\`

For Python:

\`\`\`python
from finstack import Amount

amount = Amount.from_str("100.50 USD")
\`\`\`

### Cross-references

Link to other pages using relative paths:

```markdown
See [Installation](./getting-started/installation.md) for setup instructions.
```

## mdBook Documentation

For more information about mdBook features, see:
- [mdBook User Guide](https://rust-lang.github.io/mdBook/)
- [mdBook GitHub](https://github.com/rust-lang/mdBook)

## Contributing

When contributing documentation:

1. Follow the existing structure and style
2. Keep examples simple and runnable
3. Include both Rust and Python examples where applicable
4. Test that the book builds without errors
5. Check that links work correctly

## Tips

- Use `make book-serve` for development - it's the fastest way to see changes
- The search feature is automatically generated from all pages
- Code blocks are automatically syntax-highlighted
- The theme is set to "rust" by default (matches Rust's official documentation)

