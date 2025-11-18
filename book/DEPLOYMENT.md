# Deploying the Documentation Book

This guide covers how to deploy your mdBook documentation.

## Local Development

### Build and Serve

```bash
make book-serve
```

Opens browser at `http://localhost:3000` with live reload.

### Build Only

```bash
make book-build
```

Output is in `book/book/` directory.

## GitHub Pages Deployment

### Option 1: GitHub Actions (Recommended)

1. Rename `.github/workflows/book.yml.template` to `.github/workflows/book.yml`
2. Update the `cname` field if you have a custom domain
3. Push to GitHub
4. Enable GitHub Pages in repository settings:
   - Settings → Pages
   - Source: Deploy from a branch
   - Branch: `gh-pages` / `root`

The book will be available at: `https://yourusername.github.io/finstack`

### Option 2: Manual Deployment

```bash
# Build the book
make book-build

# Create gh-pages branch
git checkout --orphan gh-pages
git reset --hard
cp -r book/book/* .
git add .
git commit -m "Deploy documentation"
git push origin gh-pages --force

# Return to main branch
git checkout main
```

## Custom Domain

1. Add a `CNAME` file to `book/src/`:
   ```
   docs.yourcompany.com
   ```

2. Configure DNS:
   - Type: `CNAME`
   - Name: `docs`
   - Value: `yourusername.github.io`

3. Enable HTTPS in GitHub Pages settings

## Other Hosting Options

### Netlify

1. Connect your GitHub repository
2. Build command: `cd book && mdbook build`
3. Publish directory: `book/book`

### Vercel

1. Import your GitHub repository
2. Framework: Other
3. Build command: `cd book && mdbook build`
4. Output directory: `book/book`

### Self-Hosted

Build locally and serve with any static file server:

```bash
make book-build
cd book/book
python3 -m http.server 8080
```

## Continuous Integration

The book build can be added to your existing CI pipeline:

```yaml
- name: Build documentation
  run: make book-build

- name: Test documentation links
  run: |
    cd book
    mdbook test
```

## Advanced Configuration

### Custom Theme

Create `book/theme/` directory and add CSS/JS:

```toml
# book/book.toml
[output.html]
additional-css = ["theme/custom.css"]
additional-js = ["theme/custom.js"]
```

### Plugins

Install mdBook plugins:

```bash
# Link checker
cargo install mdbook-linkcheck

# Mermaid diagrams
cargo install mdbook-mermaid
```

Update `book.toml`:

```toml
[preprocessor.linkcheck]

[preprocessor.mermaid]
```

### PDF Output

Install mdbook-pdf:

```bash
cargo install mdbook-pdf
```

Add to `book.toml`:

```toml
[output.pdf]
```

Build PDF:

```bash
cd book && mdbook build
```

## Troubleshooting

### Build Failures

Check for:
- Missing files referenced in `SUMMARY.md`
- Broken internal links
- Invalid Markdown syntax

Run:
```bash
cd book && mdbook build --verbose
```

### Deployment Failures

- Verify GitHub Actions has write permissions
- Check that `gh-pages` branch exists
- Ensure GitHub Pages is enabled in settings

### 404 Errors on GitHub Pages

- Wait a few minutes for propagation
- Clear browser cache
- Check base URL in `book.toml`















