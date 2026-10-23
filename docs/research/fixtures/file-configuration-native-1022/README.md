# Нативные файловые представления конфигурации

`native-export/` — результат `DumpConfigToFiles` платформы 1С
8.3.27.2342 из отдельной временной файловой базы
`/tmp/open-bsl-file-sources-native-1022`. Рабочие SQL-базы не изменялись.
`native-input/` — сконструированный вход для `LoadConfigFromFiles`, а не
свидетельство нативной структуры. При подготовке добавлены два собственных
общих макета и пустой обработчик модуля сеанса. Табличный макет использует
ранее измеренный `tests/conformance/mxl/template-basic.xml`.
Логи Designer и `native-template-stages.json` фиксируют успешный round-trip.

На нативном выходе текстовый макет представлен `Template.txt` с UTF-8 BOM,
табличный — `Template.xml`. Входной `Template.xml` с XDTO `TextDocument`
не задаёт текстовый макет: первый успешный проход выгрузил описание, но не
содержимое. Добавление `Template.txt` подтвердилось нативным выходом.
`probe-text-template.platform.txt` — самостоятельный замер XDTO, который
нельзя путать с форматом ресурса Configurator.

`edt-reference/` — точные файлы нативного EDT-проекта
[1C-Company/GitConverter](https://github.com/1C-Company/GitConverter), commit
`55d71967c41825256e48b9c7d2b13224a88a76e4`.
URL и SHA-256 каждого файла сохранены в `provenance.json`. Автор —
1C-Company и участники проекта; лицензия CC BY-SA 4.0, исходный LICENSE.md
приложен. Сохранены только описания, форма и данные макета; сторонний BSL
в проект open-bsl не копировался. Сокращённые конфигурации в тестах —
адаптации этих описаний, собственные BSL-файлы создаются отдельно.
Версия EDT IDE в выбранных артефактах не записана.

Артефакты GitConverter не являются выгрузкой demo/buh/unf. Для приёмки
22 октября созданы соответствующие проекты установленной EDT. После удаления
`target` полные XML/EDT-комплекты восстановлены в
`private/configuration-sources-1022/<база>/`: XML — `native-xml/`,
EDT — `edt-workspace/<ИМЯ БАЗЫ>/`. Квитанции производителей — `receipts.json`,
расширений — `extensions/receipts.json`; исходные секретные комплекты
не добавляются в Git. Ранний `matched-projects.json` сохраняет происхождение
первоначального прогона, включая прежние пути.
Проверки идентичности и запрет подстановки BSL из Config сохранены.

`buh-form/Form.xml` — неизменённая нативная форма справочника
`ЯзыкиПечатныхФорм` из ранее полученной полной выгрузки BUH. Нативный producer,
путь и SHA-256 записаны рядом; протокол самой выгрузки находится в
`../buh-business-process-attributes/full-config-native-receipts.json`.
На ней проверяются XML-обёртка реквизитов, `MainAttribute`, квалификаторы,
привязки и события. Это отдельный пример формы, не полный файловый комплект BUH.

Сопоставление квалификаторов дополняется официальным API EDT:
[NumberQualifiers 2025.2](https://edt.1c.ru/dev/edt/2025.2/apidocs/com/_1c/g5/v8/dt/mcore/NumberQualifiers.html)
прямо связывает `precision` с XML `Digits`, `scale` — с `FractionDigits`.
`nonNegative` хранит ограничение знака.
[StringQualifiers](https://edt.1c.ru/dev/edt/2024.2/apidocs/com/_1c/g5/v8/dt/mcore/StringQualifiers.html)
описывает `length` и `fixed`. Нативные `precision=10`, `nonNegative=true`
и пустые строковые/датовые квалификаторы присутствуют в сохранённых `.mdo`.
Синтетический тест паритета использует эти документированные соответствия,
а не выдаёт сгенерированный XML за нативную выгрузку EDT.

## Нативный импорт EDT 2026.1

`edt-native/` создан установленной 1C:EDT 2026.1.3+25 командой `1cedtcli import --configuration-files native-export --project-name NativeFileSources`. Версия производителя и SHA-256 файлов сохранены в `edt-native-provenance.json`. Проект не требует IDE для чтения нашими тестами. Импорт нормализует переводы строк BSL в LF и опускает `templateType` у табличного макета; парный тест XML/EDT проверяет эти артефакты.

## Типы, обнаруженные полным прогоном

`native-uuid/`, `native-calculation-reference/` и `native-exchange-plan/`
содержат парные Config/XML/EDT-свидетельства типов и реквизитов,
которые файловая проверка должна сопоставлять с ORM.
`native-edt-binary-module.json` закрепляет совпадение нативного образа
XML `Module.bin` и EDT `Module.bsl`.
`native-buh-defined-reference/` сохраняет поле
`Пользователи.Подразделение` и старое описание определяемого типа;
целевой справочник — в `native-buh-catalog-field/`.
Каждый комплект содержит происхождение и SHA-256 нативных байтов.
