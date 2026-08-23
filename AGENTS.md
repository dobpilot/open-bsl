# Repository Guidelines

## Working Discipline

Four rules that bias toward caution over speed. For trivial mechanical edits, use judgment.

**Think before coding.** State assumptions explicitly instead of building on them silently. When a task admits several readings, name them and choose openly; when a simpler approach exists than the one requested, say so before implementing. Confusion is a finding, not an obstacle: working interactively, ask; working unattended, record the open question instead of guessing. For platform behavior this rule has hard machinery — 1C semantics are measured, never inferred, and an unmeasured decision carries a `НЕ ИЗМЕРЕНО` marker with its registry entry and measure line (see "Measuring on the 1C platform" below). Some divergences from the platform are deliberate decisions; check the comments and history at the site before "fixing" one back.

**Simplicity first.** Write the minimum code that solves the problem: no speculative features, no configurability nobody asked for, no abstractions for single-use code, no handling of impossible errors. If the diff comes out several times larger than the task suggests, rewrite it before submitting. The dependency rule below is this principle applied to external crates, and it cuts both ways — reuse what the workspace already has rather than writing a second copy; the measured inline budget around the VM dispatch loop (see the comments on `step_cold` in `bsl-vm`) is the same principle applied to hot code, where size itself is a cost.

**Surgical changes.** Every changed line should trace to the task at hand. No drive-by reformatting, renaming, or "improving" of adjacent code — match the surrounding style, including the Russian comment prose, even where you would write it differently. Remove imports and helpers your change orphaned; leave pre-existing dead code in place and mention it instead. Files owned by machinery are not edited as a side effect: `НЕ ИЗМЕРЕНО` markers move only together with their measurement, and the Ralph loop's state files obey the ownership rules in its section below.

**Goal-driven execution.** Before implementing, turn the task into a check that can fail: a bug fix starts from a test or fixture that reproduces it, a compatibility change from measured platform output, a refactor from the suite that must stay green on both sides — plus `the_jit_agrees_with_the_interpreter_on_every_script` for VM work and an alternating A/B run against the baseline binary for any performance claim. For multi-step work, state a short plan with a verification per step. A task whose success criterion cannot be named is not understood yet; resolving that comes first.

## Project Structure & Module Organization

This repository is a Rust 2024 workspace implementing a BSL interpreter. Crates follow the execution pipeline:

- `crates/bsl-syntax`: lexer, parser, AST, and diagnostics.
- `crates/bsl-sema`: name resolution and semantic representation.
- `crates/bsl-bytecode`: bytecode instructions and the textual bytecode format (`text.rs`) used by both `--emit-bytecode` and `--run-bytecode` — printing and parsing share one format, so adding an instruction means touching `write_instr`, `parse_instr`, `OPCODES`, and the round-trip corpus (now `crates/bsl-compiler/tests/text_round_trip.rs`) together, and bumping `FORMAT_VERSION` if the encoding changes. The crate holds the representation only: it depends on neither `bsl-syntax` nor `bsl-sema`, in normal *or* dev dependencies, so its own tests build programs by hand (`tests/support`) instead of compiling BSL. `bundle.rs` marks VLIW bundles — runs of mutually independent neighbor instructions (no RAW/WAW inside a bundle, WAR allowed) that the VM executes in one dispatch; its effects classification is an exhaustive match, so a new opcode must be classified there as well. `Chunk::bundle_len` is a derived table and is never serialized: the parser recomputes it (listings only show it as `; бандл N` comments), and `crates/bsl-cli/tests/bundles.rs` re-verifies the invariants over the whole conformance corpus with `bundle::verify`.
- `crates/bsl-compiler`: code generation from `bsl-sema`'s representation into `Program`, plus `compile_dynamic_snippet` — the whole front end of `Выполнить`/`Вычислить` behind the neutral `bsl_bytecode::DynamicCompiler` contract. It sits between the front end and the representation so that `bsl-vm` can depend on the representation alone; `cargo tree -p bsl-vm -e normal` must not show `bsl-syntax` or `bsl-sema`.
- `crates/bsl-rt`, `bsl-number`, and `bsl-format`: runtime values, decimal arithmetic, and BSL formatting. `BslValue::Display` is debug-only and does not reproduce 1C formatting — use `bsl_format::format_value` for any user-visible or conformance-checked text (it backs `Строка`/`Формат` and the CLI).
- `crates/bsl-vm`: bytecode execution; examples live in `examples/`.
- `crates/bsl-cli`: script runner, REPL (syntax highlighting in `highlight.rs`, Tab completion in `complete.rs`), and end-to-end conformance runner.

