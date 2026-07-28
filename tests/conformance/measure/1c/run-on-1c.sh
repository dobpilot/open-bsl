#!/usr/bin/env bash
# Прогон .bsl-скрипта на РЕАЛЬНОЙ платформе 1С:Предприятие и снятие вывода.
#
#   ./tests/conformance/measure/1c/run-on-1c.sh                     # measure-all.bsl
#   ./tests/conformance/measure/1c/run-on-1c.sh путь/к/скрипту.bsl  # любой другой
#
# Результат кладётся в tests/conformance/measure/platform.tsv (или рядом со
# скриптом, если он не measure-all.bsl) и скармливается обратно так:
#
#   cargo run -p bsl-cli -- --ingest-measurements tests/conformance/measure/platform.tsv
#
# ЗАЧЕМ ТАК СЛОЖНО. Платформа не умеет исполнять текстовый файл: код должен
# лежать в модуле объекта метаданных. Проверено на 8.3.27, каждый шаг —
# следствие измеренного факта, а не предположения:
#
#   * `/Execute` НЕ компилирует и НЕ выполняет модуль объекта внешней
#     обработки: заведомо битый модуль собирается и «выполняется» с кодом 0.
#     Значит нужна ФОРМА — код живёт в её модуле, точка входа `ПриОткрытии`.
#   * `Сообщить` в батче никуда не попадает, а ЗАТЕНИТЬ её своей процедурой
#     нельзя: платформа молча отказывается компилировать такой модуль.
#     Поэтому генератор заменяет в КОПИИ текста `Сообщить(` на
#     `СообщитьВФайл(` — сам скрипт замеров не меняется ни на символ.
#   * Рабочий код идёт в `&НаСервере Процедура ВыполнитьТестНаСервере()`,
#     клиентская `ПриОткрытии` только зовёт её и закрывает платформу.
#
# ОКРУЖЕНИЕ (Linux, роллинг-дистрибутив). Три вещи, без которых 8.3.27 не
# стартует, и все три проверены на этой машине:
#
#   1. `LD_LIBRARY_PATH=<шим>:/usr/lib` — 1С кладёт рядом свой libstdc++
#      6.0.28, а системные libicui18n/libhwy/webkit требуют новее. У
#      бинарника RUNPATH=$ORIGIN (не RPATH), поэтому LD_LIBRARY_PATH его
#      перебивает.
#   2. Шим с `.wk41`-сборками wxWidgets — обычная собрана под
#      webkit2gtk-4.0 (libsoup2), система тянет libsoup3, процесс падает с
#      «libsoup2 symbols detected». 1С поставляет обе сборки рядом.
#   3. `GDK_BACKEND=x11` и снятый WAYLAND_DISPLAY — сборка только под X11,
#      на Wayland-сессии идёт через Xwayland.

set -u
cd "$(dirname "$0")/../../../.." || exit 1

SCRIPT=${1:-tests/conformance/measure/measure-all.bsl}
[ -f "$SCRIPT" ] || { echo "нет скрипта: $SCRIPT" >&2; exit 1; }

