# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust 2021 workspace implementing a BSL interpreter. Crates follow the execution pipeline:

- `crates/bsl-syntax`: lexer, parser, AST, and diagnostics.
- `crates/bsl-sema`: name resolution and semantic representation.
- `crates/bsl-bytecode`: bytecode instructions and compiler.
- `crates/bsl-rt`, `bsl-number`, and `bsl-format`: runtime values, decimal arithmetic, and BSL formatting.
- `crates/bsl-vm`: bytecode execution; examples live in `examples/`.
- `crates/bsl-cli`: script runner, REPL, and end-to-end conformance runner.

Unit and integration tests sit beside each crate. Shared BSL programs and oracle outputs are under `tests/conformance/fixtures/`.

## Build, Test, and Development Commands

- `cargo build --workspace` builds every crate.
- `cargo test --workspace` runs the complete test suite.
- `cargo test -p bsl-number` runs one crate's tests while iterating.
- `cargo test -p bsl-number -- --ignored` includes unresolved, explicitly ignored tests.
- `cargo run -p bsl-cli -- path/to/script.bsl` executes a BSL script.
- `cargo run -p bsl-cli` starts the REPL.
- `cargo fmt --all -- --check` verifies formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` performs lint checks.

Run formatting, Clippy, and workspace tests before submitting changes.

## Coding Style & Naming Conventions

Use standard `rustfmt` output and four-space indentation. Follow Rust conventions: `snake_case` for modules, functions, variables, and tests; `CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants. Keep functionality in the narrowest relevant crate and expose it through `lib.rs`. Prefer descriptive test names such as `division_half_up_on_exact_tie`.

## Testing Guidelines

Use Rust's built-in test framework. Add focused unit tests near changed logic and integration tests in a crate's `tests/` directory. Conformance fixtures use matching `name.bsl` and `name.expected` files. An absent `.expected` intentionally marks an unmeasured case; never generate oracle output from this interpreter. Preserve `// НЕ ИЗМЕРЕНО` markers until behavior is measured against real 1C.

## Commit & Pull Request Guidelines

Recent commits use short, imperative subjects such as `Add the string library...` and `Turn input-dependent VM panics into RtError...`. Keep each commit focused. Pull requests should explain behavior changes, identify affected crates, list validation commands, and link relevant issues. Include measured 1C output when changing compatibility semantics; screenshots are only useful for visible CLI or diagnostic changes.
