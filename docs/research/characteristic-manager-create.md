# Создание элемента плана видов характеристик

Барьер compile-only проверки demo: `СоздатьЭлемент` внутри модуля менеджера
`ДополнительныеРеквизитыИСведения` не разрешается. Существующая фабрика
подключает контекст через `configuration_method_context`; для этого вида
метаданных таблица методов ORM пока пуста.

Источник: Синтакс-помощник установленной платформы **8.3.27.2342**, русский
язык, `platform-syntax-help:syntax-context:objects/catalog125/catalog433/object441/methods/CreateItem1069.html`.
Полный текст страницы получен через `unica.documentation.get`.

`СоздатьЭлемент()` не принимает аргументов и возвращает объект конкретного
плана видов характеристик. Вызов сам по себе не записывает объект в базу.
Метод доступен на сервере, в толстом клиенте и внешнем соединении.

Существующая реализация создания объекта справочника требует его стандартных
реквизитов и зарегистрированного типа объекта справочника. Простая установка
той же таблицы методов для плана видов характеристик не обеспечивает этот
контракт: тип объекта и состав реквизитов нужно приобрести и связать отдельно.
Справка не подтверждает значения стандартных реквизитов, порядок выполнения
модуля объекта или поведение `ТипЗначения`; эти вопросы требуют платформенных
замеров перед реализацией.

## Замер нового объекта

Штатный runner AGENTS.md с `DISPLAY=:0` завершился успешно на отдельной
файловой базе `/tmp/open-bsl-characteristic-measure-1004-ib`. Тестовый план
создан через типизированный шаблон Unica; его `Type` содержит булево,
строку, дату и число. Прикладные базы не использовались. Вызова `Записать`
в пробе нет. Итоговые восемь строк:

```text
characteristic.create.type=1
characteristic.create.new=1
characteristic.create.folder=0
characteristic.create.code=
characteristic.create.description=
characteristic.create.reference.empty=1
characteristic.create.value_type.count=4
characteristic.create.description.set=probe
```

Это значения конкретного тестового плана, а не подразумеваемые настройки
всех планов. Обратная нативная XML-выгрузка выполнена успешно; её оригинал, скрипт
и результат сохранены в `docs/research/fixtures/characteristic-manager-create/`.
Платформа сохранила GUID типа объекта `1d213089-a138-4a74-a64d-4bf8a415f041`
и ссылки `1945502f-7d5b-4a44-b8b5-43810c645103`; код имеет длину 9,
наименование — 25. Это ещё не подтверждает разбор соответствующего Config
ресурса SDK. Проба не измеряет
инициализацию модуля объекта, запись, чтение сохранённого элемента или
произвольные типы характеристики.

## Проверка текущего декодера SDK

CF собственной базы содержит два последовательных контейнера. Первоначальное
чтение только первого ошибочно выглядело как отсутствие объекта. Штатный
`v8unpack.container_reader.extract` прочитал оба и нашёл ресурс владельца во
втором. Его сжатые 1717 байтов сохранены рядом с XML как
`d5d8567e-ef0b-4eba-962c-1db134b6a551.config.deflate`.

Существующий `open_sdbl::metadata::parse_config_descriptors` с именем
`d5d8567e-ef0b-4eba-962c-1db134b6a551` и этими байтами вернул один описатель:
GUID и имя совпали с XML, `catalog_standard_attributes` и
`catalog_attributes` равны `None`. Это воспроизводимый контроль отсутствующих
данных, а не ошибка UTF-8 или распаковки. Протокол сохранён в
`sdk-descriptor.receipt.txt`. Настройки плана нельзя получать подстановкой
значений справочника.

## Контекст тела модуля объекта

Дополнительный успешный замер на той же собственной базе:

```text
characteristic.body.receipt=code=;description=;types=0;write=ERROR;new=ERROR;
characteristic.body.types.after=4
characteristic.body.new.after=1
characteristic.body.description.after=
```

В теле модуля код и наименование читаются как пустые строки, `ТипЗначения`
имеет ноль типов. Запись наименования и вызов `ЭтоНовый()` отвергаются.
После создания доступны четыре настроенных типа и `ЭтоНовый()=Истина`.
Поэтому настройка типов должна происходить после инициализации тела;
простая ранняя подстановка метаданных изменила бы наблюдаемое поведение.
Модуль и завершённый эталон сохранены в соседней папке
`fixtures/characteristic-manager-body/`.