# --- платформа ---------------------------------------------------------
PLATFORM=${ONEC_PLATFORM:-}
if [ -z "$PLATFORM" ]; then
    PLATFORM=$(ls -d /opt/1cv8/x86_64/*/1cv8 2>/dev/null | sort -V | tail -1)
fi
[ -x "$PLATFORM" ] || {
    echo "не найден 1cv8. Задайте путь: ONEC_PLATFORM=/opt/1cv8/x86_64/8.3.27.2130/1cv8" >&2
    exit 1
}
PLATFORM_DIR=$(dirname "$PLATFORM")

# Шим с wk41-сборками; создаётся один раз, /opt не трогается.
SHIM=${ONEC_SHIM:-$HOME/.local/lib/1c-wk41}
if [ ! -e "$SHIM/libwx_gtk3u-3.0.so.0" ]; then
    mkdir -p "$SHIM"
    ln -sf "$PLATFORM_DIR/libwx_gtk3u-3.0.so.0.1.0.wk41" "$SHIM/libwx_gtk3u-3.0.so.0"
    ln -sf "$PLATFORM_DIR/libwx_gtk3u-3.0.so.0.1.0.wk41" "$SHIM/libwx_gtk3u-3.0.so.0.1.0"
    ln -sf "$PLATFORM_DIR/webkit2_extu-3.0.so.wk41" "$SHIM/webkit2_extu-3.0.so"
fi

onec() {
    env -u WAYLAND_DISPLAY \
        GDK_BACKEND=x11 \
        DISPLAY="${DISPLAY:-:1}" \
        XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" \
        LD_LIBRARY_PATH="$SHIM:/usr/lib" \
        "$PLATFORM" "$@"
}

# --- информационная база ------------------------------------------------
# Пустая файловая база без конфигурации: платформе нужно КУДА подключаться,
# сам код живёт во внешней обработке. Создаётся через ibcmd — ему лицензия
# для этого не нужна.
IB=${ONEC_IB:-${TMPDIR:-/tmp}/onec-llvm-ib}
if [ ! -f "$IB/1Cv8.1CD" ]; then
    echo "создаю информационную базу в $IB"
    mkdir -p "$IB"
    "$PLATFORM_DIR/ibcmd" infobase create --db-path="$IB" --create-database >/dev/null || {
        echo "не удалось создать информационную базу" >&2; exit 1; }
fi

# --- сборка внешней обработки -------------------------------------------
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cp -r tests/conformance/measure/1c/epf-src "$WORK/src"

OUT_ABS="$WORK/platform-output.tsv"
python3 tests/conformance/measure/1c/gen-form-module.py \
    "$SCRIPT" \
    "$WORK/src/Замеры/Forms/Форма/Ext/Form/Module.bsl" \
    "$OUT_ABS" || exit 1

onec DESIGNER /F"$IB" \
    /LoadExternalDataProcessorOrReportFromFiles "$WORK/src/Замеры.xml" "$WORK/Замеры.epf" \
    /DisableStartupDialogs /Out "$WORK/build.log" >/dev/null 2>&1
if [ ! -f "$WORK/Замеры.epf" ]; then
    echo "не собралась внешняя обработка:" >&2
    cat "$WORK/build.log" >&2
    exit 1
fi

# --- прогон -------------------------------------------------------------
# ЖЁСТКИЙ ТАЙМАУТ обязателен: необработанное исключение в модуле платформа
# показывает МОДАЛЬНЫМ окном, а `/DisableStartupDialogs` гасит только
# стартовые диалоги. Без таймаута такой прогон висит вечно (проверено).
# Отсюда же правило для самих скриптов замеров: каждую пробу, способную
# бросить, заворачивать в Попытка — иначе замер не падает, а зависает.
ONEC_TIMEOUT=${ONEC_TIMEOUT:-180}
timeout --kill-after=10 "$ONEC_TIMEOUT" \
    env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY="${DISPLAY:-:1}" \
        XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" \
        LD_LIBRARY_PATH="$SHIM:/usr/lib" \
        "$PLATFORM" ENTERPRISE /F"$IB" /Execute "$WORK/Замеры.epf" \
        /DisableStartupDialogs /DisableStartupMessages /Out "$WORK/run.log" >/dev/null 2>&1
RC=$?
if [ "$RC" -ge 124 ]; then
    echo "платформа не завершилась за ${ONEC_TIMEOUT}с и была снята." >&2
    echo "Типовая причина: необработанное исключение показало модальное окно." >&2
    echo "Заверните пробы в Попытка/Исключение." >&2
    exit 1
fi

# Пустой вывод при коде 0 — самый опасный исход: модуль не скомпилировался,
# а платформа промолчала. Поэтому проверяем результат, а не код возврата.
if [ ! -s "$OUT_ABS" ]; then
    echo "платформа не выдала ни строки — скорее всего модуль формы не" >&2
    echo "скомпилировался. Лог платформы:" >&2
    cat "$WORK/run.log" >&2
    exit 1
fi

# Платформа пишет UTF-8 С СИГНАТУРОЙ и переводы строк CRLF. Разбор ждёт
# первым символом идентификатор, а сравнивать файл построчно с чужим
# выводом проще без \r — снимаем и то, и другое здесь, один раз.
sed -i '1s/^\xef\xbb\xbf//' "$OUT_ABS"
sed -i 's/\r$//' "$OUT_ABS"

if [ "$SCRIPT" = "tests/conformance/measure/measure-all.bsl" ]; then
    DEST=tests/conformance/measure/platform.tsv
else
    DEST="${SCRIPT%.bsl}.platform.txt"
fi
cp "$OUT_ABS" "$DEST"
echo "снято строк: $(wc -l < "$DEST")  ->  $DEST"
echo
echo "сверка:  cargo run -p bsl-cli -- --ingest-measurements $DEST"
