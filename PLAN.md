# renpyfmt Plan

## Current State

Implemented parser coverage currently includes:

1. `label`
2. `scene`
3. `show`
4. `hide`
5. `with`
6. say statements
7. `$`
8. `python`
9. `jump`
10. `call`
11. `menu`
12. `if`
13. `while`
14. compile-time `IF` / `ELIF` / `ELSE`
15. `return`
16. `style`
17. `init`
18. `init offset`
19. `init label`
20. `define`
21. `default`
22. `pass`
23. `transform`
24. `image`
25. `show layer`
26. `camera`
27. `rpy monologue`
28. `rpy python`
29. `translate` statement family
30. syntax-preserving `testcase` / `testsuite` support

Structured parse errors also exist already via `src/error.rs`, and parser entry points use panic boundaries to convert remaining parser-facing panics into `ParseError` values with location data.

The parser has also already been split into submodules under `src/parser/` and has substantial unit coverage in `src/parser/tests.rs` and `src/lexer.rs` tests.

## Immediate Work

The remaining user-input `panic!` sites in `src/lexer.rs` have been removed. Current `panic!` usage in `src/` is limited to test assertions.

## Remaining Parser Work

### 1. Decide how far to go on `testcase` / `testsuite`

Current support preserves headers and nested raw blocks, which is enough for syntax-preserving parsing.

Open decision:

1. keep the current placeholder/raw-block representation
2. or port the internal Ren'Py test DSL into a dedicated AST

### 2. Implement `screen` / SL2 parsing properly

`screen` is registered in the parser, but `Screen::parse` is still `todo!()`.

Work remaining:

1. decide whether to start with syntax-preserving placeholder support or a fuller SL2 AST
2. implement `screen` parsing without panicking
3. add parser tests for representative screen-language forms

## Formatter Work

The parser now recognizes substantially more syntax than the formatter can emit.

### 3. Remove formatter `todo!()` coverage gaps for parsed nodes

Important remaining AST formatter cases include:

1. `while`
2. compile-time `IF`
3. `early python`
4. `default`
5. `pass`
6. `transform`
7. `show layer`
8. `camera`
9. `screen`
10. `image`
11. `rpy`
12. `translate` variants
13. `testcase`
14. `testsuite`

Important remaining ATL formatter cases include:

1. raw contains-expression nodes
2. raw child nodes
3. raw `on`
4. raw `time`
5. raw `function`
6. raw `event`

## Testing And Coverage

### 4. Expand regression coverage around the current weak spots

Priority areas:

1. string and triple-string lexing
2. python block reconstruction
3. ATL block parsing edge cases
4. say parsing involving triple-quoted strings and `clear`
5. remaining panic-to-error conversion paths

### 5. Add broader smoke tests and coverage checks

1. parse representative real-world `.rpy` files
2. add regression tests for every parser bug fixed
3. use `cargo tarpaulin` to track important parser and lexer paths

## Lower-Priority Follow-Up

### 6. Custom statement support

Replace the hardcoded custom-statement allowlist in `src/parser/registry.rs` with something closer to a registry model.

### 7. Library/API cleanup

Still open:

1. move more preprocessing logic out of `main.rs`
2. expose reusable `parse_str` / `parse_file` style APIs
3. separate CLI/demo behavior more cleanly from reusable library code

## Definition Of Done For This Stage

This stage is complete when:

1. `cargo test` passes cleanly
2. `screen` support is no longer a stub
3. formatter coverage exists for the parser features that are already implemented
4. regression tests and coverage are strong enough to extend the parser without reintroducing old failures