## Квалификация и собственный контекст менеджера

Повторный успешный замер, без записи элемента:

```text
characteristic.qualification.manager.context=1
characteristic.qualification.code=123456789
characteristic.qualification.description=1234567890123456789012345
characteristic.qualification.type.count=1
characteristic.qualification.type.string=1
characteristic.qualification.undefined=ACCEPT
characteristic.qualification.undefined.type_description=1
characteristic.qualification.undefined.count=4
characteristic.qualification.folder.write=ERROR
```

Артефакты сохранены в `fixtures/characteristic-manager-qualification/`. Длины 9/25 соответствуют нативному XML. Присваивание `Неопределено` в `ТипЗначения` не отвергается; его результат зафиксирован отдельно, а не выведен из типа свойства.

## Facade implementation checks

The independent session tests in `crates/bsl-session/tests/characteristic_objects.rs` now replay the creation and object-module-body receipts. The characteristic module uses the existing receiver property/method context and `ConfigurationInstance`; its configured value types are installed after the module body. A separate test calls bare `CreateItem` from the manager module and verifies independent object properties and module variables in two sessions. These controlled tests do not prove successful startup of an original configuration or SDK acquisition of characteristic settings.

`docs/research/fixtures/characteristic-manager-types/` contains an additional native receipt: `TypeDescription.Types()` returns a mutable array, and clearing it leaves the original description with two types. The base runtime implements this operation in its existing builtin table.

## Exchange parameters

The independent native receipt in `fixtures/characteristic-manager-exchange/` reports loading=false, recipient auto-fill=true and separate loading flags for two new characteristic objects. The characteristic object reuses the existing ORM exchange-parameters implementation per instance. This removes the missing receiver-property declaration found by demo compile-only; another original compile run is still required to identify the next barrier.

## Builtin declaration matrix update

Adding `Types` exposed the existing exhaustive name-list assertion in `notification::tests::base_method_declarations_match_native_receiver_matrices`. Its Rust expectation was not changed. Both platform matrix scripts were extended with Russian `Типы` and English `Types` and rerun on the native platform. All previous method-mask columns are unchanged; both new columns are declared on TypeDescription and absent on the other covered receivers. The extra-receiver receipt uses native English display titles in this measurement session. The platform receipts, not interpreter-generated masks, now supply the expanded matrix.

The integration assertion `native_matrix_preserves_receiver_errors_separately_from_missing_methods` also stored fixed total cell counts. After reporting the failing assertion, its counts were updated from `[181, 1083, 158]` to the measured `[183, 1113, 162]`: 18 receivers times two new names add two declarations, thirty absent-method results and four receiver errors. The previous columns were explicitly compared with pre-run receipts and are unchanged. This is a disclosed existing-test update caused by extending the independently measured matrix, not an interpreter-generated expectation.

## Own string attribute

`fixtures/characteristic-manager-attributes/` contains a separately extended native configuration, its XML, object module, driver and receipt. The variable-length string attribute reads empty during the body, rejects writes until selection, truncates post-creation assignment to 12 characters and retains independent state in a second object. The ORM consumes an optional complete characteristic attribute list using the existing immutable `DocumentAttribute` representation and reuses the existing scalar adjustment helper for the measured string shape. No parsing of Config was added to open-bsl. Original demo's attribute type remains an SDK acquisition requirement; the test attribute name alone is not evidence of its original type.

## Value-type membership and qualification replay

The additional native receipt `characteristic-manager-types/measure-characteristic-contains-type.platform.txt` confirms true for String, false for Number, and the `ContainsType` alias on a string-only description. The base builtin table implements membership; the independent session test now replays all nine lines of `characteristic-manager-qualification`, including reset through Undefined. The declaration matrices were extended with both membership names and remeasured. Their original method-mask columns are unchanged. Fixed matrix totals are now the native `[185, 1143, 166]`, adding four declarations, sixty missing-method results and eight receiver errors relative to the original matrix. This existing-test total update is disclosed separately from behavioral assertions.

## Workspace verification (2026-10-04)

The post-implementation gate `/tmp/open-bsl-characteristic-object-gates-1004.json` records terminal success for all eight checks: formatting, workspace Clippy with warnings denied, workspace tests (370.62 seconds), rustdoc with warnings denied, facade consumer build, both normal dependency trees, and strict validation of all active OpenSpec changes. The runtime and VM trees were compared with the preceding successful gate and are identical. SDK acquisition and original configuration startup remain incomplete; the characteristic change is intentionally not archived.

