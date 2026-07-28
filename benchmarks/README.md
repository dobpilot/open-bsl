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

| scenario        | bsl-cli | lua 5.4 | luajit | oscript 2.1 |
|-----------------|--------:|--------:|-------:|------------:|
| `empty_for`     |   **5** |       6 |      0 |         281 |
| `pi_leibniz`    | **724** |      30 |      3 |        1618 |
| `pi_leibniz_15` |    1064 |       — |      — |       error |
| `str_concat`    |     190 |     104 |     79 |         183 |
| `str_find`      |     553 |     439 |     52 |      **49** |
| `table_total`   | **377** |     345 |    149 |        1607 |
| `table_sort`    |    1744 |     680 |    589 |    **1467** |

oscript is the only runtime here that implements the **same language with
the same semantics** — exact decimal arithmetic, `ТаблицаЗначений`, UTF-16
strings — so it is the only column where a gap means "we are slower at the
same job" rather than "we do a different job". Lua and LuaJIT are the
outside reference: what a mature dynamic-language VM costs when it is
allowed to use hardware doubles and byte strings.

Against oscript we are **56x faster** on an empty loop, **2.2x** on decimal
arithmetic and **4.3x** on filling a value table and summing a column — but
**11x slower** on substring search and **1.2x slower** on sorting. Both of
those have a known cause, see below.

`pi_leibniz_15` fails under oscript because it calls `Округл`, and
OneScript — like 1C — spells that function **`Окр`**. Our interpreter
currently accepts `Округл`/`Round` only. The cell says `error` rather than
being silently dropped: a scenario one runtime cannot run is a fact about
the scenario, not a gap in the table.

`oscript` is discovered on `PATH`, in the usual install locations
(`/opt/oscript/bin` among them), or wherever `OSCRIPT=/path/to/oscript`
points. Its .NET start-up is not in the numbers: every script times itself
after warm-up, per the scenario contract above.

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
* **oscript** runs the very same `.bsl` file, so its column is the only
  like-for-like one. It is still not an oracle for *semantics*: this project
  targets 1C, and documented divergences from OneScript are expected
  (`pi_leibniz` prints 28 decimal digits there, 27 here, because C#
  `decimal` and `BslNumber` have different precision models).
