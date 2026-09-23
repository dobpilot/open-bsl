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

### Requirement: Frozen current information-base user
The ORM component SHALL expose `ПользователиИнформационнойБазы` / `InfoBaseUsers` as a library global. Its `ТекущийПользователь()` / `CurrentUser()` method SHALL return an immutable information-base user description whose `Имя` / `Name` equals the principal frozen for the current Session. The operation SHALL NOT re-read the user catalog, observe another Session, or permit mutation of the description.

#### Scenario: Current user during startup
- **WHEN** configuration startup calls `ПользователиИнформационнойБазы.ТекущийПользователь()`
- **THEN** the returned description has the selected Session principal's name

#### Scenario: Isolated sessions
- **WHEN** two Sessions select different principals
- **THEN** each Session observes only its own current-user description

### Requirement: Configuration enumeration managers
The ORM component SHALL expose `Перечисления` / `Enums` as a library global backed by the immutable configuration metadata catalog. Property access by a folded configuration enumeration name SHALL return that enumeration's manager and SHALL fail explicitly for an absent name. This startup slice SHALL NOT invent enumeration values that are absent from the acquired configuration metadata.

#### Scenario: Resolve an enumeration manager
- **WHEN** startup code reads an existing configuration enumeration through `Перечисления.<Имя>` or `Enums.<Name>`
- **THEN** it receives the manager linked to that enumeration metadata object in the current Session catalog

#### Scenario: Unknown enumeration
- **WHEN** startup code requests an enumeration name absent from the current Session catalog
- **THEN** property access fails explicitly

### Requirement: Reached predefined values
The ORM component SHALL provide `ПредопределенноеЗначение` / `PredefinedValue` with one required string argument. For a reached configuration enumeration path it SHALL resolve the owner and member from the immutable acquired metadata catalog and return the corresponding immutable enumeration reference. For a system-enumeration path it SHALL return the existing base-runtime enum member. An unknown path or a non-string argument SHALL fail explicitly. The implementation SHALL NOT reread configuration resources or fabricate a value absent from the acquired catalog.

#### Scenario: Resolve a configuration enumeration member
- **WHEN** startup code requests a reached configuration enumeration member by its full path
- **THEN** it receives the member identified by the acquired owner and value GUIDs

#### Scenario: Resolve a system enumeration member
- **WHEN** code requests a system-enumeration member by its full path
- **THEN** it receives the same base-runtime enum value as direct member access

#### Scenario: Reject an unknown predefined value
- **WHEN** the supplied path cannot be resolved as either a configuration value or a system-enumeration member
- **THEN** execution fails explicitly

### Requirement: Configuration exchange-plan managers
The ORM component SHALL expose `ПланыОбмена` / `ExchangePlans` as a library global backed by the immutable metadata catalog. A folded property name matching an acquired exchange-plan metadata object SHALL return that object's existing manager value. `Метаданные.ПланыОбмена` / `Metadata.ExchangePlans` SHALL expose the same acquired descriptors through the existing metadata-collection behavior. An unknown configuration-defined name SHALL fail explicitly.

#### Scenario: Resolve an acquired exchange-plan manager
- **WHEN** startup code reads a configuration exchange plan through either language name
- **THEN** it receives the manager value tied to that acquired metadata object

#### Scenario: Read exchange-plan metadata
- **WHEN** startup code reads the exchange-plan collection through the root metadata object
- **THEN** it sees the acquired exchange-plan descriptors from the same catalog snapshot

### Requirement: Reached configuration picture library
The ORM component SHALL expose `БиблиотекаКартинок` / `PictureLib` as a library global. A folded property name matching an acquired common-picture metadata object SHALL return an immutable `Картинка` / `Picture` value. Repeated access through either language name SHALL compare equal for the same metadata object and unequal for a different one, and the value SHALL reproduce the measured type display, string representation, and filled state. The initial slice SHALL NOT fabricate picture bytes or platform-standard members that are absent from the acquired catalog.

#### Scenario: Read a configuration picture
- **WHEN** startup code reads an acquired common picture through either language name
- **THEN** it receives the measured picture value associated with that metadata object

#### Scenario: Unknown picture
- **WHEN** startup code requests a picture absent from the acquired catalog
- **THEN** property access fails explicitly