Two more single-source tables work like `text.rs`. Builtins are one table in `crates/bsl-rt/src/builtin.rs`: a global function is a `BuiltinFn` variant plus a row in `BUILTIN_FN_NAMES` (the Russian name and the English alias map to the same variant), an arm in `arity_range`, and an arm in `call_builtin_fn` — or `call_builtin_fn_ctx` if it needs `RuntimeShapes` (the runtime name table, e.g. `ЗаполнитьЗначенияСвойств`). Methods work the same way through `BuiltinMethod`/`BUILTIN_METHOD_NAMES`. Everything downstream goes through `BuiltinFn::lookup`: sema resolves the call and checks arity there, `text.rs` prints and re-parses the name, and the REPL's highlighter and completer enumerate the same table — so a new builtin lights up in all of them, and a name added without an `arity_range` arm is a compile error rather than a runtime surprise. `bsl-cli` argument parsing is table-driven too: `COMMANDS` in `main.rs` is the single source for both dispatch and `--help`, and the `match` on `Kind` is exhaustive — a command cannot be implemented but undocumented, or documented but unimplemented. Add the row, then the arm.

External dependencies are deliberately few: `bsl-number` uses `num-bigint`/`num-traits`; `bsl-zip`, `bsl-spreadsheet` and `bsl-pdf` use `zip` and `flate2` for ZIP containers, XLSX export, and PDF stream compression (the in-tree inflate/deflate/ZIP implementations were retired in favour of these crates); the base runtime `bsl-rt` itself has no external dependency beyond `bsl-number`; `bsl-regexp` uses `fancy-regex` (the `regex` crate plus lookaround) for matching behind the in-tree dialect parser — the in-tree backtracking matcher was retired in its favour; `bsl-cli` uses `rustyline` for raw-mode line editing; dev-only `insta` takes the AST snapshots and `pprof` renders flamegraphs. New external crates need a reason that cannot be met in-tree.

The rule forbids a new dependency, not reuse: do not reinvent a wheel the workspace already has. Before writing an algorithm, a parser, or a helper, look for it in `std` and across the crates — DEFLATE and the ZIP container exist once (`flate2`/`zip`, shared by `bsl-zip`, the PDF stack, and the XLSX export); decimal arithmetic is `bsl-number`, user-facing formatting is `bsl_format::format_value`, regular expressions are `bsl-regexp`, bit operations are `bsl-binbuf`, byte streams are `bsl-stream`, JSON is `bsl-json`, text documents are `bsl-textdoc`, archives are `bsl-zip`. A second copy of something the workspace already implements is a defect even when it is shorter than the original, because both copies then have to stay measured-compatible with the platform. If the existing one almost fits, extend it or lift it to where both callers can see it; a deliberately separate implementation says why in a comment at the site.

When a task essentially *is* an external crate — a compression format, a hash, a text codec, a parser generator — raise it while planning, before writing anything: name the crate, say what implementing it in-tree would cost, and ask. Both answers are legitimate, and both live in this history: XSD and PDF were the question answered "in-tree", while the original in-tree inflate/deflate/ZIP — and later the regex matcher, replaced by `fancy-regex` behind the in-tree dialect parser — were retired in favour of crates, the same question revisited with new evidence. Neither taking the dependency nor writing a few thousand lines by hand is the default. Under the Ralph loop there is nobody to ask, so the planner records the choice and its price in `PLAN.md` instead of making it silently.

Unit and integration tests sit beside each crate. Shared BSL programs and oracle outputs are under `tests/conformance/fixtures/`. Longer-form design notes live in `docs/`: `docs/mxl-format.md` is the measured reverse-engineering of the 1C MXL spreadsheet format, with its fixture corpus in `tests/conformance/mxl/`.

## Build, Test, and Development Commands

