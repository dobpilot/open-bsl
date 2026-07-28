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

## String and table scenarios

Four scenarios added for the string and table subsystems, each with a Lua
twin. The `.bsl` file is also the oscript (OneScript) input — it is the same
language, so there is nothing to port.

| scenario      | what it measures                                            |
|---------------|-------------------------------------------------------------|
| `str_find`    | `СтрНайти` over a 215K-code-unit haystack, needle at the end |
| `str_concat`  | building a string by repeated concatenation (quadratic copy) |
| `table_total` | filling 200K rows, then `Итог` over a column 50 times        |
| `table_sort`  | three `Сортировать` passes plus a linear `Найти` over 100K rows |

**Scenario contract:** the script prints its result (so every runtime can be
checked to have computed the same thing) and, as the LAST line, the elapsed
milliseconds as a bare number. The script times itself; the runner does not.
That keeps process startup out of the measurement — a few milliseconds for
`bsl-cli`, tens of milliseconds of .NET warm-up for oscript — so what gets
compared is the interpreters, not the ways of launching them.

## Running everything

```bash
cargo build --release -p bsl-cli
./benchmarks/run.sh              # all scenarios, 5 runs each
./benchmarks/run.sh str_find 9   # one scenario, 9 runs
```

The runner reports the **median**, not the mean: a single scheduler hiccup
should not drag the number with it. Missing runtimes are printed as absent
and their column stays blank — invented numbers are worse than missing ones.

## Results

Median of 5 runs, milliseconds, lower is better. Intel i5-8250U, Linux
7.1.3, `--release`. Numbers are machine-specific; re-run before drawing
conclusions on other hardware.

| scenario        | bsl-cli | lua 5.4 | luajit | oscript |
|-----------------|--------:|--------:|-------:|--------:|
| `empty_for`     |       5 |       7 |      1 |       — |
| `pi_leibniz`    |     751 |      30 |      2 |       — |
| `pi_leibniz_15` |    1049 |       — |      — |       — |
| `str_concat`    |     198 |     109 |     91 |       — |
| `str_find`      |     561 |     422 |     63 |       — |
| `table_total`   |     402 |     382 |    187 |       — |
| `table_sort`    |    1666 |     700 |    825 |       — |

**oscript was not measured**: OneScript is not installed in this environment
(neither is mono/dotnet), so its column is empty by fact, not by omission.
The scripts are ready for it — `oscript benchmarks/<name>.bsl` — and
`run.sh` picks it up automatically as soon as the binary is on `PATH`.

## What the profile says

`cargo run --profile profiling --example profile -p bsl-vm` writes a
flamegraph per scenario and prints the five heaviest stacks as text.

* `str_find` — **98% of the scenario sits in `BslString::find`**, all of it
  in slice comparison. The search is a naive O(n·m) scan: ~192M code units
  per second, roughly 384 MB/s. This is the one place where a better
  algorithm (or vectorization) has an order of magnitude of headroom.
* `str_concat` — everything is in copying `Rc<[u16]>`. Quadratic by
  construction, exactly as in Lua, which is why the two are within 2x.
* `table_total` — `Итог` is ~45% of the scenario. Measured directly,
  `ValueTableData::total` costs **71 ms for 3M cells, i.e. ~24 ns per
  cell**: a `match` on a 24-byte tagged value plus a decimal addition.
  BigInt is *not* involved (the sum stays `Small`; an earlier flamegraph
  attributing 24% to `BigInt::add` was a symbolization artifact of
  inlining — checked by calling `total` directly).
* `table_sort` — ~27% goes into `collate` and the Unicode `to_upper` tables
  it calls. The collation approximation converts BOTH strings to upper case
  on EVERY comparison, i.e. two allocations per comparison. Note this is an
  unmeasured behavior (`TABLE.SORT.COLLATION`): it must not be optimized
  into a different ordering.
* `empty_for` — we beat Lua 5.4 here (5 ms vs 7 ms for a million
  iterations). Dispatch is not the bottleneck anywhere in this set; the
  `NumericForNextI64` superinstruction already does what a superinstruction
  can.

## Comparability caveats

The comparison is deliberately apples-to-oranges, and every gap has a known
cause in semantics rather than in implementation quality:

* **Numbers.** `BslNumber` is exact decimal with 27 digits after the point
  on division; Lua uses hardware `double`. `pi_leibniz` measures the price
  of that semantics, not VM speed.
* **Strings.** Lua strings are bytes, ours are UTF-16 code units. Cyrillic
  text occupies the same memory in both (2 bytes per letter), but Lua scans
  twice as many *elements*.
* **Tables.** Lua has no columnar collection; the twin models
  `ТаблицаЗначений` as an array of records, which is the idiomatic
  equivalent but a different memory layout.
* **Sorting.** `table.sort` is unstable, `Сортировать` is stable and must
  stay that way (invariant 18), and our string comparison goes through
  locale-approximating collation rather than byte order.
* **LuaJIT** is a JIT compiler and is listed separately for exactly that
  reason: comparing it with interpreters in one column would be misleading.
