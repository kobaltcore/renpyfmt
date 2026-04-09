# renpyfmt Plan

## Goals

The current priority is to make the core parser reliable before investing in formatter completeness or large parser subsystems like SL2.

Primary goals:

1. Add support for the remaining core Ren'Py statements.
2. Remove `panic!`-driven parse failures and replace them with structured error handling.
3. Add tests wherever they make sense so parser coverage can grow without regressions.

Secondary goals for later:

1. Improve formatter correctness for already parsed nodes.
2. Port large missing subsystems like SL2/screen language.
3. Improve custom statement support so it is not based on a hardcoded allowlist.

## Current State

Implemented parser coverage already includes:

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
13. `return`
14. `style`
15. `init`
16. `define`
17. `default`
18. `pass`
19. `transform`
20. `image`

Core parser gaps still remaining:

1. `testcase`
2. `testsuite`

Out of those, the most important short-term work for a reliable core parser is:

1. `testcase`
2. `testsuite`

`translate`, `testcase`, and `testsuite` should stay on the roadmap, but they can come after the smaller core-statement work and the error-handling conversion.

## Phase 1: Parser Error Model

This should happen first, because adding more statements on top of `panic!` will make the parser harder to stabilize.

Status: completed.

### Deliverables

1. Introduce a dedicated parse error type.
2. Stop using `panic!` for expected parse failures.
3. Return errors with file and line information.
4. Keep fatal internal invariants separate from user-facing parse errors.

Completed work:

1. Added `src/error.rs` with `ParseError` and a shared `Result<T>` alias.
2. Converted lexer expectation helpers to return `ParseError` instead of panicking.
3. Converted the main parser helpers and core statement parsers to propagate `Result`.
4. Added parser entry-point panic boundaries so remaining user-facing lexer/parser panics are surfaced as `ParseError` with location.
5. Removed the `unused Result` fallout from the error-conversion work in `cargo check`.
6. Added regression tests for representative error paths that previously panicked.

### Proposed shape

Add a parser error module, for example `src/error.rs`, with:

1. `ParseError`
2. a location field like `(PathBuf, usize)`
3. a human-readable message
4. helper constructors for common parser failures

Update the shared result type to use `Result<T, ParseError>` or `anyhow::Result<T>` wrapping `ParseError` consistently.

### Conversion strategy

Replace panics in this order:

1. lexer expectation helpers
2. parser helper functions
3. statement parsers
4. ATL parsing helpers
5. top-level parsing entry points

Specific hot spots to convert first:

1. `Lexer::expect_eol`
2. `Lexer::expect_block`
3. `Lexer::expect_noblock`
4. `Lexer::python_expression`
5. `parse_parameters`
6. `parse_image_specifier`
7. `parse_arguments`
8. `finish_say`
9. `parse_menu`
10. statement implementations in `src/parser.rs`

### Rules

1. Syntax problems in user input must return structured errors.
2. `unwrap()` should not be used after user-controlled parsing decisions.
3. Internal impossible states may still use `debug_assert!` or rare hard failures if truly unreachable.
4. Error messages should stay close to upstream Ren'Py wording when practical.

Phase 1 exit criteria achieved:

1. Core parser entry points now return structured parse errors instead of crashing on normal syntax failures.
2. Error values include filename and line information.
3. Core parser code no longer ignores fallible lexer helpers.
4. Parser regression tests cover multiple formerly-panic-prone syntax failures.

Remaining non-goals for Phase 1:

1. Formatter `todo!()` cases and formatter warnings.
2. Stubbed subsystems like `screen` / SL2.
3. Internal invariant checks that are not currently reachable through normal parser input.

## Phase 2: Test Harness

Add tests before or alongside the error conversion so behavior can be locked in incrementally.

### Test layers

1. Lexer unit tests
2. Parser unit tests for individual statements
3. Small integration tests that parse short script snippets
4. Error tests that assert message and location shape

### Recommended helpers

Create test helpers that:

1. build logical lines from inline script text
2. group them into blocks
3. run the lexer and parser
4. return AST or parse error

That avoids requiring fixture files for every parser case.

### Initial test targets

1. valid parse of each currently supported statement
2. invalid syntax for each currently supported statement
3. `if` and `menu` block parsing
4. ATL block parsing smoke tests
5. init behavior for `define`, `default`, `transform`, and `image`
6. one-line python and block python parsing
7. error cases that currently panic

### Regression policy

Every parser bug fixed should add at least one test.

## Phase 3: Add Remaining Small Core Statements

Once the parser error model exists, add the remaining smaller core statements.

### 3.1 `while`

Status: completed.

Work:

1. add `While` AST node
2. register statement in `ParseTrie`
3. port parser logic from upstream
4. add parser tests for normal and malformed blocks

Tests:

1. simple `while condition:` block
2. nested `while` inside label
3. missing colon
4. missing block

### 3.2 `show layer`

Status: completed.

Work:

1. add `ShowLayer` AST node
2. register `show layer` as a two-word trie entry
3. parse optional `at` list and optional ATL block
4. add tests

Tests:

1. plain `show layer master`
2. `show layer master at foo, bar`
3. `show layer master:` with ATL block
4. malformed `at` clause

### 3.3 `camera`

Status: completed.

Work:

1. add `Camera` AST node
2. register parser
3. port same shape as upstream `camera` statement
4. add tests

Tests:

1. `camera`
2. `camera at foo`
3. `camera master at foo, bar`
4. `camera:` with ATL block

### 3.4 `init offset`

Status: completed.

Work:

