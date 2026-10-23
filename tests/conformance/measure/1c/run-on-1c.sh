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

# Старые маркеры не должны выглядеть результатом нового запуска.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-transitive-arguments.bsl" ]; then
    for marker in /tmp/open-bsl-transitive-args-096-parent.marker /tmp/open-bsl-transitive-args-096-argument.marker; do
        if [ -e "$marker" ]; then
            echo "маркер уже существует, проверьте и удалите его перед повторной пробой: $marker" >&2
            exit 1
        fi
    done
fi

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
# Designer всегда запускается через `1cv8`; для замера тонкого клиента
# Enterprise можно отдельно запустить через `1cv8c` той же версии.
ENTERPRISE_PLATFORM=${ONEC_ENTERPRISE_PLATFORM:-$PLATFORM}
[ -x "$ENTERPRISE_PLATFORM" ] || {
    echo "не найден бинарник Enterprise: $ENTERPRISE_PLATFORM" >&2
    exit 1
}

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
case "$SCRIPT" in
    tests/conformance/measure/1c/measure-functional-option-source-rights-setup.bsl|tests/conformance/measure/1c/measure-functional-option-source-rights.bsl)
        if [ "$IB" != /tmp/open-bsl-functional-option-source-rights-1009-ib ]; then
            echo "для этого замера требуется собственная /tmp/open-bsl-functional-option-source-rights-1009-ib" >&2
            exit 1
        fi
        ;;
esac
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

if [ "$SCRIPT" = "docs/research/fixtures/document-field-access-1017/native/measure.bsl" ]; then
    cp -r docs/research/fixtures/document-field-access-1017/native/input-xml/. "$WORK/cfg/"
    for ROLE_NAME in ControlAdmin ControlReadOff ControlReadOn ControlDenied; do
        if ! grep -q "<Role>$ROLE_NAME</Role>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n<Role>$ROLE_NAME</Role>|" "$WORK/cfg/Configuration.xml"
        fi
    done
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [[ "$SCRIPT" == docs/research/fixtures/data-separation-frames-1017/measure*.bsl ]]; then
    if [ "$SCRIPT" = "docs/research/fixtures/data-separation-frames-1017/measure-names.bsl" ]; then
        cp -r docs/research/fixtures/data-separation-frames-1017/two-names/input-xml/. "$WORK/cfg/"
        sed -i 's|<ChildObjects>|<ChildObjects>\n<CommonAttribute>ДругоеРазделение</CommonAttribute>|' "$WORK/cfg/Configuration.xml"
    else
        cp -r docs/research/fixtures/data-separation-frames-1017/input-xml/. "$WORK/cfg/"
    fi
    sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>\n<CommonAttribute>НаименованиеЯзык1</CommonAttribute>\n<SessionParameter>ЗначениеРазделителяПробы</SessionParameter>\n<SessionParameter>ИспользованиеРазделителяПробы</SessionParameter>\n<SessionParameter>ОбычныйПараметрПробы</SessionParameter>\n<SessionParameter>НаблюдениеСтарта</SessionParameter>|' "$WORK/cfg/Configuration.xml"
fi

