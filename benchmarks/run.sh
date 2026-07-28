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
OSCRIPT=""
command -v oscript >/dev/null 2>&1 && OSCRIPT=oscript

echo "рантаймы:"
echo "  bsl-cli   $($BSL_CLI --help | head -1)"
[ -n "$LUA" ] && echo "  lua       $($LUA -v 2>&1 | head -1)" || echo "  lua       НЕТ"
[ -n "$LUAJIT" ] && echo "  luajit    $($LUAJIT -v 2>&1 | head -1)" || echo "  luajit    НЕТ"
[ -n "$OSCRIPT" ] && echo "  oscript   $($OSCRIPT -version 2>&1 | head -1)" || echo "  oscript   НЕТ (нужен OneScript)"
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

printf '%-14s %10s %10s %10s %10s\n' сценарий bsl-cli lua luajit oscript
printf '%-14s %10s %10s %10s %10s\n' -------------- ---------- ---------- ---------- ----------

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

    printf '%-14s %10s %10s %10s %10s\n' "$name" "$ours" "$lua_ms" "$luajit_ms" "$os_ms"
done

echo
echo "медиана $RUNS прогонов, миллисекунды. Прочерк — двойника на этом"
echo "языке нет либо самого рантайма нет в системе."
