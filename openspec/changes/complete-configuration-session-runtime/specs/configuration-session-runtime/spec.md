## ADDED Requirements

### Requirement: Component call into the current BSL module graph
The open-bsl facade SHALL allow a component operation to invoke an export of a linked module instance in the current State. The nested call SHALL use the existing VM, preserve the current session module catalog and host services, support by-reference parameter results, and accept an explicit `ЭтотОбъект`. It SHALL NOT create another interpreter or State.

#### Scenario: Manager export
- **WHEN** an ORM manager dispatches an exported method of its linked BSL manager module
- **THEN** the method runs in the current State with that manager as `ЭтотОбъект`

#### Scenario: Nested call failure
- **WHEN** the linked export raises an error
- **THEN** the error carries the nested BSL call context
- **AND** the session remains the same session

### Requirement: Manager method composition
A manager SHALL resolve a call first against its static platform methods, then against exports of its linked manager module, and finally report an unknown-method error. The link SHALL identify one manager module instance per State.

#### Scenario: Platform method wins
- **WHEN** a platform method and a manager-module export have the same folded name
- **THEN** the platform method is called

#### Scenario: Configuration export fallback
- **WHEN** no platform method matches and the linked module exports the requested folded name
- **THEN** that export is called

### Requirement: Measured lazy session parameters
Session parameters SHALL reproduce the measured startup, unset, nested initialization, cycle, retry, clear, and failure side-effect behavior of platform 8.3.27. Startup SHALL pass `Неопределено`; a first read of an unset parameter SHALL synchronously call the handler with that name. A name already being initialized SHALL fail without re-entering its handler. A requested name left unset SHALL fail and become retryable. Writes completed before a later failure SHALL remain.

#### Scenario: Nested different parameter
- **WHEN** the handler for unset parameter A reads unset parameter B
- **THEN** the handler is entered synchronously for B and then resumes A

#### Scenario: Initialization cycle
- **WHEN** initialization reaches a parameter already on the initialization stack
- **THEN** that read fails without another handler entry

#### Scenario: Failure after writes
- **WHEN** an initialization writes a parameter and later raises an error
- **THEN** the written value remains available
- **AND** a requested value still unset becomes retryable

### Requirement: Principal-bound data access
Every ORM query, manager read, register read, and constant read SHALL use the selected principal's roles and the current session parameters. Missing rights information, an access restriction that cannot be expanded, or a required session parameter that cannot be initialized SHALL fail closed. `ВЫБРАТЬ РАЗРЕШЕННЫЕ` SHALL apply the restrictions requested through `open-sdbl`; ordinary query access SHALL retain its distinct platform error behavior.

#### Scenario: Restriction needs a session parameter
- **WHEN** an RLS template reads an unset session parameter
- **THEN** the parameter is initialized through the session handler before the query executes

#### Scenario: Incomplete rights
- **WHEN** rights for one of the principal's roles are unavailable
- **THEN** the read fails instead of running without that role's restrictions

### Requirement: Linear query result surface
The ORM component SHALL provide the measured `Запрос` constructor with its writable `Текст` property and the `УстановитьПараметр` and `Выполнить` methods. Query execution SHALL delegate the text and the final case-insensitive parameter values to the current Session's data-access adapter. A tabular result SHALL support `Пустой`, direct `Выбрать`, direct `Выгрузить`, and a selection's `Следующий`, `Получить`, numeric indexing, and named fields. Unsupported traversal modes and unavailable data access SHALL fail explicitly.

#### Scenario: Execute with the current Session
- **WHEN** BSL executes a query after assigning a parameter more than once
- **THEN** the current Session's adapter receives the final parameter value, selected principal, and current call context
- **AND** another Session cannot observe that query object's mutable state

#### Scenario: Read a linear result
- **WHEN** the adapter returns named columns and rows
- **THEN** a direct selection exposes each current row by field name, zero-based index, and `Получить`
- **AND** direct unload returns an independent `ТаблицаЗначений` with the same columns and rows

#### Scenario: Query access is unavailable
- **WHEN** a query is executed without a data-access adapter or the adapter rejects incomplete rights or RLS information
- **THEN** execution raises an explicit error
- **AND** no empty or unrestricted result is substituted

### Requirement: Background session isolation and parameter inheritance
A background job SHALL run in a new Session for an explicitly supplied selectable principal. It SHALL use a separate database connection and module instances. Its initial session parameters SHALL be copied from the parent session, and later changes in either session SHALL not change the other.

#### Scenario: Inherited parameter
- **WHEN** a parent starts a background job after setting a session parameter
- **THEN** the background session initially observes the copied value

#### Scenario: Independent mutation
- **WHEN** the background session changes an inherited parameter
- **THEN** the parent session retains its own value