if [[ "$SCRIPT" == docs/research/fixtures/separation-initial-1017/measure-*.bsl ]]; then
    SEPARATION_PROFILE=${SCRIPT##*/measure-}
    SEPARATION_PROFILE=${SEPARATION_PROFILE%.bsl}
    cp -r "docs/research/fixtures/separation-initial-1017/$SEPARATION_PROFILE/input-xml/." "$WORK/cfg/"
    sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>\n<CommonAttribute>НаименованиеЯзык1</CommonAttribute>\n<SessionParameter>ЗначениеРазделителяПробы</SessionParameter>\n<SessionParameter>ИспользованиеРазделителяПробы</SessionParameter>\n<SessionParameter>ОбычныйПараметрПробы</SessionParameter>\n<SessionParameter>НаблюдениеСтарта</SessionParameter>|' "$WORK/cfg/Configuration.xml"
fi

if [ "$SCRIPT" = "docs/research/fixtures/subsystem-scope-1017/measure.bsl" ]; then
    cp -r docs/research/fixtures/subsystem-scope-1017/input-xml/. "$WORK/cfg/"
    for SUBSYSTEM_NAME in Родитель Другой; do
        if ! grep -q "<Subsystem>$SUBSYSTEM_NAME</Subsystem>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n\t\t\t<Subsystem>$SUBSYSTEM_NAME</Subsystem>|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

if [ "$SCRIPT" = "docs/research/fixtures/exchange-plan-session-reference-1016/measure.bsl" ]; then
    cp -r docs/research/fixtures/exchange-plan-session-reference-1016/input-xml/. "$WORK/cfg/"
    for PLAN_NAME in ЗамерОбмена ДругойОбмен; do
        if ! grep -q "<ExchangePlan>$PLAN_NAME</ExchangePlan>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n\t\t\t<ExchangePlan>$PLAN_NAME</ExchangePlan>|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

if [ "$SCRIPT" = "docs/research/fixtures/task-reference-presentation/russian.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/task-reference-presentation/english.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/task-reference-presentation/missing-english.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/task-reference-presentation/aliases.bsl" ]; then
    cp docs/research/fixtures/task-reference-presentation/Task.xml "$WORK/cfg/Tasks/ЗамерЗадачи.xml"
    cp docs/research/fixtures/task-reference-presentation/English.xml "$WORK/cfg/Languages/English.xml"
    if ! grep -q '<Language>English</Language>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Language>English</Language>|' "$WORK/cfg/Configuration.xml"
    fi
    if [ "$SCRIPT" = "docs/research/fixtures/task-reference-presentation/missing-english.bsl" ]; then
        cp docs/research/fixtures/task-reference-presentation/TaskMissingEnglish.xml "$WORK/cfg/Tasks/ЗамерЗадачи.xml"
    fi
    if [ "$SCRIPT" != "docs/research/fixtures/task-reference-presentation/russian.bsl" ]; then
        sed -i 's|<DefaultLanguage>[^<]*</DefaultLanguage>|<DefaultLanguage>Language.English</DefaultLanguage>|' "$WORK/cfg/Configuration.xml"
    else
        sed -i 's|<DefaultLanguage>[^<]*</DefaultLanguage>|<DefaultLanguage>Language.Русский</DefaultLanguage>|' "$WORK/cfg/Configuration.xml"
    fi
fi


if [ "$SCRIPT" = "docs/research/fixtures/error-description-context/instance.bsl" ]; then
    mkdir -p "$WORK/cfg/DataProcessors/Замеры/Ext"
    cp docs/research/fixtures/error-description-context/ObjectModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/data-processor-object/measure.bsl" ]; then
    mkdir -p "$WORK/cfg/DataProcessors/Замеры/Ext"
    cp docs/research/fixtures/data-processor-object/ObjectModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
    cp docs/research/fixtures/data-processor-object/ManagerModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ManagerModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/data-processor-object/fields.bsl" ]; then
    mkdir -p "$WORK/cfg/DataProcessors/Замеры/Ext"
    cp docs/research/fixtures/data-processor-object/Fields.xml "$WORK/cfg/DataProcessors/Замеры.xml"
    cp docs/research/fixtures/data-processor-object/FieldsObjectModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
    cp docs/research/fixtures/data-processor-object/ManagerModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ManagerModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/data-processor-object/synonym.bsl" ]; then
    mkdir -p "$WORK/cfg/DataProcessors/Замеры/Ext"
    cp docs/research/fixtures/data-processor-object/Synonym.xml "$WORK/cfg/DataProcessors/Замеры.xml"
    cp docs/research/fixtures/data-processor-object/FieldsObjectModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
    cp docs/research/fixtures/data-processor-object/ManagerModule.bsl "$WORK/cfg/DataProcessors/Замеры/Ext/ManagerModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/new.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/compare.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/references.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/group-order.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/cross-kind.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/constructor.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "docs/research/fixtures/document-point-in-time/cross-kind.bsl" ]; then
    for document in АльфаМомента ЯмаМомента; do
        cp "docs/research/fixtures/document-point-in-time/$document.xml" "$WORK/cfg/Documents/$document.xml"
        if ! grep -q "<Document>$document</Document>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>$document</Document>|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Проверка прав чтения независимого регистра выполняется только в собственной базе.
if [ "$SCRIPT" = "docs/research/fixtures/information-register-rights-1016/setup.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/information-register-rights-1016/read.bsl" ]; then
    if [ "$IB" != /tmp/open-bsl-information-register-rights-1016-ib ]; then
        echo "для проверки прав требуется собственная /tmp/open-bsl-information-register-rights-1016-ib" >&2
        exit 1
    fi
    python3 docs/research/fixtures/information-register-rights-1016/inject.py "$WORK/cfg"
fi

# Тот же контроль для измеренной периодичности Day, с отдельной квитанцией.
if [ "$SCRIPT" = "docs/research/fixtures/information-register-rights-1016/setup-day.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/information-register-rights-1016/read-day.bsl" ]; then
    if [ "$IB" != /tmp/open-bsl-information-register-rights-1016-ib ]; then
        echo "для проверки прав требуется собственная /tmp/open-bsl-information-register-rights-1016-ib" >&2
        exit 1
    fi
    python3 docs/research/fixtures/information-register-rights-1016/inject.py "$WORK/cfg" day
fi

# Собственный независимый регистр для контроля прямого чтения.
if [ "$SCRIPT" = "docs/research/fixtures/information-register-get/empty.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/information-register-get/nonempty.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/information-register-get/normalization.bsl" ]; then
    mkdir -p "$WORK/cfg/InformationRegisters"
    cp docs/research/fixtures/information-register-get/InformationRegister.xml "$WORK/cfg/InformationRegisters/ЗамерПолучить.xml"
    if ! grep -q '<InformationRegister>ЗамерПолучить</InformationRegister>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<InformationRegister>ЗамерПолучить</InformationRegister>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "docs/research/fixtures/information-register-get/periodic.bsl" ]; then
    mkdir -p "$WORK/cfg/InformationRegisters"
    cp docs/research/fixtures/information-register-get/PeriodicInformationRegister.xml "$WORK/cfg/InformationRegisters/ЗамерПолучитьПериод.xml"
    if ! grep -q '<InformationRegister>ЗамерПолучитьПериод</InformationRegister>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<InformationRegister>ЗамерПолучитьПериод</InformationRegister>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "docs/research/fixtures/characteristic-predefined/measure.bsl" ]; then
    mkdir -p "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext"
    cp docs/research/fixtures/characteristic-predefined/ObjectModule.bsl "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext/ObjectModule.bsl"
fi
if [ "$SCRIPT" = "docs/research/fixtures/characteristic-predefined/measure-manager.bsl" ]; then
    mkdir -p "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext"
    cp docs/research/fixtures/characteristic-manager-create/ЗамерХарактеристик.native.xml "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик.xml"
    cp docs/research/fixtures/characteristic-predefined/Predefined.xml "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext/Predefined.xml"
    cp docs/research/fixtures/characteristic-predefined/ManagerModule.bsl "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext/ManagerModule.bsl"
    if ! grep -q '<ChartOfCharacteristicTypes>ЗамерХарактеристик</ChartOfCharacteristicTypes>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<ChartOfCharacteristicTypes>ЗамерХарактеристик</ChartOfCharacteristicTypes>|' "$WORK/cfg/Configuration.xml"
    fi
fi
if [ "$SCRIPT" = "docs/research/fixtures/characteristic-deletion-mark/measure.bsl" ]; then
    mkdir -p "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext"
    cp docs/research/fixtures/characteristic-manager-create/ЗамерХарактеристик.native.xml "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик.xml"
    if ! grep -q '<ChartOfCharacteristicTypes>ЗамерХарактеристик</ChartOfCharacteristicTypes>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<ChartOfCharacteristicTypes>ЗамерХарактеристик</ChartOfCharacteristicTypes>|' "$WORK/cfg/Configuration.xml"
    fi
    cp docs/research/fixtures/characteristic-deletion-mark/ObjectModule.bsl "$WORK/cfg/ChartsOfCharacteristicTypes/ЗамерХарактеристик/Ext/ObjectModule.bsl"
fi

# Обычные контроли используют исходный пустой модуль объекта,
# независимо от модуля, оставленного предыдущей фазовой пробой.
if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-enabled.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/write.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-new-assigned.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-new-standard.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-new-section.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/new-reference-invalid.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/new-reference.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/data-exchange.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/modified-failure.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/modified.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/modified-section.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task-identity.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task-reference-identity.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task-reference.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/write-arity.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/write-empty-date.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-types.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-fill-empty.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-fill-table.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-required.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-message.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/tabular-section-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/new-reference-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/new-reference-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/data-exchange-phase.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/data-exchange-phase-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/data-exchange-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/data-exchange-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/modified-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/modified-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task-module-identity.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/head-task-module-identity-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/head-task-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/head-task-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-own-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/read-own-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-module.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/read-new-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/read-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-enabled-handler.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-enabled-handler-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-enabled-handler-empty.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-enabled-handler-empty-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-fill-handler-standard.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/tabular-section-fill-handler-standard-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-fill-handler.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/tabular-section-fill-handler-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

# Табличная часть проверяется на отдельной собственной базе с двумя колонками.
if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/tabular-section-module.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/tabular-section-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-handler-replace.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/check-filling-handler-replace-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-handler-refusal.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/check-filling-handler-refusal-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

# Проверка обязательного поля использует собственную базу с ShowError.
if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-handler.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/check-filling-handler-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

# Собственные реквизиты проверяются на отдельной базе с двумя полями.
# Меняется только модуль объекта в рабочей копии выгрузки.
if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/own-fields-module.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/own-fields-isolation.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-arity.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/check-filling-alias.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/own-fields-module-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/standard-properties.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/standard-properties-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-handler.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-handler-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-handler-inspect.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-handler-empty.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-handler-inspect-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-standard.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-standard-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-standard-direct.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-standard-direct-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/fill-standard-no-handler.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/fill-standard-no-handler-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi

# Контроль собственного состояния модуля использует уже созданный тестовый
# бизнес-процесс; модуль подменяется только в копии нативной выгрузки.
if [ "$SCRIPT" = "docs/research/fixtures/business-process-object/module-state.bsl" ]; then
    mkdir -p "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext"
    cp docs/research/fixtures/business-process-object/module-state-object.bsl "$WORK/cfg/BusinessProcesses/ЗамерБизнесПроцесса/Ext/ObjectModule.bsl"
fi
if [[ "$SCRIPT" == docs/research/fixtures/settings-storage-load/measure*.bsl ]]; then
    mkdir -p "$WORK/cfg/SettingsStorages/ЗамерХранилища/Ext"
    cp docs/research/fixtures/settings-storage-load/storage.xml "$WORK/cfg/SettingsStorages/ЗамерХранилища.xml"
    cp docs/research/fixtures/settings-storage-load/manager-module.bsl "$WORK/cfg/SettingsStorages/ЗамерХранилища/Ext/ManagerModule.bsl"
    if [ "$SCRIPT" = "docs/research/fixtures/settings-storage-load/measure-handler-error.bsl" ]; then
        cp docs/research/fixtures/settings-storage-load/manager-handler-error.bsl "$WORK/cfg/SettingsStorages/ЗамерХранилища/Ext/ManagerModule.bsl"
    fi
    if [ "$SCRIPT" = "docs/research/fixtures/settings-storage-load/measure-handler-division-error.bsl" ]; then
        cp docs/research/fixtures/settings-storage-load/manager-division-error.bsl "$WORK/cfg/SettingsStorages/ЗамерХранилища/Ext/ManagerModule.bsl"
    fi
    if [ "$SCRIPT" = "docs/research/fixtures/settings-storage-load/measure-module-initialization-error.bsl" ]; then
        cp docs/research/fixtures/settings-storage-load/manager-initialization-error.bsl "$WORK/cfg/SettingsStorages/ЗамерХранилища/Ext/ManagerModule.bsl"
    fi
    if ! grep -q '<SettingsStorage>ЗамерХранилища</SettingsStorage>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<SettingsStorage>ЗамерХранилища</SettingsStorage>|' "$WORK/cfg/Configuration.xml"
    fi
fi
if [[ "$SCRIPT" == docs/research/fixtures/report-object/measure*.bsl ]]; then
    mkdir -p "$WORK/cfg/Reports/ЗамерОтчета/Ext"
    cp docs/research/fixtures/report-object/Report.xml "$WORK/cfg/Reports/ЗамерОтчета.xml"
    cp docs/research/fixtures/report-object/ObjectModule.bsl "$WORK/cfg/Reports/ЗамерОтчета/Ext/ObjectModule.bsl"
    cp docs/research/fixtures/report-object/ManagerModule.bsl "$WORK/cfg/Reports/ЗамерОтчета/Ext/ManagerModule.bsl"
    if ! grep -q '<Report>ЗамерОтчета</Report>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Report>ЗамерОтчета</Report>|' "$WORK/cfg/Configuration.xml"
    fi
fi
if [[ "$SCRIPT" == docs/research/fixtures/document-copy/measure-*.bsl ]]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
    cp docs/research/fixtures/document-copy/document.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    if [ "$SCRIPT" = "docs/research/fixtures/document-copy/measure-tabular.bsl" ]; then
        cp docs/research/fixtures/document-copy/document-tabular.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    fi
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp docs/research/fixtures/document-copy/object-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "docs/research/fixtures/document-copy/measure-handler.bsl" ]; then
        cp docs/research/fixtures/document-copy/handler-object.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
fi
mkdir -p "$WORK/cfg/DataProcessors/Замеры/Forms/Форма/Ext/Form"

