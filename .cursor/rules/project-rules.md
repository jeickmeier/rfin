---
trigger: always_on
description:
globs:
---
- For rust code changes, always run `make lint-rust` and `make test-rust` after each set of changes and ensure 100% green.
- For wasm code, always run `make lint-wasm` and `make test-wasm` after each set of changes.
- For wasm UI code, always run `make lint-ui` and `make test-ui` after each set of changes.
- For python code changes, always run `make lint-python` and `make test-paython` after each set of changes.
- Fix any errors that are present from lints and tests before moving on to next task.
- If you change the rust library, you will need to rebuild the python and wasm bindings before using in python/wasm. Use `make python-dev` command for python and make `wasm-build` command for typescript/js/wasm
- DO NOT OVER-ENGINEER THE SOLUTIONS. AIM FOR SIMPLICITY.
