# Code Formatting Workflow

This document describes the automated code formatting workflow for the RFin project, which ensures consistent code style across all languages and components.

## Overview

The code formatting system includes:

- **GitHub Actions workflow** that runs on every PR and push
- **Pre-commit hooks** that run locally before each commit
- **Manual script** for on-demand formatting
- **IDE integration** for format-on-save

## Supported Languages and Tools

| Language | Formatter | Linter | Config Files |
|----------|-----------|--------|--------------|
| Rust | `rustfmt` | `clippy` | `rustfmt.toml`, `Cargo.toml` |
| Python | `ruff format` | `ruff check` | `pyproject.toml` |
| TypeScript/JavaScript | `prettier` | `eslint` | `package.json`, `.eslintrc.*` |
| Markdown | `markdownlint-cli2` | - | `.markdownlint.json` |

## GitHub Actions Workflow

The workflow is located at `.github/workflows/code-formatting.yml` and runs:

### Triggers

- On push to `master`, `main`, or `develop` branches
- On pull requests (opened, synchronized, reopened)
- Manual dispatch with options

### Jobs

1. **format-code**: Main formatting job
   - Formats all code or checks formatting based on input
   - Can create PRs with formatting fixes
   - Supports language-specific formatting

2. **check-formatting**: Fast check on PRs
   - Only checks formatting, fails fast on issues
   - Runs in parallel with other checks

3. **comment-pr**: Comments on PRs with formatting status

### Workflow Options

When running manually, you can specify:

- `check_only`: Only check without fixing
- `rust_only`: Format only Rust code
- `python_only`: Format only Python code
- `wasm_only`: Format only WASM/TS code
- `create_pr`: Create PR with fixes

## Local Setup

### 1. Pre-commit Hooks (Recommended)

#### Option A: Using pre-commit framework (Recommended)

Install pre-commit hooks to run formatters automatically before each commit:

```bash
# Install pre-commit
pip install pre-commit

# Install hooks
pre-commit install

# Run on all files (first time)
pre-commit run --all-files
```

The pre-commit configuration is in `.pre-commit-config.yaml` and includes:

- Rust formatting and linting
- Python formatting and linting
- TypeScript/JavaScript formatting and linting
- Markdown formatting
- File hygiene checks

#### Option B: Using Git Hook Directly

If you prefer not to use the pre-commit framework, you can install the git hook directly:

```bash
# Copy the hook to git hooks directory
cp scripts/pre-commit-format .git/hooks/pre-commit

# Make it executable
chmod +x .git/hooks/pre-commit
```

This will run formatting checks before each commit and block commits if formatting is needed.

### 2. Manual Formatting Script

Use the provided script for manual formatting:

```bash
# Format all code
./scripts/format-code

# Check formatting without fixing
./scripts/format-code --check-only

# Format specific language
./scripts/format-code --rust-only
./scripts/format-code --python-only
./scripts/format-code --wasm-only
# Show help
./scripts/format-code --help
```

### 3. IDE Integration

#### VS Code

Copy the example settings to your workspace:

```bash
# Copy the example settings
cp docs/vscode-settings-example.json .vscode/settings.json
```

The settings file includes:

- Format on save for all supported languages
- Code actions on save (fix imports, fix lint issues)
- Language-specific formatters
- Editor configuration (line length, tab size)

Or add to your `.vscode/settings.json` manually:

```json
{
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll.eslint": "explicit",
    "source.organizeImports": "explicit"
  },
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[python]": {
    "editor.defaultFormatter": "charliermarsh.ruff"
  },
  "[typescript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[typescriptreact]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[javascript]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[javascriptreact]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  },
  "[markdown]": {
    "editor.defaultFormatter": "esbenp.prettier-vscode"
  }
}
```

Recommended VS Code extensions:

- Rust Analyzer
- Ruff
- ESLint
- Prettier

#### Neovim

Using `null-ls` or `conform.nvim`:

```lua
-- For conform.nvim
require('conform').setup({
  formatters_by_ft = {
    rust = { "rustfmt" },
    python = { "ruff_format" },
    typescript = { "prettier" },
    typescriptreact = { "prettier" },
    javascript = { "prettier" },
    javascriptreact = { "prettier" },
    markdown = { "prettier" },
  },
  format_on_save = {
    timeout_ms = 500,
    lsp_fallback = true,
  },
})
```

#### Vim/Neovim with ALE

```vim
let g:ale_rust_rustfmt_options = '--edition 2021'
let g:ale_python_ruff_format_options = '--line-length 120'
let g:ale_fixers = {
  \ 'rust': ['rustfmt'],
  \ 'python': ['ruff_format', 'ruff'],
  \ 'typescript': ['prettier', 'eslint'],
  \ 'typescriptreact': ['prettier', 'eslint'],
  \ 'javascript': ['prettier', 'eslint'],
  \ 'javascriptreact': ['prettier', 'eslint'],
  \ 'markdown': ['prettier'],
\ }
let g:ale_fix_on_save = 1
```

## Configuration Details

### Rust Configuration

The Rust formatter and linter are configured in:

- `Cargo.toml`: Workspace-wide lints and clippy profile
- `.rustfmt.toml`: Formatting rules (if present)

Key settings:

- Line length: 100 characters
- Use of `rustfmt` for consistent formatting
- Strict clippy lints with custom profile

### Python Configuration

Python formatting is configured in `pyproject.toml`:

```toml
[tool.ruff]
line-length = 120
target-version = "py312"

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
line-ending = "lf"
```

Key settings:

- 120 character line length
- Double quotes for strings
- LF line endings
- Google-style docstrings

### TypeScript/JavaScript Configuration

Configuration in each `package.json`:

```json
{
  "prettier": {
    "semi": true,
    "trailingComma": "es5",
    "singleQuote": false,
    "printWidth": 120,
    "tabWidth": 2
  }
}
```

ESLint configuration in `.eslintrc.*` files.

## Best Practices

1. **Always run formatters before committing**
   - Use pre-commit hooks to automate this
   - CI will fail if formatting is incorrect

2. **Configure your IDE for format-on-save**
   - Prevents manual formatting steps
   - Ensures consistency

3. **Check CI logs for formatting issues**
   - The workflow provides detailed feedback
   - Shows exactly what needs fixing

4. **Use the manual script for bulk changes**
   - When updating formatting rules
   - When formatting large codebases

5. **Review auto-fixes**
   - Some lint fixes may change behavior
   - Always review before merging

## Troubleshooting

### Common Issues

1. **Formatter not found**

   ```bash
   # Install missing tools
   rustup component add rustfmt
   pip install ruff
   npm install -g prettier
   ```

2. **Conflicting configurations**
   - Check for multiple config files
   - Ensure IDE settings match project config

3. **Pre-commit hooks not running**

   ```bash
   # Reinstall hooks
   pre-commit uninstall
   pre-commit install
   ```

4. **CI formatting differs from local**
   - Check tool versions in CI
   - Ensure consistent configuration

### Debugging

Enable verbose output:

```bash
./scripts/format-code --verbose
```

Check specific formatter:

```bash
# Rust
cargo fmt --all -- --check

# Python
uv run ruff format . --check

# TypeScript
cd finstack-wasm && npm run format -- --check .
```

## Contributing

When adding new languages or updating formatting rules:

1. Update the GitHub Actions workflow
2. Update the pre-commit configuration
3. Update the manual formatting script
4. Update this documentation
5. Test in a clean environment

## Related Documents

- [Pre-commit configuration](../.pre-commit-config.yaml)
- [Test and Fix Workflow](./TEST_AND_FIX_WORKFLOW.md)
- [Development Setup](../README.md#development)