if [[ "$SCRIPT" == docs/research/fixtures/document-check-filling/measure-*.bsl ]] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-concurrent.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-stored.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-string-handler.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-numeric-handler.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-numeric.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-body.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-year.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-handler.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
    rm -f "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    if [[ "$SCRIPT" == docs/research/fixtures/document-check-filling/measure-*.bsl ]]; then
        mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
        if [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-tabular-required.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-messages.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-messages-ru.bsl" ]; then
            cp docs/research/fixtures/document-check-filling/document-tabular-required.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
        fi
        if [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-tabular-column.bsl" ]; then
            cp docs/research/fixtures/document-check-filling/document-tabular-column.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
        fi
        if [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-own-attribute.bsl" ]; then
            cp docs/research/fixtures/document-check-filling/document-own-attribute.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
        fi
        if [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-path-mutation.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-own-attribute.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-tabular-column.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-tabular-required.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-messages.bsl" ] ||
           [ "$SCRIPT" = "docs/research/fixtures/document-check-filling/measure-messages-ru.bsl" ]; then
            cp docs/research/fixtures/document-check-filling/path-mutation-object.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
        else
        cp docs/research/fixtures/document-check-filling/object-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
        fi
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-numeric.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-numeric-handler.bsl" ]; then
        sed -i 's|<NumberType>String</NumberType>|<NumberType>Number</NumberType>|' "$WORK/cfg/Documents/ЗамерДокумента.xml"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-numeric-handler.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-string-handler.bsl" ]; then
        mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
        cp tests/conformance/measure/1c/document-new-number-numeric-handler-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-body.bsl" ]; then
        mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
        cp tests/conformance/measure/1c/document-new-number-body-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-year.bsl" ]; then
        sed -i 's|<NumberPeriodicity>Nonperiodical</NumberPeriodicity>|<NumberPeriodicity>Year</NumberPeriodicity>|; s|<NumberLength>9</NumberLength>|<NumberLength>11</NumberLength>|' "$WORK/cfg/Documents/ЗамерДокумента.xml"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-number-handler.bsl" ]; then
        mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
        cp tests/conformance/measure/1c/document-new-number-handler-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-new-reference.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    for document in ЗамерДокумента ЗамерЧужогоДокумента; do
        if ! grep -q "<Document>$document</Document>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>$document</Document>|" "$WORK/cfg/Configuration.xml"
        fi
    done
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
    cp tests/conformance/measure/1c/document-new-reference-other.xml "$WORK/cfg/Documents/ЗамерЧужогоДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-new-reference-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements-flags.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements-register.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-movements-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements-register.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements-flags.bsl" ]; then
        cp tests/conformance/measure/1c/document-movements.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
        mkdir -p "$WORK/cfg/InformationRegisters"
        cp tests/conformance/measure/1c/document-movements-register.xml "$WORK/cfg/InformationRegisters/РасписаниеЗагрузкиРесурсов.xml"
        if ! grep -q '<InformationRegister>РасписаниеЗагрузкиРесурсов</InformationRegister>' "$WORK/cfg/Configuration.xml"; then
            sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<InformationRegister>РасписаниеЗагрузкиРесурсов</InformationRegister>|' "$WORK/cfg/Configuration.xml"
        fi
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-movements-flags.bsl" ]; then
        cp tests/conformance/measure/1c/document-movements-flags-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-tabular-sections.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-tabular-sections-properties.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-tabular-sections-english.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
    cp tests/conformance/measure/1c/document-tabular-sections.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-tabular-sections-properties.bsl" ]; then
        mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
        cp tests/conformance/measure/1c/document-tabular-sections-module.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    else
        rm -f "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    fi
fi

# Инициализация собственного модуля объекта проверяется отдельно от формы.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-module-initializer.bsl" ]; then
    cp tests/conformance/measure/1c/native-image-module-initializer.Module.bin \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-module-initializer-source.bsl" ]; then
    cp tests/conformance/measure/1c/native-image-module-initializer-source.bsl \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-private38-call.bsl" ]; then
    cp target/diagnostics/readonly-startup/original-native-call38.Module.bin \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-private37-call.bsl" ]; then
    cp target/diagnostics/readonly-startup/original-native-call37.Module.bin \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-wrapper-baseline.bsl" ]; then
    cp target/diagnostics/readonly-startup/original-native-wrapper-baseline.Module.bin \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-object-load.bsl" ]; then
    cp target/diagnostics/readonly-startup/original-native-load.Module.bin \
        "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

# Один и тот же пустой кадр проверяется в контексте объекта обработки.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-declared41.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slot-six-clear-eight.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-linked-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-twelve-parameters.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-twelve-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slot-ten.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slot-eleven.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slot-six.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slot-nine.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slots-six-eight.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-slots-nine-eleven.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-first-six.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-last-six.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-external-only.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-flags-only.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-no-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-string.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-original-initializer.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-initializer.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-baseline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-original.bsl" ]; then
    FRAME_IMAGE="${SCRIPT%.*}"
    FRAME_IMAGE="${FRAME_IMAGE/measure-native-image/native-image}"
    cp "$FRAME_IMAGE.Module.bin" "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/DataProcessors/Замеры/Ext/ObjectModule.bsl"
fi

# Серверные общие модули замеров (cfg-src-server) накладываются тем же
# способом: это ТОТ ЖЕ каталог целей, которым двусторонний скрипт
# measure-background-jobs.bsl исполняется и здесь, и на open-bsl (runner —
# тест the_background_jobs_measure_script_runs_with_the_platform_catalog).
cp -r tests/conformance/measure/1c/cfg-src-server/. "$WORK/cfg/"
if [ "$SCRIPT" = "docs/research/fixtures/imported-procedure-expression/measure-expression.bsl" ]; then
    cp docs/research/fixtures/imported-procedure-expression/common-module.bsl "$WORK/cfg/CommonModules/ЗамерыЛенивойАрности/Ext/Module.bsl"
fi
if [ "$SCRIPT" = "docs/research/fixtures/imported-procedure-expression/measure-arguments.bsl" ]; then
    cp docs/research/fixtures/imported-procedure-expression/argument-module.bsl "$WORK/cfg/CommonModules/ЗамерыЛенивойАрности/Ext/Module.bsl"
fi

# Ошибочный дочерний модуль изолирован от остальных проб.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-initializer.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private38-unused-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-ordinary-backward-shared.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private27-unused-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private27-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-ordinary-shared-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-entry-blockend-5406.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string-frame18.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-called-invalid-entry.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-unused-invalid-entry.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-backward-bool.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-conditional.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-trampoline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-40.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-41.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-return-join-40.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-return-join-41.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-expression-target.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-dead-target.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-slot-only.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-flags-only.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-trace-baseline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-trace.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-observe.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-flag.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-overlap-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-try-body.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-unused-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-label-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-metadata.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-original.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-baseline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-procedure-queue.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-queue.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-procedure-effect.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-effect.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-path.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-path.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-return-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-procedure.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-frame.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-baseline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-unusual.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-constant.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-slot-one.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-read.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-write.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-write-observe-constant.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-direct-observation.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-arities.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-two-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-distinct-second-argument.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-reference-body.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-tail-both.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-tail.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-execute.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-execute-compiled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-execute-source.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-eval-outcome.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-eval.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-three.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-flag2.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-try-return-compiled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-try-return-common.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-transitive-unused.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-transitive-called.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-transitive-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-unknown-common-method.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-compiled-arithmetic.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-compiled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-extended-source.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-undefined.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-new.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-global.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-date.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-constant.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-value.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-add.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-year.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-branch.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-not.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-default-undefined.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-index.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-comparisons.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-external-methods.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-branch-loop.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-branch-back-edge.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-date-construction.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-loop-branch.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-omitted-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-function-early-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-conditional-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-conditional-expression.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-external-omitted.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-external-omitted.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-operator-snapshots.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-builtin-snapshots.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-upper-snapshot.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-counted-loop-exits.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-goto-labels-local.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-goto-labels-both.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-goto-labels-root.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-boolean-skips.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-goto-backward.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-unreachable-cycle.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-opcode32-values.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-number-conversion.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-opcode32-snapshot.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-method-flag2.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-method-flag3.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-opcode32.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-short-circuit32.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-constant-short-circuit.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-next-jumps.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-unreachable-probe.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-jump-true.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-no-lines.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-label-tables.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-label-tables.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-defaults-small.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-local-var-small.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-local-var.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-proc-small.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-const-small.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-cmd-small.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-proc.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-var.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-const.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-table-count-cmd.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scsu.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-mixed-conditional.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-length-trim-snapshots.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-external-function-hole.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-string-snapshot.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-remaining-string-snapshots.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-expression-snapshot.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-statement-try.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-counted-limit-call.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-while-continue.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-while-break.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-break.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-compact-break.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-continue.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-terminal-branches.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-backedge-trampoline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-branch-trampoline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-no-line-trampoline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-backward-tail.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-renamed-iterator.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scalar-builtins.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-nine-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-discarded-results.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-local-omitted-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-boolean-defaults.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-division.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-min-max.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-day-time-boundaries.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-short-date-boundaries.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-type-functions.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-early-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-exit-no-line.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-compact-return.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-dead-join.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-dead-condition.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-shared-loop.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-empty-guard.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-left.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-backward-bool.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-function-guard.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-format.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-while-context-branch.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-function-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-constructor-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-local-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-procedures.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-date-boundaries.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-call.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-one.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-string-replace.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-foreach-branch.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-procedure-loop.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-context-zero.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-while.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-local-function.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-export-procedure.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-local-call.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-string.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-method.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-property.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-property-write.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-method-write.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-call-order.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-transitive-unused/. "$WORK/cfg/"
    for module in ЗамерыТранзитивногоРодителя ЗамерыТранзитивногоДочернего; do
        if ! grep -q "<CommonModule>$module</CommonModule>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\\n\\t\\t\\t<CommonModule>$module</CommonModule>|" "$WORK/cfg/Configuration.xml"
        fi
    done
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-transitive-arguments.bsl" ]; then
        cp tests/conformance/measure/1c/transitive-arguments-parent-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/transitive-arguments-child-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоДочернего/Ext/Module.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-execute.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-procedure-execute.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-execute-compiled.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-execute-compiled.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-execute-source.bsl" ]; then
        cp tests/conformance/measure/1c/native-image-execute-control-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-eval.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-eval-outcome.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-procedure-eval.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-three.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-procedure-three.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-flags-only.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-flags-only.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-slot-only.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-slot-only.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-trace-baseline.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-trace-baseline.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-trace.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-trace.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-observe.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-observe.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-variable-flag.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-variable-flag.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-overlap-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-overlap-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-baseline.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-return-baseline.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-two-arguments.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-frame-no-defaults-two-arguments.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-distinct-second-argument.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-frame-no-defaults-distinct-second-argument.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults-arities.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-frame-no-defaults-arities.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-direct-observation.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-alias-direct-observation.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-write-observe-constant.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-alias-write-observe-constant.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-no-defaults.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-frame-no-defaults.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-alias-write.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-alias-write.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-frame-read.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-frame-read.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-slot-one.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-return-slot-one.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-constant.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-return-constant.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-return-unusual.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-return-unusual.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-label-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-label-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-private-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-initializer.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-initializer.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private38-unused-context.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private38-unused-context.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-ordinary-backward-shared.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-ordinary-backward-shared.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private27-unused-context.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private27-unused-context.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private27-context.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private27-context.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-ordinary-shared-context.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-ordinary-shared-context.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-entry-blockend-5406.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-entry-blockend-5406.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string-frame18.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-procedure-27-string-frame18.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27-string.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-procedure-27-string.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-27.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-procedure-27.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-called-invalid-entry.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-called-invalid-entry.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-unused-invalid-entry.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-unused-invalid-entry.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-backward-bool.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-backward-bool.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-conditional.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-foreach-outside-conditional.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-outside-trampoline.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-foreach-outside-trampoline.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-40.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-40.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-infeasible-call-edge-41.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-infeasible-call-edge-41.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-return-join-40.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-return-join-40.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-return-join-41.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-return-join-41.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-expression-target.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-foreach-expression-target.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-dead-target.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-foreach-dead-target.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-scoped-foreach-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-scoped-foreach-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-metadata.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-twelve-frame-entry-metadata.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-original.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-twelve-frame-entry-original.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-entry-baseline.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-twelve-frame-entry-baseline.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-procedure-queue.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-procedure-queue.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-queue.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-procedure-queue.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-procedure-effect.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-procedure-effect.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-procedure-effect.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-procedure-effect.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-path.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-private-path.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-path.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-private-path.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-return-defaults.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-private-return-defaults.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-return.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-private-return.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-return.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-private-return.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-zero-label-private-procedure.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-zero-label-private-procedure.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-private-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-nonzero-label-private-defaults.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-nonzero-label-private-defaults.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-unused-frame.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-unused-frame.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-try-body.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-try-body.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-reference-body.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-reference-body.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-tail-both.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-tail-both.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-shared-tail.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-shared-tail.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-private-procedure-flag2.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-private-procedure-flag2.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-try-return-compiled.bsl" ]; then
        rm -f "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/native-image-try-return-compiled.Module.bin \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-try-return-common.bsl" ]; then
        cp tests/conformance/measure/1c/native-image-try-return-control-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image.bsl" ]; then
        cp tests/conformance/measure/1c/native-image-control-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-extended-source.bsl" ]; then
        cp tests/conformance/measure/1c/native-image-extended-control-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
    fi
    case "$SCRIPT" in
        tests/conformance/measure/1c/measure-native-image-undefined.bsl|\
        tests/conformance/measure/1c/measure-native-image-new.bsl|\
        tests/conformance/measure/1c/measure-native-image-global.bsl|\
        tests/conformance/measure/1c/measure-native-image-date.bsl|\
        tests/conformance/measure/1c/measure-native-image-compiled-arithmetic.bsl|\
        tests/conformance/measure/1c/measure-native-image-compiled.bsl|\
        tests/conformance/measure/1c/measure-native-image-constant.bsl|\
        tests/conformance/measure/1c/measure-native-image-reference.bsl|\
        tests/conformance/measure/1c/measure-native-image-value.bsl|\
        tests/conformance/measure/1c/measure-native-image-add.bsl|\
        tests/conformance/measure/1c/measure-native-image-year.bsl|\
        tests/conformance/measure/1c/measure-native-image-branch.bsl|\
        tests/conformance/measure/1c/measure-native-image-not.bsl|\
        tests/conformance/measure/1c/measure-native-image-default-undefined.bsl|\
        tests/conformance/measure/1c/measure-native-image-index.bsl|\
        tests/conformance/measure/1c/measure-native-image-comparisons.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-external-methods.bsl|\
        tests/conformance/measure/1c/measure-native-image-branch-loop.bsl|\
        tests/conformance/measure/1c/measure-native-image-branch-back-edge.bsl|\
        tests/conformance/measure/1c/measure-native-image-date-construction.bsl|\
        tests/conformance/measure/1c/measure-native-image-loop-branch.bsl|\
        tests/conformance/measure/1c/measure-native-image-omitted-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-function-early-return.bsl|\
        tests/conformance/measure/1c/measure-native-image-conditional-return.bsl|\
        tests/conformance/measure/1c/measure-native-image-conditional-expression.bsl|\
        tests/conformance/measure/1c/measure-native-image-external-omitted.bsl|\
        tests/conformance/measure/1c/measure-native-image-external-omitted.bsl|\
        tests/conformance/measure/1c/measure-native-image-operator-snapshots.bsl|\
        tests/conformance/measure/1c/measure-native-image-builtin-snapshots.bsl|\
        tests/conformance/measure/1c/measure-native-image-upper-snapshot.bsl|\
        tests/conformance/measure/1c/measure-native-image-counted-loop-exits.bsl|\
        tests/conformance/measure/1c/measure-native-image-goto-labels-local.bsl|\
        tests/conformance/measure/1c/measure-native-image-goto-labels-both.bsl|\
        tests/conformance/measure/1c/measure-native-image-goto-labels-root.bsl|\
        tests/conformance/measure/1c/measure-native-image-boolean-skips.bsl|\
        tests/conformance/measure/1c/measure-native-image-goto-backward.bsl|\
        tests/conformance/measure/1c/measure-native-image-unreachable-cycle.bsl|\
        tests/conformance/measure/1c/measure-native-image-opcode32-values.bsl|\
        tests/conformance/measure/1c/measure-native-image-number-conversion.bsl|\
        tests/conformance/measure/1c/measure-native-image-opcode32-snapshot.bsl|\
        tests/conformance/measure/1c/measure-native-image-method-flag2.bsl|\
        tests/conformance/measure/1c/measure-native-image-method-flag3.bsl|\
        tests/conformance/measure/1c/measure-native-image-opcode32.bsl|\
        tests/conformance/measure/1c/measure-native-image-short-circuit32.bsl|\
        tests/conformance/measure/1c/measure-native-image-constant-short-circuit.bsl|\
        tests/conformance/measure/1c/measure-native-image-next-jumps.bsl|\
        tests/conformance/measure/1c/measure-native-image-unreachable-probe.bsl|\
        tests/conformance/measure/1c/measure-native-image-jump-true.bsl|\
        tests/conformance/measure/1c/measure-native-image-no-lines.bsl|\
        tests/conformance/measure/1c/measure-native-image-label-tables.bsl|\
        tests/conformance/measure/1c/measure-native-image-label-tables.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-defaults-small.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-defaults.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-local-var-small.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-local-var.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-proc-small.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-const-small.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-cmd-small.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-proc.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-var.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-const.bsl|\
        tests/conformance/measure/1c/measure-native-image-table-count-cmd.bsl|\
        tests/conformance/measure/1c/measure-native-image-scsu.bsl|\
        tests/conformance/measure/1c/measure-native-image-mixed-conditional.bsl|\
        tests/conformance/measure/1c/measure-native-image-length-trim-snapshots.bsl|\
        tests/conformance/measure/1c/measure-native-image-external-function-hole.bsl|\
        tests/conformance/measure/1c/measure-native-image-external-function-nstr-hole.bsl|\
        tests/conformance/measure/1c/measure-native-image-string-snapshot.bsl|\
        tests/conformance/measure/1c/measure-native-image-remaining-string-snapshots.bsl|\
        tests/conformance/measure/1c/measure-native-image-expression-snapshot.bsl|\
        tests/conformance/measure/1c/measure-native-image-statement-try.bsl|\
        tests/conformance/measure/1c/measure-native-image-counted-limit-call.bsl|\
        tests/conformance/measure/1c/measure-native-image-while-continue.bsl|\
        tests/conformance/measure/1c/measure-native-image-while-break.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-break.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-compact-break.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-continue.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-terminal-branches.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-backedge-trampoline.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-branch-trampoline.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-no-line-trampoline.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-backward-tail.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-renamed-iterator.bsl|\
        tests/conformance/measure/1c/measure-native-image-scalar-builtins.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-nine-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-discarded-results.bsl|\
        tests/conformance/measure/1c/measure-native-image-local-omitted-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-boolean-defaults.bsl|\
        tests/conformance/measure/1c/measure-native-image-division.bsl|\
        tests/conformance/measure/1c/measure-native-image-min-max.bsl|\
        tests/conformance/measure/1c/measure-native-image-day-time-boundaries.bsl|\
        tests/conformance/measure/1c/measure-native-image-short-date-boundaries.bsl|\
        tests/conformance/measure/1c/measure-native-image-type-functions.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-early-return.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-exit-no-line.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-compact-return.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-dead-join.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-dead-condition.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-shared-loop.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-empty-guard.bsl|\
        tests/conformance/measure/1c/measure-native-image-left.bsl|\
        tests/conformance/measure/1c/measure-native-image-backward-bool.bsl|\
        tests/conformance/measure/1c/measure-native-image-function-guard.bsl|\
        tests/conformance/measure/1c/measure-native-image-format.bsl|\
        tests/conformance/measure/1c/measure-native-image-while-context-branch.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-function-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-constructor-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-local-arguments.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-procedures.bsl|\
        tests/conformance/measure/1c/measure-native-image-date-boundaries.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-call.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-one.bsl|\
        tests/conformance/measure/1c/measure-native-image-string-replace.bsl|\
        tests/conformance/measure/1c/measure-native-image-foreach-branch.bsl|\
        tests/conformance/measure/1c/measure-native-image-procedure-loop.bsl|\
        tests/conformance/measure/1c/measure-native-image-context-zero.bsl|\
        tests/conformance/measure/1c/measure-native-image-while.bsl|\
        tests/conformance/measure/1c/measure-native-image-local-function.bsl|\
        tests/conformance/measure/1c/measure-native-image-export-procedure.bsl|\
        tests/conformance/measure/1c/measure-native-image-local-call.bsl|\
        tests/conformance/measure/1c/measure-native-image-string.bsl|\
        tests/conformance/measure/1c/measure-native-image-method.bsl|\
        tests/conformance/measure/1c/measure-native-image-property.bsl|\
        tests/conformance/measure/1c/measure-native-image-property-write.bsl|\
        tests/conformance/measure/1c/measure-native-image-method-write.bsl|\
        tests/conformance/measure/1c/measure-native-image-call-order.bsl)
            image_name=${SCRIPT##*/}
            image_name=${image_name#measure-}
            image_name=${image_name%.bsl}
            cp "${ONEC_CONTROL_MODULE_BIN:-tests/conformance/measure/1c/$image_name.Module.bin}" \
                "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bin"
            rm "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
            ;;
    esac
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-unknown-common-method.bsl" ]; then
        cp tests/conformance/measure/1c/unknown-common-method-parent-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоРодителя/Ext/Module.bsl"
        cp tests/conformance/measure/1c/unknown-common-method-child-module.bsl \
            "$WORK/cfg/CommonModules/ЗамерыТранзитивногоДочернего/Ext/Module.bsl"
    fi
fi

# Неверный конструктор нужен только двум пробам ленивой компиляции:
# остальные замеры должны получать исходный исправный общий модуль.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-unknown-constructor-lazy.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-unknown-constructor-unused.bsl" ]; then
    cat >> "$WORK/cfg/CommonModules/ЗамерыЛенивойАрности/Ext/Module.bsl" <<'EOF'

Функция НеизвестныйКонструктор() Экспорт
	Возврат Новый НеСуществующийТип();
КонецФункции
EOF
fi

# Минимальный справочник нужен пробам временных таблиц и ссылок менеджера.
# Запускайте её на отдельной `ONEC_IB`, чтобы состав остальных баз замеров
# не зависел от уже загруженных дополнительных объектов.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute-types.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute-parent.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-name.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-selection-parent.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-selection-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-selection-populated.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-selection-empty.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-temp-tables-results.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-reference-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-reference-display.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-query-catalog-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-item.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-object-instance.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-catalog-module-initializer.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-catalog-module-initializer-source.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-object-properties.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-new-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-folder-flag.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-deletion-mark.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-items.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-variables.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-uninitialized.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references-reversed.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-empty-document-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-parent.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-parent-hierarchical.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-item-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-assign-string.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-assign-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-reference-storage.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-storage-reference-serialization.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-storage-reference-prefix.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-context-priority.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-own-declaration.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Catalog>ЗамерВременныхТаблиц</Catalog>|' \
            "$WORK/cfg/Configuration.xml"
    fi
fi

# Проверка привилегированного чтения справочника и регистра только в собственной базе.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights-setup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights.bsl" ]; then
    if [ "$IB" != /tmp/open-bsl-functional-option-source-rights-1009-ib ]; then
        echo "для этого замера требуется собственная /tmp/open-bsl-functional-option-source-rights-1009-ib" >&2
        exit 1
    fi
    mkdir -p "$WORK/cfg/Catalogs" "$WORK/cfg/InformationRegisters" "$WORK/cfg/FunctionalOptions" "$WORK/cfg/FunctionalOptionsParameters"
    cp tests/conformance/measure/1c/functional-option-catalog.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp tests/conformance/measure/1c/functional-option-register.xml "$WORK/cfg/InformationRegisters/ОпцииПробы.xml"
    cp tests/conformance/measure/1c/functional-option-catalog-parameter.xml "$WORK/cfg/FunctionalOptionsParameters/ОбъектОпции.xml"
    for option in ОпцияСправочника ОпцияСправочникаОбычная ОпцияРегистра ОпцияРегистраОбычная; do
        cp "tests/conformance/measure/1c/functional-option-$option.xml" "$WORK/cfg/FunctionalOptions/$option.xml"
        if ! grep -q "<FunctionalOption>$option</FunctionalOption>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n<FunctionalOption>$option</FunctionalOption>|" "$WORK/cfg/Configuration.xml"
        fi
    done
    for parameter in ПараметрПервый ПараметрВторой; do
        cp "tests/conformance/measure/1c/functional-option-$parameter.xml" "$WORK/cfg/FunctionalOptionsParameters/$parameter.xml"
        if ! grep -q "<FunctionalOptionsParameter>$parameter</FunctionalOptionsParameter>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n<FunctionalOptionsParameter>$parameter</FunctionalOptionsParameter>|" "$WORK/cfg/Configuration.xml"
        fi
    done
    for entry in '<Catalog>ЗамерВременныхТаблиц</Catalog>' '<InformationRegister>ОпцииПробы</InformationRegister>' '<FunctionalOptionsParameter>ОбъектОпции</FunctionalOptionsParameter>'; do
        if ! grep -q "$entry" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n$entry|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Непериодический регистр с двумя измерениями для функциональных опций.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-register.bsl" ]; then
    mkdir -p "$WORK/cfg/InformationRegisters" "$WORK/cfg/FunctionalOptions" "$WORK/cfg/FunctionalOptionsParameters"
    cp tests/conformance/measure/1c/functional-option-register.xml "$WORK/cfg/InformationRegisters/ОпцииПробы.xml"
    for option in ОпцияРегистра ЧисловаяОпцияРегистра; do
        cp "tests/conformance/measure/1c/functional-option-$option.xml" "$WORK/cfg/FunctionalOptions/$option.xml"
    done
    for parameter in ПараметрПервый ПараметрВторой; do
        cp "tests/conformance/measure/1c/functional-option-$parameter.xml" "$WORK/cfg/FunctionalOptionsParameters/$parameter.xml"
    done
    sed -i 's|<ChildObjects>|<ChildObjects>\n<InformationRegister>ОпцииПробы</InformationRegister>\n<FunctionalOption>ОпцияРегистра</FunctionalOption>\n<FunctionalOption>ЧисловаяОпцияРегистра</FunctionalOption>\n<FunctionalOptionsParameter>ПараметрПервый</FunctionalOptionsParameter>\n<FunctionalOptionsParameter>ПараметрВторой</FunctionalOptionsParameter>|' "$WORK/cfg/Configuration.xml"
fi

# Параметризованные опции над реквизитами собственного справочника.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-catalog.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs" "$WORK/cfg/FunctionalOptions" "$WORK/cfg/FunctionalOptionsParameters"
    cp tests/conformance/measure/1c/functional-option-catalog.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp tests/conformance/measure/1c/functional-option-catalog-parameter.xml "$WORK/cfg/FunctionalOptionsParameters/ОбъектОпции.xml"
    for option in ОпцияСправочника ЧисловаяОпцияСправочника; do
        cp "tests/conformance/measure/1c/functional-option-$option.xml" "$WORK/cfg/FunctionalOptions/$option.xml"
    done
    sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>\n<FunctionalOption>ОпцияСправочника</FunctionalOption>\n<FunctionalOption>ЧисловаяОпцияСправочника</FunctionalOption>\n<FunctionalOptionsParameter>ОбъектОпции</FunctionalOptionsParameter>|' "$WORK/cfg/Configuration.xml"
fi

# Функциональные опции над константой создаются только в отдельной базе замера.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-content.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-constant.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-arguments.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-option-full-name.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-option-full-name-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-rights-setup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-rights.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights-setup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights.bsl" ]; then
    mkdir -p "$WORK/cfg/Constants" "$WORK/cfg/FunctionalOptions"
    cp tests/conformance/measure/1c/functional-option-constant.xml "$WORK/cfg/Constants/ЗначениеОпцииПробы.xml"
    for option in ОпцияПробы ОпцияПробыПривилегированная; do
        cp "tests/conformance/measure/1c/functional-option-$option.xml" "$WORK/cfg/FunctionalOptions/$option.xml"
    done
    for entry in '<Constant>ЗначениеОпцииПробы</Constant>' '<FunctionalOption>ОпцияПробы</FunctionalOption>' '<FunctionalOption>ОпцияПробыПривилегированная</FunctionalOption>'; do
        if ! grep -q "$entry" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n$entry|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Непустой состав опций проверяется в собственной базе без прикладных записей.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-content.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs"
    cp tests/conformance/measure/1c/functional-option-catalog.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
    for option in ОпцияПробы ОпцияПробыПривилегированная; do
        cp "tests/conformance/measure/1c/functional-option-content-$option.xml" "$WORK/cfg/FunctionalOptions/$option.xml"
    done
fi

# Роли для отдельных аутентифицированных сеансов измерения опций.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-rights-setup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-rights.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights-setup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-functional-option-source-rights.bsl" ]; then
    for role in FunctionalAdmin FunctionalDenied; do
        mkdir -p "$WORK/cfg/Roles/$role/Ext"
        cp "tests/conformance/measure/1c/functional-option-$role.xml" "$WORK/cfg/Roles/$role.xml"
        cp "tests/conformance/measure/1c/functional-option-$role.Rights.xml" "$WORK/cfg/Roles/$role/Ext/Rights.xml"
        if ! grep -q "<Role>$role</Role>" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n<Role>$role</Role>|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Общий реквизит проверяется отдельно от собственных реквизитов справочника.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-runtime.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-common-attributes.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-full-name.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-full-name-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-properties.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-properties-auto.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-properties-separate.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-infobase-user-data-separation-values.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-current-user-separation-snapshot.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-data-separator-use-before-value.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-shared.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number-shared.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-surface.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-lookup.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order-reversed.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-runtime-auto.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    mkdir -p "$WORK/cfg/CommonAttributes" "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-owner-first.xml "$WORK/cfg/Catalogs/ВладелецПробы.xml"
    cp tests/conformance/measure/1c/common-attribute-runtime.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-properties-separate.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-infobase-user-data-separation-values.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-current-user-separation-snapshot.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-data-separator-use-before-value.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-shared.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number-shared.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-surface.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-lookup.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order-reversed.bsl" ]; then
        cp tests/conformance/measure/1c/common-attribute-metadata-separate.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        mkdir -p "$WORK/cfg/SessionParameters"
        for parameter in ЗначениеРазделителяПробы ИспользованиеРазделителяПробы; do
            cp "tests/conformance/measure/1c/common-attribute-metadata-$parameter.xml" "$WORK/cfg/SessionParameters/$parameter.xml"
        done
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<SessionParameter>ЗначениеРазделителяПробы</SessionParameter>\n\t\t\t<SessionParameter>ИспользованиеРазделителяПробы</SessionParameter>|' "$WORK/cfg/Configuration.xml"
        if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-data-separator-use-before-value.bsl" ]; then
            mkdir -p "$WORK/cfg/Ext"
            cp tests/conformance/measure/1c/separator-use-session-module.bsl "$WORK/cfg/Ext/SessionModule.bsl"
            cp tests/conformance/measure/1c/separator-use-ordinary-parameter.xml "$WORK/cfg/SessionParameters/ОбычныйПараметрПробы.xml"
            sed -i 's|<ChildObjects>|<ChildObjects>\n<SessionParameter>ОбычныйПараметрПробы</SessionParameter>|' "$WORK/cfg/Configuration.xml"
        fi
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-runtime-auto.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-properties-auto.bsl" ]; then
        cp tests/conformance/measure/1c/common-attribute-runtime-auto.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-surface.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-content-lookup.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order-reversed.bsl" ]; then
        cp tests/conformance/measure/1c/common-attribute-metadata-condition.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        mkdir -p "$WORK/cfg/Constants"
        cp tests/conformance/measure/1c/functional-option-constant.xml "$WORK/cfg/Constants/ЗначениеОпцииПробы.xml"
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Constant>ЗначениеОпцииПробы</Constant>|' "$WORK/cfg/Configuration.xml"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order-reversed.bsl" ]; then
        cp tests/conformance/measure/1c/common-attribute-order-catalog.xml "$WORK/cfg/Catalogs/БСправочник.xml"
        cp tests/conformance/measure/1c/common-attribute-order-constant.xml "$WORK/cfg/Constants/АКонстанта.xml"
        if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-metadata-condition-order.bsl" ]; then
            cp tests/conformance/measure/1c/common-attribute-metadata-condition-order.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        else
            cp tests/conformance/measure/1c/common-attribute-metadata-condition-order-reversed.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        fi
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Constant>АКонстанта</Constant>\n<Catalog>БСправочник</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
    cp tests/conformance/measure/1c/common-attribute-runtime-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-shared.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number-shared.bsl" ]; then
        # Наблюдатель тела другого контроля не участвует в создании строк областей.
        : > "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
        cp tests/conformance/measure/1c/common-attribute-conditional-separation.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number.bsl" ]; then
            cp tests/conformance/measure/1c/common-attribute-conditional-separation-number.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
            cp tests/conformance/measure/1c/conditional-separation-number-parameter.xml "$WORK/cfg/SessionParameters/ЗначениеРазделителяПробы.xml"
        fi
        if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-shared.bsl" ]; then
            cp tests/conformance/measure/1c/common-attribute-conditional-separation-shared.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
        fi
        if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-common-attribute-conditional-separation-number-shared.bsl" ]; then
            cp tests/conformance/measure/1c/common-attribute-conditional-separation-number-shared.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
            cp tests/conformance/measure/1c/conditional-separation-number-parameter.xml "$WORK/cfg/SessionParameters/ЗначениеРазделителяПробы.xml"
        fi
        mkdir -p "$WORK/cfg/Constants"
        cp tests/conformance/measure/1c/functional-option-constant.xml "$WORK/cfg/Constants/ЗначениеОпцииПробы.xml"
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Constant>ЗначениеОпцииПробы</Constant>|' "$WORK/cfg/Configuration.xml"
    fi
    sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Catalog>ЗамерВременныхТаблиц</Catalog>\n\t\t\t<Catalog>ВладелецПробы</Catalog>\n\t\t\t<CommonAttribute>НаименованиеЯзык1</CommonAttribute>|' "$WORK/cfg/Configuration.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute-types.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-find-attribute-types.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute-parent.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-find-attribute-parent.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-find-attribute.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-find-attribute.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-selection-parent.bsl" ]; then
    sed -i 's|<Hierarchical>false</Hierarchical>|<Hierarchical>true</Hierarchical>|' \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-value-storage-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-value-storage.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references-reversed.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-document-references-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-get-ref.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-document-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-default.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-empty-document-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-body-flags.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-conversion-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-whitespace.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-attribute-body.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-references.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-attribute-conversion.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-primitives.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-context.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-cancel-coercion.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-replace.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-explicit.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-no-handler.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-empty-two.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-empty.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-three.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-three.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-fixed.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-string.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-disabled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-disabled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-new-reference.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-disabled.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-object-properties-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-body-flags.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-body-flags-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-three.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-three-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-three.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-three-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-empty.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-empty-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-empty-two.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-empty-two-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-attributes-no-handler.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-no-handler-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references-reversed.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-mixed-references-reversed.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-mixed-references.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-mixed-references.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-document-references-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-document-references.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-document-references.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-references.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-references.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-references-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-default.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-enumerations.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-default.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-enumerations-default.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-default.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-enumerations.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-manager.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/predefined-full-name-1017/measure-russian.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/predefined-full-name-1017/measure-english.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference-russian.bsl" ]; then
    mkdir -p "$WORK/cfg/Enums"
    cp tests/conformance/measure/1c/document-attribute-enum.xml "$WORK/cfg/Enums/ЗамерПеречисления.xml"
    if ! grep -q '<Enum>ЗамерПеречисления</Enum>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Enum>ЗамерПеречисления</Enum>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "docs/research/fixtures/predefined-full-name-1017/measure-english.bsl" ]; then
    sed -i 's|<ScriptVariant>Russian</ScriptVariant>|<ScriptVariant>English</ScriptVariant>|' "$WORK/cfg/Configuration.xml"
fi

if [ "$SCRIPT" = "docs/research/fixtures/contextual-string-operations-1017/measure-array-russian-script.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/predefined-full-name-1017/measure-russian.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/contextual-string-operations-1017/measure-count-russian-script.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/contextual-string-operations-1017/measure-compare-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-full-name-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-metadata-option-full-name-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference-russian-script.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference-russian.bsl" ]; then
    sed -i 's|<ScriptVariant>English</ScriptVariant>|<ScriptVariant>Russian</ScriptVariant>|' "$WORK/cfg/Configuration.xml"
fi
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-configuration-enum-reference-russian.bsl" ]; then
    mkdir -p "$WORK/cfg/Languages"
    cp tests/conformance/measure/1c/configuration-enum-russian-language.xml "$WORK/cfg/Languages/Русский.xml"
    sed -i 's|<DefaultLanguage>[^<]*</DefaultLanguage>|<DefaultLanguage>Language.Русский</DefaultLanguage>|' "$WORK/cfg/Configuration.xml"
    if ! grep -q '<Language>Русский</Language>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<Language>Русский</Language>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-primitives.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-attribute-conversion.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-primitives.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-primitives-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-cancel-coercion.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/catalog-cancel-coercion-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-context.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-object-context-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-replace.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-replace-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-explicit.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-attributes-no-handler-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-attribute-body.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-attribute-body-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-whitespace.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-attributes.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp tests/conformance/measure/1c/document-fill-whitespace-module.bsl \
        "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
fi

# Настройки номера изменяются только в изолированной конфигурации замера.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-conversion-number.bsl" ]; then
    sed -i 's|<NumberType>String</NumberType>|<NumberType>Number</NumberType>|' \
        "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-properties-disabled.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-disabled.bsl" ]; then
    sed -i 's|<NumberLength>9</NumberLength>|<NumberLength>0</NumberLength><InputByString/>|' \
        "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-parent.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-parent-hierarchical.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-new-reference.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-parent-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    # Вторая ссылка нужна для проверки присваивания чужого типа справочника.
    python3 - "$WORK/cfg" <<'PY_PARENT'
import pathlib
import re
import sys
import uuid
cfg = pathlib.Path(sys.argv[1])
source = cfg / "Catalogs/ЗамерВременныхТаблиц.xml"
text = source.read_text().replace("ЗамерВременныхТаблиц", "ЗамерЧужойСсылки")
text = re.sub(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}",
              lambda match: str(uuid.uuid5(uuid.NAMESPACE_URL, "open-bsl-parent/" + match.group())), text)
(cfg / "Catalogs/ЗамерЧужойСсылки.xml").write_text(text)
root = cfg / "Configuration.xml"
root.write_text(root.read_text().replace("<ChildObjects>", "<ChildObjects>\n\t\t\t<Catalog>ЗамерЧужойСсылки</Catalog>", 1))
PY_PARENT
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-parent-hierarchical.bsl" ]; then
        sed -i 's|<Hierarchical>false</Hierarchical>|<Hierarchical>true</Hierarchical>|' \
            "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-catalog-module-initializer.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/native-image-module-initializer.Module.bin \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bin"
    rm -f "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-catalog-module-initializer-source.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/native-image-module-initializer-source.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-object-instance.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-object-instance-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-name.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-items.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-variables.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-uninitialized.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-predefined-items.xml \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/Predefined.xml"
    cp tests/conformance/measure/1c/catalog-predefined-manager.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ManagerModule.bsl"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-variables.bsl" ]; then
        cp tests/conformance/measure/1c/catalog-predefined-variable-manager.bsl \
            "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ManagerModule.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-uninitialized.bsl" ]; then
        sed -i 's|<PredefinedDataUpdate>Auto</PredefinedDataUpdate>|<PredefinedDataUpdate>DontAutoUpdate</PredefinedDataUpdate>|' \
            "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-name.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-predefined-name-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-deletion-mark.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-deletion-mark-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-folder-flag.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-folder-flag-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-new-reference.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-new-reference-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-object-properties.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-object-properties-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

