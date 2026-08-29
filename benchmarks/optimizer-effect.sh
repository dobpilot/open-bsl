#!/usr/bin/env bash
# Замер эффекта оптимизирующих проходов компилятора.
#
# Существует потому, что описанный прозой прогон замером не является: числа
# в docs/ssa-hotspot-analysis.md обязаны пересниматься одной командой, иначе
# проверить их нельзя. Каждый раздел печатает ровно то, на что документ
# ссылается, и ничего сверх.
#
#   ./benchmarks/optimizer-effect.sh                  все разделы
#   ./benchmarks/optimizer-effect.sh static budget    выборочно
#
# Разделы: static (снятые инструкции), residual (остаток позднего прохода),
# coldness (сколько раз этот остаток исполняется), dynamic (исполненные
# инструкции), budget (цена компиляции), complexity (рост этой цены по
# глубине выражения).
set -euo pipefail

cd "$(dirname "$0")/.."
CLI=target/release/bsl-cli
COST=target/release/examples/compile-cost
SCRATCH="${TMPDIR:-/tmp}/open-bsl-optimizer-effect"
mkdir -p "$SCRATCH"

# Измерительная выборка: бенчмарки плюс фикстуры конформанса. Это НЕ выборка
# дифференциального теста, у той другой состав (фикстуры плюс measure) и
# другой размер — см. документ.
sample() { ls benchmarks/*.bsl tests/conformance/fixtures/*.bsl; }

build_cli() { cargo build --release -q -p bsl-cli "$@"; }
build_cost() { cargo build --release -q -p open-bsl --example compile-cost; }

# Полный листинг. Ошибка компиляции — это отказ, а не ноль инструкций:
# именно так первая редакция замера намерила стоимость падающей компиляции.
emit_full() {
    local flags="$1" script="$2" out
    if ! out=$($CLI $flags --emit-bytecode "$script" 2>&1); then
        printf 'ОШИБКА компиляции: %s (%s)\n%s\n' "$script" "${flags:-без ключей}" "$out" >&2
        return 1
    fi
    printf '%s\n' "$out"
}

# Только строки инструкций — для счёта и для показа различий.
emit() { emit_full "$1" "$2" | grep -P '^\s+[0-9]{4} ' || true; }
count() { emit "$1" "$2" | wc -l; }

section_static() {
    echo "== static: снятые инструкции, база -> --optimize=const-fold =="
    local base fold total_b=0 total_f=0 n=0
    while read -r f; do
        base=$(count "" "$f"); fold=$(count "--optimize=const-fold" "$f")
        n=$((n + 1)); total_b=$((total_b + base)); total_f=$((total_f + fold))
        if [ "$base" -ne "$fold" ]; then
            printf '  %-26s %6d -> %6d  (-%d)\n' "$(basename "$f" .bsl)" "$base" "$fold" "$((base - fold))"
        fi
    done < <(sample)
    awk -v n="$n" -v a="$total_b" -v b="$total_f" \
        'BEGIN{printf "скриптов %d, инструкций %d -> %d, снято %d (%.2f%%)\n", n, a, b, a-b, (a-b)*100/a}'
}

section_residual() {
    echo "== residual: что поздний проход меняет ПОВЕРХ ранней свёртки =="
    # Поздний проход инструкции не удаляет, а заменяет, поэтому считается
    # различие листингов, а не разность длин.
    local d total=0
    while read -r f; do
        d=$(diff <(emit "--optimize=const-fold" "$f") \
                 <(emit "--optimize=const-fold,const-prop" "$f") | grep -c '^<' || true)
        if [ "$d" -gt 0 ]; then
            printf '  %-26s %d\n' "$(basename "$f" .bsl)" "$d"
            total=$((total + d))
        fi
    done < <(sample)
    echo "итого заменённых инструкций: $total"
}

# Холодность остатка. «В начале чанка» не означает «однажды» — пролог
# функции исполняется на каждый вызов, — поэтому число исполнений берётся
# из гистограммы опкодов, а не из расположения инструкции. Общий счётчик
# здесь слеп: поздний проход инструкцию заменяет, а не удаляет.
section_coldness() {
    echo "== coldness: сколько раз исполняется то, что заменил поздний проход =="
    build_cli --features counters
    local before after
    printf '  %-24s %8s %10s %10s\n' "сценарий" "опкод" "fold" "+const-prop"
    for f in "$@"; do
        for op in Mul Div; do
            before=$($CLI --optimize=const-fold "$f" 2>&1 >/dev/null |
                awk -F'\t' -v o="$op" '$1==o {print $2; exit}')
            after=$($CLI --optimize=const-fold,const-prop "$f" 2>&1 >/dev/null |
                awk -F'\t' -v o="$op" '$1==o {print $2; exit}')
            before=${before:-0}; after=${after:-0}
            if [ "$before" != "$after" ]; then
                printf '  %-24s %8s %10d %10d\n' "$(basename "$f" .bsl)" "$op" "$before" "$after"
            fi
        done
    done
    echo "  n-body-pow-variant не измеряется: не завершается по построению"
    build_cli
}

section_dynamic() {
    echo "== dynamic: исполненные инструкции =="
    build_cli --features counters
    local base fold
    printf '  %-24s %14s %14s %8s\n' "сценарий" "база" "const-fold" "Δ шт"
    for f in "$@"; do
        base=$($CLI "$f" 2>&1 >/dev/null | grep -oP '^# всего инструкций\t\K[0-9]+')
        fold=$($CLI --optimize=const-fold "$f" 2>&1 >/dev/null | grep -oP '^# всего инструкций\t\K[0-9]+')
        printf '  %-24s %14d %14d %8d\n' "$(basename "$f" .bsl)" "$base" "$fold" "$((base - fold))"
    done
    build_cli
}

# Пара A/B снимается ЧЕРЕДУЯСЬ: сначала база, сразу за ней кандидат, и так
# нужное число пар. Все базовые прогоны подряд, а потом все кандидатские —
# это не A/B: между блоками машина успевает уйти по частоте и по тепловому
# режиму, и разница блоков смешивается с разницей конфигураций.
#
# Меряется КОМПИЛЯЦИЯ, без печати листинга: `--emit-bytecode` форматирует
# весь листинг даже в `/dev/null`, и на фикстуре это дороже самой
# компиляции, так что доля свёртки в такой доле занижена.
ab_compile() {
    local script="$1" label="$2" iters="${3:-50}" pairs="${4:-7}" i
    for ((i = 0; i < pairs; i++)); do
        perf stat -x, -e instructions:u -- $COST "$script" "$iters" 2>&1 |
            awk -F, '/instructions:u/{print "base", $1}'
        perf stat -x, -e instructions:u -- $COST "$script" "$iters" const-fold 2>&1 |
            awk -F, '/instructions:u/{print "fold", $1}'
    done | awk -v l="$label" '{c[$1]++; t[$1]+=$2}
        END {a=t["base"]/c["base"]; b=t["fold"]/c["fold"];
             printf "  %-30s база %14.0f  свёртка %14.0f  Δ %+.2f%%\n", l, a, b, (b-a)*100/a}'
}

section_budget() {
    echo "== budget: цена компиляции (без форматирования листинга) =="
    build_cost
    python3 -c "open('$SCRATCH/wide.bsl','w').write('А = 1;\n' + 'Б = А + 1 + 1 + 1 + 1;\n'*4000)"
    ab_compile "$SCRATCH/wide.bsl" "синтетика 4000 операторов" 5
    ab_compile benchmarks/csv_write.bsl "csv_write" 200
    ab_compile tests/conformance/fixtures/binary-streams.bsl "binary-streams (самая большая)" 50

    echo "  -- сквозное исполнение: свёртка не должна утяжелять запуск --"
    local i
    for ((i = 0; i < 7; i++)); do
        perf stat -x, -e instructions:u,cycles:u -- $CLI benchmarks/empty_for.bsl 2>&1 >/dev/null |
            awk -F, '/instructions:u/{n=$1} /cycles:u/{c=$1} END{print "base", n, c}'
        perf stat -x, -e instructions:u,cycles:u -- $CLI --optimize=const-fold benchmarks/empty_for.bsl 2>&1 >/dev/null |
            awk -F, '/instructions:u/{n=$1} /cycles:u/{c=$1} END{print "fold", n, c}'
    done | awk '{k[$1]++; a[$1]+=$2; b[$1]+=$3}
        END {printf "  %-30s инструкции %+.2f%%, такты %+.2f%% (частота НЕ зафиксирована)\n",
             "empty_for", (a["fold"]/k["fold"]-a["base"]/k["base"])*100/(a["base"]/k["base"]),
             (b["fold"]/k["fold"]-b["base"]/k["base"])*100/(b["base"]/k["base"])}'
}

# Рост цены свёртки по глубине. Цепочка унарных минусов, а не сложений:
# у неё временные регистры не выделяются, поэтому лимит кадра её не
# ограничивает — именно на ней обход был квадратичным.
section_complexity() {
    echo "== complexity: накладные свёртки на цепочке унарных минусов =="
    build_cost
    local prev=0 base fold over n
    printf '  %8s %14s %14s %12s %8s\n' "глубина" "база" "const-fold" "накладные" "рост"
    for n in 400 800 1600 3200; do
        python3 -c "open('$SCRATCH/u.bsl','w').write('А = 1;\nБ = ' + '- '*$n + 'А;\n')"
        read -r base fold < <(
            for ((i = 0; i < 5; i++)); do
                perf stat -x, -e instructions:u -- $COST "$SCRATCH/u.bsl" 50 2>&1 |
                    awk -F, '/instructions:u/{print "base", $1}'
                perf stat -x, -e instructions:u -- $COST "$SCRATCH/u.bsl" 50 const-fold 2>&1 |
                    awk -F, '/instructions:u/{print "fold", $1}'
            done | awk '{c[$1]++; t[$1]+=$2} END {print t["base"]/c["base"], t["fold"]/c["fold"]}'
        )
        over=$(awk -v a="$base" -v b="$fold" 'BEGIN{printf "%.0f", b-a}')
        awk -v n="$n" -v a="$base" -v b="$fold" -v o="$over" -v p="$prev" \
            'BEGIN{printf "  %8d %14.0f %14.0f %12d %8s\n", n, a, b, o,
                   (p > 0 ? sprintf("%.2fx", o/p) : "—")}'
        prev=$over
    done
    echo "  линейному обходу отвечает рост около 2x на удвоение, квадратичному — около 4x"
}

RESIDUAL_SCRIPTS=(benchmarks/table_compare2.bsl benchmarks/table_save_load.bsl
    tests/conformance/fixtures/n-body-smoke.bsl tests/conformance/fixtures/n-body-precision.bsl
    tests/conformance/fixtures/n-body-perf.bsl)
DYNAMIC_SCRIPTS=(benchmarks/pi_leibniz.bsl benchmarks/pi_leibniz_15.bsl
    benchmarks/simple_parquet_reader.bsl benchmarks/csv_write.bsl
    tests/conformance/fixtures/n-body-smoke.bsl tests/conformance/fixtures/coercion.bsl
    tests/conformance/fixtures/arithmetic.bsl)

# Без аргументов — все разделы. Через `set --`, а не через "${@:-...}":
# последнее склеивает список в ОДИН аргумент, и скрипт отказывал на
# «неизвестном разделе», чьё имя было целой строкой.
if (($# == 0)); then
    set -- static residual coldness dynamic budget complexity
fi

build_cli
for section in "$@"; do
    case "$section" in
        static) section_static ;;
        residual) section_residual ;;
        coldness) section_coldness "${RESIDUAL_SCRIPTS[@]}" ;;
        dynamic) section_dynamic "${DYNAMIC_SCRIPTS[@]}" ;;
        budget) section_budget ;;
        complexity) section_complexity ;;
        *) echo "неизвестный раздел «$section»" >&2; exit 2 ;;
    esac
done
