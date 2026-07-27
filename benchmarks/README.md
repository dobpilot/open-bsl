# Microbenchmarks

`pi_leibniz.bsl` and `pi_leibniz.lua` calculate the same one-million-term
Leibniz series and print both the result and elapsed milliseconds.

Build the interpreter and run either implementation:

```bash
cargo build --release -p bsl-cli
target/release/bsl-cli benchmarks/pi_leibniz.bsl
lua benchmarks/pi_leibniz.lua
```

Run each command several times and compare the median rather than a single
run. The BSL timer reports UTC wall-clock milliseconds; Lua's standard
library only exposes `os.clock()`, which reports process CPU time. For this
single-threaded, CPU-bound benchmark the values are comparable, but an
external benchmark runner should be used for rigorous measurements.

This is not an arithmetic-equivalence benchmark. `BslNumber` performs exact
decimal operations and produces 27 decimal places for division, while
standard Lua uses hardware binary `double`. Consequently, this benchmark
measures the cost of each language's actual numeric semantics in addition to
VM dispatch.

## Empty numeric loop

`empty_for.bsl` and `empty_for.lua` execute an inclusive loop from zero through
one million with an empty body. The loop is repeated ten times in one process
and the scripts print the mean milliseconds per pass. Repetition reduces
timer, startup, and scheduler noise. The benchmark isolates loop-control
overhead: counter initialization, comparison, increment, and instruction
dispatch.

```bash
target/release/bsl-cli benchmarks/empty_for.bsl
lua benchmarks/empty_for.lua
```

Both loops execute exactly 1,000,001 iterations.

## CSV output

`csv_write.bsl` and `csv_write.lua` each write 300,001 rows with 21
semicolon-separated fields to `test.csv` in the current directory:

```bash
mkdir -p /tmp/onec-csv-bench
cd /tmp/onec-csv-bench
target/release/bsl-cli /path/to/onec_llvm/benchmarks/csv_write.bsl
lua /path/to/onec_llvm/benchmarks/csv_write.lua
```

Build `bsl-cli` with `cargo build --release -p bsl-cli` first. The BSL
version uses buffered `ЗаписьТекста`; Lua uses its default output stream.
The repeated `d13` field is intentional. Run both on the same filesystem
and alternate their order because filesystem and page-cache behavior can
dominate the result. BSL reports wall-clock time; Lua's `os.clock()` reports
process CPU time, so use an external timer for a strict comparison.

`csv_write_batched.bsl` is the application-level optimized variant. It
builds the invariant CSV row before the loop and performs one buffered
write per row. Keep using `csv_write.bsl` when comparing the original
42-call workload with Lua.