## Квалификаторы типа значения

Завершённый независимый замер `fixtures/characteristic-value-type-qualifiers/` подтвердил, что `ПривестиЗначение` применяет настройки типа характеристики: строка из 120 символов становится строкой из 100, `1.234` становится `1.23`, `-1.235` — `-1.24`, а переполнение Число(15,2) насыщается значением `9999999999999.99`. Булево и дата со временем сохраняются. Для `Неопределено`, `Null` и отсутствующего аргумента возвращается `Неопределено`. Первоначально ORM сохранял только идентификаторы типов. Исправление добавляет `BslValue::new_qualified_type_description`: приобретённые квалификаторы остаются в runtime, а приведение использует существующий механизм колонок. Три новых теста runtime и один тест через фасад сеанса прошли; общий прогон workspace также прошёл, вместе с rustdoc, Clippy, сборкой потребителя и OpenSpec strict.

## Подключение SDK 0.9.7, 2026-10-05

Ветка SDK `feat/characteristic-settings-0.9.7`, коммиты `bb10b7f` и
`73d2431`, добавляет приобретённые настройки и полный упорядоченный состав
собственных реквизитов. Независимый полный прогон SDK: 833 теста прошли,
13 пропущены, ошибок нет; fmt, Clippy, rustdoc и строгая валидация OpenSpec
также прошли. Прежние результаты об отсутствии настроек относятся к SDK
до 0.9.7 и сохраняются как история исследования.

Адаптер open-bsl использует существующую проекцию `ColumnType` и
`DocumentAttribute`, включая ссылочные типы планов видов характеристик.
Три новых теста проверяют исходный нативный ресурс, длины 9/25,
квалификаторы строки 100/1 и собственного реквизита 12/1, ссылочный тип,
неизвестный и известный пустой состав, явный отказ при неизвестном типе.
Полный набор `bsl-config-db --lib`: 323 теста прошли. Тип контрольного
реквизита не подставляется в оригинальную demo.

Readonly-компиляция demo с SDK 0.9.7: конфигурация прочитана за 4,51 с,
соединение закрыто перед проекцией; BSL не исполнялся. Проверка завершилась
через 25,8 с ошибкой проекции типа значения характеристики
`1055d15b-8cb5-4ff0-a526-7fd20a08a96c`. Повторная диагностическая проверка
(23,3 с, чтение 4,50 с) установила 11 приобретённых ссылочных GUID без
соответствующего `object_reference_type` в описателях. SDK-агенту передан
запрос исследования. Пропуск этих типов и выдача неполного описания
не допускаются. Два прежних непрочитанных ресурса `.7` отмечены SDK.
Это новый подтверждённый барьер проекции, а не успешная компиляция или
инициализация параметров сеанса.

Полный прогон после подключения SDK 0.9.7 завершился: все восемь шагов
`/tmp/open-bsl-characteristic-sdk097-gates-1005.json` имеют код 0.
Workspace-тесты заняли 358,43 с; fmt, all-targets Clippy, rustdoc с
запрещёнными предупреждениями, consumer и OpenSpec также прошли.
Нормальные деревья `bsl-rt` и `bsl-vm` совпадают с предыдущим успешным
прогоном. Новые настройки характеристик не добавили зависимостей ядру.
Успешный полный запуск оригинальной demo этим не доказан.

## Ссылочные типы и следующий барьер demo

SDK `7f43c25` исправляет позицию Ref GUID для проверенных собственных
форм бизнес-процесса30/49 и задачи33/52. Нативный компиляторный контроль
и input XML сохранены в `fixtures/demo-business-task-reference-types`.
Оба значения флага0/1 имеют отдельный native CF. Финальные SDK gates
подтвердили 837 обычных тестов, 0 ошибок, 13 пропущены, а поздний контроль
флага0 отдельно прошёл со всеми пятью новыми тестами и fmt/Clippy.

ORM объявляет эти два ссылочных вида через существующие неизменяемые
описания типов; адаптер сохраняет их в типах характеристик. Root полный
прогон `open-bsl-business-task-reference-gates-1005.json`: все восемь шагов0,
workspace tests382,51 с. Ожидания существующих тестов не менялись.

