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

`csv_write.bsl` and `csv_write.lua` each write 300,001 rows of 21
semicolon-separated fields to `test.csv` in the current directory — 16.5 MB.
The repeated `d13` field is intentional. `csv_write_batched.bsl` is the
application-level optimized variant: it builds the invariant row once before
the loop and does one buffered write per row instead of 42.

These scenarios are part of `run.sh` and need no special handling, but they
are treated as **heavy**: they run in a scratch directory outside the source
tree (they open a relative path) and default to 3 passes instead of 5, since
one pass writes 16.5 MB per runtime. `HEAVY_RUNS=5 ./benchmarks/run.sh`
overrides that.

They are also the one group whose scenarios print nothing but the elapsed
milliseconds — there is no result line to compare. The cross-check is
therefore the **produced file**: the runner keeps each runtime's output and
compares them byte for byte, and the platform's own output is compared too
when a 1C run has left one. All five agree, including real 1C:

```
csv_write: вывод 1c совпал с нашим побайтно
csv_write: вывод lua совпал с нашим побайтно
csv_write: вывод luajit совпал с нашим побайтно
csv_write: вывод oscript совпал с нашим побайтно
```

That check is not decoration. It is what caught this interpreter writing
the file wrong: `ЗаписьТекста` emitted raw UTF-8 with LF line endings,
while the platform writes a **UTF-8 BOM** and expands a line feed inside a
written string into **CRLF**. Four probes on 8.3.27 pinned the rule (they
need `ДвоичныеДанные`, so they live in
`tests/conformance/measure/measure-unsupported.bsl` and are anchored in
`bsl_rt::open_questions`):

| written | bytes on disk | reading |
|---------|---------------|---------|
| `"A" + Символ(10) + "B"` | `EFBBBF 41 0D0A 42` | BOM, and LF becomes CRLF |
| `"A" + Символ(13) + "B"` | `EFBBBF 41 0D 42` | a lone CR passes through |
| `"A" + Символ(13) + Символ(10) + "B"` | `EFBBBF 41 0D 0D0A 42` | CR passes, LF expands |
| `"Ая"` | `EFBBBF D090 D18F` | the default encoding is UTF-8 |

The Lua twin was changed to emit the same bytes, so the column compares
runtimes rather than file formats.

## String and table scenarios

Four scenarios added for the string and table subsystems, each with a Lua
twin. The `.bsl` file is also the oscript (OneScript) input — it is the same
language, so there is nothing to port.

| scenario      | what it measures                                            |
|---------------|-------------------------------------------------------------|
| `call_overhead` | a million calls to a user function whose body is one addition |
| `str_find`    | `СтрНайти` over a 215K-code-unit haystack, needle at the end |
| `str_concat`  | building a string by repeated concatenation, ten times over  |
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

Median of 7 runs, milliseconds, lower is better (`csv_write*`: 3 runs).
Intel i5-8250U, Linux 7.1.3, `--release`. Numbers are machine-specific;
re-run before drawing conclusions on other hardware.

**Function alignment is a build flag, not a detail.** `.cargo/config.toml`
passes `-align-all-functions=5`. It went in after adding shims to the JIT
module slowed `str_find` by 45% — with the search code untouched and
`--jit` not even in play. `perf stat` showed identical instruction counts
in both binaries (651M) and 50% more cycles: the hot loop had simply moved
off a favourable boundary. Without the flag, an edit anywhere can shift an
unrelated benchmark by tens of percent, and a real regression cannot be
told from layout luck.

**Take every column from one session.** An earlier draft of this table
mixed a 1C column measured hours before with the rest, and the machine had
meanwhile dropped to its minimum 800 MHz — every other runtime was 30-40%
slower, which reads exactly like a regression and is not one. The numbers
below were all taken with the CPU at 2.4-2.9 GHz.

| scenario            |  bsl-cli | `--jit` | lua 5.4 | luajit | oscript 2.1 | 1C 8.3.27 |
|---------------------|---------:|--------:|--------:|-------:|------------:|----------:|
| `call_overhead`     |      146 |     149 |      27 |      1 |         928 |      1868 |
| `csv_write`         |     1833 |    1699 |    1022 |    120 |       23636 |      5033 |
| `csv_write_batched` |       65 |  **58** |       — |      — |        1169 |       237 |
| `empty_for`         |    **2** |   **2** |       4 |      1 |         203 |       319 |
| `pi_leibniz`        |      520 | **351** |      21 |      1 |        1294 |      1282 |
| `pi_leibniz_15`     |      714 | **545** |       — |      — |        1486 |      1388 |
| `str_concat`        |        2 |   **1** |     576 |    619 |        1255 |       164 |
| `str_find`          |       49 |      45 |     329 |     37 |      **35** |       134 |
| `table_total`       |      299 | **280** |     252 |    126 |        1140 |      3272 |
| `table_sort`        |     1658 |    1636 |     560 |    467 |        1112 |      3356 |