# Числовой код и отключённое наименование проверяются отдельно от строковой пробы.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-item-number.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-assign-number.bsl" ]; then
    sed -i 's|<CodeType>String</CodeType>|<CodeType>Number</CodeType>|; s|<DescriptionLength>25</DescriptionLength>|<DescriptionLength>0</DescriptionLength>|; s|<DefaultPresentation>AsDescription</DefaultPresentation>|<DefaultPresentation>AsCode</DefaultPresentation>|' \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    sed -i '/\.StandardAttribute\.Description<\/xr:Field>/d' \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-context-priority.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-own-declaration.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-manager-context/. "$WORK/cfg/"
    if ! grep -q '<CommonModule>ЗамерКонтекстаМенеджера</CommonModule>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<CommonModule>ЗамерКонтекстаМенеджера</CommonModule>|' \
            "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-own-declaration.bsl" ]; then
    cat >> "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ManagerModule.bsl" <<'EOF'

Функция НайтиПоКоду(Код)
    Возврат "own";
КонецФункции
EOF
fi

# Контекст менеджера измеряется на том же минимальном справочнике;
# исходный модуль добавляется только в отдельную базу этой пробы.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-manager-references.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/manager-references-module.bsl \
        "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ManagerModule.bsl"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-value-storage-preserve.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-fill-value-storage.bsl" ]; then
    cp tests/conformance/measure/1c/document-fill-value-storage.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-object-assignment-fixed.bsl" ]; then
    sed -i 's|<NumberAllowedLength>Variable</NumberAllowedLength>|<NumberAllowedLength>Fixed</NumberAllowedLength>|' "$WORK/cfg/Documents/ЗамерДокумента.xml"
