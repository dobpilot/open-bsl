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

# Сценарии берутся ИЗ ТОГО ЖЕ worktree и оттуда же запускаются. Причин
# две. Первая: замер обязан описывать выбранный коммит, а не то, что
# оказалось в рабочем дереве. Вторая обнаружилась дороже — часть
# сценариев читает данные по ОТНОСИТЕЛЬНОМУ пути от корня дерева, и,
# запущенный из чужого каталога, `simple_parquet_reader` печатал «не
# открылся» и завершался успешно. Мерился при этом ранний выход, а не
# сценарий.
BENCH="$OUT/with/benchmarks"

# Каждый сценарий сперва прогоняется НАСУХО и его вывод проверяется на
# признак раннего выхода. Молча измерить не то — главный способ получить
# красивое число ни о чём.
check_runs() {
    local out
    out=$( cd "$OUT/with" && "$OUT/cli-with" "benchmarks/$1.bsl" 2>&1 ) || {
        echo "  $1: прогон завершился ошибкой" >&2
        return 1
    }
    if printf '%s' "$out" | grep -qiE "не открыл|не найден|гоняется из корня"; then
        echo "  $1: сценарий не выполнился — $(printf '%s' "$out" | head -1)" >&2
        return 1
    fi
}

# Парные дельты, а не два средних. На этих сценариях прогонный разброс
# сопоставим с самим эффектом, и одно среднее читалось бы точнее, чем
# оно есть. Печатается медиана пар и их диапазон.
#
# Порядок внутри пары ЧЕРЕДУЕТСЯ (AB, BA, AB, ...): при одном и том же
# порядке систематическая разница «первый запуск дороже второго» целиком
# легла бы на одну сторону.
printf '  %-24s %10s %10s %10s   %s\n' "сценарий" "медиана" "минимум" "максимум" "пар"
status=0
for b in "${SCENARIOS[@]}"; do
    [ -f "$BENCH/$b.bsl" ] || { echo "  $b: сценарий не найден" >&2; status=1; continue; }
    check_runs "$b" || { status=1; continue; }
    (
        cd "$OUT/with"
        for ((i = 0; i < PAIRS; i++)); do
            one() {
                perf stat -x, -e instructions:u -- "$OUT/cli-$1" "benchmarks/$b.bsl" 2>&1 >/dev/null |
                    awk -F, '/instructions:u/{print $1}'
            }
            if ((i % 2 == 0)); then
                a=$(one without); c=$(one with)
            else
                c=$(one with); a=$(one without)
            fi
            echo "$a $c"
        done
    ) | awk -v n="$b" '
        {d[NR] = ($2 - $1) * 100 / $1}
        END {
            asort(d)
            m = (NR % 2) ? d[(NR + 1) / 2] : (d[NR / 2] + d[NR / 2 + 1]) / 2
            printf "  %-24s %+9.3f%% %+9.3f%% %+9.3f%%   %d\n", n, m, d[1], d[NR], NR
        }'
done
echo "  метрика instructions:u; такты на рабочей машине вердикта не выдерживают"
echo "  разброс печатается рядом со сдвигом: где диапазон перекрывает медиану,"
echo "  утверждать можно лишь порядок величины"
exit "$status"
