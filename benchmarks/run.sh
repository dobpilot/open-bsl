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
#   python3 benchmarks/1c/build-combined.py 1
#   ONEC_TIMEOUT=1800 ./tests/conformance/measure/1c/run-on-1c.sh \
#       benchmarks/1c/combined.bsl
#
# Повтор ОДИН, а не три. Платформа тратит на invoice_doc_generator около
# 760 с (из них ~700 — сохранение в PDF) и 250 с на textdoc_invoice; при
# трёх повторах 26 сценариев не укладываются и в час, и прогон снимается
# таймаутом, не записав результата. Дрожание платформы на фоне таких
# величин пренебрежимо, усреднять там нечего.
#
# Нет файла — в колонке прочерк. Пересняли сценарии — пересними и файл,
# иначе колонка будет мерить старую редакцию бенчмарка.

set -u

cd "$(dirname "$0")/.." || exit 1

RUNS=${2:-5}
ONLY=${1:-}
ROOT=$(pwd)

# Тяжёлые сценарии меряются меньшим числом прогонов: один проход csv_write
# кладёт 15 МБ, а table_compare работает с миллионами строк таблиц значений.
# Переопределяется: HEAVY_RUNS=5 ...
HEAVY_RUNS=${HEAVY_RUNS:-3}
HEAVY_SEEN=""

# CSV-сценарии пишут в отдельный каталог, а не в дерево проекта: файл
# "test.csv" они открывают относительно текущего каталога.
SCRATCH=${TMPDIR:-/tmp}/onec-bench-scratch

is_heavy() {
    case $1 in csv_write*|table_compare|table_compare2|table_save_load) return 0 ;; *) return 1 ;; esac
}

# Абсолютный: сценарии с файловым выводом гоняются из другого каталога.
BSL_CLI=$ROOT/target/release/bsl-cli
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

# Python-двойники используют только стандартную библиотеку. Путь можно
# переопределить так же, как для OneScript: PYTHON=/путь/python3 ...
PYTHON=${PYTHON:-}
if [ -z "$PYTHON" ]; then
    for candidate in python3 python; do
        if command -v "$candidate" >/dev/null 2>&1 \
            && "$candidate" -c 'import sys; raise SystemExit(sys.version_info.major != 3)' 2>/dev/null; then
            PYTHON=$candidate
            break
        fi
    done
fi

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
[ -n "$PYTHON" ] && echo "  python    $($PYTHON --version 2>&1 | head -1)" || echo "  python    НЕТ"
[ -n "$OSCRIPT" ] && echo "  oscript   $($OSCRIPT -version 2>&1 | head -1)  ($OSCRIPT)" || echo "  oscript   НЕТ (нужен OneScript; путь можно задать в OSCRIPT=...)"
echo

