# Спецификация справочника BSL API

## Purpose

Определить воспроизводимый справочник публичных функций, объектов, методов и
свойств стандартного движка open-bsl.

## Requirements

### Requirement: Генерация справочника

`bsl-cli` SHALL предоставлять команду `--emit-api-reference [выход.md]`,
которая выдаёт Markdown-каталог API стандартного движка.

#### Scenario: Вывод в stdout

- **WHEN** команда вызвана без пути
- **THEN** справочник печатается в stdout и процесс завершается с кодом 0

#### Scenario: Вывод в файл

- **WHEN** после команды передан путь
- **THEN** справочник записывается в этот файл без печати документа в stdout

### Requirement: Дескрипторы как источник истины

Справочник SHALL получать имена, псевдонимы, арности, типы, методы и свойства
из тех же runtime-дескрипторов, которые использует исполнение.

#### Scenario: Изменение публичного дескриптора

- **WHEN** разработчик меняет публичный runtime-дескриптор
- **THEN** сгенерированный справочник меняется без правки второй таблицы API

### Requirement: Проверяемый снимок

Проект SHALL хранить сгенерированный справочник в
`docs/reference/bsl-api/api.md` и проверять его равенство текущему генератору.

#### Scenario: Снимок устарел

- **WHEN** runtime API изменился, а `api.md` не обновлён
- **THEN** тест CLI завершается ошибкой и показывает необходимость регенерации

### Requirement: Declared-only names are marked
A global function, constructor or component type that is registered only so that configuration code resolves and passes arity checks, with no implementation behind it, SHALL be flagged as declared-only in its descriptor. Calling or constructing it SHALL raise one uniform runtime error that names it. The generated API reference SHALL mark every declared-only entry so that it cannot be read as supported.

#### Scenario: Calling a declared-only function
- **WHEN** a script calls a declared-only global function
- **THEN** compilation succeeds
- **AND** the call raises the uniform declared-only error naming the function

#### Scenario: Reference generation
- **WHEN** the API reference is generated
- **THEN** every declared-only function, constructor and type carries the declared-only mark
- **AND** implemented entries carry no such mark
