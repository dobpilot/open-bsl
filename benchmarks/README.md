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

The 1C column is not filled by that runner. The platform takes tens of
seconds to come up (create an infobase, load an external data processor),
so paying that per scenario is pointless when the scenario times itself.
Instead every scenario is stitched into one script and run once:

```bash
python3 benchmarks/1c/build-combined.py 3
ONEC_TIMEOUT=900 ./tests/conformance/measure/1c/run-on-1c.sh benchmarks/1c/combined.bsl
```

That writes `benchmarks/1c/combined.platform.txt`, from which `run.sh`
takes the medians. No file — the column stays a dash. Edit a scenario and
the file goes stale, so re-take it.

The runner reports the **median**, not the mean: a single scheduler hiccup
should not drag the number with it. Missing runtimes are printed as absent
and their column stays blank — invented numbers are worse than missing ones.

## Results

Median of 7 runs, milliseconds, lower is better. Intel i5-8250U, Linux
7.1.3, `--release`. Numbers are machine-specific; re-run before drawing
conclusions on other hardware.

| scenario        | bsl-cli  | lua 5.4 | luajit | oscript 2.1 | 1C 8.3.27 |
|-----------------|---------:|--------:|-------:|------------:|----------:|
| `empty_for`     |    **3** |       3 |      0 |         191 |       326 |
| `pi_leibniz`    |  **491** |      21 |      1 |        1213 |      1194 |
| `pi_leibniz_15` |  **688** |       — |      — |        1410 |      1351 |
| `str_concat`    |      130 |      71 |     53 |         136 |    **17** |
| `str_find`      |      532 |     297 |     35 |      **34** |       132 |
| `table_total`   |  **273** |     246 |    128 |        1078 |      3205 |
| `table_sort`    | **1565** |     549 |    492 |        1115 |      3289 |

Two of these columns run **the same language with the same semantics** —
exact decimal arithmetic, `ТаблицаЗначений`, UTF-16 strings — and are the
only ones where a gap means "slower at the same job" rather than "doing a
different job": **1C**, which is what this project targets, and **oscript**,
an independent implementation of it. Lua and LuaJIT are the outside
reference: what a mature dynamic-language VM costs when it is allowed to
use hardware doubles and byte strings.

Against the platform we are ahead on five scenarios of seven — 100x on an
empty loop, 2.4x on decimal arithmetic, 12x on filling a value table and
summing a column, 2.1x on sorting — and behind on the two string ones:
**7.6x slower on concatenation** and **4x slower on substring search**. Both
gaps have a known cause, see below; both are the honest reading, since the
platform is doing exactly our job on exactly our input.

`pi_leibniz` is also an accuracy result and not only a speed one: the
platform printed `3,141591653589793238712644144`, digit for digit what we
print. A million exact decimal divisions landing on the same 27 places is
independent confirmation of the division scale, which was chosen from
documentation rather than measured.

oscript is faster than the platform on almost everything (it runs on .NET
with `decimal` and native strings) and slower than us on collections. Its
one crushing win is `str_find`: 34 ms against our 532, and against the
platform's 132 — a hand-written naive scan losing to a library search, in
both directions.

Running the suite against oscript is also what caught a naming bug:
`pi_leibniz_15` used to fail there because this interpreter spelled the
rounding builtin `Округл`, while OneScript — like 1C — spells it **`Окр`**.
The function has since been renamed and `Округл` no longer resolves at all,
which is the point: a name that does not exist on the platform must not
compile here either. All three BSL runtimes now print the same digits for
the 15-digit Leibniz sum.

`oscript` is discovered on `PATH`, in the usual install locations
(`/opt/oscript/bin` among them), or wherever `OSCRIPT=/path/to/oscript`
points. Its .NET start-up is not in the numbers: every script times itself
after warm-up, per the scenario contract above.

### Where we lose, and why

* **`str_concat`, 7.6x behind the platform.** Our string is an immutable
  `Rc<[u16]>`, so `Текст = Текст + Кусок` copies everything accumulated so
  far: quadratic in the number of steps. 17 ms for 3000 steps says the
  platform does not copy each time — some form of amortized growth or a
  rope. This is the clearest optimization target in the project: the
  pattern is the most common way application BSL builds text.
* **`str_find`, 4x behind the platform and 15x behind oscript.**
  `BslString::find` is a naive loop comparing slices, with no skip table
  and no vectorization. Nothing about the semantics requires that.

### Where the platform loses

`table_total` and `table_sort` are 12x and 2.1x in our favour, and
`empty_for` is 100x. The value-table gap is layout: we store a column as a
contiguous vector, and `Итог` walks it without touching the other columns.
The loop gap is dispatch — a register VM with the counter in a register,
against whatever the platform does per iteration; both count in exact
decimal, so the semantics are equal here.

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
