#!/usr/bin/env bash
# Замер эффекта оптимизирующих проходов компилятора.
#
# Существует потому, что описанный прозой прогон замером не является: числа
# в docs/ssa-hotspot-analysis.md обязаны пересниматься одной командой, иначе
# проверить их нельзя. Каждый раздел печатает ровно то, на что документ
# ссылается, и ничего сверх.
#
#   ./benchmarks/optimizer-effect.sh                  все разделы, кроме ворот
#   ./benchmarks/optimizer-effect.sh static budget    выборочно
#   ./benchmarks/optimizer-effect.sh gate             ворота устранения копий
#
# Разделы: static (снятые инструкции), residual (остаток позднего прохода),
# coldness (сколько раз этот остаток исполняется), dynamic (исполненные
# инструкции), budget (цена компиляции), complexity (рост этой цены по
# глубине выражения), gate (ворота допуска устранения копий; в набор по
# умолчанию не входит — он длинный и требует зафиксированной частоты),
# window (чем питаются копии в окно аргументов вызова).
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
    local script="$1" label="$2" pass="${3:-const-fold}" iters="${4:-50}" pairs="${5:-7}" i
    for ((i = 0; i < pairs; i++)); do
        perf stat -x, -e instructions:u -- $COST "$script" "$iters" 2>&1 |
            awk -F, '/instructions:u/{print "base", $1}'
        perf stat -x, -e instructions:u -- $COST "$script" "$iters" "$pass" 2>&1 |
            awk -F, '/instructions:u/{print "cand", $1}'
    done | awk -v l="$label" -v p="$pass" '{c[$1]++; t[$1]+=$2}
        END {a=t["base"]/c["base"]; b=t["cand"]/c["cand"];
             printf "  %-14s %-30s база %14.0f  проход %14.0f  Δ %+.2f%%\n", p, l, a, b, (b-a)*100/a}'
}

