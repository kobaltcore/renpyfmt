# AGENTS.md

This repository is a port of the Ren'Py AST parser, meant to become a tool for automatically formatting arbitrary Ren'Py script (and embedded Python within it). It's a relatively close port of the Python-based parser (see `../renpy/renpy`). The `renpy` repository is the source of truth and behavior should match the Python reference implementation.

## Test Coverage

Use `cargo tarpaulin` to check for code coverage. We should aim to cover all important code paths.
