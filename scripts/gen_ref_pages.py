"""Generate API reference pages from .pyi stubs."""

from pathlib import Path

import mkdocs_gen_files

nav = mkdocs_gen_files.Nav()
root = Path("finstack-py/finstack")

for path in sorted(root.rglob("*.pyi")):
    # Skip the compiled extension stub
    if path.name == "finstack.pyi":
        continue

    module_path = path.relative_to(root).with_suffix("")
    parts = tuple(module_path.parts)

    # __init__ → use the parent module
    if parts[-1] == "__init__":
        parts = parts[:-1]
    if not parts:
        continue

    doc_path = Path("reference", *parts, "index.md")
    full_module = "finstack." + ".".join(parts)

    with mkdocs_gen_files.open(doc_path, "w") as fd:
        fd.write(f"::: {full_module}\n")

    mkdocs_gen_files.set_edit_path(doc_path, path)
    # Nav paths must be relative to reference/ since SUMMARY.md lives there
    nav[parts] = str(Path(*parts, "index.md"))

with mkdocs_gen_files.open("reference/SUMMARY.md", "w") as nav_file:
    nav_file.writelines(nav.build_literate_nav())