1. add parser support
2. update lexer state mutation path
3. decide whether this needs an AST node or should remain parser-side state only
4. add tests that verify subsequent init priorities are affected

Tests:

1. `init offset = 5`
2. offset affecting later `define`
3. malformed integer

### 3.5 `init label`

Status: completed.

Work:

1. register `init label`
2. reuse `label` parsing with `init = true`
3. add tests

Tests:

1. simple `init label foo:`
2. nested statements in init label
3. parameterized init label if supported by upstream behavior

### 3.6 `rpy monologue`

Status: completed.

Work:

1. register parser
2. update lexer/parser state for monologue delimiter
3. preserve upstream accepted values: `none`, `single`, `double`
4. add tests

Tests:

1. each accepted mode
2. invalid mode
3. verify say parsing behavior if practical

### 3.7 `rpy python`

Status: completed.

Work:

1. add `RPY` AST node or equivalent syntax-preserving representation
2. register parser
3. parse one or more names separated by commas
4. add tests

Tests:

1. `rpy python __future__`
2. comma-separated values
3. malformed trailing comma or missing name

## Phase 4: Add Remaining Complex Core Statements

These are still core language features, but larger than the previous group.

### 4.1 `IF` / `ELIF` / `ELSE`

Status: completed as syntax-preserving parser support.

Notes:

1. This is compile-time conditional parsing upstream.
2. It may require deciding whether this project will evaluate conditions or preserve them structurally.

Recommended approach:

1. preserve syntax first, avoid eager evaluation if possible
2. if exact upstream semantics are required, isolate condition evaluation behind a dedicated layer
3. add explicit tests for selection behavior

### 4.2 `translate`

Status: completed for the main parser family.

Completed coverage:

1. `translate <language> <identifier>:` blocks
2. `translate <language> strings:` blocks
3. `translate <language> python:` blocks
4. `translate <language> style:` blocks

Notes:

1. This is a larger family, not a single statement.
2. It requires several AST nodes and block forms.

Recommended approach:

1. add minimal syntax-preserving AST coverage first
2. support `translate <lang> <id>:` blocks
3. then add `translate strings`
4. then `translate python` and `translate style`

### 4.3 `testcase` / `testsuite`

Notes:

1. These matter less for general script formatting.
2. They are worth adding for parser completeness once core script support is stable.

Recommended approach:

1. either implement syntax-preserving placeholder AST nodes
2. or clearly defer them until a dedicated test-language pass

## Phase 5: Parser Cleanup While Expanding Coverage

As statements are added, clean up the parser shape instead of letting `src/parser.rs` keep growing without structure.

### Recommended refactors

1. group helper functions by feature area
2. separate statement parsers into submodules if `parser.rs` becomes harder to navigate
3. move ATL parsing to its own parser module if necessary
4. move reusable statement parsing helpers into dedicated functions

### Immediate code health targets

1. remove debug `println!` calls from parser/trie paths
2. reduce `unwrap()` after parse branches
3. keep AST data syntax-preserving where possible
4. document any intentional deviation from upstream parser behavior

## Testing Plan By Milestone

### Milestone A: Error handling foundation

1. tests for parse errors replacing panics
2. tests for error locations
3. tests for malformed blocks and expressions

### Milestone B: Small remaining core statements

1. positive and negative parser tests for each new statement
2. init-state interaction tests where relevant
3. ATL attachment tests for `show layer` and `camera`

### Milestone C: Complex remaining statements

1. compile-time conditional tests
2. translation syntax tests
3. mixed-script integration tests

### Milestone D: Real-world smoke tests

1. parse a curated set of representative `.rpy` files
2. ensure parser returns AST instead of panicking
3. track unsupported syntax explicitly when encountered

## Suggested Execution Order

1. Completed: introduce `ParseError` and convert lexer expectation helpers.
2. Completed: add parser test helpers and initial error regression tests.
3. Completed: convert the most panic-heavy parse helpers.
4. Completed: add `while`, `show layer`, and `camera`.
5. Completed: add `init offset` and `init label`.
6. Completed: add `rpy monologue` and `rpy python`.
7. Completed for the current core parser path: finish panic-to-error conversion across core statement parsers and parser entry points.
8. Completed: add `IF` / `ELIF` / `ELSE`.
9. Completed: add `translate` support.
10. Next: add `testcase` and `testsuite` or explicitly defer them behind a parser limitation.

## Future Goals

These are important, but intentionally lower priority than core parser reliability.

### Formatter correctness

1. make formatting faithful for already parsed nodes
2. remove formatter `todo!()` cases for core AST and ATL nodes
3. preserve details currently dropped by formatting, such as label parameters, menu clauses, call arguments, and define operators

### SL2 and screen language

1. port `screen` parsing properly
2. grow `slast.rs` into a real syntax tree
3. add a dedicated screen-language parser module instead of keeping it in the main parser file

### Custom statements

1. replace the hardcoded user-statement allowlist with a registry model
2. support upstream-style custom parsing hooks where practical
3. preserve unknown custom statements without panicking

### Architecture improvements

1. move preprocessing logic out of `main.rs` into library code
2. expose `parse_str` and `parse_file` APIs
3. separate CLI/demo code from reusable parser code
4. use ordered data structures where source order matters for formatting

## Definition of Done For This Stage

This stage is complete when:

1. the remaining smaller core statements are supported
2. the parser no longer panics on normal syntax errors
3. parse failures include useful file and line information
4. tests cover supported core statements and representative failure modes
5. new parser work can be added by extending tests first instead of debugging crashes after the fact
