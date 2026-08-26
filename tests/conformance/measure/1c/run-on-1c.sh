#!/usr/bin/env bash
# Прогон .bsl-скрипта на РЕАЛЬНОЙ платформе 1С:Предприятие и снятие вывода.
#
#   ./tests/conformance/measure/1c/run-on-1c.sh                     # measure-all.bsl
#   ./tests/conformance/measure/1c/run-on-1c.sh путь/к/скрипту.bsl  # любой другой
#
# ЗАЩИТА ОТ ОПАСНЫХ ДЕЙСТВИЙ БОЛЬШЕ НЕ МЕШАЕТ. Раньше код замеров жил во
# ВНЕШНЕЙ обработке, а платформа считает её открытие потенциально опасным
# действием и показывает модальное «Разрешить открывать данный файл?» —
# прогон без человека умирал по таймауту. Ключа командной строки для этого
# предупреждения нет, а гасящий его параметр `DisableUnsafeActionProtection`
# живёт в root-принадлежащем conf.cfg вне проекта и правкой изнутри
# репозитория не годится.
#
# Обходной путь — не открывать файл вовсе. Обработка «Замеры» с той же
# формой встраивается ПРЯМО В КОНФИГУРАЦИЮ одноразовой информационной базы,
# а стартовым обработчиком служит `ПриНачалеРаботыСистемы` в модуле
# управляемого приложения — платформа запускается без `/Execute`. Открытие
# объекта из состава своей же конфигурации опасным действием не считается
# (проверено: тот же сценарий на ИБ с ДРУГИМ путём, заведомо не упомянутым
# ни в каком conf.cfg, отрабатывает без единого модального окна), поэтому
# путь ИБ больше ничем не выделен и его можно свободно переопределять.
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
#   * Модуль объекта обработки (`Ext/ObjectModule.bsl`) не годится тем же
#     способом, что и раньше: точка входа — ФОРМА, код живёт в её модуле
#     `ПриОткрытии`.
#   * `Сообщить` в батче никуда не попадает, а ЗАТЕНИТЬ её своей процедурой
#     нельзя: платформа молча отказывается компилировать такой модуль.
#     Поэтому генератор заменяет в КОПИИ текста `Сообщить(` на
#     `СообщитьВФайл(` — сам скрипт замеров не меняется ни на символ.
#   * Рабочий код идёт в `&НаСервере Процедура ВыполнитьТестНаСервере()`,
#     клиентская `ПриОткрытии` только зовёт её и закрывает платформу.
#   * `DumpConfigToFiles`/`LoadConfigFromFiles` понимают XML-описания
#     объектов конфигурации почти так же, как формат внешней обработки, но
#     не совсем: у объекта в конфигурации обязателен узел `InternalInfo` с
#     ДВУМЯ `xr:GeneratedType` (`...Объект.Замеры` категории `Object` и
#     `...Менеджер.Замеры` категории `Manager`) — без него платформа отвечает
#     «отсутствует один или более типов объекта DataProcessor». У внешней
#     обработки в старой схеме такой узел был один и другой категории —
#     верно для внешних обработок, не для встроенных.
#   * `ПриНачалеРаботыСистемы` в модуле управляемого приложения платформа
#     подхватывает по ФИКСИРОВАННОМУ имени файла `Ext/ManagedApplicationModule.bsl`
#     на верхнем уровне выгрузки — никакой ссылки на него в Configuration.xml
#     не требуется (проверено округлым рейсом: то, что туда положено,
#     возвращается обратно тем же DumpConfigToFiles).
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
# Пустая файловая база: код замеров каждый прогон вписывается в её
# конфигурацию заново (см. ниже). Создаётся через ibcmd — ему лицензия для
# этого не нужна. Путь ничем не выделен и свободно переопределяется через
# ONEC_IB — в отличие от старой схемы через внешнюю обработку, здесь он не
# участвует в защите от опасных действий.
IB=${ONEC_IB:-${TMPDIR:-/tmp}/open-bsl-ib}
if [ ! -f "$IB/1Cv8.1CD" ]; then
    echo "создаю информационную базу в $IB"
    mkdir -p "$IB"
    "$PLATFORM_DIR/ibcmd" infobase create --db-path="$IB" --create-database >/dev/null || {
        echo "не удалось создать информационную базу" >&2; exit 1; }