*This session ran about 8% slower across every runtime than the one before
it — compare within the table, not with an older copy of it.*

`--jit` is the same binary with the flag of the same name: bytecode
compiled to x86-64 machine code (see the JIT section in the root README).
It is a separate column rather than a replacement because the JIT is
opt-in, and the interpreter number has to stay visible.

Two of these columns run **the same language with the same semantics** —
exact decimal arithmetic, `ТаблицаЗначений`, UTF-16 strings — and are the
only ones where a gap means "slower at the same job" rather than "doing a
different job": **1C**, which is what this project targets, and **oscript**,
an independent implementation of it. Lua and LuaJIT are the outside
reference: what a mature dynamic-language VM costs when it is allowed to
use hardware doubles and byte strings.

Against the platform we are ahead on eight scenarios of nine — 100x on an
empty loop, 2.4x on decimal arithmetic, 12x on filling a value table and
summing a column, 2.1x on sorting — and behind on the two string ones:
and behind only on `csv_write`, at 2.7x. Both string scenarios used to be
losses — 7.6x on concatenation, 4.4x on search — and both were fixed after
this table first showed them; see "Where we lose" below for what they cost
now.

The two CSV scenarios say the same thing in three runtimes at once. Going
from 42 writer calls per row to one costs 1720 -> 68 ms for us, 4709 -> 224
on the platform, 23512 -> 1166 on oscript: a 20-25x drop everywhere. What
that measures is per-call overhead, not I/O — the bytes written are
identical in all three, and `csv_write` is the one scenario where the
platform still beats us end to end. oscript's absolute number is the
outlier: 23 seconds, 5x the platform, for the same 12.6M method calls.

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

### What the JIT is worth

| scenario | interpreter | `--jit` | |
|---|---:|---:|---:|
| `pi_leibniz` | 509 | 368 | 1.38x |
| `pi_leibniz_15` | 703 | 522 | 1.35x |
| `table_total` | 265 | 241 | 1.10x |
| `csv_write_batched` | 64 | 60 | 1.07x |
| `csv_write` | 1674 | 1604 | 1.04x |
| `table_sort` | 1591 | 1555 | 1.02x |
| `str_find`, `str_concat`, `empty_for` | | | 1.00x |
| `call_overhead` | 134 | 138 | **0.97x** |

The gain is confined to loops made of arithmetic and comparisons — the
only instructions compiled. Everything else still runs interpreted, and
the JIT hands control straight back, which is why the string, collection
and file scenarios do not move.

`pi_leibniz` is the clearest case, and it moves the comparison with the
platform too: **375 ms against 1186**, 3.2x rather than 2.4x.

What unlocked most of that was compiling the instruction that CLOSES a
numeric loop (`NumericForNext`). Without it the loop body compiled fine
but the counter step did not, so native code left for the interpreter on
every single iteration and the native jumps bought nothing. With it the
whole body of `pi_leibniz` — eleven instructions — stays native, and the
scenario went 378 -> 354 ms in a direct A/B of the two builds.

`empty_for` staying at 2 ms is not waiting for the same fix, contrary to
what this file said before it was measured. Its loop is a single
`NumericForNextI64`, and the interpreter already services that in a
compact loop inside `drive_with` that never enters the dispatcher at all
— there is no dispatch left there for a JIT to remove.

`GetIndex`, `SetIndex`, `CallBuiltin`, `GetProp`, `SetProp` and
`CallMethod` are compiled as well. Adding the last three is what moved
`table_total`: its fill loop is nine instructions, of which `CallMethod`
and two `SetProp` were not compiled, so native code left for the
interpreter twice per row across 200 000 rows. With them the loop stays
native end to end — 251 -> 238 ms in a direct A/B, 1.10x against the
interpreter overall.

Those six share a trait worth noting: they have more operands than the
three the shim call passes, or an operand that is not a number (a name id,
a builtin or method code). They therefore read their own instruction out
of the chunk — by `pc` they find it anyway, and matching one known variant
costs nothing next to the opcode dispatch they replace. `GetProp` and
`SetProp` use the inline cache cell of THAT instruction, the same one the
interpreter uses: a separate cache for the JIT would warm a monomorphic
site twice and differently.