### Requirement: Data composition field value
The base runtime SHALL provide the `ПолеКомпоновкиДанных` / `DataCompositionField` constructor with exactly one string argument and the immutable value behavior measured on platform 8.3.27. Its string representation SHALL preserve the supplied data-path spelling, while equality SHALL compare paths without regard to case and remain consistent when the value is used as a map key. This value SHALL NOT create or imply a data-composition engine.

#### Scenario: Construct a field from a data path
- **WHEN** startup code constructs a data composition field from a string path
- **THEN** it receives the standard data composition field value for that path

#### Scenario: Compare fields
- **WHEN** two separately constructed fields contain paths that the platform considers equal
- **THEN** the runtime equality operation returns true

### Requirement: Data composition comparison type
The base runtime SHALL expose the closed system enum `ВидСравненияКомпоновкиДанных` / `DataCompositionComparisonType` with the complete member set and string representations measured on platform 8.3.27. Both language names SHALL resolve to the same enum values.

#### Scenario: Read a comparison member
- **WHEN** startup code reads a member through either language name
- **THEN** it receives the corresponding standard enum value

### Requirement: Reached standard beginning date surface
The base runtime SHALL expose `СтандартнаяДатаНачала` / `StandardBeginningDate` with the one-argument constructor from `ВариантСтандартнойДатыНачала` / `StandardBeginningDateVariant` reached by the recorded startup graph. The closed variant enum SHALL contain the complete member set and string representations measured on platform 8.3.27. Zero-argument and date-based construction, property access, and relative-date evaluation are outside this startup slice.

#### Scenario: Construct from a relative variant
- **WHEN** startup code constructs a standard beginning date from a variant member
- **THEN** it receives a standard beginning date value retaining that variant

### Requirement: Reached spreadsheet file-type values
The base runtime SHALL resolve the measured `ANSITXT`, `DOCX`, `HTML5`, `MXL`, `ODS`, `TXT`, `XLS`, and `XLSX` members of `ТипФайлаТабличногоДокумента` / `SpreadsheetDocumentFileType` reached by startup code independently of whether the spreadsheet component can write each format. Passing a value SHALL NOT imply export support for that format.

#### Scenario: Read a reached file-type member
- **WHEN** startup code reads a reached member through either language name
- **THEN** it receives the same standard enum value with the measured representation

### Requirement: Frozen configuration-changed state
The ORM component SHALL provide `КонфигурацияИзменена()` / `ConfigurationChanged()` with zero arguments and return the configuration-change state acquired for the factory's metadata snapshot. The value SHALL be frozen with that snapshot and shared by Sessions opened from the factory. If the provider did not supply the state, the call SHALL fail explicitly instead of assuming either boolean value.

#### Scenario: Provider reports a changed configuration
- **WHEN** the acquired metadata snapshot records that the main configuration differs from the database configuration
- **THEN** the function returns `Истина`

#### Scenario: Change state is unavailable
- **WHEN** the provider did not acquire the main-versus-database configuration state
- **THEN** the function fails explicitly

### Requirement: Refresh reusable values
The configuration-session runtime SHALL provide the zero-argument global procedure `ОбновитьПовторноИспользуемыеЗначения()` / `RefreshReusableValues()`. Calling it SHALL invalidate all reusable-return-value state owned by the current Session. While the runtime does not cache reusable function results, the operation SHALL complete successfully and preserve the equivalent uncached behavior.

#### Scenario: Runtime has no reusable-value cache
- **WHEN** startup code calls the procedure in a Session whose functions always execute
- **THEN** the call completes successfully and subsequent function calls continue to execute normally

### Requirement: Session-local dynamic database-configuration change state
The configuration-session runtime SHALL provide `КонфигурацияБазыДанныхИзмененаДинамически()` / `DataBaseConfigurationChangedDynamically()` with zero arguments. It SHALL return the explicit boolean state supplied for the current Session and SHALL fail explicitly when the host did not supply that state. A separately opened foreground or background Session SHALL receive its own state through its own resources and SHALL NOT inherit the value from another Session.

#### Scenario: Host reports a dynamic update during the Session
- **WHEN** the current Session's resources record that the database configuration was dynamically updated during its lifetime
- **THEN** the function returns `Истина`

#### Scenario: Dynamic-update state is unavailable
- **WHEN** the current Session has no host-supplied dynamic-update state
- **THEN** the function fails explicitly instead of assuming either boolean value

