#!/usr/bin/env bash
# Прогон бенчмарков по всем доступным рантаймам с медианой.
#
#   ./benchmarks/run.sh                # все сценарии, 5 прогонов
#   ./benchmarks/run.sh str_find 9     # один сценарий, 9 прогонов
#
# КОНТРАКТ СЦЕНАРИЯ: скрипт печатает результат (для сверки, что все
# рантаймы посчитали одно и то же) и ПОСЛЕДНЕЙ строкой — прошедшие
# миллисекунды числом. Время меряет сам скрипт, а не эта обёртка: так из
# замера выпадает старт процесса (у bsl-cli это единицы миллисекунд, у
# oscript — десятки на прогрев .NET), и сравниваются интерпретаторы, а не
# способы их запуска.
#
# Медиана, а не среднее: один выброс от планировщика ОС не должен утащить
# число за собой.
#
# `.bsl` скармливается И нашему bsl-cli, И oscript — это один и тот же
# язык. Если oscript в системе нет, строка так и печатается пропуском:
# выдумывать его числа нельзя, как и любые другие неизмеренные.
#
# Колонка 1С заполняется не здесь: платформа поднимается десятки секунд, и
# гонять её в цикле по сценариям бессмысленно. Числа берутся из
# benchmarks/1c/combined.platform.txt — вывода реальной 8.3.27, снятого
# так:
#
#   python3 benchmarks/1c/build-combined.py 3
#   ONEC_TIMEOUT=900 ./tests/conformance/measure/1c/run-on-1c.sh \
#       benchmarks/1c/combined.bsl
#
# Нет файла — в колонке прочерк. Пересняли сценарии — пересними и файл,
# иначе колонка будет мерить старую редакцию бенчмарка.

set -u

cd "$(dirname "$0")/.." || exit 1

RUNS=${2:-5}
ONLY=${1:-}

BSL_CLI=target/release/bsl-cli
if [ ! -x "$BSL_CLI" ]; then
    echo "нет $BSL_CLI — соберите: cargo build --release -p bsl-cli" >&2
    exit 1
fi

# Какой Lua есть. luajit меряется ОТДЕЛЬНО от обычного: это JIT, и
# смешивать его с интерпретаторами в одной колонке — вводить себя в
# заблуждение.
LUA=""
for candidate in lua5.4 lua5.3 lua; do
    if command -v "$candidate" >/dev/null 2>&1; then
        LUA=$candidate
        break
    fi
done
LUAJIT=""
command -v luajit >/dev/null 2>&1 && LUAJIT=luajit

# OneScript ставится куда угодно и в PATH попадает не всегда, поэтому
# кроме PATH смотрим типовые места. Переопределить всё — переменной
# окружения: OSCRIPT=/путь/к/oscript ./benchmarks/run.sh
OSCRIPT=${OSCRIPT:-}
if [ -z "$OSCRIPT" ]; then
    if command -v oscript >/dev/null 2>&1; then
        OSCRIPT=oscript
    else
        for candidate in \
            /opt/oscript/bin/oscript \
            /opt/onescript/bin/oscript \
            /usr/local/oscript/bin/oscript \
            "$HOME/.local/share/oscript/bin/oscript"; do
            if [ -x "$candidate" ]; then
                OSCRIPT=$candidate
                break
            fi
        done
    fi
fi

echo "рантаймы:"
echo "  bsl-cli   $($BSL_CLI --help | head -1)"
[ -n "$LUA" ] && echo "  lua       $($LUA -v 2>&1 | head -1)" || echo "  lua       НЕТ"
[ -n "$LUAJIT" ] && echo "  luajit    $($LUAJIT -v 2>&1 | head -1)" || echo "  luajit    НЕТ"
[ -n "$OSCRIPT" ] && echo "  oscript   $($OSCRIPT -version 2>&1 | head -1)  ($OSCRIPT)" || echo "  oscript   НЕТ (нужен OneScript; путь можно задать в OSCRIPT=...)"
echo

