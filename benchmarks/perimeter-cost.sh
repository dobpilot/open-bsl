#!/bin/bash
# Цена ОБЩИХ ПРОВЕРОК периметра образа, в инструкциях, чередующимся A/B.
#
# Что меряется. `image::verify` идёт один раз на программу, до первой
# инструкции, но идёт на КАЖДОМ запуске: он проверяет границы регистров,
# номеров констант и модульных слотов, длины инлайн-кэшей и свежесть
# разметки бандлов. Диспетчер эта цена не трогает вовсе, поэтому
# `hot-code-diff.sh` о ней ничего не говорит — нужен отдельный замер.
#
# Как меряется. Сторона «без проверок» получается вырезанием блока между
# якорями «НАЧАЛО/КОНЕЦ ОБЩИХ ПРОВЕРОК» в `crates/bsl-bytecode/src/image.rs`.
# Переключателя для этого в продукте нет намеренно: выключаемая проверка
# образа — это способ однажды выключить её случайно. Обе стороны
# собираются в отдельных worktree и своих каталогах сборки, поэтому
# рабочее дерево не трогается и параллельный агент не может подменить
# бинарник между сборкой и копированием.
#
# Метрика — `instructions:u`, а не такты: замер идёт на рабочей машине,
# где load average бывает и семь, и такты вердикта не выдерживают.
# Инструкции же считаются на процесс и от загрузки не зависят.
#
#   ./benchmarks/perimeter-cost.sh [коммит] [пар]
#
# По умолчанию коммит — HEAD, пар — 5. Прогоны ЧЕРЕДУЮТСЯ: база, кандидат,
# база, кандидат — а не блоками, иначе разница блоков смешалась бы с
# разницей конфигураций.
set -euo pipefail

BASE_REF="${1:-HEAD}"
PAIRS="${2:-5}"
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

OUT="$(mktemp -d "${TMPDIR:-/tmp}/open-bsl-perimeter.XXXXXXXX")"
cleanup() {
    git worktree remove --force "$OUT/with" 2>/dev/null || true
    git worktree remove --force "$OUT/without" 2>/dev/null || true
    rm -rf "$OUT"
}
trap cleanup EXIT

# `.cargo/config.toml` задаёт `-align-all-functions=5`; чужие флаги
# перекрыли бы её, и стороны собрались бы по-разному.
if [ -n "${RUSTFLAGS:-}${CARGO_ENCODED_RUSTFLAGS:-}" ]; then
    echo "  RUSTFLAGS задан и снимается для обеих сборок" >&2
fi
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS

echo "== цена общих проверок периметра: $BASE_REF, пар $PAIRS =="
for side in with without; do
    git worktree add -q --detach "$OUT/$side" "$BASE_REF"
done

python3 - "$OUT/without/crates/bsl-bytecode/src/image.rs" <<'PY'
import io, sys
p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()
a = s.index('    // НАЧАЛО ОБЩИХ ПРОВЕРОК')
b = s.index('    // КОНЕЦ ОБЩИХ ПРОВЕРОК')
assert a < b, 'якоря общих проверок не найдены или переставлены'
io.open(p, 'w', encoding='utf-8').write(s[:a] + s[b:])
PY

for side in with without; do
    (cd "$OUT/$side" && CARGO_TARGET_DIR="$OUT/target-$side" cargo build --release -q -p bsl-cli)
    cp "$OUT/target-$side/release/bsl-cli" "$OUT/cli-$side"
done

SCENARIOS=(empty_for goto_bench call_overhead cbr_rates simple_parquet_reader)
SCRATCH="${TMPDIR:-/tmp}/onec-bench-scratch"
mkdir -p "$SCRATCH"
printf '  %-24s %14s %14s %8s\n' "сценарий" "без проверок" "с проверками" "Δ%"
for b in "${SCENARIOS[@]}"; do
    f="$ROOT/benchmarks/$b.bsl"
    [ -f "$f" ] || { echo "  $b: сценарий не найден" >&2; continue; }
    (
        cd "$SCRATCH"
        for ((i = 0; i < PAIRS; i++)); do
            perf stat -x, -e instructions:u -- "$OUT/cli-without" "$f" 2>&1 >/dev/null |
                awk -F, '/instructions:u/{print "a", $1}'
            perf stat -x, -e instructions:u -- "$OUT/cli-with" "$f" 2>&1 >/dev/null |
                awk -F, '/instructions:u/{print "b", $1}'
        done
    ) | awk -v n="$b" '{t[$1] += $2; c[$1]++}
        END {a = t["a"] / c["a"]; b = t["b"] / c["b"];
             printf "  %-24s %14.0f %14.0f %+7.3f\n", n, a, b, (b - a) * 100 / a}'
done
echo "  метрика instructions:u; такты на рабочей машине вердикта не выдерживают"