#### Scenario: A background Session is opened separately
- **WHEN** a background Session is opened with its own resources after a parent Session
- **THEN** its dynamic-update state comes only from those resources while the separately specified session parameters retain their existing inheritance behavior

### Requirement: Reached error-processing surface
The configuration-session runtime SHALL expose the global `ОбработкаОшибок` / `ErrorProcessing` manager. Its reached brief and detailed description methods SHALL accept one `ИнформацияОбОшибке` / `ErrorInfo` value and return that value's existing brief and detailed representations. Its zero-argument `ПолучитьНастройкиПользователя()` / `GetUserSettings()` method SHALL return the current Session's explicitly supplied user-settings snapshot or fail when that snapshot is absent. The initial settings value SHALL expose the read-only reached properties `ОтправлятьОтчетНаКлиенте` / `SendReportOnClient` and `АдресСервисаОбработкиОшибок` / `ErrorProcessingServiceAddress`.

The base runtime SHALL expose the complete measured `РежимОтправкиИнформацииОбОшибке` / `ErrorReportingMode` enum with `Авто` / `Auto`, `НеОтправлять` / `DontSend`, `СпрашиватьПользователя` / `AskUser`, and `Отправлять` / `Send`.

#### Scenario: Format an error through the manager
- **WHEN** startup code passes an error-information value to a reached description method
- **THEN** the result equals the brief or detailed representation already stored in that value

#### Scenario: Read current-user error settings
- **WHEN** the Session has an explicit error-processing settings snapshot
- **THEN** the manager returns its sending mode and service address without observing another Session

#### Scenario: Error settings are absent
- **WHEN** startup code requests user settings without a Session snapshot
- **THEN** the method fails explicitly

### Requirement: Reached transaction procedures
The ORM component SHALL provide the reached zero-argument procedures `НачатьТранзакцию()` / `BeginTransaction()`, `ЗафиксироватьТранзакцию()` / `CommitTransaction()`, and `ОтменитьТранзакцию()` / `RollbackTransaction()`. Each operation SHALL delegate to the current Session's data-access adapter with its frozen principal and SHALL use the same concrete connection as managed data locks, queries, and constants. Missing transaction capability SHALL fail explicitly. Transaction nesting, commit, rollback, and release SHALL remain owned by that connection-bound adapter.

#### Scenario: Commit a Session transaction
- **WHEN** startup begins and commits a transaction with a capable adapter
- **THEN** both operations execute in order on the current Session's connection

#### Scenario: Roll back after a caught failure
- **WHEN** startup begins a transaction and calls rollback from its exception handler
- **THEN** the adapter rolls back that same Session transaction

#### Scenario: Transaction capability is absent
- **WHEN** startup invokes a transaction procedure without a capable data adapter
- **THEN** execution fails explicitly instead of simulating a transaction in memory

### Requirement: Connection-bound exclusive mode
The ORM component SHALL provide `МонопольныйРежим()` / `ExclusiveMode()` and the reached one-boolean-argument form of `УстановитьМонопольныйРежим` / `SetExclusiveMode`. Both operations SHALL delegate to the current Session's data-access adapter and concrete connection. The runtime SHALL NOT emulate exclusive ownership with a process-local flag. Missing capability or a provider refusal SHALL fail explicitly.

#### Scenario: Read and change exclusive mode
- **WHEN** a capable adapter enters and then leaves exclusive mode
- **THEN** the getter reports the state of that same Session connection after each operation

#### Scenario: Exclusive-mode capability is absent
- **WHEN** startup reads or changes exclusive mode without a capable adapter
- **THEN** execution fails explicitly instead of reporting or storing a fabricated state

### Requirement: Session-bound integration-services manager
The ORM component SHALL provide the global read-only property `СервисыИнтеграции` / `IntegrationServices` as an integration-services manager. Its documented zero-argument methods `ВыполнитьОбработку` / `ExecuteProcessing` and `ОстановитьОбработку` / `StopProcessing` SHALL delegate to the current Session's data-access adapter and frozen principal. A dynamic integration-service property SHALL be resolved by that adapter for the same Session. The runtime SHALL NOT start jobs or retain service-manager state in a process-global substitute. Missing processing or service-resolution capability SHALL fail explicitly.

#### Scenario: Start integration-service processing
- **WHEN** startup calls `СервисыИнтеграции.ВыполнитьОбработку()` with a capable adapter
- **THEN** the adapter starts processing for the current Session and principal

