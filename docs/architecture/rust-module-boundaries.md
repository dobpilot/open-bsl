# Внутренние границы Rust-модулей

Текущая раскладка после функциональной реализации
[separate-rust-module-responsibilities](../../openspec/changes/separate-rust-module-responsibilities/proposal.md),
7 сентября 2026 года. Публичные Rust-пути и BSL-поведение сохранены.
Это карта владельцев, не второй checklist: ход приёмки ведётся только в
[tasks.md пакета](../../openspec/changes/separate-rust-module-responsibilities/tasks.md).
Итоговое ревью, измерения серии и архивирование остаются владельцу.

## Runtime: одно значение, разные предметные операции

[lib.rs](../../crates/bsl-rt/src/lib.rs) сохраняет прежние реэкспорты
`BslValue`, `RtError`, `ComponentError`, `RtResult`.
Новые модули приватны; предметные методы — inherent-реализации того же
`BslValue`, без сервисных обёрток, extension traits или новых вариантов.

| Владелец | Ответственность |
|---|---|
| [error.rs](../../crates/bsl-rt/src/error.rs) | Ошибки, преобразования и их диагностическое представление |
| [value/mod.rs](../../crates/bsl-rt/src/value/mod.rs) | Представление значения, базовые приведения, арифметика, равенство/Hash и debug-only Display |
| [value/strings.rs](../../crates/bsl-rt/src/value/strings.rs) | Строковые операции значения через существующий BslString |
| [value/dates.rs](../../crates/bsl-rt/src/value/dates.rs) | Операции дат через BslDate и часы окружения |
| [value/collections.rs](../../crates/bsl-rt/src/value/collections.rs) | Полиморфные коллекции, индексация, свойства и кэшированный доступ |
| [value/tables.rs](../../crates/bsl-rt/src/value/tables.rs) | Проверка аргументов и делегирование табличных методов |
| [value/io.rs](../../crates/bsl-rt/src/value/io.rs) | Существующие операции текстовой записи и двоичных данных/буфера |

Алгоритмы остаются единственными: строки — `string.rs`, даты —
`date.rs`, таблицы — `table.rs`, двоичные данные и кодировки —
`bindata.rs`/`encoding.rs`, числа — `bsl-number`.
Пользовательский текст по-прежнему формирует `bsl-format`, не
диагностический `BslValue::Display`. Общие тестовые помощники не
копируются; предметные unit-тесты находятся у владельцев операций.

## Фоновые задания: учёт отдельно от исполнения

[jobs.rs](../../crates/open-bsl/src/jobs.rs) — прежний фасад
`open_bsl::jobs`; все пять модулей реализации приватны.
Топология компонентов и потоков определена в
[component-architecture.md](component-architecture.md).

| Владелец | Ответственность |
|---|---|
| [registry.rs](../../crates/open-bsl/src/jobs/registry.rs) | DTO-записи, admission, резервирование ключей, FIFO, бюджеты, история и конечные переходы |
| [runtime.rs](../../crates/open-bsl/src/jobs/runtime.rs) | Конфигурация, host-профили, источники времени/ID, общий runtime, ленивый старт, shutdown и супервизор |
| [prepare.rs](../../crates/open-bsl/src/jobs/prepare.rs) | Разрешение цели, восстановление worker Engine, сборка entry и материализация параметров сеанса |
| [worker.rs](../../crates/open-bsl/src/jobs/worker.rs) | Резиденты потока, helping-драйвер, запуск/poll, claim/commit/rollback и гарды |
| [service.rs](../../crates/open-bsl/src/jobs/service.rs) | Адаптеры BackgroundJobService и маршрут пользовательских сообщений |

Реестр не владеет `State` или `ProgramExecution` и не импортирует
драйвер/сервисы. Исполняемые BSL-значения остаются в потоке worker;
граница очереди — владеющие DTO. `RunningJob`, `StartGuard` и их
освобождение находятся вместе: перенос модулей не менял захват локов,
области guard, порядок публикации и моменты Drop.
Взаимные внутренние импорты runtime/worker/service допустимы:
они отражают существующее helping-ожидание, а не новый слой абстракции.

## VM: периферия вокруг связного ядра

[lib.rs](../../crates/bsl-vm/src/lib.rs) сохраняет диспетчер
`step`/`step_cold`, `poll_linked`, семейство `drive*`,
`ProgramExecution`, кадры и закреплённые inline-помощники.
Подробный список ядра находится в принятом
[плане VM](../plans/bsl-vm-refactor.md), а история раскладки —
в [vm-code-layout.md](vm-code-layout.md).

| Приватный владелец | Ответственность |
|---|---|
| [snippet.rs](../../crates/bsl-vm/src/snippet.rs) | Динамические фрагменты, ограничение глубины и перенумерация библиотек; фронтенд предоставляется через DynamicCompiler |
| [linking.rs](../../crates/bsl-vm/src/linking.rs) | HostIo, связанные компоненты, карты разрешения методов/свойств и проверка связывания |
| [scheduler.rs](../../crates/bsl-vm/src/scheduler.rs) | Задачи, обещания, HTTP-завершения, очередь, квантование и холодное возобновление |
| [modules.rs](../../crates/bsl-vm/src/modules.rs) | Каталог и экземпляры модулей сеанса, ленивая инициализация, выбор таблиц исполнения |
| [debug.rs](../../crates/bsl-vm/src/debug.rs) | Отладочные интерфейсы и доступ к остановленным кадрам; evaluate вызывает существующий snippet |
| [entry.rs](../../crates/bsl-vm/src/entry.rs) | Прежние run/REPL/call-module входы и внутренние формы вызова |

Публичные входы и типы доступны прежними реэкспортами из корня.
Необходимая видимость полей внутри VM согласована владельцем:
она обслуживает существующие обращения ядра, не открывая модули
внешним крейтам и не вводя новых клонирований или обёрток.

### Кэши принадлежат запуску, а не образу

`RunCaches` остаётся во владении `ProgramExecution`.
`catalog_caches` — отдельный вектор, не поле `ModuleInstance`:
цикл держит кэши разделяемо одновременно с изменяемым заимствованием
сеанса через `ModulesCtx`. Слияние этих владельцев не производилось.

`CatalogContext::execution_parts` выбирает программу, линковку и кэши
одним номером модуля. В `poll_linked` этот аксессор заменяет прежнюю
тройку обращений; проверки сохраняют порядок и тексты ошибок.
Создание кэшей для всего каталога и политика прогрева не изменены.
`Chunk` не получил состояние запуска; обычные зависимости VM
по-прежнему не включают frontend.

## Граница выполненной работы

[Карта приёмки](../../openspec/changes/separate-rust-module-responsibilities/acceptance.md)
содержит соответствие сценариев тестам, проверку приватности,
инвентарь и оговорки покрытия. Отложенные кандидаты исходного аудита
(resolver, compiler, XPath, регистрация компонентов и другие)
этим изменением не реализованы. Исторические результаты измерений
не пересматриваются; текущий пакет остаётся открытым до задач владельца.