fi

# Коллизия свойства собственного получателя и общего модуля измеряется отдельно.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-receiver-alias-collision.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    mkdir -p "$WORK/cfg/CommonModules/ДополнительныеСвойства/Ext" "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/receiver-alias-collision-common.xml "$WORK/cfg/CommonModules/ДополнительныеСвойства.xml"
    cp tests/conformance/measure/1c/receiver-alias-collision-common.bsl "$WORK/cfg/CommonModules/ДополнительныеСвойства/Ext/Module.bsl"
    cp tests/conformance/measure/1c/receiver-alias-collision-object.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    for ENTRY in '<Catalog>ЗамерВременныхТаблиц</Catalog>' '<CommonModule>ДополнительныеСвойства</CommonModule>'; do
        if ! grep -q "$ENTRY" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n$ENTRY|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Проверка заполнения создаёт объект только в памяти, без записи в базу.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages-ru.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages-code.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages-code-ru.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym-ru.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-scalars.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-common.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-handler.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-cancel-coercion.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-check-filling-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-cancel-coercion.bsl" ]; then
        cp tests/conformance/measure/1c/catalog-cancel-coercion-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-handler.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages-code.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-messages-code-ru.bsl" ]; then
        cp tests/conformance/measure/1c/catalog-check-filling-handler-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute-synonym-ru.bsl" ]; then
        cp tests/conformance/measure/1c/catalog-check-filling-own-attribute.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
        cp tests/conformance/measure/1c/catalog-check-filling-own-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
        if [ "$SCRIPT" != "tests/conformance/measure/1c/measure-catalog-check-filling-own-attribute.bsl" ]; then
            cp tests/conformance/measure/1c/catalog-check-filling-own-attribute-synonym.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
        fi
    fi
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-common.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-check-filling-own-attribute.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp tests/conformance/measure/1c/catalog-check-filling-handler-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    mkdir -p "$WORK/cfg/CommonAttributes"
    cp tests/conformance/measure/1c/common-attribute-check-filling.xml "$WORK/cfg/CommonAttributes/НаименованиеЯзык1.xml"
    if ! grep -q '<CommonAttribute>НаименованиеЯзык1</CommonAttribute>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<CommonAttribute>НаименованиеЯзык1</CommonAttribute>|' "$WORK/cfg/Configuration.xml"
    fi