#### Scenario: Resolve a configured integration service
- **WHEN** startup reads a dynamic property of `СервисыИнтеграции`
- **THEN** the current Session adapter resolves and returns that service manager

#### Scenario: Integration-services capability is absent
- **WHEN** startup calls a manager method or reads a service without a capable adapter
- **THEN** execution fails explicitly instead of returning a fabricated manager

### Requirement: Information-base predefined-data update mode
The ORM component SHALL provide the closed `ОбновлениеПредопределенныхДанных` / `PredefinedDataUpdate` enumeration and the zero-argument getter `ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы` / `GetInfoBasePredefinedData`. The reached setter `УстановитьОбновлениеПредопределенныхДанныхИнформационнойБазы` / `SetInfoBasePredefinedDataUpdate` SHALL require exactly one enumeration argument, matching the measured 8.3.27 behavior. Getter and setter SHALL delegate to the current Session's data-access adapter and concrete information-base connection. Missing capability SHALL fail explicitly.

#### Scenario: Read and change the predefined-data update mode
- **WHEN** a capable adapter changes the mode and the same Session reads it
- **THEN** the getter returns the adapter's current closed enumeration value

#### Scenario: Setter argument is omitted
- **WHEN** startup calls the setter without an argument
- **THEN** compilation fails with an arity error

#### Scenario: Predefined-data update capability is absent
- **WHEN** startup reads or changes the mode without a capable adapter
- **THEN** execution fails explicitly instead of storing a process-local mode

### Requirement: Initialize predefined data through the Session adapter
The ORM component SHALL provide the zero-argument procedure `ИнициализироватьПредопределенныеДанные` / `InitializePredefinedData`. It SHALL delegate initialization to the current Session's data-access adapter with its frozen principal and concrete information-base connection. The runtime SHALL NOT fabricate predefined records or track initialization in process memory. Missing capability SHALL fail explicitly.

#### Scenario: Initialize predefined data
- **WHEN** startup calls the procedure with a capable adapter
- **THEN** the adapter initializes predefined data for the current Session's information-base area

#### Scenario: Initialization capability is absent
- **WHEN** startup calls the procedure without a capable adapter
- **THEN** execution fails explicitly

### Requirement: Settings description value
The Session component SHALL provide the zero-argument `ОписаниеНастроек` / `SettingsDescription` constructor. A new value SHALL have writable arbitrary `КлючОбъекта` / `ObjectKey` and `КлючНастроек` / `SettingsKey` properties initialized to `Неопределено`, writable string `Пользователь` / `User` and `Представление` / `Presentation` properties initialized to the empty string, and a read-only `ДополнительныеСвойства` / `AdditionalProperties` property containing a distinct mutable empty `Структура` / `Structure`.

#### Scenario: Fill a settings description
- **WHEN** startup constructs a description and assigns all four writable properties and inserts an additional property
- **THEN** subsequent reads return those values

#### Scenario: Replace the additional-properties object
- **WHEN** startup assigns to `ДополнительныеСвойства` / `AdditionalProperties`
- **THEN** execution fails because the property itself is read-only

### Requirement: Current Session date
The base runtime SHALL provide `ТекущаяДатаСеанса()` / `CurrentSessionDate()` as a zero-argument function returning the current instant converted to the effective Session time zone supplied by the host State. A fixed State clock and fixed time zone SHALL therefore produce a deterministic local `Дата` / `Date`. The runtime SHALL use the existing per-State clock and time-zone capabilities and SHALL NOT read a process-global Session zone.

#### Scenario: Session zone differs from UTC
- **WHEN** the State clock denotes a UTC instant and its effective Session zone has a positive offset
- **THEN** the returned naive date is shifted from UTC by that offset

#### Scenario: Two States use different Session zones
- **WHEN** two States of the same Engine use the same fixed clock and different zones
- **THEN** each call returns the local date for its own State

### Requirement: Reached managed data locks
The ORM component SHALL provide the zero-argument `БлокировкаДанных` / `DataLock` constructor with the measured `Data lock` type. Its reached `Добавить` / `Add` method SHALL accept zero or one string lock-space name and return a measured `Data lock item`. A one-argument call SHALL stage that lock space on the object. `Заблокировать()` / `Lock()` SHALL pass all staged named spaces to the current Session's data-access adapter together with its frozen principal and SHALL fail explicitly when no adapter can provide the lock. Locks SHALL remain owned by the adapter's concrete Session connection; the ORM value SHALL NOT emulate cross-Session exclusion in process memory.