fi

# --- сборка конфигурации -------------------------------------------------
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# Выгружаем ТЕКУЩУЮ конфигурацию базы в XML — на первом прогоне это пустой
# скелет (Configuration.xml, ConfigDumpInfo.xml, Languages/), на последующих
# в нём уже есть обработка «Замеры» от прошлого прогона, что безвредно: её
# файлы ниже перезаписываются тем же содержимым, а модуль формы — новым.
mkdir -p "$WORK/cfg"
onec DESIGNER /F"$IB" /DumpConfigToFiles "$WORK/cfg" /DisableStartupDialogs \
    /Out "$WORK/dump.log" >/dev/null 2>&1
if [ ! -f "$WORK/cfg/Configuration.xml" ]; then
    echo "не удалось выгрузить конфигурацию базы:" >&2
    cat "$WORK/dump.log" >&2
    exit 1
fi

# Накладываем обработку «Замеры» и модуль управляемого приложения поверх
# скелета. Содержимое статично между прогонами — меняется только модуль
# формы, который генерируется ниже из скрипта замеров.
cp -r tests/conformance/measure/1c/cfg-src/. "$WORK/cfg/"
mkdir -p "$WORK/cfg/DataProcessors/Замеры/Forms/Форма/Ext/Form"

# Первый прогон на этой базе должен ЗАРЕГИСТРИРОВАТЬ обработку в составе
# конфигурации; на последующих прогонах DumpConfigToFiles уже возвращает её
# в ChildObjects, и повторное добавление испортило бы XML.
if ! grep -q '<DataProcessor>Замеры</DataProcessor>' "$WORK/cfg/Configuration.xml"; then
    sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<DataProcessor>Замеры</DataProcessor>|' \
        "$WORK/cfg/Configuration.xml"
fi

OUT_ABS="$WORK/platform-output.tsv"
python3 tests/conformance/measure/1c/gen-form-module.py \
    "$SCRIPT" \
    "$WORK/cfg/DataProcessors/Замеры/Forms/Форма/Ext/Form/Module.bsl" \
    "$OUT_ABS" || exit 1

onec DESIGNER /F"$IB" /LoadConfigFromFiles "$WORK/cfg" /UpdateDBCfg \
    /DisableStartupDialogs /Out "$WORK/build.log" >/dev/null 2>&1
BUILD_RC=$?
if [ "$BUILD_RC" -ne 0 ] || ! grep -Eqi "успешно завершено|successfully updated" "$WORK/build.log"; then
    echo "не удалось обновить конфигурацию базы:" >&2
    cat "$WORK/build.log" >&2
    exit 1
fi

# --- прогон -------------------------------------------------------------
# ЖЁСТКИЙ ТАЙМАУТ по-прежнему обязателен: необработанное исключение в
# модуле платформа показывает МОДАЛЬНЫМ окном, а `/DisableStartupDialogs`
# гасит только стартовые диалоги. Без таймаута такой прогон висит вечно
# (проверено). Отсюда же правило для самих скриптов замеров: каждую пробу,
# способную бросить, заворачивать в Попытка — иначе замер не падает, а
# зависает. Внешних обработок в этой схеме больше нет, поэтому
# предупреждение об опасном действии как причина зависания отпало — но не
# как класс: платформа показывает столь же немое модальное окно на любое
# необработанное исключение при старте.
ONEC_TIMEOUT=${ONEC_TIMEOUT:-180}
timeout --kill-after=10 "$ONEC_TIMEOUT" \
    env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY="${DISPLAY:-:1}" \
        XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" \
        LD_LIBRARY_PATH="$SHIM:/usr/lib" \
        "$PLATFORM" ENTERPRISE /F"$IB" \
        /DisableStartupDialogs /DisableStartupMessages /Out "$WORK/run.log" >/dev/null 2>&1
RC=$?
if [ "$RC" -ge 124 ]; then
    echo "платформа не завершилась за ${ONEC_TIMEOUT}с и была снята." >&2
    echo "Типовая причина — необработанное исключение в скрипте, которое" >&2
    echo "платформа показывает МОДАЛЬНЫМ окном: заверните пробу в" >&2
    echo "Попытка/Исключение." >&2
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
