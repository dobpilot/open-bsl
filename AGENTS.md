# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust 2021 workspace implementing a BSL interpreter. Crates follow the execution pipeline:

- `crates/bsl-syntax`: lexer, parser, AST, and diagnostics.
- `crates/bsl-sema`: name resolution and semantic representation.
- `crates/bsl-bytecode`: bytecode instructions, compiler, and the textual bytecode format (`text.rs`) used by both `--emit-bytecode` and `--run-bytecode` — printing and parsing share one format, so adding an instruction means touching `write_instr`, `parse_instr`, `OPCODES`, and the round-trip corpus together.
- `crates/bsl-rt`, `bsl-number`, and `bsl-format`: runtime values, decimal arithmetic, and BSL formatting.
- `crates/bsl-vm`: bytecode execution; examples live in `examples/`.
- `crates/bsl-cli`: script runner, REPL (syntax highlighting in `highlight.rs`, Tab completion in `complete.rs`), and end-to-end conformance runner.

Everything except `bsl-cli` is dependency-free (`bsl-number` uses `num-bigint`/`num-traits`; `bsl-cli` uses `rustyline` for raw-mode line editing). Keep it that way: new external crates need a reason that cannot be met in-tree.

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

### Comments and Rustdoc

Write comments and rustdoc in clear Russian technical prose. Explain intent, invariants, compatibility constraints, and non-obvious tradeoffs rather than restating the code. Use complete sentences and normal sentence case; avoid conversational emphasis in all capitals. Prefer natural Russian wording over unnecessary calques such as «персистить», `see`, `literal`, or `top-level`; established project terms such as «чанк» and JIT are acceptable when they are clearer. Write «X — это Y» with an em dash.

Use `//` for implementation notes, `///` for API documentation, and `//!` for crate or module overviews. Enclose Rust identifiers, opcodes, commands, code fragments, and marker IDs in backticks. Public functions returning `Result` should document failure conditions under `# Errors`; document genuine panic conditions under `# Panics`. Keep rustdoc links resolvable and ensure `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` succeeds.

Preserve compatibility markers exactly as `` `НЕ ИЗМЕРЕНО(AREA.QUESTION)` `` so both rustdoc and the registry scanner recognize them. Do not reword or remove a marker without the corresponding measurement and registry updates described below.

## Testing Guidelines

Use Rust's built-in test framework. Add focused unit tests near changed logic and integration tests in a crate's `tests/` directory. Conformance fixtures use matching `name.bsl` and `name.expected` files. An absent `.expected` intentionally marks an unmeasured case; never generate oracle output from this interpreter. Preserve `// НЕ ИЗМЕРЕНО(ID)` markers until behavior is measured against real 1C.

Every decision made by reasoning rather than by checking against the platform needs all three of: a `// НЕ ИЗМЕРЕНО(AREA.QUESTION)` marker at the code site, an entry in `crates/bsl-rt/src/open_questions.rs`, and one line in `tests/conformance/measure/measure-all.bsl`. The test `open_questions_registry_matches_source_markers` fails if any of the three is missing. Platform results come back through `bsl-cli --ingest-measurements`, which never edits code.

## Commit & Pull Request Guidelines

Recent commits use short, imperative subjects such as `Add the string library...` and `Turn input-dependent VM panics into RtError...`. Keep each commit focused. Pull requests should explain behavior changes, identify affected crates, list validation commands, and link relevant issues. Include measured 1C output when changing compatibility semantics; screenshots are only useful for visible CLI or diagnostic changes.
