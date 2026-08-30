#!/bin/bash
# Сравнение МАШИННОГО КОДА диспетчерского цикла VM между текущим деревом
# и базовым коммитом.
#
# Зачем это вместо чередующегося A/B. Горячий цикл `bsl-vm` живёт на
# грани uop-кэша процессора, и в этом проекте уже случалось, что сдвиг
# раскладки менял сценарий на десятки процентов при НЕИЗМЕННОМ числе
# инструкций (см. комментарии у `step_cold`). Поэтому счётчика
# инструкций мало, а такты требуют тихой машины: на загруженной разброс
# базовых прогонов доходил до 102 %, и вердикт по ним не выносится.
#
# Сравнение кода от загрузки не зависит вовсе и отвечает на тот самый
# вопрос: изменилось ли то, что исполняет процессор. Если размер и
# последовательность инструкций `step`/`step_cold` совпали, изменение в
# горячем пути бесплатно; расхождение — повод мерить такты на тихой
# машине.
#
#   ./benchmarks/hot-code-diff.sh [базовый-коммит]
#
# По умолчанию база — HEAD, то есть сравнивается незакоммиченное дерево с
# последним коммитом.
set -euo pipefail

BASE="${1:-HEAD}"
ROOT="$(git rev-parse --show-toplevel)"
OUT="${TMPDIR:-/tmp}/open-bsl-hot-code"
mkdir -p "$OUT"
cd "$ROOT"

build() {
    cargo build --release -q -p bsl-cli
    cp target/release/bsl-cli "$OUT/cli-$1"
}

# `.cargo/config.toml` задаёт `-align-all-functions=5` — это гигиена
# измерения, и обе сборки обязаны идти с ней. Поэтому `RUSTFLAGS` здесь
# не выставляется: он бы её перекрыл, и сравнивались бы две разные
# раскладки.

# Функции сравниваются по РАЗМЕРУ и по последовательности мнемоник.
# Байтовое сравнение не годится: функция встаёт по другому адресу, и
# RIP-относительные смещения к строковым литералам отличаются, ничего не
# говоря о самом коде.
dump() {
    local bin="$1" fn="$2" out="$3"
    local sym
    sym=$(nm "$bin" | grep -oE "_RNvCs[A-Za-z0-9_]*_6bsl_vm${fn}\$" | head -1)
    if [ -z "$sym" ]; then
        echo "  символ ${fn} не найден в $(basename "$bin")" >&2
        return 1
    fi
    objdump -t "$bin" | grep -E "$sym\$" | awk '{print $5}' > "$out.size"
    objdump -d --disassemble="$sym" "$bin" |
        awk -F'\t' 'NF>=3{print $3}' | awk '{print $1}' > "$out.mnem"
}

echo "== сравнение горячего кода: $BASE против рабочего дерева =="
build candidate

# База собирается в ОТДЕЛЬНОМ worktree, а не переключением текущего.
# Причина не стилистическая: `git stash` с `checkout` тронул бы общее
# рабочее дерево, а в нём может идти чужая работа — и, оборвись скрипт
# посередине, вернуть его было бы некому. Worktree не трогает ничего.
WT="$OUT/base-worktree"
rm -rf "$WT"
git worktree remove --force "$WT" 2>/dev/null || true
git worktree add -q --detach "$WT" "$BASE"
trap 'git worktree remove --force "$WT" 2>/dev/null || true' EXIT
(
    cd "$WT"
    CARGO_TARGET_DIR="$OUT/base-target" cargo build --release -q -p bsl-cli
    cp "$OUT/base-target/release/bsl-cli" "$OUT/cli-base"
)

status=0
for fn in 4step 9step_cold; do
    dump "$OUT/cli-base" "$fn" "$OUT/base-$fn" || continue
    dump "$OUT/cli-candidate" "$fn" "$OUT/cand-$fn" || continue
    size_a=$(cat "$OUT/base-$fn.size")
    size_b=$(cat "$OUT/cand-$fn.size")
    n=$(wc -l < "$OUT/base-$fn.mnem")
    if [ "$size_a" = "$size_b" ] && cmp -s "$OUT/base-$fn.mnem" "$OUT/cand-$fn.mnem"; then
        printf '  %-12s размер 0x%s, %s инструкций — СОВПАДАЕТ\n' "$fn" "$size_a" "$n"
    else
        printf '  %-12s размер 0x%s -> 0x%s, расхождений мнемоник %s — ИЗМЕНИЛСЯ\n' \
            "$fn" "$size_a" "$size_b" "$(diff "$OUT/base-$fn.mnem" "$OUT/cand-$fn.mnem" | grep -c '^<' || true)"
        status=1
    fi
done
[ "$status" = 0 ] && echo "  горячий путь не изменился — такты мерить незачем" ||
    echo "  горячий путь изменился — нужен чередующийся A/B на тихой машине"
exit "$status"
