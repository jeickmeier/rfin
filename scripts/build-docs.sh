#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="site"
rm -rf "$SITE_DIR"
mkdir -p "$SITE_DIR/api/rust" "$SITE_DIR/api/python" "$SITE_DIR/guide"

# 1. Rustdoc
printf "Building Rust API docs...\n"
CARGO_INCREMENTAL=1 cargo doc \
    --workspace --exclude finstack-py --exclude finstack-wasm \
    --no-deps --all-features
cp -r target/doc/* "$SITE_DIR/api/rust/"

# 2. mdBook
printf "Building mdBook guide...\n"
cd book && mdbook build && cd ..
cp -r book/book/* "$SITE_DIR/guide/"

# 3. mkdocs (Python API)
printf "Building Python API docs...\n"
uv run --group docs mkdocs build -f mkdocs.yml -d "$SITE_DIR/api/python"

# 4. Landing page
cat > "$SITE_DIR/index.html" << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Finstack Documentation</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 600px; margin: 80px auto; }
    h1 { color: #333; }
    ul { list-style: none; padding: 0; }
    li { margin: 12px 0; }
    a { color: #4051b5; text-decoration: none; font-size: 1.1em; }
    a:hover { text-decoration: underline; }
    .desc { color: #666; font-size: 0.9em; }
  </style>
</head>
<body>
  <h1>Finstack Documentation</h1>
  <ul>
    <li>
      <a href="guide/">Guide &amp; Cookbooks</a>
      <div class="desc">Architecture, how-tos, extending, conventions</div>
    </li>
    <li>
      <a href="api/rust/finstack/">Rust API Reference</a>
      <div class="desc">Auto-generated from rustdoc comments</div>
    </li>
    <li>
      <a href="api/python/">Python API Reference</a>
      <div class="desc">Auto-generated from .pyi type stubs</div>
    </li>
  </ul>
</body>
</html>
EOF

printf "Documentation built in %s/\n" "$SITE_DIR"
