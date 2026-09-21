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

### Requirement: Session connection string
The session runtime SHALL expose `СтрокаСоединенияИнформационнойБазы()` from a connection string supplied by the host for that Session factory. It SHALL return the registration string appropriate to the represented session kind. The runtime SHALL NOT derive credentials from, normalize, or fabricate the string.

#### Scenario: Configured connection string
- **WHEN** configuration code calls `СтрокаСоединенияИнформационнойБазы()` in a Session whose factory has a host-supplied string
- **THEN** it receives that exact string

#### Scenario: Missing connection string
- **WHEN** configuration code calls the function in a Session whose host did not supply a string
- **THEN** the call fails explicitly instead of returning an empty or synthetic string

### Requirement: Session connection number
The session runtime SHALL expose `НомерСоединенияИнформационнойБазы()`/`InfoBaseConnectionNumber()` as the positive number supplied by the host for the concrete connection owned by that Session. The number SHALL remain stable for the Session lifetime. A background Session SHALL receive the number of its separate connection and SHALL NOT inherit the parent's number.

#### Scenario: Two sessions of one factory
- **WHEN** the host opens two Sessions from one factory with distinct connection numbers
- **THEN** each Session returns its own number through both language names

#### Scenario: Background connection
- **WHEN** the host opens a background Session with a new connection number
- **THEN** the background Session returns the new number
- **AND** the parent continues to return its original number

### Requirement: Authorized information-base connection snapshot
The session runtime SHALL provide `ПолучитьСоединенияИнформационнойБазы()`/`GetInfoBaseConnections()` from an immutable connection snapshot supplied by the host for the selected principal. Every returned connection SHALL expose the measured read-only `НомерСоединения`/`ConnectionNumber`, `ИмяПриложения`/`ApplicationName`, and `ИмяКомпьютера`/`ComputerName` properties. Session startup SHALL reject a supplied snapshot that does not contain the concrete Session connection number. Missing or denied connection-list access SHALL fail explicitly instead of returning an empty or current-only synthetic list.

#### Scenario: Authorized connection list
- **WHEN** the host supplies an authorized connection snapshot containing the concrete Session connection
- **THEN** both language names return a new array of immutable connection values from that snapshot
- **AND** the current connection can be found by the value returned from `НомерСоединенияИнформационнойБазы()`

#### Scenario: Connection list unavailable
- **WHEN** the selected principal lacks `АктивныеПользователи` or the host cannot acquire a complete connection snapshot
- **THEN** requesting the list raises an explicit error
- **AND** no partial or unrestricted list is returned

#### Scenario: Inconsistent connection snapshot
- **WHEN** a host attempts to open a Session with a connection snapshot that omits its concrete connection number
- **THEN** startup fails before configuration code executes

### Requirement: Session event-log writer
The configuration-session runtime SHALL provide `ЗаписьЖурналаРегистрации`/`WriteLogEvent` with the platform arity of one through six arguments and the `УровеньЖурналаРегистрации`/`EventLogLevel` values `Примечание`/`Note`, `Информация`/`Information`, `Предупреждение`/`Warning`, and `Ошибка`/`Error`. It SHALL also provide the sixth-argument `РежимТранзакцииЗаписиЖурналаРегистрации`/`EventLogEntryTransactionMode` values `Независимая`/`Independent` and `Транзакционная`/`Transactional`. A call SHALL be delivered to an event-log writer owned by the current Session. If the host has not supplied a writer, the call SHALL fail explicitly instead of discarding the event.

#### Scenario: Configuration records a startup error
- **WHEN** configuration code writes an event with an error level and a comment
- **THEN** the current Session's writer receives that event and its supplied values

#### Scenario: Event logging is unavailable
- **WHEN** configuration code writes an event and the current Session has no event-log writer
- **THEN** the call fails explicitly

### Requirement: Principal-bound client settings
For web-client and thin-client Sessions, the runtime SHALL expose `ХранилищеОбщихНастроек`/`CommonSettingsStorage` and `ХранилищеСистемныхНастроек`/`SystemSettingsStorage` through a client-settings adapter explicitly supplied for that Session. The standard manager SHALL provide `Загрузить`/`Load` with one through four arguments, `Сохранить`/`Save` with three through five arguments, and `Выбрать`/`Select` with zero or one argument. The adapter SHALL receive the selected principal and SHALL enforce the documented user-data-save and data-administration rights. Missing storage data, denied access, and an absent adapter SHALL remain distinguishable and SHALL NOT become an unrestricted or process-global store.

#### Scenario: Load current-user settings
- **WHEN** managed-client code loads an existing setting without an explicit user name
- **THEN** the adapter reads the setting for the current Session principal

#### Scenario: Access another user's settings
- **WHEN** code requests another user's setting without data-administration permission
- **THEN** the operation fails instead of reading or writing that user's value

#### Scenario: Client settings are unavailable
- **WHEN** managed-client code uses a settings store without a client-settings adapter
- **THEN** the operation fails explicitly

