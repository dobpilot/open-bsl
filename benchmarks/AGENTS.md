# Repository Guidelines

## Project Structure & Module Organization

This directory contains cross-runtime benchmarks for the Open BSL Rust workspace. Each scenario starts with a `name.bsl` file; matching `name.lua` and `name.py` files provide comparable implementations where practical. `run.sh` discovers scenarios and reports median timings for `bsl-cli`, its JIT, Lua, LuaJIT, Python, OneScript, and recorded 1C results. Static inputs live in `data/`; platform aggregation lives in `1c/`. `lib/slaxml.lua` is vendored and locally patched—preserve the marked `ПРАВКА open-bsl` changes.

## Build, Test, and Development Commands

Run commands from the workspace root unless noted:

- `cargo build --release -p bsl-cli` builds the executable used by benchmarks.
- `./benchmarks/run.sh` runs every scenario with five samples (heavy cases use `HEAVY_RUNS`, default 3).
- `./benchmarks/run.sh str_find 9` runs one scenario nine times.
- `PYTHON=python3 OSCRIPT=/path/to/oscript ./benchmarks/run.sh` overrides optional runtimes.
- `python3 benchmarks/1c/build-combined.py` regenerates the combined platform script; only refresh `combined.platform.txt` from a real 1C run.

## Scenario and Coding Conventions

Keep implementations equivalent across languages. A scenario must print its correctness result and print elapsed milliseconds as a numeric final line. Measure inside the script so process startup is excluded. Use `snake_case` filenames matching the scenario name. Follow `rustfmt` for Rust changes, four-space indentation in scripts, and clear Russian technical prose for implementation comments. Do not add dependencies for benchmark twins; Python versions use the standard library.

## Testing Guidelines

Treat output equivalence as part of performance correctness. `run.sh` byte-compares files produced by heavy scenarios and fails mismatches. Before submitting, run the affected scenario repeatedly, then run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Never derive 1C oracle values from this interpreter.

## Commit & Pull Request Guidelines

Use short, imperative commit subjects, consistent with history (for example, `Add CSV write benchmarks`). Keep each commit focused. Pull requests should describe the measured workload, list validation commands and available runtimes, explain output-equivalence checks, and include real platform output when changing 1C comparisons. Note missing runtimes or environmental limitations explicitly; do not invent benchmark numbers.
