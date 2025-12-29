---
alwaysApply: true
---
- Always run make lint and make test-rust after each set of changes. Fix any errors that are present before moving on to next task. 
- If you change the rust library, you will need to rebuild the python bindings before using in python. Use `make python-dev` command
- Do not nest functions i.e. creating new functions that just wrap other functions. We aim for a simple and concise API design. 
- DO NOT OVER-ENGINEER THE SOLUTIONS. 