### Requirement: Home-page settings values
The session runtime SHALL provide the zero-argument `НастройкиНачальнойСтраницы`/`HomePageSettings` and `СоставФормНачальнойСтраницы`/`HomePageForms` constructors. A home-page settings value SHALL return and accept the current form composition through `ПолучитьСоставФорм`/`GetForms` and `УстановитьСоставФорм`/`SetForms`. A form composition SHALL expose read-only `ЛеваяКолонка`/`LeftColumn` and `ПраваяКолонка`/`RightColumn` array properties. Initial and updated compositions SHALL cross the same principal-bound client-settings adapter instead of being shared between Sessions.

#### Scenario: Read and update the form composition
- **WHEN** managed-client code reads the current composition, supplies a valid replacement, and reads it again
- **THEN** the adapter observes the replacement for that Session principal
- **AND** another Session does not acquire the value without its own adapter result

#### Scenario: Invalid form composition
- **WHEN** code passes a value other than `СоставФормНачальнойСтраницы` to `УстановитьСоставФорм`
- **THEN** the call fails before the adapter is changed

### Requirement: Stored value snapshot
The ORM component SHALL provide `ХранилищеЗначения`/`ValueStorage` and `СжатиеДанных`/`Deflation` constructors with the platform arities. `СжатиеДанных` SHALL accept an optional integral level from `-1` through `9`. Constructing a stored value SHALL snapshot the supplied serializable value. Every `Получить`/`Get` call SHALL return a newly materialized value, and source aliases reached through separate collection entries SHALL become independent restored values. Unsupported values and invalid compression arguments SHALL fail explicitly.

#### Scenario: Construction snapshots a mutable value
- **WHEN** code stores an array and later changes the source array
- **THEN** `Получить()` returns the value as it was during construction

#### Scenario: Every read is detached
- **WHEN** code mutates a collection returned by `Получить()` and calls `Получить()` again
- **THEN** the later result retains the stored contents

#### Scenario: Source aliases become independent values
- **WHEN** two collection entries refer to the same mutable object before storage
- **THEN** mutating the first restored entry does not change the second restored entry

### Requirement: Current information-base session
The session component SHALL provide `ПолучитьТекущийСеансИнформационнойБазы`/`GetCurrentInfoBaseSession`. Its result SHALL have the measured `Active session` type and provide `ПолучитьФоновоеЗадание`/`GetBackgroundJob`. A foreground session SHALL return `Неопределено`. A background Session SHALL return a `Background job` value whose `ИмяМетода`/`MethodName` is the exact job target explicitly supplied when that Session is opened. The session layer SHALL fail explicitly for unsupported job members rather than fabricate a complete job-service snapshot.

#### Scenario: Foreground session
- **WHEN** foreground configuration code requests the background job of its current information-base session
- **THEN** it receives `Неопределено`

#### Scenario: Background session
- **WHEN** a background Session is opened for `Модуль.Метод`
- **THEN** its current information-base session returns a `Background job`
- **AND** that job's `ИмяМетода` is `Модуль.Метод`

### Requirement: Frame-scoped security modes
The session runtime SHALL provide the boolean startup surface `УстановитьБезопасныйРежим`/`SetSafeMode`, `БезопасныйРежим`/`SafeMode`, `УстановитьПривилегированныйРежим`/`SetPrivilegedMode`, and `ПривилегированныйРежим`/`PrivilegedMode` with the behavior measured on platform 8.3.27. Each enabling call SHALL increment a counter owned by the current BSL call frame. A disabling call SHALL decrement only that frame's counter and SHALL fail if the frame has no matching enable. Leaving a frame normally or by exception SHALL remove its remaining enables. An active safe mode SHALL make the privileged-mode query false without discarding privileged enables. Unsupported named security profiles SHALL fail explicitly until a profile catalog is supplied by the host.

#### Scenario: Nested frame cannot disable its caller
- **WHEN** a caller enables a mode and a nested function attempts to disable it without enabling it itself
- **THEN** the nested disable raises an error
- **AND** the caller's mode remains enabled

#### Scenario: Frame exits with a mode enabled
- **WHEN** a function enables a mode and returns or raises an exception without disabling it
- **THEN** the mode is disabled after that frame is left

#### Scenario: Safe mode suppresses privilege
- **WHEN** one frame enables safe mode and privileged mode
- **THEN** `ПривилегированныйРежим()` returns false while safe mode is active
- **AND** it returns true after safe mode is disabled in that frame

### Requirement: Background session isolation and parameter inheritance
A background job SHALL run in a new Session for an explicitly supplied selectable principal. It SHALL use a separate database connection and module instances. Its initial session parameters SHALL be copied from the parent session, and later changes in either session SHALL not change the other.

#### Scenario: Inherited parameter
- **WHEN** a parent starts a background job after setting a session parameter
- **THEN** the background session initially observes the copied value

#### Scenario: Independent mutation
- **WHEN** the background session changes an inherited parameter
- **THEN** the parent session retains its own value