**`call_overhead` is the one scenario where `--jit` LOSES**, 134 -> 138 ms.
Its loop leaves native code at every `Call` and again at every `Возврат`,
because an instruction that changes the frame stack cannot be compiled —
execution continues in a different chunk. So each call pays two native
exits and two native entries on top of the two dispatches the interpreter
would have done anyway. Nothing else in the baseline shows this because
nothing else calls anything in a loop.

Making the entry cheaper does not fix it. Hoisting the JIT context out of
the dispatch loop and looking the compiled chunk up once instead of twice
removed 8M instructions from a 1.48G run — and the same build spent 20M
MORE cycles and measured 4% slower. That is layout again, below the floor
this machine can resolve even with function alignment pinned. The real fix
is native transfer between chunks, which is a different piece of work.

The `csv_write` cross-check covers the JIT as well: the file it produces
under `--jit` is compared byte for byte with the interpreter's, alongside
the other runtimes.

### What BMI2 and ADX turned out to be worth

The question was whether the decimal arithmetic should use ADX (ADCX/ADOX)
and BMI2 (MULX). Measured answers, in the order they were found:

* **Simply enabling them is a regression.** `-C target-feature=+bmi2,+adx`
  put 159 adcx/adox/mulx instructions in the binary against 45, and
  `pi_leibniz` went 465 -> 480 ms. `-C target-cpu=native` made it 535. LLVM
  will use the instructions; using them is not the same as being faster.
* **The bottleneck they would address is not the bottleneck.** The hot
  arithmetic is 128-bit DIVISION — `u128_div_rem` was 6.45% of the profile.
  Neither ADX nor MULX divides anything.

Two things did help, and neither needed the extensions:

* **Exact division by ten, by modular inverse.** `normalize_small` strips
  trailing zeros one at a time, and each strip was a software `__divti3`.
  But divisibility is already known at that point, and an exact division
  needs no division: ten is two times five, dividing by two is an
  arithmetic shift, and dividing by five is multiplication by 5⁻¹ mod 2¹²⁸.
  `u128_div_rem` fell from 6.45% to 5.26%. Worth about 1%, because the
  normalization loop usually exits on the first divisibility check and
  never divides at all.
* **Hardware 128-by-64 division.** compiler-rt's `u128_div_rem` is generic:
  73 instructions of case analysis around five hardware `div`s. When the
  divisor fits in 64 bits — which a decimal divisor mantissa usually does —
  one or two `div` instructions are the whole algorithm. Worth about 2%.

And then the profile said what the remaining cost is. `u128_div_rem`
disappeared from it entirely, and `BslNumber::div` grew by exactly its
share, 5.81% -> 11.63%: the division is now inline, and **what is left is
the latency of the `div` instruction itself**, not the dispatch around it.
No arrangement of ADX or BMI2 changes that. Going faster would mean not
dividing — reciprocal multiplication, which is where MULX would genuinely
pay — and that only amortizes when the same divisor repeats. Here every
division has a different one.

### Where the string scenarios went

Both string losses in the first version of this table were implementation,
not semantics, and both are now the other way round.

* **`str_concat`: 134 -> 2 ms, against 171 on the platform.** The string
  was an immutable `Rc<[u16]>`, so `Текст = Текст + Кусок` copied
  everything accumulated so far — quadratic in the number of steps. It is
  now `Rc<Vec<u16>>`, and `Add` appends in place when the reference count
  is one, which makes the loop linear. Getting the buffer into sole
  ownership took a codegen change too: an operand that is already a plain
  variable is no longer copied into a temporary register, so the
  accumulating assignment compiles to `Add dst=2 a=2 b=0` and the VM can
  take the value out of the register it is about to overwrite. Value
  semantics are unaffected — `Rc::get_mut` hands over the buffer only when
  nobody else holds it — and four tests pin that: a copy in a variable, in
  an array, in a structure, `Х = Х + Х`, and a byref parameter.
  *The scenario was rescaled to 10 builds at this point: one build now
  finishes below the resolution of a millisecond timer.*
* **`str_find`: 573 -> 75 ms, against 134 on the platform.** The search was
  a slice comparison at every position. It now skips to the first matching
  code unit with a scan LLVM vectorizes, then rejects on the last unit
  before comparing at all. `СтрЗаменить` and `СтрРазделить` had their own
  copies of the same naive loop and now share this one engine.
  LuaJIT and oscript are still ~2x ahead here; both hand the job to a
  library routine.

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