# Медиана последних строк N прогонов. Печатает `-`, если рантайм не смог.
# Третий аргумент — рабочий каталог: сценарии с файловым выводом гоняются
# не в дереве проекта. Путь к скрипту поэтому абсолютный.
median_ms() {
    local cmd=$1 script=$2 workdir=${3:-.} runs=$RUNS
    is_heavy "$(basename "$script")" && runs=$HEAVY_RUNS
    local script_path=$script
    case $script_path in
        /*) ;;
        *) script_path=$ROOT/$script_path ;;
    esac
    local values=()
    for _ in $(seq "$runs"); do
        local out
        out=$(cd "$workdir" && $cmd "$script_path" 2>/dev/null | tail -1) || return 1
        # Запятая как разделитель дробной части — это русская локаль нашего
        # `Формат`, у Lua всегда точка.
        out=${out//,/.}
        case $out in
            ''|*[!0-9.]*) return 1 ;;
        esac
        values+=("$out")
    done
    printf '%s\n' "${values[@]}" | sort -n | awk -v n="$runs" 'NR==int((n+1)/2) { printf "%.0f", $1 }'
}

# Строка `// @prepend-bsl путь` подключает перед сценарием общий модуль.
# Это позволяет мерить неизменённый сторонний исходник, не размножая его
# многотысячную копию в `benchmarks/`. Результат живёт только в scratch.
materialize_bsl() {
    local script=$1 prepend source dest
    prepend=$(sed -n 's|^// @prepend-bsl ||p' "$script")
    if [ -z "$prepend" ]; then
        printf '%s\n' "$script"
        return 0
    fi
    case $prepend in
        /*|*..*) return 1 ;;
    esac
    source=$ROOT/$prepend
    [ -f "$source" ] || return 1
    mkdir -p "$SCRATCH"
    dest=$SCRATCH/$(basename "$script")
    # Исходный модуль Connector хранится с BOM; внутри составного файла
    # BOM допустим только в самом начале и потому снимается здесь.
    sed $'1s/^\xEF\xBB\xBF//' "$source" > "$dest" || return 1
    printf '\n' >> "$dest"
    sed '/^\/\/ @prepend-bsl /d' "$script" >> "$dest" || return 1
    printf '%s\n' "$dest"
}

# Файл, оставленный сценарием, откладывается под именем рантайма — в конце
# они сверяются между собой. Для файлового сценария это и есть проверка,
# что все посчитали одно и то же: печатать ему, кроме миллисекунд, нечего.
keep_output() {
    local name=$1 runtime=$2
    [ -f "$SCRATCH/test.csv" ] || return 0
    mv "$SCRATCH/test.csv" "$SCRATCH/$name.$runtime.out"
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

printf '%-18s %10s %10s %10s %10s %10s %10s %10s\n' сценарий bsl-cli bsl-cli+jit lua luajit python oscript 1С
printf '%-18s %10s %10s %10s %10s %10s %10s %10s\n' ------------------ ---------- ---------- ---------- ---------- ---------- ---------- ----------

for bsl in benchmarks/*.bsl; do
    name=$(basename "$bsl" .bsl)
    [ -n "$ONLY" ] && [ "$ONLY" != "$name" ] && continue
    scenario_bsl=$(materialize_bsl "$bsl") || {
        echo "$name: не удалось подключить общий BSL-модуль" >&2
        continue
    }
    # Тяжёлые сценарии в общий прогон входят, но повторяются меньше раз.
    # Для csv_write* дополнительно нужен рабочий каталог вне дерева проекта.
    workdir=$ROOT
    if is_heavy "$name"; then
        mkdir -p "$SCRATCH"
        # Только СВОИ файлы: `$name.1c.out` кладёт отдельный прогон на
        # платформе, и стирать его здесь значило бы каждый раз терять
        # эталон, с которым сличаемся.
        for rt in bsl-cli bsl-cli-jit lua luajit python oscript; do
            rm -f "$SCRATCH/$name.$rt.out"
        done
        rm -f "$SCRATCH/test.csv"
        case $name in csv_write*) workdir=$SCRATCH ;; esac
        HEAVY_SEEN=yes
    fi

    ours=$(median_ms "$BSL_CLI" "$scenario_bsl" "$workdir") || ours="ошибка"
    is_heavy "$name" && keep_output "$name" bsl-cli
    # Тот же бинарник с ключом --jit: компиляция байт-кода в машинный код.
    # Отдельной колонкой, а не заменой: JIT включается только ключом, и
    # обычное число обязано остаться на виду.
    jit_ms=$(median_ms "$BSL_CLI --jit" "$scenario_bsl" "$workdir") || jit_ms="ошибка"
    is_heavy "$name" && keep_output "$name" bsl-cli-jit
    lua_ms="-"
    luajit_ms="-"
    python_ms="-"
    os_ms="-"
    if [ -f "benchmarks/$name.lua" ]; then
        if [ -n "$LUA" ]; then
            lua_ms=$(median_ms "$LUA" "benchmarks/$name.lua" "$workdir") || lua_ms="ошибка"
            is_heavy "$name" && keep_output "$name" lua
        fi
        if [ -n "$LUAJIT" ]; then
            luajit_ms=$(median_ms "$LUAJIT" "benchmarks/$name.lua" "$workdir") || luajit_ms="ошибка"
            is_heavy "$name" && keep_output "$name" luajit
        fi
    fi
    if [ -n "$PYTHON" ] && [ -f "benchmarks/$name.py" ]; then
        python_ms=$(median_ms "$PYTHON" "benchmarks/$name.py" "$workdir") || python_ms="ошибка"
        is_heavy "$name" && keep_output "$name" python
    fi
    if [ -n "$OSCRIPT" ]; then
        os_ms=$(median_ms "$OSCRIPT" "$scenario_bsl" "$workdir") || os_ms="ошибка"
        is_heavy "$name" && keep_output "$name" oscript
    fi

    onec_ms=$(onec_median "$name")
    printf '%-18s %10s %10s %10s %10s %10s %10s %10s\n' "$name" "$ours" "$jit_ms" "$lua_ms" "$luajit_ms" "$python_ms" "$os_ms" "$onec_ms"
done

echo
echo "медиана $RUNS прогонов, миллисекунды. Прочерк — двойника на этом"
echo "языке нет либо самого рантайма нет в системе."
if [ -n "$HEAVY_SEEN" ]; then
    echo "тяжёлые сценарии — $HEAVY_RUNS прогона."
    echo "csv_write* пишет в $SCRATCH и после замера сверяется побайтно."
    # Сверка: сценарий с файловым выводом сам ничего не печатает, кроме
    # миллисекунд, поэтому "все посчитали одно и то же" проверяется
    # СЛИЧЕНИЕМ ФАЙЛОВ, а не строкой в выводе.
    for produced in "$SCRATCH"/*.bsl-cli.out; do
        [ -f "$produced" ] || continue
        base=$(basename "$produced" .bsl-cli.out)
        for other in "$SCRATCH/$base".*.out; do
            [ "$other" = "$produced" ] && continue
            rt=$(basename "$other" .out); rt=${rt#"$base."}
            if cmp -s "$produced" "$other"; then
                echo "  $base: вывод $rt совпал с нашим побайтно"
            else
                echo "  $base: ВЫВОД $rt РАСХОДИТСЯ С НАШИМ — числа несравнимы"
            fi
        done
    done
fi
if [ -f "$ONEC_FILE" ]; then
    echo "колонка 1С — медиана из $ONEC_FILE (снято отдельным прогоном"
    echo "на платформе, см. шапку этого файла)."
fi
