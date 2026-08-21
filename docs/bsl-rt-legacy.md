# Реестр legacy- и мёртвого кода после рефакторинга

## Контекст

Аудит проведён 2026-08-21 на вершине `bsl-rt-ver2` (`8aa2331`),
после завершения шести частей рефакторинга (`docs/bsl-rt-refactor.md`,
`docs/bsl-rt-abi.md`): вынесения компонентов, введения
`CreateObject`/`CallComponent`/`CallObjectMethod`/`GetObjectProp`/
`SetObjectProp`, таблиц методов и свойств, снятия `legacy_type_id` и
распайки монолитов. Цель — найти, что из старых путей и переходных
адаптеров осталось неубранным, а что является мёртвым кодом без
вызовов.

Каждая находка верифицирована прямым поиском по дереву (`rg`).
«Мёртвый» означает: нуль вызовов во всём workspace. «Legacy» означает:
код для пути, который больше не проходит ни production-конвейер, ни
тесты. Оправданные удержания (опкоды для десериализации старого
байт-кода, измеренные горячие пути) перечислены отдельно.

## 1. Мёртвый код (нуль вызовов — удаляется безопасно)

| # | Файл:строка | Символ | Доказательство |
|---|---|---|---|
| 1 | `crates/bsl-rt/src/enums.rs:2000` | `pub fn members_of` | `rg` по репозиторию: только определение + реэкспорт `lib.rs:61`. Вызовов нет. |
| 2 | `crates/bsl-rt/src/open_questions.rs:3430` | `pub fn question` | Только определение. Вызовов нет. |
| 3 | `crates/bsl-rt/src/open_questions.rs:3435` | `pub fn anchor` | Только определение. Вызовов нет. |
| 4 | `crates/bsl-vm/src/lib.rs:212` | `pub fn run_program_jit` | Non-registry JIT-вход. `open-bsl` использует `run_program_jit_with_registry_and_io` (`state.rs:109`); CLI — тот же. Вызовов нет. |
| 5 | `crates/bsl-vm/src/lib.rs:292` | `pub fn run_repl_chunk` | Non-registry REPL-вход. CLI использует `run_repl_chunk_with_registry` (`repl.rs:324`). Вызовов нет. |

## 2. Мёртвые enum-варианты (никогда не конструируются)

| # | Файл:строка | Вариант | Доказательство |
|---|---|---|---|
| 6 | `crates/bsl-rt/src/component.rs:386` | `FunctionKind::Intrinsic` | Ни один `FunctionDescriptor` не ставит `Intrinsic` (все 28 сайтов — `Function`/`Procedure`). Сравнение в `resolver.rs:658` недостижимо. |
| 7 | `crates/bsl-rt/src/lib.rs:287` | `RtError::UnsupportedLocale` | Только определение + arm в `Display` (`:344`). Нигде не конструируется. |

## 3. Legacy-адаптеры к удалённым путям байт-кода

| # | Файл:строка | Что | Доказательство |
|---|---|---|---|
| 8 | `crates/bsl-json/src/bridge.rs:1115, 1131` (+ `:57-61`, `:96-100`, комментарии `:13`, `:73`) | Ветвь `None` в `component_read_json`/`component_write_json` и параметр `Option<JsonCallByName>` | `CallComponent` всегда строит `CallContext::with_function_caller` (`bsl-vm/src/lib.rs:1963`), поэтому `execution_parts()` всегда возвращает `Some`. Ветвь `None` — legacy-путь для удалённых JSON-опкодов. Комментарии прямо говорят «для нового и legacy-байткода». |
| 9 | `crates/bsl-bytecode/src/compiler.rs:594-608` | Arm `RegexGetGroups` special-case | Переходный путь «CLI без реестра». CLI теперь ходит через реестр (`repl.rs:294`, `307`, `324`). Corner-case (core-приёмник `ПолучитьГруппы`) ведёт себя идентично generic-arm `:609`. Комментарий `resolver.rs:931`: «legacy-пути regexp в компиляторе». |

## 4. Мёртвые ветви компилятора (опкоды оставлены для legacy-сериализации)

| # | Файл:строка | Что | Доказательство |
|---|---|---|---|
| 10 | `crates/bsl-bytecode/src/compiler.rs:657-662` | `if *open` → `Instr::GetObjectProp` | Резолвер всегда ставит `Field.open = false` (`resolver.rs:932`, комментарий: «Доступ к свойству всегда компилируется в закрытый `GetProp`»). Ветка никогда не выполняется. Опкод `GetObjectProp` намеренно оставлен в формате 18 для десериализации старого байт-кода. |
| 11 | `crates/bsl-bytecode/src/compiler.rs:930-935` | `if *open` → `Instr::SetObjectProp` | Симметрично: `AssignField.open = false` (`resolver.rs:636`, комментарий: «Запись свойства всегда компилируется в закрытый `SetProp`»). |