section_budget() {
    echo "== budget: цена компиляции (без форматирования листинга) =="
    echo "  $(freq_state)"
    build_cost
    python3 -c "open('$SCRATCH/wide.bsl','w').write('А = 1;\n' + 'Б = А + 1 + 1 + 1 + 1;\n'*4000)"
    local pass
    for pass in const-fold copy-elim; do
        ab_compile "$SCRATCH/wide.bsl" "синтетика 4000 операторов" "$pass" 5
        ab_compile benchmarks/csv_write.bsl "csv_write" "$pass" 200
        ab_compile benchmarks/table_total.bsl "table_total" "$pass" 200
        ab_compile tests/conformance/fixtures/binary-streams.bsl "binary-streams (самая большая)" "$pass" 50
    done

    echo "  -- сквозное исполнение: свёртка не должна утяжелять запуск --"
    local i
    for ((i = 0; i < 7; i++)); do
        perf stat -x, -e instructions:u,cycles:u -- $CLI benchmarks/empty_for.bsl 2>&1 >/dev/null |
            awk -F, '/instructions:u/{n=$1} /cycles:u/{c=$1} END{print "base", n, c}'
        perf stat -x, -e instructions:u,cycles:u -- $CLI --optimize=const-fold benchmarks/empty_for.bsl 2>&1 >/dev/null |
            awk -F, '/instructions:u/{n=$1} /cycles:u/{c=$1} END{print "fold", n, c}'
    done | awk '{k[$1]++; a[$1]+=$2; b[$1]+=$3}
        END {printf "  %-30s инструкции %+.2f%%, такты %+.2f%%\n",
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

# Состояние частоты — часть отчёта, а не примечание к нему. Прогон без
# фиксации помечается явно: правило документа прямо запрещает молчать об
# этом, потому что стенное время под `powersave` меряет governor.
freq_state() {
    local turbo lo hi govs n_perf n_all
    turbo=$(cat /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || echo "?")
    lo=$(cat /sys/devices/system/cpu/intel_pstate/min_perf_pct 2>/dev/null || echo "?")
    hi=$(cat /sys/devices/system/cpu/intel_pstate/max_perf_pct 2>/dev/null || echo "?")
    # Governor смотрится у ВСЕХ ядер, а не у нулевого: ядро, оставшееся под
    # `powersave`, сделает замер на нём недействительным, а по cpu0 этого не
    # видно.
    govs=$(cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor 2>/dev/null || true)
    n_all=$(printf '%s\n' "$govs" | grep -c . || true)
    n_perf=$(printf '%s\n' "$govs" | grep -c '^performance$' || true)
    if [ "$n_all" -gt 0 ] && [ "$n_perf" = "$n_all" ] && [ "$turbo" = 1 ] && [ "$lo" = "$hi" ]; then
        echo "частота ЗАФИКСИРОВАНА (performance на всех $n_all, турбо выкл., perf_pct $lo)"
    else
        echo "частота НЕ зафиксирована (performance на $n_perf из $n_all, no_turbo $turbo, perf_pct $lo-$hi)"
    fi
}

# Фактические частоты — утверждение о фиксации проверяется измерением, а не
# только чтением настроек: троттлинг настройки не спрашивает.
cpu_speeds() {
    awk '/^cpu MHz/ {v=$4; s+=v; n++; if (v<lo || n==1) lo=v; if (v>hi) hi=v}
         END {printf "по факту %d ядер, %.0f-%.0f МГц (среднее %.0f)", n, lo, hi, s/n}' /proc/cpuinfo
}

# Один прогон сценария под perf. Рабочий каталог отдельный: тяжёлые
# сценарии пишут файлы рядом с собой, и мерить запись в репозиторий незачем.
run_once() {
    local script="$1" flags="$2" workdir="$SCRATCH/run" root
    root=$(pwd)
    mkdir -p "$workdir"
    ( cd "$workdir" && perf stat -x, -e cycles:u,instructions:u -- \
        "$root/$CLI" $flags "$root/$script" 2>&1 >/dev/null ) |
        awk -F, '/cycles:u/{c=$1} /instructions:u/{i=$1} END{print c, i}'
}

# Ворота допуска: обе метрики, чередующиеся тройки «база, кандидат, база».
#
# Разброс считается ПО ТРОЙКАМ, а не как разность двух общих средних.
# Прежняя формула `|mean(A) - mean(C)|` разбросом не была: отклонения
# разных знаков в ней гасят друг друга, и пятнадцать троек `table_total`
# с внутритроечным дрейфом 0,42…3,70 % давали в ней 0,27 %. Здесь берётся
# средний модуль внутритроечной разницы — величина, которая от взаимного
# гашения не страдает.
#
# Вердикт выносится по ОБЕИМ метрикам: регрессия или недобор порога в
# любой из них блокирует. Порог — параметр, по умолчанию пять процентов,
# как назначено целям первой итерации.
gate_one() {
    local script="$1" mode="$2" rounds="${3:-7}" thresh="${4:-5}"
    local base_flags="" cand_flags="--optimize=copy-elim"
    if [ "$mode" = jit ]; then
        base_flags="--jit"; cand_flags="--jit --optimize=copy-elim"
    fi
    local i
    for ((i = 0; i < rounds; i++)); do
        echo "r $(run_once "$script" "$base_flags") $(run_once "$script" "$cand_flags") $(run_once "$script" "$base_flags")"
    done | awk -v name="$(basename "$script" .bsl)" -v mode="$mode" -v thresh="$thresh" -v rounds="$rounds" '
        # Поля тройки: $2,$3 — база A (такты, инстр), $4,$5 — кандидат,
        # $6,$7 — база C.
        {
            n++;
            b = ($2 + $6) / 2; ib = ($3 + $7) / 2;
            spread += ($2 > $6 ? $2 - $6 : $6 - $2) / b;
            eff += ($4 - b) / b; ieff += ($5 - ib) / ib;
        }
        END {
            dcyc = eff * 100 / n; dins = ieff * 100 / n; noise = spread * 100 / n;
            adins = (dins < 0 ? -dins : dins);
            # Порядок разбора существен. Неизменное число инструкций
            # означает, что проход этого кода не касался ВООБЩЕ, и тогда
            # разница тактов принадлежит машине, а не правке, — сколь бы
            # велика она ни была. Ровно этот случай канарейка `goto_bench`
            # однажды предъявила как +12,16 % стенного времени при нуле
            # инструкций.
            if (adins < 0.05) verdict = "КОД НЕ ЗАТРОНУТ";
            else if (dcyc > 0 || dins > 0) verdict = "РЕГРЕССИЯ";
            else if (-dcyc < 2 * noise) verdict = "НЕРАЗРЕШИМО";
            else if (-dcyc < thresh || -dins < thresh) verdict = "НИЖЕ ПОРОГА";
            else verdict = "выигрыш";
            printf "  %-20s %-6s такты %+7.2f%%  инстр %+7.2f%%  разброс %5.2f%%  n=%-2d %s\n",
                   name, mode, dcyc, dins, noise, rounds, verdict
        }'
}

section_gate() {
    echo "== gate: ворота допуска прохода устранения копий =="
    echo "  $(freq_state)"
    echo "  порог ${GATE_THRESHOLD:-5} %, раундов ${GATE_ROUNDS:-7} (GATE_ROUNDS=15 для спорных)"
    echo "  сверх порога эффект обязан вдвое превышать разброс той же тройки"
    echo "  $(printf '%s' "$(cpu_speeds)")"
    local group list f
    for group in цели канарейки; do
        echo "  -- $group --"
        if [ "$group" = цели ]; then list=("${GATE_TARGETS[@]}"); else list=("${GATE_CANARIES[@]}"); fi
        for f in "${list[@]}"; do
            gate_one "$f" interp "$GATE_ROUNDS" "$GATE_THRESHOLD"
            gate_one "$f" jit "$GATE_ROUNDS" "$GATE_THRESHOLD"
        done
    done
}

GATE_ROUNDS=${GATE_ROUNDS:-7}
GATE_THRESHOLD=${GATE_THRESHOLD:-5}

GATE_TARGETS=(benchmarks/csv_write.bsl benchmarks/table_total.bsl
    benchmarks/bmp_rotate.bsl benchmarks/csv_write_batched.bsl)
GATE_CANARIES=(benchmarks/goto_bench.bsl benchmarks/call_overhead.bsl
    benchmarks/empty_for.bsl benchmarks/pi_leibniz.bsl)

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
    set -- static residual coldness dynamic budget complexity window
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
        gate) section_gate ;;
        window) section_window ;;
        *) echo "неизвестный раздел «$section»" >&2; exit 2 ;;
    esac
done