#### Scenario: Acquire staged lock spaces
- **WHEN** startup stages named lock spaces and calls `Заблокировать()` in a Session with a data-lock-capable adapter
- **THEN** the adapter acquires those spaces for that Session connection and principal

#### Scenario: Data-lock capability is absent
- **WHEN** startup calls `Заблокировать()` without an adapter that supports data locks
- **THEN** execution fails explicitly instead of continuing without exclusion

#### Scenario: Another Session opens separately
- **WHEN** two Sessions create data-lock values
- **THEN** each value delegates only to its own connection-bound adapter

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

`Выбрать`/`Select` SHALL return a standard-settings selection with `Следующий`/`Next` and read-only `КлючОбъекта`/`ObjectKey`, `КлючНастроек`/`SettingsKey`, `Пользователь`/`User`, and `Настройки`/`Settings` properties. The selection SHALL traverse only the immutable row snapshot returned by the current Session adapter.

#### Scenario: Load current-user settings
- **WHEN** managed-client code loads an existing setting without an explicit user name
- **THEN** the adapter reads the setting for the current Session principal

#### Scenario: Access another user's settings
- **WHEN** code requests another user's setting without data-administration permission
- **THEN** the operation fails instead of reading or writing that user's value

#### Scenario: Client settings are unavailable
- **WHEN** managed-client code uses a settings store without a client-settings adapter
- **THEN** the operation fails explicitly

#### Scenario: Traverse selected settings
- **WHEN** the adapter returns matching settings rows and configuration code advances the selection
- **THEN** the selection exposes each row through the documented properties in adapter order

#### Scenario: Read outside a selected row
- **WHEN** configuration code reads a selection property before the first successful `Следующий` or after it returns `Ложь`
- **THEN** the read fails explicitly as measured on platform 8.3.27.2342

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
The session runtime SHALL provide the boolean startup surface `УстановитьБезопасныйРежим`/`SetSafeMode`, `БезопасныйРежим`/`SafeMode`, `УстановитьПривилегированныйРежим`/`SetPrivilegedMode`, and `ПривилегированныйРежим`/`PrivilegedMode` with the behavior measured on platform 8.3.27. It SHALL also provide `УстановитьОтключениеБезопасногоРежима`/`SetSafeModeDisabled` with exactly one boolean argument and `ПолучитьОтключениеБезопасногоРежима`/`GetSafeModeDisabled` with no arguments. Each enabling call SHALL increment a counter owned by the current BSL call frame. The getter SHALL report whether that counter is nonzero in the current frame only; it SHALL NOT report suppression inherited from a caller or the frame latch after the counter is balanced. A disabling call SHALL decrement only that frame's corresponding counter and SHALL fail if the frame has no matching enable or suppression. Calling `SetSafeModeDisabled(true)` SHALL latch safe-mode suppression for the rest of the current frame; balancing it with `false` SHALL NOT clear that latch. Leaving a frame normally or by exception SHALL remove its remaining enables, counters, and suppression latch. A safe-mode suppression SHALL make the safe-mode query false without deleting safe-mode enables. An effective safe mode SHALL make the privileged-mode query false without discarding privileged enables. Unsupported named security profiles SHALL fail explicitly until a profile catalog is supplied by the host.

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

#### Scenario: Current frame suppresses safe mode
- **WHEN** a frame enables safe mode and then calls `УстановитьОтключениеБезопасногоРежима(Истина)`
- **THEN** `БезопасныйРежим()` returns false until the frame exits
- **AND** balancing the suppression with `Ложь` does not reveal the still-enabled safe mode inside that frame

#### Scenario: Nested frame cannot balance its caller's suppression
- **WHEN** a caller suppresses safe mode and a nested function calls `УстановитьОтключениеБезопасногоРежима(Ложь)` without its own suppression
- **THEN** the nested call raises an error
- **AND** the caller's suppression remains active

#### Scenario: Suppression ends with its frame
- **WHEN** a nested frame suppresses safe mode and then returns or raises an exception
- **THEN** its caller observes the safe mode that was effective before entering the nested frame

#### Scenario: Read current-frame suppression command
- **WHEN** a caller enables suppression and invokes the getter directly and through a nested function
- **THEN** the direct getter returns true
- **AND** the nested getter returns false
- **AND** the direct getter returns false after the caller balances its suppression counter