- `cargo build --workspace` builds every crate.
- `cargo test --workspace` runs the complete test suite.
- `cargo test -p bsl-number` runs one crate's tests while iterating; `cargo test -p bsl-number --test oracle` narrows to one integration test file, and `cargo test -p bsl-number division_half_up` to tests matching a name substring.
- The workspace's only `#[ignore]`d test is `dump_for_the_platform_crosscheck` in `crates/bsl-rt/tests/mxl_oracle.rs`: it dumps MXL files for the platform to read back rather than asserting anything, so `cargo test -p bsl-rt -- --ignored` succeeding means files were written, not that a check passed.
- `cargo test -p bsl-cli -- --nocapture` shows the conformance run together with the skipped-fixture summary.
- `cargo run -p bsl-cli -- path/to/script.bsl` executes a BSL script; without a path it starts the REPL, and `--help` is generated from the `COMMANDS` table in `main.rs`.
- `cargo run -p bsl-cli -- --emit-bytecode script.bsl [out.bslc]` prints the textual bytecode; `--run-bytecode out.bslc` executes it.
- `cargo run -p bsl-cli --release -- --jit script.bsl` runs with the template JIT (x86-64 Linux only; elsewhere the flag is accepted and ignored).
- `cargo fmt --all -- --check` verifies formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` performs lint checks.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` verifies that rustdoc links resolve.

Run formatting, Clippy, and workspace tests before submitting changes.

### Benchmarks and profiling

- `cargo build --release -p bsl-cli && ./benchmarks/run.sh` runs every scenario (median of 5); `./benchmarks/run.sh str_find 9` runs one scenario with 9 runs, `./benchmarks/run.sh "" 7` every scenario with 7 runs.
- `cargo run --release --example bench -p bsl-vm` runs the in-process VM scenarios; `cargo run --profile profiling --example profile -p bsl-vm` writes one flamegraph SVG per scenario.
- `run.sh` compares seven columns — `bsl-cli` interpreted, `bsl-cli --jit`, Lua, LuaJIT, CPython, OneScript, and the 1C platform — and skips any runtime it cannot find. OneScript is looked up in `PATH` and a few usual install prefixes; override with `OSCRIPT=/path/to/oscript`, and the Python interpreter with `PYTHON=`.
- The heavy scenarios (`csv_write*`, `table_compare`, `table_compare2`, `table_save_load`) run `HEAVY_RUNS` passes each (default 3) with file output redirected into `${TMPDIR:-/tmp}/onec-bench-scratch`, and the script byte-compares the files the runtimes produced — a benchmark that computed something different is a failed benchmark, not a fast one.
- The 1C column is *not* measured live: it is read from `benchmarks/1c/combined.platform.txt`, regenerated by building one combined script with `python3 benchmarks/1c/build-combined.py` and running it on the platform.
- `benchmarks/lib/slaxml.lua` is the workspace's only vendored third-party code (MIT, used by the `xml_*` Lua twins, never built into the interpreter). It is deliberately not stock: five spots marked `ПРАВКА open-bsl` widen the element/attribute *name* character classes to UTF-8 high bytes, because upstream's `%a` is ASCII-only and returns zero nodes on the Cyrillic benchmark document. Do not "update" it from upstream or point the twins at the system copy — see `benchmarks/lib/README.md`.
- `.cargo/config.toml` sets `-C llvm-args=-align-all-functions=5` for every build. This is measurement hygiene, not a micro-optimization: without it an unrelated edit can shift code layout and move a benchmark by tens of percent. Setting `RUSTFLAGS` overrides it, which is what profiling runs do.

## Coding Style & Naming Conventions