## 5. Legacy-вход без реестра (используется только тестами)

| # | Файл:строка | Символ | Статус |
|---|---|---|---|
| 12 | `crates/bsl-vm/src/lib.rs:156` | `pub fn run_program` | Non-registry вход. Не используется `bsl-cli`/`open-bsl` (фасад — `run_program_with_registry_and_io` / `run_program_jit_with_registry_and_io` в `state.rs:109, 116`). Используется только `bsl-vm/src/tests.rs` и `examples/`. Рефакторинг-док (`:239-241`, `:1085-1087`) называет его «ограниченный адаптер, сохраняющий измеренный горячий путь» — сознательное удержание, но production-путь его не проходит. |

## 6. Метод трейта без production-переопределений

| # | Файл:строка | Метод | Доказательство |
|---|---|---|---|
| 13 | `crates/bsl-rt/src/object_protocol.rs:214` | `ObjectProtocol::set_index` | 0 production-переопределений (только 1 test в `bsl-vm/src/tests.rs:143`). Все production-типы используют default `Err(NotIndexable)`. Мутабельный index-write у `BinaryBuffer` идёт через `BslValue::set_index` (`lib.rs:1903`) в обход трейта. |

## 7. Избыточно экспортированные реэкспорты

Используются внутри `bsl-rt`, но реэкспорт из `lib.rs` не нужен ни
одному downstream-крейту. Кандидаты на `pub(crate)` или снятие
реэкспорта (не мёртвый код — внутренние вызовы есть):

`value_to_string_internal`, `value_from_string_internal`,
`call_method_from_table`, `get_property_from_table`,
`set_property_from_table`, `DateBoundary`, `DatePart`, `ENUM_NAMES`,
`MapData`, `MAX_TEMPLATE_ARGS`, `ColumnVstr`, `FunctionCaller`,
`ValueFormatter`, `Ordering` (реэкспорт std-типа).

## Что НЕ мёртвое (оправданное удержание)

- `Instr::GetObjectProp`/`SetObjectProp` — опкод оставлен в формате 18
  для десериализации legacy-байт-кода. Нет компиляторного emit (находки
  №10-11), но есть parse (`text.rs`), interpret (`step_cold`) и
  JIT-shim (`jit/mod.rs:975, 1007`). Удаление опкода потребовало бы
  подъёма `FORMAT_VERSION`.
- `CallBuiltin`/`BuiltinFn`/`CallMethod`/`BuiltinMethod` — измеренный
  горячий путь ядра, не legacy. Рефакторинг-док (`:392-399`,
  `:1085-1087`) фиксирует как сознательное отступление.
- `NewTextWriter` — активный опкод core-типа (`ЗаписьТекста`).
- `BslObject` enum — все 16 вариантов живы (`type_name`/`type_of`/
  `is_filled`/`collection_len`/`get_index` — исчерпывающие match без
  `_`).
- `run_program` (находка №12) — используется тестами `bsl-vm`;
  рефакторинг-док фиксирует как сознательный адаптер.
- Все 3 `#[allow(dead_code)]`:
  - `XPathResolver::doc` (`bsl-xml/src/xpath.rs:1975`) — keep-alive
    `Rc`, без которого слабые ссылки вверх по дереву схлопнутся;
  - `mxl_write::bits` (`bsl-spreadsheet/src/document/mxl_write.rs:31`)
    — измеренные номера бит MXL-формата, ещё не подключенные к
    палитрам;
  - `shim!` macro (`bsl-vm/src/jit/mod.rs:523`) — параметры макроса,
    не все используются каждым конкретным шимом.
- `BuiltinFn` / `BuiltinMethod` — все варианты достижимы через
  `BUILTIN_FN_NAMES` / `BUILTIN_METHOD_NAMES`; нет осиротевших
  вариантов после вынесения компонентов.

## Метод проверки

- `RUSTFLAGS="-W dead_code -W unused" cargo check --workspace --all-targets` —
  нулевых предупреждений (компилятор не находит мёртвого кода; всё
  перечисленное либо `pub`, либо используется внутри определения
  крейта).
- Прямой `rg` по `crates/` (с исключением определяющего крейта) для
  каждого `pub fn`, варианта enum и метода трейта.
- Cross-check: `BuiltinFn`/`BuiltinMethod` variants → `BUILTIN_*_NAMES`
  → `call_builtin_*`; `RExpr` variants → resolver produce → compiler
  consume; `Instr` variants → compiler emit → VM dispatch.
