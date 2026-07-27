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