Use standard `rustfmt` output and four-space indentation. Follow Rust conventions: `snake_case` for modules, functions, variables, and tests; `CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants. Keep functionality in the narrowest relevant crate and expose it through `lib.rs`. Prefer descriptive test names such as `division_half_up_on_exact_tie`.

### Comments and Rustdoc

Write comments and rustdoc in clear Russian technical prose. Explain intent, invariants, compatibility constraints, and non-obvious tradeoffs rather than restating the code. Use complete sentences and normal sentence case; avoid conversational emphasis in all capitals. Prefer natural Russian wording over unnecessary calques such as «персистить», `see`, `literal`, or `top-level`; established project terms such as «чанк» and JIT are acceptable when they are clearer. Write «X — это Y» with an em dash.

Use `//` for implementation notes, `///` for API documentation, and `//!` for crate or module overviews. Enclose Rust identifiers, opcodes, commands, code fragments, and marker IDs in backticks. Public functions returning `Result` should document failure conditions under `# Errors`; document genuine panic conditions under `# Panics`. Keep rustdoc links resolvable and ensure `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` succeeds.

Preserve compatibility markers exactly as `` `НЕ ИЗМЕРЕНО(AREA.QUESTION)` `` so both rustdoc and the registry scanner recognize them. Do not reword or remove a marker without the corresponding measurement and registry updates described below.

## Testing Guidelines

Use Rust's built-in test framework. Add focused unit tests near changed logic and integration tests in a crate's `tests/` directory. Conformance fixtures use matching `name.bsl` and `name.expected` files. An absent `.expected` intentionally marks an unmeasured case; never generate oracle output from this interpreter. Preserve `// НЕ ИЗМЕРЕНО(ID)` markers until behavior is measured against real 1C.

The only snapshot tests are the parser ones in `crates/bsl-syntax/tests/snapshots/`, taken over the n-body fixtures — a whole-AST shape check, so any grammar change shows up there first. `insta` writes `.snap.new` next to the stored snapshot; read the diff and accept it deliberately (`cargo insta review` if `cargo-insta` is installed, otherwise move the file yourself). Never accept it just to make the run green.

One `bsl-cli` test is silently environment-gated: `crates/bsl-cli/tests/table_compare2.rs` replays the value-table diff on large confidential cases from the directory in `OPEN_BSL_TABLE_COMPARE2_CASES` (one subdirectory per case, four `ЗначениеВФайл` dumps each). Without the variable it skips with a note — a green run does not mean it ran.

Divergence between the interpreter and the JIT is caught by `the_jit_agrees_with_the_interpreter_on_every_script`, which runs the whole fixture corpus both ways.

### Measuring on the 1C platform

1C behavior is measured on the real platform, never inferred from this implementation's output. Every decision made by reasoning rather than by checking against the platform needs all three of: a `// НЕ ИЗМЕРЕНО(AREA.QUESTION)` marker at the code site, an entry in `crates/bsl-rt/src/open_questions.rs`, and exactly one line with the same ID in `tests/conformance/measure/measure-all.bsl`. The test `open_questions_registry_matches_source_markers` fails if any of the three is missing or extra, in both directions. `MEASURED_ANCHORS` in the same file are known platform values used as session canaries — if an anchor comes back wrong, the whole measurement session is suspect.

The measurement round trip:

```bash
./tests/conformance/measure/1c/run-on-1c.sh                                  # measure-all.bsl -> platform.tsv
./tests/conformance/measure/1c/run-on-1c.sh tests/conformance/measure/measure-xml.bsl   # -> measure-xml.platform.txt
cargo run -p bsl-cli -- --ingest-measurements tests/conformance/measure/platform.tsv
```

The runner takes an optional script path: `measure-all.bsl` lands in `platform.tsv` (the registry oracle), anything else in `<script>.platform.txt` next to it. Both are committed. It wraps the script in a form module and loads that straight into the configuration of a throwaway file infobase (`tests/conformance/measure/1c/cfg-src` holds the static object XML; `DumpConfigToFiles`/`LoadConfigFromFiles` do the merging), then starts `ENTERPRISE` with no `/Execute` — the configuration's own startup handler opens the form. This sidesteps the platform's unsafe-action prompt for opening an external file entirely, which is why `ONEC_IB` is a free-form knob now rather than a fixed, `conf.cfg`-exempted path. Needs a display. Other knobs: `ONEC_PLATFORM` (path to `1cv8`), `ONEC_TIMEOUT` (default 180 s), `ONEC_SHIM`. Exit code 0 with an empty result file means the form module did not compile, not that the run passed; the runner checks the file rather than the exit code, and prints the platform log.