### Requirement: Background session isolation and parameter inheritance
A background job SHALL run in a new Session for an explicitly supplied selectable principal. It SHALL use a separate database connection and module instances. Its initial session parameters SHALL be copied from the parent session, and later changes in either session SHALL not change the other.

#### Scenario: Inherited parameter
- **WHEN** a parent starts a background job after setting a session parameter
- **THEN** the background session initially observes the copied value

#### Scenario: Independent mutation
- **WHEN** the background session changes an inherited parameter
- **THEN** the parent session retains its own value

### Requirement: Data item send and receive enumerations
The base runtime SHALL provide the closed `ОтправкаЭлементаДанных` / `DataItemSend` system enumeration with the members `Авто` / `Auto`, `Удалить` / `Delete`, and `Игнорировать` / `Ignore`. It SHALL also provide the closed `ПолучениеЭлементаДанных` / `DataItemReceive` system enumeration with the members `Авто` / `Auto`, `Принять` / `Accept`, and `Игнорировать` / `Ignore`. Both language names SHALL resolve to the same corresponding values, and their presentations and member types SHALL match the values measured on platform 8.3.27.

#### Scenario: Ignore an exchange data item
- **WHEN** configuration startup reads `ОтправкаЭлементаДанных.Игнорировать` or `DataItemSend.Ignore`
- **THEN** both expressions return the same `DataItemSend` value with the measured `Ignore` presentation

#### Scenario: Ignore a received exchange data item
- **WHEN** configuration startup reads `ПолучениеЭлементаДанных.Игнорировать` or `DataItemReceive.Ignore`
- **THEN** both expressions return the same `DataItemReceive` value with the measured `Ignore` presentation

### Requirement: Metadata access-right checks
The ORM component SHALL provide `ПравоДоступа` / `AccessRight` with two through four arguments: a right name, a configuration metadata object, an optional information-base user or configuration role, and an optional standard-attribute or standard-tabular-section path. An omitted or `Неопределено` subject SHALL select the frozen current Session principal. The operation SHALL delegate effective role rights and their hierarchy to the current Session's rights-aware adapter. An explicit subject SHALL retain its identity for the adapter's administrative-access check. A privileged BSL frame SHALL return true after validating the argument forms. Missing rights data or an unsupported subject SHALL fail explicitly rather than grant access.

#### Scenario: Check the current principal
- **WHEN** configuration code calls `ПравоДоступа` with a right and a metadata object and the Session adapter grants the effective right
- **THEN** the call returns true
- **AND** `AccessRight` with the corresponding English arguments returns the same value

#### Scenario: Check an explicit role
- **WHEN** configuration code supplies a role metadata object as the third argument
- **THEN** the adapter receives that role identity separately from the current principal

#### Scenario: Rights adapter is unavailable
- **WHEN** configuration code calls `ПравоДоступа` in an ordinary frame without a Session data adapter
- **THEN** the operation fails explicitly instead of returning true

### Requirement: Current user name
The ORM component SHALL provide the zero-argument `ИмяПользователя()` / `UserName()` function. It SHALL return the name from the immutable principal snapshot owned by the current Session. It SHALL return an empty string for the platform user with an empty name, and SHALL NOT reread the information-base user table during the Session.

#### Scenario: Named session principal
- **WHEN** a Session is opened for a selectable principal named `Operator`
- **THEN** `ИмяПользователя()` and `UserName()` both return `Operator`

#### Scenario: Empty platform user
- **WHEN** a Session is opened for the platform user with an empty name
- **THEN** `ИмяПользователя()` returns an empty string of the String type

### Requirement: Mutable map from a fixed map
The runtime SHALL accept `Новый Соответствие(ФиксированноеСоответствие)` and `New Map(FixedMap)`. The resulting mutable map SHALL contain the source pairs in source order. Replacing or removing a pair in the mutable result SHALL NOT change the fixed source. An explicit `Неопределено`, a mutable map, or another value type SHALL fail instead of constructing an empty map.

#### Scenario: Copy and mutate a fixed map
- **WHEN** a fixed map contains one pair and code constructs a mutable map from it and replaces that pair
- **THEN** the mutable map contains the replacement
- **AND** the fixed source retains its original value

#### Scenario: Reject another source type
- **WHEN** code passes `Неопределено` or a mutable map to the one-argument constructor
- **THEN** construction fails explicitly