После этой правки readonly demo прошла проекцию, загрузила3953модуля
и остановилась при компиляции исходного модуля объекта
ДополнительныеРеквизитыИСведения на UndefinedVariable("ДополнительныеСвойства").
Чтение11,43с, вся проверка75,14с при параллельных workspace gates.
Это времена конкретного прогона, не измерение производительности.
Соединение закрыто до компиляции; прикладной BSL не исполнялся.
Свойство отдельно измерено на своей базе; квитанция сохранена в
`fixtures/characteristic-additional-properties`. Восемь характеристических
фасадных тестов теперь зелёные. Справочник API перегенерирован.

Повторная readonly demo после ДополнительныеСвойства: чтение4,62с,
3953 модуля, общая проверка60,90с. Предыдущий отказ модуля характеристик
снят. Новый барьер компиляции: общий модуль РаботаВМоделиСервисаБТС,
UndefinedFunction("УстановитьЧасовойПоясИнформационнойБазы"). Исполнение
прикладного BSL и записи в оригинальную базу не выполнялись.

Полный прогон после ДополнительныеСвойства завершился успешно:
`/tmp/open-bsl-characteristic-additional-gates-1005.json` содержит код0
для всех восьми шагов, workspace tests366,97с. Проверены fmt,
all-targets Clippy, rustdoc, consumer, нормальные деревья RT/VM и strict
OpenSpec47/47. Прежние ожидания тестов не менялись.

Для следующего отказа компиляции получена справка установленной
платформы8.3.27.2342 через `unica.documentation.get`:
`objects/Global context/methods/catalog8/SetInfoBaseTimeZone4016.html`.
Это необязательный строковый аргумент; операция меняет настройку в базе
и требует монопольного доступа. Существующий opt-in отложения ошибок
общих модулей покрывает UndefinedVariable, но пока не UndefinedFunction.
Начат отдельный нативный контроль неизвестной функции в неиспользованном
дочернем общем модуле; по одной справке setter не подменяется успешной
фиктивной реализацией.

## Повторная компиляция demo после измеренного отложения неизвестной функции

После согласованного изменения политики ошибок имён 2026-10-05
проверка только компиляции прочитала 3953 модуля. Чтение заняло4,59с,
весь прогон59,50с при параллельном workspace-прогоне. Соединение закрыто
до компиляции; исходный BSL не исполнялся.

Прежний барьер `УстановитьЧасовойПоясИнформационнойБазы` больше не
останавливает сборку неиспользуемого общего модуля. Следующий отказ:
`РаботаСРабочимКалендаремСервер`,
`UnsupportedConstructor("ЭлементПланировщика")`. Это не свидетельство
успешной инициализации сеанса; конструктор и достижимость его вызова
при оригинальном запуске ещё требуют проверки.

## Demo после регистрации элемента планировщика

2026-10-06 проверка только компиляции прочитала3953 модуля за4,77с;
весь прогон60,46с при параллельных workspace-тестах. Соединение закрыто
до компиляции, исходный код не исполнялся. Барьер конструктора
`ЭлементПланировщика` пройден. Следующий отказ:
`РегистрСведений.УчастникиМероприятия.МодульМенеджера`,
`UndefinedFunction("СоздатьКлючЗаписи")`. Он относится к собственному
контексту менеджера регистра сведений; неизвестные функции частных
модулей по-прежнему проверяются строго.

## Demo после объявления сигнатуры создания ключа записи

2026-10-06 проверка только компиляции прочитала3953 модуля за6,93с;
весь прогон62,79с при параллельных workspace-тестах. Соединение закрыто
до компиляции, исходный код не исполнялся. Сигнатура `СоздатьКлючЗаписи`
сняла прежний барьер связывания; само создание ключа не реализовано.
Следующий отказ: `Справочник.КлассификаторЕдиницИзмерения.МодульМенеджера`,
`UndefinedFunction("НайтиПоКоду")`.

## Demo после обычного поиска кода и наименования

2026-10-06 проверка только компиляции прочитала3953 модуля за6,99с;
весь прогон64,34с при параллельных workspace-тестах. Соединение закрыто
до компиляции; запросы поиска и исходный BSL не исполнялись.
Прежний менеджер справочника связался. Следующий отказ:
`БизнесПроцесс.Исполнение.МодульМенеджера`,
`UndefinedVariable("ТочкиМаршрута")`.