`--ingest-measurements` records results and prints discrepancies. It never edits code — every discrepancy is a human decision. It always writes `platform.tsv` beside the file you hand it, so ingesting a `measure-*.platform.txt` from that same directory clobbers the `measure-all` oracle.

Writing a new `measure-*.bsl` (the contract scripts next to `measure-all.bsl`) has five rules that each cost a wasted 40-second platform round trip to learn:

- **It must run on both sides.** The script is executed by the platform *and* by this interpreter so the two outputs diff line by line. Anything unimplemented here — `ЧтениеТекста`, a type `Новый` does not know, a method name the resolver rejects at compile time — breaks the whole run, not one line. Probing something the platform might not have goes through `Вычислить` inside `Попытка`, where the failure is catchable on both sides.
- **IDs must be literal.** The scanner in `open_questions_registry.rs` reads string literals as written, so `М("AREA." + Имя, ...)` registers a truncated ID. Unroll the loop.
- **One `М()` per ID.** The rule is exactly one line per ID across all scripts, so the `Попытка`/`Исключение` pair must assign to a variable and report once, not call `М()` in both branches.
- **A modal dialog on the platform reads as a timeout.** An unhandled exception at startup shows a modal: no output, 180 s, killed. Wrap every probe. A form module compiles lazily, so a syntax error — for instance a variable named `И`, which is a keyword — shows up the same way, with an empty log and a successfully updated configuration.
- **A variable named like a form property is that property.** The script runs inside a *managed form* module, so `Ширина = "";` assigns an empty string to the form's width, not to a local — and the resulting type error is a modal, i.e. the same silent empty-output timeout. `Попытка` does not help: the assignment is the probe's setup, not its body. Hence `Ширина40`/`ШиринаЗнаков` rather than `Ширина`. Suspect this whenever a script hangs with output that stops right before a block whose first statement assigns to a short, generic Russian noun (`Ширина`, `Высота`, `Заголовок`).

## Commit & Pull Request Guidelines

Recent commits use short, imperative subjects such as `Add the string library...` and `Turn input-dependent VM panics into RtError...`. Keep each commit focused. Pull requests should explain behavior changes, identify affected crates, list validation commands, and link relevant issues. Include measured 1C output when changing compatibility semantics; screenshots are only useful for visible CLI or diagnostic changes.

## Ralph loop (unattended plan → implement → review)

`ralph/ralph.sh` runs an unattended loop over `claude -p` that drains `TASKS.md`. Its state lives on disk under repo-root filenames, and several of those files have ownership rules an agent can break by editing them directly — do not touch them outside the loop:

- `TASKS.md` — the backlog. Line format `- [ ] (slug) [hard] description`; the optional `[hard]` tag triggers a two-reviewer panel (both must PASS). Only the loop's arbiter flips `- [ ]` to `- [x]` on a PASS — never tick a box by hand mid-run.
- `PLAN.md`, `REVIEW_FABLE.md`, `REVIEW_OPUS.md` — phase hand-off state. A lingering `VERDICT: FAIL` in a `REVIEW_*.md` means the last attempt was rejected, so the next planning phase plans the fix rather than picking a new task.
- `COMMIT_MSG.md` — the implementer's commit message for the whole task. On PASS the arbiter squashes the task's iteration checkpoints back to their base and commits once with this message; the mechanical `ralph: iter N slug PASS|FAIL` commits survive only while a task is still open. History therefore carries one commit per task, not one per iteration.
- `PROGRESS.md` — append-only run log; read it afterwards, never rewrite it.

Prompts for the three phases live in `ralph/prompts/`; `commit-style.md` is the commit-message style the implementer follows and the reviewer checks. The model for each phase is an env knob (`PLAN_MODEL`, `IMPL_MODEL`, `REVIEW_MODEL`, `REVIEW2_MODEL`, all default `opus`), which is how a per-model usage limit is worked around without editing the script. Run from the repo root: `./ralph/ralph.sh` (tune iterations with `MAX_ITERS=40`). The current roadmap feeding the backlog is `docs/std-library-plan.md`.

`CLAUDE.md` is gitignored and only re-exports this file to Claude Code via `@AGENTS.md`; team-wide conventions go in `AGENTS.md`, never in `CLAUDE.md` — edits there are silently lost.