fi

if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-check-filling-scalars.bsl" ]; then
    cp tests/conformance/measure/1c/catalog-check-filling-scalar-attributes.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp tests/conformance/measure/1c/catalog-check-filling-handler-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
fi

# Создание группы проверяется в четырёх сочетаниях признака и вида иерархии.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-hierarchical.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-flat.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-flat-items.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-items.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-folder-flag-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
    if [ "$SCRIPT" != "tests/conformance/measure/1c/measure-catalog-create-folder-flat.bsl" ] &&
       [ "$SCRIPT" != "tests/conformance/measure/1c/measure-catalog-create-folder-flat-items.bsl" ]; then
        sed -i 's|<Hierarchical>false</Hierarchical>|<Hierarchical>true</Hierarchical>|' "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-items.bsl" ] ||
       [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-create-folder-flat-items.bsl" ]; then
        sed -i 's|<HierarchyType>HierarchyFoldersAndItems</HierarchyType>|<HierarchyType>HierarchyOfItems</HierarchyType>|' "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
fi

# Признак предопределённости нового объекта проверяется без записи.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-predefined-flag.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-temp-tables/. "$WORK/cfg/"
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp tests/conformance/measure/1c/catalog-predefined-items.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/Predefined.xml"
    cp tests/conformance/measure/1c/catalog-predefined-flag-module.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
fi

# Владелец справочника измеряется без сохранения объектов и строк базы.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-owner.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-owner-multiple.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВладельца/Ext"
    cp tests/conformance/measure/1c/catalog-owner-first.xml "$WORK/cfg/Catalogs/ВладелецПробы.xml"
    cp tests/conformance/measure/1c/catalog-owner-second.xml "$WORK/cfg/Catalogs/ВторойВладелецПробы.xml"
    cp tests/conformance/measure/1c/catalog-owner-child.xml "$WORK/cfg/Catalogs/ЗамерВладельца.xml"
    if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-catalog-owner-multiple.bsl" ]; then
        cp tests/conformance/measure/1c/catalog-owner-child-multiple.xml "$WORK/cfg/Catalogs/ЗамерВладельца.xml"
    fi
    cp tests/conformance/measure/1c/catalog-owner-module.bsl "$WORK/cfg/Catalogs/ЗамерВладельца/Ext/ObjectModule.bsl"
    for ENTRY in '<Catalog>ВладелецПробы</Catalog>' '<Catalog>ВторойВладелецПробы</Catalog>' '<Catalog>ЗамерВладельца</Catalog>'; do
        if ! grep -q "$ENTRY" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n$ENTRY|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Неизвестное значение перечисления сохраняется из нативной формы ссылки.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-dangling-enum-filling.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-dangling-enum-storage.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-dangling-enum-storage-russian-ui.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-document-attribute-metadata.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents" "$WORK/cfg/Enums"
    cp tests/conformance/measure/1c/document-dangling-enum-filling.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    cp tests/conformance/measure/1c/document-attribute-enum.xml "$WORK/cfg/Enums/ЗамерПеречисления.xml"
    for ENTRY in '<Document>ЗамерДокумента</Document>' '<Enum>ЗамерПеречисления</Enum>'; do
        if ! grep -q "$ENTRY" "$WORK/cfg/Configuration.xml"; then
            sed -i "s|<ChildObjects>|<ChildObjects>\n$ENTRY|" "$WORK/cfg/Configuration.xml"
        fi
    done
fi

# Определяемый тип УНФ проверяется целиком в изолированной конфигурации.
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-unf-event-basis.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-document-reference/. "$WORK/cfg/"
    sed -i 's|<ChildObjects>|<ChildObjects>\n<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    python3 tests/conformance/measure/1c/prepare-unf-event-basis.py "$WORK/cfg" || exit 1
fi

# Поиск документа измеряется на собственной записи с откатом транзакции.
if [ "$SCRIPT" = "docs/research/fixtures/document-find-attribute/measure.bsl" ]; then
    mkdir -p "$WORK/cfg/Documents/ЗамерДокумента/Ext"
    cp docs/research/fixtures/document-find-attribute/document.source.xml "$WORK/cfg/Documents/ЗамерДокумента.xml"
    cp docs/research/fixtures/document-find-attribute/ManagerModule.bsl "$WORK/cfg/Documents/ЗамерДокумента/Ext/ManagerModule.bsl"
    : > "$WORK/cfg/Documents/ЗамерДокумента/Ext/ObjectModule.bsl"
    if ! grep -q '<Document>ЗамерДокумента</Document>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Document>ЗамерДокумента</Document>|' "$WORK/cfg/Configuration.xml"
    fi
fi

# Глобальный подсчёт объёма измеряется на собственном регистре.
if [ "$SCRIPT" = "docs/research/fixtures/database-data-size/measure.bsl" ]; then
    mkdir -p "$WORK/cfg/InformationRegisters"
    cp docs/research/fixtures/database-data-size/InformationRegister.source.xml "$WORK/cfg/InformationRegisters/ЗамерПолучить.xml"
    if ! grep -q '<InformationRegister>ЗамерПолучить</InformationRegister>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<InformationRegister>ЗамерПолучить</InformationRegister>|' "$WORK/cfg/Configuration.xml"
    fi
fi

# Уровень объекта справочника измеряется на собственной иерархии с откатом.
if [ "$SCRIPT" = "docs/research/fixtures/catalog-level/measure.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/catalog-level/measure-flat.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp docs/research/fixtures/catalog-level/Catalog.source.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp docs/research/fixtures/catalog-level/ObjectModule.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "docs/research/fixtures/catalog-level/measure-flat.bsl" ]; then
        sed -i 's|<Hierarchical>true</Hierarchical>|<Hierarchical>false</Hierarchical>|' "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
fi

# Код справочника измеряется на собственном объекте, без записи элементов.
if [ "$SCRIPT" = "docs/research/fixtures/catalog-new-code/measure.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/catalog-new-code/measure-en.bsl" ] ||
   [ "$SCRIPT" = "docs/research/fixtures/catalog-new-code/measure-numeric.bsl" ]; then
    mkdir -p "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext"
    cp docs/research/fixtures/catalog-new-code/Catalog.source.xml "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    cp docs/research/fixtures/catalog-new-code/ObjectModule.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    if [ "$SCRIPT" = "docs/research/fixtures/catalog-new-code/measure-en.bsl" ]; then
        cp docs/research/fixtures/catalog-new-code/ObjectModule-en.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
    fi
    if [ "$SCRIPT" = "docs/research/fixtures/catalog-new-code/measure-numeric.bsl" ]; then
        cp docs/research/fixtures/catalog-new-code/ObjectModule-numeric.bsl "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц/Ext/ObjectModule.bsl"
        sed -i 's|<CodeType>String</CodeType>|<CodeType>Number</CodeType>|' "$WORK/cfg/Catalogs/ЗамерВременныхТаблиц.xml"
    fi
    if ! grep -q '<Catalog>ЗамерВременныхТаблиц</Catalog>' "$WORK/cfg/Configuration.xml"; then
        sed -i 's|<ChildObjects>|<ChildObjects>\n<Catalog>ЗамерВременныхТаблиц</Catalog>|' "$WORK/cfg/Configuration.xml"
    fi
fi

# Первый прогон на этой базе должен ЗАРЕГИСТРИРОВАТЬ обработку в составе
# конфигурации; на последующих прогонах DumpConfigToFiles уже возвращает её
# в ChildObjects, и повторное добавление испортило бы XML.
if ! grep -q '<DataProcessor>Замеры</DataProcessor>' "$WORK/cfg/Configuration.xml"; then
    sed -i 's|<ChildObjects>|<ChildObjects>\n\t\t\t<DataProcessor>Замеры</DataProcessor>|' \
        "$WORK/cfg/Configuration.xml"
fi

# Ошибки инициализации контекстных проб изолированы от прочих замеров.
MODULE_XML_FILES=(tests/conformance/measure/1c/cfg-src-server/CommonModules/*.xml)
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-private37-call.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-private38-call.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-wrapper-baseline.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-original-object-load.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-declared41.bsl" ] ||
   [ "$SCRIPT" = "tests/conformance/measure/1c/measure-native-image-twelve-frame-object-linked-context.bsl" ]; then
    cp -r tests/conformance/measure/1c/native-frame-context/. "$WORK/cfg/"
    MODULE_XML_FILES+=(tests/conformance/measure/1c/native-frame-context/CommonModules/*.xml)
fi
if [ "$SCRIPT" = "tests/conformance/measure/1c/measure-module-argument-context.bsl" ]; then
    cp -r tests/conformance/measure/1c/cfg-src-module-argument-context/. "$WORK/cfg/"
    MODULE_XML_FILES+=(tests/conformance/measure/1c/cfg-src-module-argument-context/CommonModules/*.xml)
fi

# Общие модули регистрируются так же — по одному разу на базу.
for MODULE_XML in "${MODULE_XML_FILES[@]}"; do
    MODULE_NAME=$(basename "$MODULE_XML" .xml)
    if ! grep -q "<CommonModule>$MODULE_NAME</CommonModule>" "$WORK/cfg/Configuration.xml"; then
        sed -i "s|<ChildObjects>|<ChildObjects>\n\t\t\t<CommonModule>$MODULE_NAME</CommonModule>|" \
            "$WORK/cfg/Configuration.xml"
    fi
done

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
        "$ENTERPRISE_PLATFORM" ENTERPRISE /F"$IB" \
        /DisableStartupDialogs /DisableStartupMessages /Out "$WORK/run.log" >/dev/null 2>&1
RC=$?
if [ "$RC" -ne 0 ]; then
    case "$RC" in
        124)
            echo "платформа не завершилась за ${ONEC_TIMEOUT}с; timeout остановил запуск." >&2
            ;;
        137)
            echo "платформа завершилась с кодом 137 (SIGKILL); возможна принудительная остановка timeout." >&2
            ;;
        139)
            echo "платформа завершилась с кодом 139 (SIGSEGV); полного замера нет." >&2
            ;;
        *)
            echo "платформа завершилась с ошибкой, код ${RC}; полного замера нет." >&2
            ;;
    esac
    if [ -s "$WORK/run.log" ]; then
        cat "$WORK/run.log" >&2
    fi
    if [ -s "$OUT_ABS" ]; then
        echo "Вывод, записанный до остановки:" >&2
        cat "$OUT_ABS" >&2
    fi
    if [ "$RC" -eq 124 ] || [ "$RC" -eq 137 ]; then
        echo "Возможная причина зависания — необработанное исключение в скрипте," >&2
        echo "которое платформа показывает модальным окном. Заверните пробу в" >&2
        echo "Попытка/Исключение." >&2
    fi
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

# Файл с одной UTF-8-сигнатурой проходит проверку размера выше, но не содержит
# измерений. Такой исход после ошибки модуля объекта не является успешным замером.
if [ ! -s "$OUT_ABS" ]; then
    echo "после удаления сигнатуры вывод замера пуст. Лог платформы:" >&2
    cat "$WORK/run.log" >&2
    exit 1
fi

if [ "$SCRIPT" = "tests/conformance/measure/measure-all.bsl" ]; then
    DEST=tests/conformance/measure/platform.tsv
else
    DEST="${SCRIPT%.bsl}.platform.txt"
fi
cp "$OUT_ABS" "$DEST"
echo "снято строк: $(wc -l < "$DEST")  ->  $DEST"
echo
echo "сверка:  cargo run -p bsl-cli -- --ingest-measurements $DEST"
