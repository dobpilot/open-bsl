#!/bin/bash
# Плоские профили perf для пар «прошлый main / текущий main».
# Профилируются profiling-сборки (release + debug) из снимков A/B 23 октября;
# сценарии — подготовленные драйвером A/B, с теми же путями данных и вывода.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../../../.." && pwd)
OUT=$(cd "$(dirname "$0")" && pwd)
BUILD=$ROOT/target/full-ab-2026-10-23-build
SCRIPTS=$ROOT/target/full-ab-2026-10-23/scripts
WORK=$ROOT/target/hotspots-2026-10-24
FREQ=${FREQ:-2999}
mkdir -p "$WORK"
for scenario in ${SCENARIOS:-background_jobs str_index xml_parse goto_bench}; do
  for side in main current; do
    cwd=$WORK/$scenario-$side
    rm -rf "$cwd"; mkdir -p "$cwd/benchmarks" "$cwd/scratch"
    binary=$BUILD/$side-src/target/profiling/bsl-cli
    echo "PROFILE $scenario $side"
    (cd "$cwd" && perf record -q -F "$FREQ" -o "$WORK/$scenario-$side.data" -- \
        "$binary" "$SCRIPTS/$scenario.bsl" > "$OUT/$scenario-$side.out" 2> "$OUT/$scenario-$side.err")
    perf report -i "$WORK/$scenario-$side.data" --no-children --sort symbol \
        --percent-limit 0.5 --stdio 2>/dev/null | grep -v '^$' > "$OUT/$scenario-$side.report.txt"
  done
  # Символы сопоставляются по имени: бинарники собраны из разных снимков.
  perf diff -c delta-abs --sort symbol \
      "$WORK/$scenario-main.data" "$WORK/$scenario-current.data" 2>/dev/null \
      | grep -v '^$' | head -60 > "$OUT/$scenario.diff.txt" || true
done
echo PROFILED