# Медиана последних строк N прогонов. Печатает `-`, если рантайм не смог.
median_ms() {
    local cmd=$1 script=$2
    local values=()
    for _ in $(seq "$RUNS"); do
        local out
        out=$($cmd "$script" 2>/dev/null | tail -1) || return 1
        # Запятая как разделитель дробной части — это русская локаль нашего
        # `Формат`, у Lua всегда точка.
        out=${out//,/.}
        case $out in
            ''|*[!0-9.]*) return 1 ;;
        esac
        values+=("$out")
    done
    printf '%s\n' "${values[@]}" | sort -n | awk -v n="$RUNS" 'NR==int((n+1)/2) { printf "%.0f", $1 }'
}

# Медианы платформы: разбор снятого файла. Сценарий помечен строкой
# `#имя`, его миллисекунды — последнее число перед следующей меткой.
ONEC_FILE=benchmarks/1c/combined.platform.txt
onec_median() {
    [ -f "$ONEC_FILE" ] || { echo "-"; return; }
    awk -v want="$1" '
        # Внутри блока сценария числовых строк бывает несколько (сначала
        # результат вроде 3,14159..., потом миллисекунды) — берём
        # ПОСЛЕДНЮЮ: таков контракт сценария.
        function flush(   ) {
            if (name == want && cand != "") vals[++n] = cand + 0
            cand = ""
        }
        /^#/ { flush(); name = substr($0, 2); next }
        {
            v = $0
            gsub(",", ".", v)
            if (v ~ /^[0-9]+(\.[0-9]+)?$/) cand = v
        }
        END {
            flush()
            if (n == 0) { print "-"; exit }
            # Медиана: сортировка вставками, значений единицы.
            for (i = 2; i <= n; i++) {
                x = vals[i]
                for (j = i - 1; j >= 1 && vals[j] > x; j--) vals[j + 1] = vals[j]
                vals[j + 1] = x
            }
            printf "%.0f", vals[int((n + 1) / 2)]
        }' "$ONEC_FILE"
}

printf '%-14s %10s %10s %10s %10s %10s\n' сценарий bsl-cli lua luajit oscript 1С
printf '%-14s %10s %10s %10s %10s %10s\n' -------------- ---------- ---------- ---------- ---------- ----------

for bsl in benchmarks/*.bsl; do
    name=$(basename "$bsl" .bsl)
    [ -n "$ONLY" ] && [ "$ONLY" != "$name" ] && continue
    # csv_write* пишут на диск сотни мегабайт и меряют файловый ввод-вывод,
    # а не интерпретатор — в общий прогон не входят, запускаются руками.
    case $name in csv_write*) continue ;; esac

    ours=$(median_ms "$BSL_CLI" "$bsl") || ours="ошибка"
    lua_ms="-"
    luajit_ms="-"
    os_ms="-"
    if [ -f "benchmarks/$name.lua" ]; then
        [ -n "$LUA" ] && { lua_ms=$(median_ms "$LUA" "benchmarks/$name.lua") || lua_ms="ошибка"; }
        [ -n "$LUAJIT" ] && { luajit_ms=$(median_ms "$LUAJIT" "benchmarks/$name.lua") || luajit_ms="ошибка"; }
    fi
    [ -n "$OSCRIPT" ] && { os_ms=$(median_ms "$OSCRIPT" "$bsl") || os_ms="ошибка"; }

    onec_ms=$(onec_median "$name")
    printf '%-14s %10s %10s %10s %10s %10s\n' "$name" "$ours" "$lua_ms" "$luajit_ms" "$os_ms" "$onec_ms"
done

echo
echo "медиана $RUNS прогонов, миллисекунды. Прочерк — двойника на этом"
echo "языке нет либо самого рантайма нет в системе."
if [ -f "$ONEC_FILE" ]; then
    echo "колонка 1С — медиана из $ONEC_FILE (снято отдельным прогоном"
    echo "на платформе, см. шапку этого файла)."
fi