Обычные строковые поиски реализованы через существующий запросный
адаптер; сопоставление кандидатов проверяется по нативным квитанциям.
Полный иерархический код, числовые преобразования и отборы по родителю
или владельцу пока возвращают явную ошибку неподдерживаемого режима.
Это не полная реализация всех вариантов поисковых методов и не
подтверждение успешного запуска оригинального сеанса.

Отдельное чтение исходника менеджера demo подтвердило обращения к точкам
`Исполнить`, `ОтветственноеИсполнение`, `Контролировать`, `Проверить`.
На собственном тестовом бизнес-процессе платформа подтвердила доступность
пустой коллекции: количество и число итераций равны нулю. Квитанция и
границы вывода сохранены в
[business-process-route-points](fixtures/business-process-route-points/README.md).
Доступный SDK 0.9.7 пока не предоставляет состав точек; неизвестный
состав не может быть представлен достоверно пустой коллекцией.

## Demo после подключения точек маршрута

2026-10-06 SDK 0.9.8 предоставил составную идентичность точек
бизнес-процесса. ORM проецирует вложенные точки, тип ссылки и синонимы;
фасад предоставляет квалифицированное свойство и собственный контекст
модуля менеджера. Нативный непустой контроль, присваивание и имена типов
сохранены в `fixtures/business-process-route-points/`.

Четыре новых теста подтверждают нативную проекцию и исполнение коллекции
через сеанс, различие владельцев и приоритет явно объявленных переменных.
Первый полный workspace-прогон после реализации прошёл все восемь
проверок, тесты — 371,49 с. Последующее измерение уточнило полное русское
имя типа коллекции: `ТочкиМаршрутаБизнесПроцесса`; короткое имя свойства
платформа не принимает как строку типа. После уточнения focused-тесты
прошли повторно. Окончательный полный прогон
`/tmp/open-bsl-route-points-final-gates-1006.json` завершился успешно:
workspace-тесты — 360,88 с, rustdoc — 6,14 с; все восемь проверок зелёные.
Оба нормальных дерева зависимостей совпали с прогоном до точек маршрута.

Read-only demo: 3953 модуля, приобретение 8,92 с, всего 68,65 с.
Соединение закрыто до компиляции, BSL не исполнялся. Прежний барьер
`ТочкиМаршрута` пройден. Следующий отказ —
`РегистрНакопления.КоличествоЗадачПоИсполнителям.МодульМенеджера`,
`UndefinedFunction("СоздатьНаборЗаписей")`.
Лог: `/tmp/open-bsl-demo-route-points-1006.log`.

Отдельное чтение исходника следующего менеджера заняло 25,15 с;
приобретение — 4,76 с. В `ОбновитьПоДействиюЗадачи` после создания
набора следуют отбор по регистратору, чтение, добавление/удаление строк
и запись. Исходник сохранён только в `/tmp`; BSL и операции регистра
не выполнялись. Синтакс-помощник 1С 8.3.27.2342 подтверждает метод
без аргументов, создающий пустой набор конкретного регистра:
`platform-syntax-help:syntax-context:objects/catalog125/catalog204/object210/methods/CreateRecordSet542.html`.
Для связывания добавлена сигнатура с явной ошибкой неподдержанного
исполнения по существующему образцу регистров сведений. Это не
реализация чтения или записи набора.

После декларации набора накопления новая read-only компиляция demo
завершилась за 64,95 с, приобретение — 4,57 с, 3953 модуля. Прежний
барьер пройден; в том же менеджере не разрешается `Остатки`.
Справка 8.3.27.2342 подтверждает английское имя `Balance` и четыре
необязательных параметра: момент, структура отбора, список измерений,
список ресурсов. Источник:
`platform-syntax-help:syntax-context:objects/catalog125/catalog204/object210/methods/Balance715.html`.
В исходнике используется `Остатки(, Отбор, Измерения)`; BSL не исполнялся.

Три новых теста набора накопления и два прежних теста ключей сведений
прошли. Общий прогон `/tmp/open-bsl-accumulation-record-set-gates-1006.json`
прошёл fmt и Clippy, но workspace-тесты отказали на прежнем
`factory_arity_is_checked_in_the_finite_receiver_context`: его ожидание
запрещает `СоздатьНаборЗаписей` у регистра накопления. Тест не изменён;
пользователю предложена точная корректировка только этого ожидания,
с сохранением проверки отсутствия менеджера записи и всех проверок арности.
Дальнейший зелёный прогон и архивация пока не подтверждены.
