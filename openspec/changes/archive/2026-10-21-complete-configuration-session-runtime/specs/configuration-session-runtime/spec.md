## ADDED Requirements

### Requirement: Component-owned manager module reachability
Rooted configuration compilation SHALL retain the module associated by the
host with a statically reached component-global property. This association
SHALL participate in the facade's semantic dependency closure and SHALL NOT
replace ordinary runtime property access or platform-first method dispatch.
Local and module variable shadowing SHALL preserve the existing resolution
priority. Unreached manager modules SHALL NOT be compiled solely because
their metadata exists.

#### Scenario: Manager reached through its component global
- **WHEN** the session handler calls an exported manager method through `Справочники.Версии`
- **AND** the host associates that component property with the private manager module
- **THEN** the rooted catalog includes that module and its semantic dependencies
- **AND** the callback executes in the Session-owned module instance

#### Scenario: Shadowed manager root
- **WHEN** a local or module variable shadows the component global
- **THEN** property access on the variable does not add the component module dependency

#### Scenario: Unreached manager
- **WHEN** the metadata describes a manager module that no selected module reaches
- **THEN** its source does not enter the rooted compilation closure


### Requirement: Declared entry sources contribute configuration dependencies
A host SHALL be able to supply unmodified entry-source texts before freezing a
configuration graph. The ordinary open-bsl semantic resolver SHALL include
component-owned module dependencies reached from those texts in the same
closure as the declared configuration roots. Source declaration SHALL NOT
execute the source, initialize its dependencies, expose private modules as BSL
imports, rewrite the session module or change variable-shadowing priority.
`bsl-cli --config-db` SHALL declare its input script for this analysis. Hosts
that do not declare entry sources SHALL retain their existing graph behavior.

#### Scenario: Object module reached only by the CLI script
- **GIVEN** an empty session initializer and an acquired own catalog object module
- **WHEN** the CLI input script creates the catalog object and invokes its event
- **THEN** its object module is available without a fabricated reference in the session initializer
- **AND** the original entry script executes once

#### Scenario: Shadowed component root in a declared source
- **GIVEN** a declared source whose local or module variable shadows a component global
- **WHEN** the graph dependencies are selected
- **THEN** the shadowed expression does not select that component's private module

#### Scenario: Unreached private module remains dormant
- **GIVEN** a private module outside the roots and declared entry dependencies
- **WHEN** the graph is built and another entry executes
- **THEN** that module's body is not semantically compiled or initialized merely because entry sources were supplied

### Requirement: Reached Windows certificate constructors
The registered HTTP component SHALL resolve `СертификатКлиентаWindows` /
`WindowsClientCertificate` with the documented zero- and one-argument
constructor forms. Without a host adapter for the Windows certificate store,
executing either form SHALL fail explicitly. Registering the constructor SHALL
NOT imply that the host supplies a certificate or a private key.
The component SHALL also resolve the documented zero-argument
`СертификатыУдостоверяющихЦентровWindows` /
`WindowsCertificationAuthorityCertificates` constructor, with the same
explicit failure when the Windows store is unavailable.

#### Scenario: Dormant platform-specific branch
- **WHEN** a configuration method contains a Windows certificate constructor in an unexecuted branch
- **THEN** the configuration module can be compiled

#### Scenario: Windows store is unavailable
- **WHEN** the registered constructor executes without the required host adapter
- **THEN** execution fails with an error identifying the unsupported Windows store

#### Scenario: Windows authority certificates
- **WHEN** a dormant configuration branch constructs Windows authority certificates without arguments
- **THEN** the module compiles
- **AND** executing that constructor without a Windows store adapter fails explicitly


### Requirement: Acquired metadata language code
Language metadata objects SHALL expose their acquired code through the
read-only `КодЯзыка` / `LanguageCode` property. The adapter SHALL preserve
the descriptor string, including an empty string, and reject an unknown
descriptor shape instead of deriving a code from the object's name.
Other metadata kinds SHALL NOT acquire this property implicitly.

#### Scenario: Native language descriptor
- **WHEN** the language descriptor contains the code `ru`
- **THEN** both property names return the string `ru`
- **AND** the result agrees with the native XML `LanguageCode` value

#### Scenario: Missing language code
- **WHEN** a language description lacks the acquired code
- **THEN** accessing the property fails explicitly


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

#### Scenario: Replaced constant identity
- **WHEN** the effective configuration constant and the data snapshot resolve the same name to different object identifiers
- **THEN** reading that constant fails before SQL instead of returning the base object's value

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
The ORM component SHALL provide the reached zero-argument procedures `НачатьТранзакцию()` / `BeginTransaction()`, `ЗафиксироватьТранзакцию()` / `CommitTransaction()`, and `ОтменитьТранзакцию()` / `RollbackTransaction()`, plus the zero-argument function `ТранзакцияАктивна()` / `TransactionActive()`. Each operation SHALL delegate to the current Session's data-access adapter with its frozen principal and SHALL use the same concrete connection as managed data locks, queries, and constants. Missing transaction capability SHALL fail explicitly. Transaction nesting, active-state reporting, commit, rollback, and release SHALL remain owned by that connection-bound adapter.

#### Scenario: Commit a Session transaction
- **WHEN** startup begins and commits a transaction with a capable adapter
- **THEN** both operations execute in order on the current Session's connection

#### Scenario: Roll back after a caught failure
- **WHEN** startup begins a transaction and calls rollback from its exception handler
- **THEN** the adapter rolls back that same Session transaction

#### Scenario: Query the concrete Session transaction
- **WHEN** startup queries transaction state before, during, and after a transaction
- **THEN** the current Session adapter reports false, true, and false from its concrete connection

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

#### Scenario: Single-session CLI host
- **WHEN** `bsl-cli --config-db` opens its sole Session in a process
- **THEN** that Session receives the host-assigned connection number `1`
- **AND** this number does not imply that the underlying database connection is a 1C platform connection

### Requirement: Information-base session number is a distinct host value
The session runtime SHALL resolve `НомерСеансаИнформационнойБазы()` /
`InfoBaseSessionNumber()` and the current information-base session's read-only
`НомерСеанса` / `SessionNumber` property. Both SHALL return the same positive
number explicitly supplied for that Session. The value SHALL remain stable
for the Session lifetime and SHALL NOT be derived from its connection number
or inherited by a background Session. If the host has not supplied it,
access SHALL fail explicitly.

#### Scenario: Function and object agree
- **WHEN** a Session has a host-supplied session number
- **THEN** both language spellings and the current-session property return it

#### Scenario: Host has no platform session number
- **WHEN** a Session without a supplied number calls the function or reads the property
- **THEN** each access fails explicitly instead of returning a synthetic number

### Requirement: Button representation is a closed system enumeration
The base runtime SHALL expose all four members of `ОтображениеКнопки` /
`ButtonRepresentation` through its existing system-enumeration table. The
Russian and English member names SHALL identify the same values, with the
type and presentations measured on platform 8.3.27.

#### Scenario: Read the combined picture and text member
- **WHEN** BSL reads `ОтображениеКнопки.КартинкаИТекст` and `ButtonRepresentation.PictureAndText`
- **THEN** both are equal and present as `Icon and text`

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
For web-client and thin-client Sessions, the runtime SHALL expose
`ХранилищеОбщихНастроек`/`CommonSettingsStorage`,
`ХранилищеСистемныхНастроек`/`SystemSettingsStorage`,
`ХранилищеНастроекДанныхФорм`/`FormDataSettingsStorage`,
`ХранилищеПользовательскихНастроекОтчетов`/`ReportsUserSettingsStorage`, and
`ХранилищеПользовательскихНастроекДинамическихСписков`/
`DynamicListsUserSettingsStorage` through a client-settings adapter explicitly
supplied for that Session when the configuration uses their standard stores.
The standard manager SHALL provide `Загрузить`/`Load` with one through four
arguments, `Сохранить`/`Save` with three through five arguments,
`ПолучитьСписок`/`GetList` with one or two arguments,
`ПолучитьОписание`/`GetDescription` with one through three arguments,
`Удалить`/`Delete` with three arguments, and `Выбрать`/`Select` with zero or
one argument. The adapter SHALL receive the selected principal and the selected
storage kind and SHALL enforce the documented user-data-save and
data-administration rights. Missing storage data, denied access, and an absent
adapter SHALL remain distinguishable and SHALL NOT become an unrestricted or
process-global store.

Database-backed common and system settings loads SHALL accept another exact
user-name namespace when the current principal has DataAdministration or the
call has effective privileged mode. Loading the current principal's exact
namespace SHALL NOT require SaveUserData. An omitted user name SHALL select
the current principal; an explicit empty name, different letter case and a
nonexistent user name SHALL remain literal storage namespaces. Authorization
SHALL precede reading a different namespace, including an absent namespace.
Unknown or unacquired rights SHALL fail explicitly rather than grant access.
These read capabilities SHALL NOT turn unsupported writes into successful
no-ops or imply support for another storage's database layout.

`Выбрать`/`Select` SHALL return a standard-settings selection with `Следующий`/`Next` and read-only `КлючОбъекта`/`ObjectKey`, `КлючНастроек`/`SettingsKey`, `Пользователь`/`User`, and `Настройки`/`Settings` properties. The selection SHALL traverse only the immutable row snapshot returned by the current Session adapter.

#### Scenario: Load current-user settings
- **WHEN** managed-client code loads an existing setting without an explicit user name
- **THEN** the adapter reads the setting for the current Session principal

#### Scenario: Load form-data settings
- **WHEN** managed-client code loads a setting through `ХранилищеНастроекДанныхФорм` or `FormDataSettingsStorage`
- **THEN** the adapter receives the current Session principal and the form-data storage kind

#### Scenario: Read report setting keys
- **WHEN** managed-client code requests the setting list through
  `ReportsUserSettingsStorage`
- **THEN** the current Session adapter receives the report-user storage kind
  and the returned keys are exposed as a `ValueList`

#### Scenario: Delete dynamic-list settings
- **WHEN** managed-client code deletes settings through
  `DynamicListsUserSettingsStorage`
- **THEN** the current Session adapter receives all three selectors unchanged,
  including `Undefined` selectors meaning all matching values

#### Scenario: Access another user's settings
- **WHEN** code requests another user's setting without data-administration permission
- **THEN** the operation fails instead of reading or writing that user's value

#### Scenario: Authorized cross-user database load
- **WHEN** a nonprivileged principal with DataAdministration loads another user's acquired common or system setting
- **THEN** the adapter returns that namespace's stored value
- **AND** SaveUserData is not required for the read

#### Scenario: Literal empty and differently cased user names
- **WHEN** a caller explicitly supplies an empty or differently cased user name
- **THEN** the adapter authorizes and queries that exact namespace
- **AND** it does not replace it with the current user's canonical name
- **AND** absent data becomes Undefined only after authorization succeeds

#### Scenario: Settings namespace does not identify an existing user
- **WHEN** an authorized administrator requests a seeded namespace with no corresponding information-base user
- **THEN** the adapter returns its value without inventing a user or rejecting the namespace

#### Scenario: Privileged settings load
- **WHEN** a caller without DataAdministration has effective privileged mode and loads another namespace
- **THEN** the supported common or system database load succeeds as measured

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

### Requirement: Data-separation safe mode is frame-scoped and Session-bound
The session runtime SHALL provide `УстановитьБезопасныйРежимРазделенияДанных` / `SetDataSeparationSafeMode` with exactly two arguments: a String common-attribute name and a mode converted through the measured ordinary Boolean rules. Numbers use zero/nonzero; strings accept `Истина`, `Да`, `True`, `Ложь`, `Нет`, and `False` without case and with surrounding whitespace, while `yes`, `no`, numeric strings, and empty strings fail. Conversion failures SHALL leave counters unchanged. The host SHALL supply the data-separation common-attribute names for the concrete configuration through Session resources. An absent list, an unknown name, or an invalid argument type SHALL fail before mode state changes. Each BSL call frame SHALL own a separate counter for each folded name: `Истина` / `True` increments it, `Ложь` / `False` decrements it, and decrementing a name that the current frame did not enable SHALL fail. An effective name SHALL remain active while any frame in the current synchronous call stack has a nonzero counter and SHALL be removed on normal return or exception unwinding. Another Session SHALL start without active data-separation safe modes. The facade call context SHALL expose an effective per-name query to Session-bound data adapters.

#### Scenario: Nested call observes and cannot remove the caller's mode
- **WHEN** one frame enables a configured common attribute and a nested frame queries it
- **THEN** the nested frame observes the effective mode
- **AND** attempting to disable the caller's enable fails

#### Scenario: Frame exit clears an unbalanced enable
- **WHEN** a frame enables a configured common attribute and returns or raises an exception without disabling it
- **THEN** its caller no longer observes that frame's enable

#### Scenario: Reject an unknown separator
- **WHEN** code names a common attribute absent from the Session resource list
- **THEN** the operation fails before changing any data-separation mode

#### Scenario: Sessions are isolated
- **WHEN** one Session enables a configured common attribute
- **THEN** another Session does not observe it

#### Scenario: Convert a mode without changing state on failure
- **WHEN** code passes numeric zero/nonzero or a recognized Boolean string as the mode
- **THEN** it disables/enables the current frame counter according to the measured conversion
- **AND** an unconvertible string, Undefined, Null, collection, or Date fails without changing the counter

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
The ORM component SHALL provide `ПравоДоступа` / `AccessRight` with two through four arguments: a right name, a configuration metadata object, an optional information-base user or configuration role, and an optional standard-attribute or standard-tabular-section path. An omitted or `Неопределено` subject SHALL select the frozen current Session principal. The operation SHALL delegate effective role rights and their hierarchy to the current Session's rights-aware adapter. An explicit subject SHALL retain its identity for the adapter's administrative-access check. Effective privileged mode SHALL NOT bypass validation of right names, standard paths or right applicability. An applicable current-principal check may bypass grants only after that validation. In the retained Code/Description standard-attribute cases, Read is inapplicable: AccessRight SHALL raise a catchable applicability error, whereas VerifyAccessRights SHALL succeed without a denial event. Empty and nonexistent standard paths SHALL remain errors in both modes. Missing rights data or an unsupported subject SHALL fail explicitly rather than grant access.

#### Scenario: Check the current principal
- **WHEN** configuration code calls `ПравоДоступа` with a right and a metadata object and the Session adapter grants the effective right
- **THEN** the call returns true
- **AND** `AccessRight` with the corresponding English arguments returns the same value

#### Scenario: Check an explicit role
- **WHEN** configuration code supplies a role metadata object as the third argument
- **THEN** the adapter receives that role identity separately from the current principal

#### Scenario: Explicit catalog role preserves its grants in privileged mode
- **GIVEN** the retained Read/View catalog control and an explicitly supplied acquired role
- **WHEN** AccessRight checks that role in an ordinary or privileged caller frame
- **THEN** the result follows the supplied role's grants and standard-attribute policy in both modes
- **AND** the caller's privileged mode does not turn a refused explicit-role grant into true

#### Scenario: Explicit catalog user requires administrative authority
- **GIVEN** the retained catalog control and an explicitly supplied information-base user description, including the caller's own description
- **WHEN** an applicable right is checked without privileged mode
- **THEN** the caller must have Administration before the supplied user's role grants are evaluated
- **AND** privileged mode may bypass that administrative prerequisite but the result still follows the supplied user's grants

#### Scenario: Applicability precedes the explicit-user administrative check
- **WHEN** the retained nonadministrative caller supplies its own user description for Read on catalog Code or Description
- **THEN** AccessRight reports the applicability error before an administrative-access error

#### Scenario: Explicit subject has missing role rights
- **WHEN** any role needed for the explicitly requested subject has no loaded rights resource
- **THEN** the operation fails explicitly instead of omitting that role or borrowing the caller's grants

#### Scenario: Rights adapter is unavailable
- **WHEN** configuration code calls `ПравоДоступа` in an ordinary frame without a Session data adapter
- **THEN** the operation fails explicitly instead of returning true

#### Scenario: Privileged mode preserves structural validation
- **WHEN** the current frame is privileged and AccessRight receives an unknown right, nonexistent standard path or inapplicable standard Read
- **THEN** it raises the corresponding catchable error instead of returning true without validation

#### Scenario: Standard View follows the acquired role profile
- **WHEN** View is requested for retained Code/Description targets
- **THEN** native parent gating, defaults and explicit refusals match the retained CF/XML role profiles
- **AND** target identity uses the complete member path rather than its GUID alone

#### Scenario: Two roles preserve the parent boundary of a standard grant
- **GIVEN** the retained Code/View profiles with attribute defaults false and independent child rights disabled
- **WHEN** one role grants owner View but refuses Code View and another grants Code View but refuses owner View
- **THEN** AccessRight returns false and VerifyAccessRights reports a denial
- **AND** granting owner View in the second role changes the Code check to allowed

#### Scenario: Unknown right precedes an empty standard path
- **WHEN** either operation receives UnknownNativeRight and an empty String standard path
- **THEN** the adapter reports the unknown-right error before the invalid-path error, including in privileged mode
- **AND** the ORM forwards String contents for structural validation instead of rejecting emptiness before the adapter

#### Scenario: Independent child verification differs from ordinary access
- **GIVEN** the retained single-role profile with independent child rights enabled, attribute defaults false, owner View false and Code View true
- **WHEN** the current principal checks Code View without privileged mode
- **THEN** AccessRight returns false but VerifyAccessRights succeeds without recording a denial
- **AND** each operation delegates its distinct role policy to the acquired SDK evaluator

### Requirement: Current user name
The ORM component SHALL provide the zero-argument `ИмяПользователя()` / `UserName()` function. It SHALL return the name from the immutable principal snapshot owned by the current Session. It SHALL return an empty string for the platform user with an empty name, and SHALL NOT reread the information-base user table during the Session.

#### Scenario: Named session principal
- **WHEN** a Session is opened for a selectable principal named `Operator`
- **THEN** `ИмяПользователя()` and `UserName()` both return `Operator`

#### Scenario: Empty platform user
- **WHEN** a Session is opened for the platform user with an empty name
- **THEN** `ИмяПользователя()` returns an empty string of the String type

### Requirement: Current user full name

The ORM component SHALL provide zero-argument `ПолноеИмяПользователя()` / `UserFullName()`. It SHALL return the full-name field frozen in the current Session's principal. Database-backed principal candidates SHALL copy this field from the selected `v8users` record separately from the short user name. A principal without a supplied full name SHALL return an empty String, matching the empty-user 1C oracle `tests/conformance/measure/1c/measure-user-full-name.platform.txt` from platform 8.3.27.2342. The function SHALL NOT reread the user table during the Session.

#### Scenario: Named principal has a distinct full name

- **WHEN** a Session is opened for a principal named `Operator` with full name `Operations Team`
- **THEN** both language aliases return `Operations Team`
- **AND** `ИмяПользователя()` still returns `Operator`

#### Scenario: Platform user list is empty

- **WHEN** a Session is opened with the default empty principal
- **THEN** `ПолноеИмяПользователя()` returns an empty String

### Requirement: Mutable map from a fixed map
The runtime SHALL accept `Новый Соответствие(ФиксированноеСоответствие)` and `New Map(FixedMap)`. The resulting mutable map SHALL contain the source pairs in source order. Replacing or removing a pair in the mutable result SHALL NOT change the fixed source. An explicit `Неопределено`, a mutable map, or another value type SHALL fail instead of constructing an empty map.

#### Scenario: Copy and mutate a fixed map
- **WHEN** a fixed map contains one pair and code constructs a mutable map from it and replaces that pair
- **THEN** the mutable map contains the replacement
- **AND** the fixed source retains its original value

#### Scenario: Reject another source type
- **WHEN** code passes `Неопределено` or a mutable map to the one-argument constructor
- **THEN** construction fails explicitly

### Requirement: Configuration code can construct a passive job schedule value
The session component SHALL provide the zero-argument `РасписаниеРегламентногоЗадания` / `JobSchedule` constructor. The value SHALL expose the mutable date, time, numeric and array properties documented by platform 8.3.27.2342. A fresh value SHALL contain empty dates, zero numeric properties except `ПериодНедель` / `WeeksPeriod` equal to one, and three separate empty arrays. Constructing or editing the value SHALL NOT register or execute a scheduled job.

#### Scenario: A fresh schedule has the measured defaults
- **WHEN** configuration code constructs a job schedule
- **THEN** its dates are empty, its week period is one, its other numbers are zero, and its week-day, month and detailed-day arrays are empty

#### Scenario: Schedule properties are session-local mutable data
- **WHEN** configuration code writes the documented properties and changes their arrays
- **THEN** reads from that same value observe the writes and a separately constructed value remains unchanged

#### Scenario: The value does not create a scheduled job
- **WHEN** configuration code constructs and edits a job schedule
- **THEN** no background or scheduled job is submitted

### Requirement: Functional-option evaluation uses the current Session data adapter
The ORM component SHALL provide `ПолучитьФункциональнуюОпцию` / `GetFunctionalOption` with one required string name and one optional Structure of parameter values. It SHALL copy that request and delegate evaluation to the `DataAccess` attached to the current Session together with the frozen principal and current call context. It SHALL fail explicitly when the adapter does not implement functional-option evaluation.

#### Scenario: Evaluate an unparameterized functional option
- **WHEN** configuration code calls the function with a name only
- **THEN** the current Session adapter receives that name and an empty parameter set and its value is returned unchanged

#### Scenario: Evaluate a parameterized functional option
- **WHEN** configuration code calls the function with a name and a Structure
- **THEN** the current Session adapter receives the Structure fields in insertion order together with their values

#### Scenario: Reject an unsupported adapter
- **WHEN** configuration code calls the function and the current Session adapter does not implement functional-option evaluation
- **THEN** the call fails explicitly instead of returning an invented disabled value

### Requirement: A temporary-tables manager is bound to one Session adapter
The ORM component SHALL provide the zero-argument `МенеджерВременныхТаблиц` / `TempTablesManager` constructor and its zero-argument `Закрыть` / `Close` method. Construction SHALL obtain an opaque manager handle from the current Session `DataAccess`; every close call SHALL return that handle to the same adapter. Physical-table removal and the validity of later data operations SHALL remain the adapter's responsibility.

#### Scenario: Create and close a manager
- **WHEN** configuration code constructs a temporary-tables manager and closes it
- **THEN** the current Session adapter receives one create operation and one close operation for the same opaque handle

#### Scenario: Close a manager twice
- **WHEN** configuration code calls Close twice for one manager
- **THEN** both calls reach the same Session adapter, which can provide the measured idempotent result

#### Scenario: An unsupported adapter fails explicitly
- **WHEN** configuration code constructs a temporary-tables manager and the current Session adapter does not implement it
- **THEN** construction fails instead of creating an in-memory substitute

### Requirement: Query execution carries its temporary-tables manager
`Запрос.МенеджерВременныхТаблиц` / `Query.TempTablesManager` SHALL be a mutable property whose initial value is `Неопределено` / `Undefined`. It SHALL accept a manager created by the same Session adapter. Query execution SHALL include that manager's opaque handle in `QueryRequest`, including after Close so the adapter can distinguish an operation that does not need temporary tables from one that uses a removed table. A manager from another Session adapter SHALL be rejected explicitly.

#### Scenario: Execute two queries in one temporary-table scope
- **WHEN** code assigns one manager to two Query objects and executes them
- **THEN** both immutable requests carry the same adapter-issued handle

#### Scenario: Reject a manager from another Session
- **WHEN** code assigns a manager created through another Session adapter
- **THEN** assignment fails before a query reaches either adapter

#### Scenario: Place, read, and destroy a temporary table
- **WHEN** two queries use one open manager to place and then read a temporary table
- **THEN** the second query sees the rows placed by the first in that Session only
- **AND** the placement returns a QueryResult with one `Count` row holding the number of placed rows
- **AND** a final `УНИЧТОЖИТЬ` returns `Неопределено`

#### Scenario: A closed manager is still assignable
- **WHEN** code assigns a closed manager to a query that does not use temporary tables
- **THEN** the query can execute
- **AND** a later `ПОМЕСТИТЬ` through that manager fails explicitly

#### Scenario: A placed value-table source is a snapshot
- **WHEN** a typed `ТаблицаЗначений` parameter is placed into a temporary table and then changed
- **THEN** a later query through the same manager reads the values placed before the change
- **AND** an untyped or unsupported source column fails explicitly instead of being assigned a guessed query type

#### Scenario: Append reports only newly placed rows
- **WHEN** a query appends one row to a temporary table that already contains one row
- **THEN** the append returns a QueryResult with `Count = 1`
- **AND** a later query through the manager reads two rows

#### Scenario: A placed database source is a snapshot
- **WHEN** a query places a row read from a database table and that source row changes afterward
- **THEN** a later query through the same manager reads the value from placement time
- **AND** a direct query of the source table reads its changed value

### Requirement: Configuration code can construct string qualifiers
The base runtime SHALL provide `КвалификаторыСтроки` / `StringQualifiers` with zero through two arguments. The value SHALL expose its measured numeric length and closed `ДопустимаяДлина` / `AllowedLength` member through read-only properties. Omitting arguments SHALL produce the measured zero length and variable-length member. The allowed-length enumeration SHALL contain the complete platform member set measured for this value.

#### Scenario: Construct default string qualifiers
- **WHEN** configuration code constructs string qualifiers without arguments
- **THEN** the value reports zero length and the variable-length member

#### Scenario: Construct fixed string qualifiers
- **WHEN** configuration code supplies a positive length and the fixed-length member
- **THEN** both read-only properties report those supplied values

#### Scenario: Normalize the measured numeric lengths
- **WHEN** configuration code supplies `-1` or `1.5` as the length
- **THEN** the value reports `0` or `1`, respectively

#### Scenario: Reject property mutation
- **WHEN** configuration code attempts to replace either qualifier property
- **THEN** the write fails explicitly

### Requirement: Reached type descriptions accept string qualifiers
The base runtime SHALL accept the direct two-through-five-argument `ОписаниеТипов` / `TypeDescription` constructor family in addition to its existing one-argument form. The reached three-argument form SHALL accept type names, an omitted number qualifier, and a `КвалификаторыСтроки` / `StringQualifiers` value. It SHALL retain the same internal type identities as the corresponding one-argument description. Supported qualifier objects SHALL be accepted by their own type across the optional qualifier positions, matching the measured platform behavior. Another value type SHALL fail explicitly. The separate constructor based on an existing type description, `Типы` / `Types`, and `СодержитТип` / `ContainsType` are outside this startup slice.

#### Scenario: Construct a string-qualified type description
- **WHEN** configuration code constructs a String description with an omitted number qualifier and a fixed String qualifier
- **THEN** construction succeeds with the TypeDescription type and the String identity retained internally

#### Scenario: Accept a supported qualifier across optional positions
- **WHEN** configuration code supplies a date qualifier in the nominal String-qualifier position
- **THEN** construction succeeds, as on platform 8.3.27

### Requirement: XML type lookup reuses the XML component mapping
The XML component SHALL provide `XMLТипЗнч` / `XMLTypeOf` with exactly one argument and the two-string `ТипДанныхXML` / `XMLDataType` constructor. A mapped primitive value SHALL return an immutable XML-data-type value containing the measured local name and namespace. Equal name and namespace pairs SHALL compare equal. A value without a measured mapping SHALL return `Неопределено` / `Undefined`. The mapping SHALL be shared with the XML/XDTO primitive mapping rather than maintained as a second table.

#### Scenario: Resolve primitive XML Schema types
- **WHEN** configuration code asks for the XML type of a String, Number, Boolean, or Date
- **THEN** it receives `string`, `decimal`, `boolean`, or `dateTime` in the XML Schema namespace

#### Scenario: Return Undefined for an unmapped value
- **WHEN** configuration code asks for the XML type of Undefined or an unsupported object
- **THEN** the function returns Undefined

#### Scenario: Construct and compare an XML data type
- **WHEN** configuration code constructs two XMLDataType values with equal names and namespaces
- **THEN** their read-only properties match the supplied strings and the values compare equal

### Requirement: Global XDTO serializer
The XML component SHALL expose `СериализаторXDTO` / `XDTOSerializer` as a library global backed by the existing XDTO serializer and the same configuration XDTO factory as global `ФабрикаXDTO` / `XDTOFactory`. It SHALL NOT construct an unrelated empty factory. If configuration XDTO metadata is unavailable, reading the global SHALL fail explicitly with the configuration-factory error.

#### Scenario: Compile a global serializer call
- **WHEN** configuration code invokes a documented serializer method through either global language name
- **THEN** the resolver binds the name to the XML component's serializer value

#### Scenario: Configuration factory is unavailable
- **WHEN** code reads the global serializer without configuration XDTO metadata
- **THEN** the operation fails explicitly instead of returning a serializer with an empty model

### Requirement: XML type-assignment values match the platform enumeration
The base runtime SHALL expose the complete `НазначениеТипаXML` / `XMLTypeAssignment` system enumeration with `Неявное` / `Implicit` and `Явное` / `Explicit`. Each member SHALL have the presentation measured on platform 8.3.27.2074 and both language names SHALL identify the same value.

#### Scenario: Read every XML type-assignment member
- **WHEN** configuration code reads either member through its Russian or English name
- **THEN** it receives the same typed enumeration value and measured presentation

### Requirement: Configuration startup can compare strings without case
The base runtime SHALL provide `СтрСравнить` / `StrCompare` with exactly two arguments. It SHALL convert both values through the ordinary unformatted String representation, compare the resulting strings without case using the ordering measured on platform 8.3.27, and return `-1`, `0`, or `1` as a Number. It SHALL NOT use the identifier-folding rule as an unmeasured replacement for the platform string ordering.

#### Scenario: Compare strings that differ only by case
- **WHEN** configuration code compares two strings that differ only by letter case
- **THEN** the function returns zero

#### Scenario: Compare strings in measured order
- **WHEN** configuration code compares two unequal strings covered by the platform measurement
- **THEN** the function returns a negative or positive Number with the same sign as the platform result

#### Scenario: Compare values after standard String conversion
- **WHEN** configuration code compares Number `1` with String `"1"` or Undefined with an empty String
- **THEN** the function returns zero

### Requirement: Current language is fixed when a Session opens
The Session runtime SHALL provide `ТекущийЯзык()` / `CurrentLanguage()`. The host SHALL resolve the current principal's selected interface language and fall back to the configuration's primary language before running startup code. For the reached managed-Session form, every call SHALL return the same configuration Language metadata object without rereading user or configuration tables. A separately opened foreground or background Session SHALL receive its own resolved language identity. Missing or inconsistent language data SHALL fail explicitly when the function is requested. The platform's documented String result variant is outside this reached slice.

#### Scenario: Use the current principal's language
- **WHEN** a Session opens with an interface language selected for its principal
- **THEN** both language names return the immutable value resolved for that Session

#### Scenario: Fall back to the configuration language
- **WHEN** the current principal has no selected interface language
- **THEN** the host resolves the configuration's primary language before startup and the function returns it

#### Scenario: Keep Sessions isolated
- **WHEN** two Sessions open with different resolved language values
- **THEN** each function call returns the value owned by its own Session

#### Scenario: Reject an unknown configured user language before CLI startup
- **WHEN** `bsl-cli --config-db` selects a named user whose acquired interface-language field is unknown, or whose selected language is absent from configuration metadata
- **THEN** startup fails explicitly before running the Session module
- **AND** only a known nil selection or the platform empty user uses the primary configuration language

### Requirement: Configuration startup can name document posting modes
The base runtime SHALL provide the complete measured `РежимПроведенияДокумента` / `DocumentPostingMode` enumeration. It SHALL contain the operational and non-operational members with their measured Russian and English names and presentations. Providing these values SHALL NOT imply support for document posting or register writes.

#### Scenario: Use either posting mode
- **WHEN** configuration code reads either Russian member
- **THEN** it receives a value of the DocumentPostingMode type with the measured presentation

#### Scenario: Use the English aliases
- **WHEN** configuration code reads the corresponding English enum and member names
- **THEN** it receives the same values as through the Russian names

### Requirement: Configuration startup can name document write modes
The base runtime SHALL provide the complete measured `РежимЗаписиДокумента` / `DocumentWriteMode` enumeration. It SHALL contain the write, posting, and undo-posting members with their measured Russian and English names and presentations. Providing these values SHALL NOT imply support for document persistence or posting.

#### Scenario: Use every document write mode
- **WHEN** configuration code reads the write, posting, or undo-posting member
- **THEN** it receives a value of the DocumentWriteMode type with the measured presentation

#### Scenario: Use the English write-mode aliases
- **WHEN** configuration code reads the corresponding English enum and member names
- **THEN** it receives the same values as through the Russian names

### Requirement: Reference search uses the current Session data adapter
The ORM component SHALL provide `НайтиПоСсылкам` / `FindByRef` with one required Array and three optional Array-or-Undefined arguments. It SHALL delegate the reference list and search restrictions to the current Session's rights-aware data adapter together with the frozen principal and current call context. The adapter's neutral tabular result SHALL be returned as a standard ValueTable. An empty result SHALL retain the measured `Ссылка`, `Данные`, `Метаданные`, and `РазделениеДанных` columns (`Ref`, `Data`, `Metadata`, and `DataSeparation` in the English platform presentation). Missing adapter support and invalid argument forms SHALL fail explicitly.

#### Scenario: Search with only a reference list
- **WHEN** configuration code supplies one Array
- **THEN** the adapter receives that list with all three restrictions absent and its rows are materialized as a ValueTable

#### Scenario: Preserve explicit search restrictions
- **WHEN** configuration code supplies search-area, include, or exclude arrays
- **THEN** the adapter receives those values distinctly and in their declared positions

#### Scenario: Reject unavailable reference search
- **WHEN** the Session adapter does not implement reference search
- **THEN** the call fails explicitly instead of returning an empty table

### Requirement: Configuration startup can count substring occurrences
The base runtime SHALL provide `СтрЧислоВхождений` / `StrOccurrenceCount` with exactly two arguments. It SHALL convert both values through the ordinary unformatted String representation, count non-overlapping case-sensitive matches, and return the count as a Number. An empty substring SHALL return zero. The operation SHALL use the runtime's existing string representation rather than introduce another text representation.

#### Scenario: Count repeated substring occurrences
- **WHEN** configuration code counts a substring that occurs several times
- **THEN** the function returns the platform-measured occurrence count

#### Scenario: Preserve measured edge behavior
- **WHEN** the searched substring overlaps with itself, is empty, or differs only by letter case
- **THEN** only non-overlapping case-sensitive matches are counted and an empty substring produces zero

#### Scenario: Convert non-string arguments
- **WHEN** either argument is not a String
- **THEN** the function searches their ordinary unformatted String representations

### Requirement: Configuration startup can name platform types
The base runtime SHALL provide the complete platform 8.3.27 `ТипПлатформы` / `PlatformType` enumeration with all seventeen documented members and their measured presentations. Russian and English enumeration names SHALL resolve to the same values. Providing the enumeration SHALL NOT imply detection of the current host platform.

#### Scenario: Use every platform type
- **WHEN** configuration code reads any documented PlatformType member
- **THEN** it receives a value of the PlatformType with the measured presentation

#### Scenario: Use the English enumeration name
- **WHEN** configuration code reads a member through `PlatformType`
- **THEN** it receives the same value as through `ТипПлатформы`

### Requirement: Standard time offsets use the State time-zone capability
The base runtime SHALL provide `СмещениеСтандартногоВремени` / `StandardTimeOffset` with zero through two arguments: an optional String-or-Undefined zone identifier and an optional Date, canonical date String, or Undefined UTC time. It SHALL return the standard offset from UTC in seconds as a Number without the daylight-time adjustment. An omitted or Undefined zone SHALL use the current State zone. An omitted or Undefined time SHALL use the current State clock. A canonical date String SHALL use the same conversion as the one-argument `Дата(...)` constructor. Named IANA zones and documented GMT offset identifiers SHALL be resolved by the State's time-zone capability. Unknown zones and other invalid argument types SHALL fail explicitly.

#### Scenario: Read the current State zone
- **WHEN** configuration code omits the zone and supplies either a UTC date or no date
- **THEN** the result comes from the current State's time-zone capability and clock

#### Scenario: Read a named standard offset
- **WHEN** configuration code supplies a valid IANA or GMT zone identifier and a UTC date
- **THEN** it receives that zone's standard offset without a daylight-time adjustment

#### Scenario: Keep State zones isolated
- **WHEN** two States carry different time-zone capabilities
- **THEN** each call observes only its own current and named-zone results

#### Scenario: Reject an unavailable zone
- **WHEN** configuration code supplies an unknown zone identifier or an invalid argument type
- **THEN** the call fails explicitly

### Requirement: Session time-zone identity belongs to the State
The base runtime SHALL provide zero-argument `ЧасовойПоясСеанса` / `SessionTimeZone` and return the identifier of the current State's time-zone capability as a String. A system zone SHALL report its configured IANA identity when available, and a fixed zone SHALL report an equivalent documented GMT identifier. Two States MAY carry different identities. If a custom host zone provides offsets without an identity, the call SHALL fail explicitly. This getter SHALL NOT add process-global time-zone mutation or the unreached session-zone setter.

#### Scenario: Read a State time-zone identifier
- **WHEN** configuration code calls either language name in a State whose zone has an identity
- **THEN** both calls return that State's identifier as a String

#### Scenario: Keep zone identities isolated
- **WHEN** two States carry different time-zone identities
- **THEN** each getter returns only its own identity

#### Scenario: Reject a missing identity
- **WHEN** a custom State time-zone capability has no identifier
- **THEN** the getter fails explicitly

### Requirement: Acquired documents use the existing manager path
The ORM component SHALL expose `Документы` / `Documents` as a library global backed by acquired `MetadataKind::Document` descriptors. A folded property name matching a Document SHALL return its existing manager value and SHALL dispatch through the same platform-first, exported-manager-module-second path as other configuration managers. `Метаданные.Документы` / `Metadata.Documents` SHALL expose the same acquired descriptors through the existing metadata-collection behavior. Unknown names and members SHALL fail explicitly. This projection SHALL NOT imply document persistence, posting, or register writes.

#### Scenario: Resolve a document manager
- **WHEN** configuration code reads an acquired document through either manager-root language name
- **THEN** it receives the manager linked to that Document descriptor

#### Scenario: Call an exported document manager method
- **WHEN** configuration code calls an exported method not owned by the platform surface
- **THEN** the existing manager-module instance handles the call with its manager as `ЭтотОбъект`

#### Scenario: Read document metadata
- **WHEN** configuration code reads the same document through `Метаданные.Документы` or `Metadata.Documents`
- **THEN** it receives the matching immutable metadata descriptor

### Requirement: Acquired document journals use the existing manager path
The ORM component SHALL expose `ЖурналыДокументов` / `DocumentJournals` as a library global backed by acquired `MetadataKind::DocumentJournal` descriptors. A folded property name matching a DocumentJournal SHALL return its existing manager value and SHALL dispatch through the same platform-first, exported-manager-module-second path as other configuration managers. `Метаданные.ЖурналыДокументов` / `Metadata.DocumentJournals` SHALL expose the same acquired descriptors through the existing metadata-collection behavior. Unknown names and members SHALL fail explicitly. This projection SHALL NOT imply journal selection, filtering, or form behavior.

#### Scenario: Resolve a document-journal manager
- **WHEN** configuration code reads an acquired document journal through either manager-root language name
- **THEN** it receives the manager linked to that DocumentJournal descriptor

#### Scenario: Call an exported document-journal manager method
- **WHEN** configuration code calls an exported method not owned by the platform surface
- **THEN** the existing manager-module instance handles the call with its manager as `ЭтотОбъект`

#### Scenario: Read document-journal metadata
- **WHEN** configuration code reads the same document journal through `Метаданные.ЖурналыДокументов` or `Metadata.DocumentJournals`
- **THEN** it receives the matching immutable metadata descriptor

### Requirement: Acquired reports use the existing manager path
The ORM component SHALL expose `Отчеты` / `Reports` as a library global backed by acquired `MetadataKind::Report` descriptors. A folded property name matching a Report SHALL return its existing manager value and SHALL dispatch through the same platform-first, exported-manager-module-second path as other configuration managers. `Метаданные.Отчеты` / `Metadata.Reports` SHALL expose the same acquired descriptors through the existing metadata-collection behavior. Unknown names and members SHALL fail explicitly. This projection SHALL NOT imply report-form opening or data-composition execution.

#### Scenario: Resolve a report manager
- **WHEN** configuration code reads an acquired report through either manager-root language name
- **THEN** it receives the manager linked to that Report descriptor

#### Scenario: Call an exported report-manager method
- **WHEN** configuration code calls an exported method not owned by the platform surface
- **THEN** the existing manager-module instance handles the call with its manager as `ЭтотОбъект`

#### Scenario: Read report metadata
- **WHEN** configuration code reads the same report through `Метаданные.Отчеты` or `Metadata.Reports`
- **THEN** it receives the matching immutable metadata descriptor

### Requirement: Acquired data processors use the existing manager path
The ORM component SHALL expose `Обработки` / `DataProcessors` as a library global backed by acquired `MetadataKind::DataProcessor` descriptors. A folded property name matching a DataProcessor SHALL return its existing manager value and SHALL dispatch through the same platform-first, exported-manager-module-second path as other configuration managers. `Метаданные.Обработки` / `Metadata.DataProcessors` SHALL expose the same acquired descriptors through the existing metadata-collection behavior. Unknown names and members SHALL fail explicitly. This projection SHALL NOT imply data-processor object creation or form opening.

#### Scenario: Resolve a data-processor manager
- **WHEN** configuration code reads an acquired data processor through either manager-root language name
- **THEN** it receives the manager linked to that DataProcessor descriptor

#### Scenario: Call an exported data-processor manager method
- **WHEN** configuration code calls an exported method not owned by the platform surface
- **THEN** the existing manager-module instance handles the call with its manager as `ЭтотОбъект`

#### Scenario: Read data-processor metadata
- **WHEN** configuration code reads the same data processor through `Метаданные.Обработки` or `Metadata.DataProcessors`
- **THEN** it receives the matching immutable metadata descriptor

### Requirement: Every acquired database-backed manager kind is projected
The ORM component SHALL expose the remaining acquired database-backed manager kinds through the standard Russian and English roots: `РегистрыНакопления` / `AccumulationRegisters`, `РегистрыБухгалтерии` / `AccountingRegisters`, `РегистрыРасчета` / `CalculationRegisters`, `ПланыСчетов` / `ChartsOfAccounts`, `ПланыВидовХарактеристик` / `ChartsOfCharacteristicTypes`, `ПланыВидовРасчета` / `ChartsOfCalculationTypes`, `БизнесПроцессы` / `BusinessProcesses`, `Задачи` / `Tasks`, and `ХранилищаНастроек` / `SettingsStorages`. Each root and its corresponding `Метаданные` / `Metadata` collection SHALL project descriptors already present in the immutable catalog through the existing generic manager path. This projection SHALL NOT imply persistence, register writes, workflow execution, or settings-storage operations.

#### Scenario: Resolve every acquired manager kind
- **WHEN** the catalog contains one descriptor of each listed kind
- **THEN** both language names resolve its manager and the corresponding metadata collection resolves the same descriptor

#### Scenario: Reject an unknown configured name
- **WHEN** configuration code reads an unknown property from any listed manager root
- **THEN** the existing explicit unknown-property error is returned

### Requirement: Acquired sequences use the existing manager path
The database configuration loader SHALL classify platform Sequence descriptors as `MetadataKind::Sequence`. The ORM component SHALL expose them through `Последовательности` / `Sequences` and `Метаданные.Последовательности` / `Metadata.Sequences` using the existing generic manager and metadata-collection behavior. This projection SHALL NOT imply sequence-boundary registration or mutation.

#### Scenario: Resolve an acquired sequence
- **WHEN** the acquired configuration contains a Sequence descriptor
- **THEN** both manager-root language names and both metadata-root language names resolve that descriptor case-insensitively

### Requirement: Allowed-sign values match the platform enumeration
The base runtime SHALL expose the complete `ДопустимыйЗнак` / `AllowedSign` system enumeration with `Любой` / `Any` and `Неотрицательный` / `Nonnegative`. Each member SHALL have the presentation measured on platform 8.3.27 and both language names SHALL identify the same value.

#### Scenario: Read every allowed-sign member
- **WHEN** configuration code reads either member through its Russian or English name
- **THEN** it receives the same typed enumeration value and measured presentation

### Requirement: Number qualifiers preserve measured values
The base runtime SHALL provide `Новый КвалификаторыЧисла` / `New NumberQualifiers` with zero through three arguments and read-only `Разрядность` / `Digits`, `РазрядностьДробнойЧасти` / `FractionDigits`, and `ДопустимыйЗнак` / `AllowedSign` properties. Omitted counts SHALL be zero and the omitted sign SHALL be `ДопустимыйЗнак.Любой`. Numeric counts SHALL be truncated to integers, clamped to zero, and the fractional count SHALL be capped at the total count. A supplied sign of another type SHALL fail explicitly.

#### Scenario: Construct explicit number qualifiers
- **WHEN** configuration code supplies total digits, fractional digits, and an allowed sign
- **THEN** both language property names expose the normalized counts and the same typed sign

#### Scenario: Construct default number qualifiers
- **WHEN** configuration code omits every argument
- **THEN** both counts are zero and the sign is `ДопустимыйЗнак.Любой`

#### Scenario: Reject mutation
- **WHEN** configuration code assigns any number-qualifier property
- **THEN** the assignment fails explicitly

### Requirement: Unsafe-operation protection descriptions are mutable values
The Session runtime SHALL provide zero-argument `Новый ОписаниеЗащитыОтОпасныхДействий` / `New UnsafeOperationProtectionDescription`. The resulting value SHALL expose the Boolean read-write property `ПредупреждатьОбОпасныхДействиях` / `UnsafeOperationWarnings`, initially `Истина`; another assigned type SHALL fail explicitly. The value SHALL be independent of the host process and SHALL NOT itself disable runtime safety checks.

#### Scenario: Construct and change the protection description
- **WHEN** configuration startup constructs the value and assigns a Boolean through either property name
- **THEN** the property initially reads `Истина` and both property names read the assigned value from that instance

#### Scenario: Reject another property type
- **WHEN** configuration code assigns a non-Boolean value
- **THEN** the assignment fails explicitly and preserves the preceding value

### Requirement: Reached external-component attachment calls are explicit
The Session runtime SHALL resolve `ПодключитьВнешнююКомпоненту` / `AttachAddIn`
with the documented one-through-four-argument family so the recorded
configuration startup graph can compile. Until the host supplies an audited
external-component loader, invoking any form SHALL fail explicitly and SHALL
NOT report a successful attachment or load process code implicitly.

#### Scenario: Compile every documented attachment form
- **WHEN** configuration code uses the identifier form or the location-and-name form with its optional arguments
- **THEN** semantic resolution accepts one through four arguments through both language names

#### Scenario: Invoke attachment without a host loader
- **WHEN** configuration code invokes the function in a Session without an external-component loader
- **THEN** the call fails explicitly without loading code or reporting success

### Requirement: Reached query-schema construction is explicit
The ORM runtime SHALL resolve the documented zero-argument `Новый СхемаЗапроса` / `New QuerySchema` constructor so the recorded configuration startup graph can compile. Until the query-schema object model is implemented, invoking the constructor SHALL fail explicitly and SHALL NOT return a partial or unrelated query value.

#### Scenario: Compile query-schema construction
- **WHEN** configuration code constructs `СхемаЗапроса` through either language name
- **THEN** semantic resolution accepts exactly zero arguments

#### Scenario: Invoke query-schema construction before its object model exists
- **WHEN** configuration code invokes the constructor
- **THEN** the call fails explicitly without returning a partial object

#### Scenario: Dormant query-schema method call
- **WHEN** a compiled configuration method contains a call on a `СхемаЗапроса` value but startup never invokes that method
- **THEN** linking that module succeeds and startup continues
- **AND** invoking the unsupported constructor still fails explicitly

### Requirement: Date fractions match the complete platform enumeration
The base runtime SHALL expose the complete `ЧастиДаты` / `DateFractions` system enumeration with `Время` / `Time`, `Дата` / `Date`, and `ДатаВремя` / `DateTime`. Each member SHALL have the presentation measured on platform 8.3.27, and both language names SHALL identify the same typed values.

#### Scenario: Read every date-fraction member
- **WHEN** configuration code reads any member through its Russian or English name
- **THEN** it receives the same typed enumeration value and measured presentation

### Requirement: Automatic change recording matches the complete platform enumeration
The base runtime SHALL expose the complete `АвтоРегистрацияИзменений` /
`AutoChangeRecord` system enumeration with `Запретить` / `Deny` and
`Разрешить` / `Allow`. Each member SHALL have the presentation and value
type measured on platform 8.3.27.2342, and both language names SHALL identify
the same typed values. Exposing the values SHALL NOT imply exchange-plan writes
or automatic change-record persistence.

#### Scenario: Read every automatic-change-recording member
- **WHEN** configuration code reads either member through its Russian or English name
- **THEN** it receives the same typed enumeration value and measured presentation

### Requirement: Descriptor-backed base-runtime objects keep their declared methods
An object constructed by the base-runtime library and associated with an
`ObjectMembersDescriptor` SHALL resolve methods through that descriptor. Its
library provenance SHALL NOT redirect the call to a same-named legacy
`BuiltinMethod` implementation. Omitted argument positions SHALL reach the
descriptor method with the measured platform default semantics.

#### Scenario: Value-list add skips the check argument
- **WHEN** configuration code calls `ValueList.Add` with value, presentation,
  an omitted check position, and a picture
- **THEN** the declared value-list method creates the item with the measured
  default check value without collapsing the omitted position

### Requirement: Predefined values expose their full platform name
The ORM component SHALL provide
`ПолучитьПолноеИмяПредопределенногоЗначения` /
`GetPredefinedValueFullName` with one required arbitrary argument. For a
supported system-enumeration member it SHALL return canonical enum and member
names in the acquired configuration ScriptVariant, as measured on platform 8.3.27.2342. For a supported configuration-enumeration reference it SHALL
return the canonical kind, owner, and member names from the same immutable
catalog; an empty reference SHALL use `ПустаяСсылка` or `EmptyRef`. An unsupported value SHALL fail explicitly.
The operation SHALL NOT read the database or create another reference value.

#### Scenario: Round-trip a system enumeration
- **WHEN** configuration code passes a system-enumeration value previously
  returned by `PredefinedValue`
- **THEN** the function returns its canonical full name in the acquired configuration ScriptVariant

#### Scenario: Name a configuration enumeration reference
- **WHEN** configuration code passes a configuration-enumeration reference
- **THEN** the function returns the full name of its catalog owner and member, including the measured empty-reference member name

### Requirement: Navigation links use the current Session data adapter
The ORM component SHALL expose `ПолучитьНавигационнуюСсылку` / `GetURL` with
the one-through-four-position overload family documented by platform 8.3.27.
It SHALL preserve supplied and omitted positions and delegate link construction
to the `DataAccess` attached to the current Session together with that
Session's immutable principal and call context. If the adapter does not support
navigation links, the call SHALL fail explicitly. The ORM component SHALL NOT
construct a link from a value's display text.

#### Scenario: Get a link for an applied object reference
- **WHEN** configuration code passes one applied object reference
- **THEN** the current Session adapter returns its navigation-link string

#### Scenario: Adapter lacks navigation-link support
- **WHEN** configuration code requests a link and the current adapter does not support it
- **THEN** the call fails instead of returning an empty or fabricated link

### Requirement: User password policies come from the current Session adapter
The ORM component SHALL expose the read-only global
`ПолитикиПаролейПользователей` / `UserPasswordPolicies`. Its
`НайтиПоИмени` / `FindByName` method SHALL accept zero or one string argument
and delegate lookup to the `DataAccess` attached to the current Session together
with that Session's immutable principal and call context. A found policy SHALL
be returned as a detached snapshot exposing the numeric read-only property
`МинимальнаяДлинаПаролей` / `PasswordMinLength`; an unknown policy SHALL return
`Неопределено`. If the adapter does not support policy lookup, the call SHALL
fail explicitly. The ORM component SHALL NOT cache a policy across Sessions or
implement the unreached policy mutation and password-validation surface.

#### Scenario: Read a named password policy
- **WHEN** configuration startup finds a policy by name and reads its minimum password length
- **THEN** the current Session adapter receives the name and returns the value exposed by the detached policy snapshot

#### Scenario: A policy does not exist
- **WHEN** the current Session adapter finds no policy with the supplied name
- **THEN** `FindByName` returns `Неопределено`

#### Scenario: Adapter lacks password-policy support
- **WHEN** configuration code searches for a policy through an adapter without that operation
- **THEN** the call fails explicitly instead of reporting that the policy does not exist

### Requirement: The information-base password minimum uses the current Session adapter
The ORM component SHALL expose the zero-argument
`ПолучитьМинимальнуюДлинуПаролейПользователей` /
`GetUserPasswordMinLength` function documented by platform 8.3.27. It SHALL
delegate the numeric information-base-wide limit to the `DataAccess` attached
to the current Session together with that Session's immutable principal and
call context. If the adapter does not support this read, the function SHALL
fail explicitly. The ORM component SHALL NOT cache the limit or expose the
unreached setter as part of this slice.

#### Scenario: Read the password minimum
- **WHEN** configuration startup requests the information-base password minimum
- **THEN** it receives the number returned by the current Session adapter

#### Scenario: Adapter lacks the password-minimum read
- **WHEN** configuration code requests the limit through an adapter without that operation
- **THEN** the call fails explicitly instead of returning a default

### Requirement: Random-password generation does not use weak fallback entropy
The ORM component SHALL resolve the measured zero-argument
`ГенераторСлучайныхПаролей` / `RandomPasswordGenerator` constructor and its
zero-through-two-argument `СлучайныйПароль` / `RandomPassword` method through
both language names. Until the host supplies an audited cryptographic random
source suitable for passwords, invoking the method SHALL fail explicitly. It
SHALL NOT use the UUID random capability's non-cryptographic fallback, return a
predictable placeholder, or ignore the memorable-password argument while
reporting success.

#### Scenario: Compile the reached password-generator call
- **WHEN** configuration code constructs the generator and calls its method with an explicit length
- **THEN** semantic resolution accepts the measured constructor and method arity

#### Scenario: Invoke generation without password-grade entropy
- **WHEN** configuration code invokes the method before the host supplies the required capability
- **THEN** the call fails explicitly without returning a password

### Requirement: Event-log event use is a mutable detached value
The Session runtime SHALL provide `Новый ИспользованиеСобытияЖурналаРегистрации` /
`New EventLogEventUse` with zero through two arguments. The first argument SHALL
be Boolean and default to `Истина`; the second SHALL accept an arbitrary value
and default to `Неопределено`. The resulting value SHALL expose mutable
`Использование` / `Use` and `ОписаниеИспользования` / `UseDescription`
properties. Assignment to `Use` SHALL apply the measured Boolean conversion,
while construction with a non-Boolean first argument SHALL fail explicitly.
Constructing or changing this detached value SHALL NOT itself reconfigure the
event log.

#### Scenario: Construct the default event use
- **WHEN** configuration code omits both constructor arguments
- **THEN** `Use` is `Истина` and `UseDescription` is `Неопределено`

#### Scenario: Construct and mutate explicit values
- **WHEN** configuration code supplies both arguments and later assigns both properties through either language name
- **THEN** the same object exposes the resulting Boolean and arbitrary description values

#### Scenario: Reject a non-Boolean constructor argument
- **WHEN** configuration code supplies another type as the first constructor argument
- **THEN** construction fails explicitly

### Requirement: Event-log event-use operations use the current Session adapter
The Session runtime SHALL expose `ПолучитьИспользованиеСобытияЖурналаРегистрации` /
`GetEventLogEventUse` with one required string event name and
`УстановитьИспользованиеСобытияЖурналаРегистрации` / `SetEventLogEventUse`
with a required string name and `EventLogEventUse` value. Both operations SHALL
delegate to the event-log adapter attached to the current Session. The read
SHALL return a detached copied value; the write SHALL pass its current Boolean
and arbitrary description to the adapter. A missing adapter or unsupported
operation SHALL fail explicitly. The Session runtime SHALL NOT retain a
process-local event-use setting.

#### Scenario: Read a named event setting
- **WHEN** configuration code asks for a named event's use setting
- **THEN** it receives the copied value returned by the current Session event-log adapter

#### Scenario: Write a named event setting
- **WHEN** configuration code passes an `EventLogEventUse` value to the setter
- **THEN** the current Session adapter receives the name, Boolean, and description

#### Scenario: Adapter lacks event-use administration
- **WHEN** configuration code reads or writes a setting through an adapter without those operations
- **THEN** the operation fails explicitly instead of using process-local state

### Requirement: Client settings are a detached mutable value
The Session runtime SHALL provide the zero-argument
`НастройкиКлиентскогоПриложения` / `ClientSettings` constructor. The resulting
value SHALL expose mutable Boolean properties
`ОтображатьПанелиНавигацииИДействий` / `ShowNavigationAndActionsPanels` and
`ОтображатьПанельРазделов` / `ShowSectionsPanel`, defaulting to true. It SHALL
also expose mutable `РежимОткрытияФормПриложения` /
`ApplicationFormsOpenningMode`, defaulting to `Закладки` / `Tabs`, and
`ВариантИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceVariant`, defaulting to `Версия8_2` / `Version8_2`.
The value SHALL be detached: construction and property mutation SHALL perform
no I/O and SHALL NOT create shared state between Sessions.

The base runtime SHALL provide the complete measured closed enumerations
`РежимОткрытияФормПриложения` / `ApplicationFormsOpenningMode` with members
`Закладки` / `Tabs` and `ОтдельныеОкна` / `SingleWindows`, and
`ВариантИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceVariant` with members `Версия8_2` / `Version8_2`
and `Такси` / `Taxi`. Assigning a member of another enumeration to either
enumeration property SHALL fail explicitly.

#### Scenario: Construct defaults
- **WHEN** configuration code constructs `ClientSettings` without arguments
- **THEN** both panel properties are true, opening mode is `Tabs`, and interface variant is `Version8_2`

#### Scenario: Mutate through both language spellings
- **WHEN** configuration code assigns all four properties through Russian or English names using members of the corresponding enumerations
- **THEN** the same object exposes the assigned Boolean and enumeration values

#### Scenario: Reject an unrelated enumeration
- **WHEN** configuration code assigns a member of another enumeration to either enumeration property
- **THEN** assignment fails explicitly

### Requirement: Command-interface settings are a detached mutable value
The Session runtime SHALL provide the zero-argument
`НастройкиКомандногоИнтерфейса` / `CommandInterfaceSettings` constructor. The
resulting detached value SHALL expose the mutable
`ОтображениеПанелиРазделов` / `SectionsPanelRepresentation` property,
defaulting to `КартинкаИТекст` / `PictureAndText`. Construction and mutation
SHALL perform no I/O and SHALL NOT share state between Sessions.

The base runtime SHALL provide the complete measured closed enumeration
`ОтображениеПанелиРазделов` / `SectionsPanelRepresentation`: `Картинка` /
`Picture`, `КартинкаИТекст` / `PictureAndText`, `КартинкаСверхуИТекст` /
`PictureOnTopAndText`, `КартинкаСлеваИТекст` / `PictureOnLeftAndText`, and
`Текст` / `Text`. Assigning a member of another enumeration SHALL fail
explicitly.

#### Scenario: Construct the default command-interface settings
- **WHEN** configuration code constructs `CommandInterfaceSettings` without arguments
- **THEN** its section-panel representation is `PictureAndText`

#### Scenario: Mutate through either language spelling
- **WHEN** configuration code assigns a measured section-panel member through the Russian or English property name
- **THEN** the same object exposes that member

#### Scenario: Reject an unrelated section-panel value
- **WHEN** configuration code assigns a member of another enumeration
- **THEN** assignment fails explicitly

### Requirement: Client-interface content is an explicit detached value graph
The Session runtime SHALL provide the zero-argument
`НастройкиИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceSettings`,
`НастройкиСоставаИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceContentSettings`, and
`ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceContentSettingsGroup` constructors, together with
the one-string `ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceContentSettingsItem` constructor.

A content value SHALL expose four distinct read-only group properties:
`Верх` / `Top`, `Лево` / `Left`, `Низ` / `Bottom`, and `Право` / `Right`.
A group SHALL accept item or group values through `Добавить` / `Add` and
SHALL expose the reached `Количество` / `Count` and `Получить` / `Get`
operations. An item SHALL expose its read-only `Имя` / `Name` string.

`УстановитьСостав` / `SetContent` SHALL retain an explicitly supplied content
value in the settings object, and `ПолучитьСостав` / `GetContent` SHALL return
that value. Reading before an explicit set, omitting the content argument, or
supplying a value of another type SHALL fail explicitly; the runtime SHALL NOT
fabricate the configuration-derived initial composition.

#### Scenario: Build and retain the reached composition
- **WHEN** configuration code puts a named item into a group, puts that group into one of the four position groups, and passes the content to `SetContent`
- **THEN** `Count` and `Get` expose the inserted values and `GetContent` returns the explicitly supplied content graph

#### Scenario: Use English names
- **WHEN** the same graph is constructed and populated through the English constructor, property, and method names
- **THEN** it has the same types, item names, and group contents

#### Scenario: Initial host composition is unavailable
- **WHEN** configuration code calls `GetContent` before supplying content explicitly
- **THEN** execution fails explicitly instead of returning an invented empty composition

#### Scenario: Reject a value outside the content graph
- **WHEN** `Add` receives neither a group nor an item, or `SetContent` receives no content value or a value of another type
- **THEN** execution fails explicitly

### Requirement: An access-event use description is a detached mutable value
The Session runtime SHALL provide
`ОписаниеИспользованияСобытияДоступЖурналаРегистрации` /
`EventLogAccessEventUseDescription` construction with zero arguments or one
optional string object name. The value SHALL expose mutable `Объект` /
`Object`, `ПоляДоступа` / `AccessFields`, and `ПоляРегистрации` /
`LoggedFields` properties. The object name SHALL be a string, both field
properties SHALL be arrays, and the zero-argument defaults SHALL be an empty
string and two distinct empty arrays. Construction and mutation SHALL perform
no I/O.

#### Scenario: Construct and mutate an access-event description
- **WHEN** configuration code constructs the value without arguments and then assigns an object name and two arrays
- **THEN** the initial values have the measured empty forms and the same object exposes all three assigned values

#### Scenario: Construct from a metadata name
- **WHEN** configuration code passes a string through either language spelling of the constructor
- **THEN** the resulting `Object` property contains that string

#### Scenario: Reject wrong property types
- **WHEN** configuration code assigns a non-string object or a non-array field collection
- **THEN** assignment fails explicitly

### Requirement: Event-log level use comes from the current Session adapter
The Session runtime SHALL provide the zero-argument
`ПолучитьИспользованиеЖурналаРегистрации` / `GetEventLogUsing` function. It
SHALL ask the current Session's event-log adapter for the enabled
`УровеньЖурналаРегистрации` / `EventLogLevel` members and SHALL return them
as a fresh array. The runtime SHALL NOT infer enabled levels from local writes,
cache them across Sessions, or substitute the levels observed in a measurement
base. An absent adapter or an adapter without this operation SHALL fail
explicitly.

#### Scenario: Read enabled levels
- **WHEN** the current Session adapter reports a subset of event-log levels
- **THEN** the function returns an array containing that subset in adapter order

#### Scenario: Keep Sessions isolated
- **WHEN** two Sessions have adapters reporting different enabled levels
- **THEN** each call returns only its own Session adapter's snapshot

#### Scenario: Event-log administration is unavailable
- **WHEN** the current Session has no event-log adapter or its adapter does not implement enabled-level reading
- **THEN** the call fails explicitly

### Requirement: Authentication-lock settings belong to the current information base
The ORM runtime SHALL expose the read-only `БлокировкаАутентификации` /
`AuthenticationLock` global as an
`МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы` /
`InfoBaseUserAuthenticationLockManager`. Its `ПолучитьНастройки` /
`GetSettings` method SHALL obtain a complete copied settings snapshot from the
current Session's `DataAccess`. The snapshot SHALL expose mutable numeric
`ДлительностьБлокировки` / `LockDuration` and
`МаксимальноеКоличествоНеуспешныхПопыток` /
`MaxUnsuccessfulAttemptsCount` properties and mutable string
`КодыДополненияИмениПользователя` / `UserNameAdditionCodes`.

`УстановитьНастройки` / `SetSettings` SHALL pass all three current property
values and the current principal to the same `DataAccess`. Omitting the
optional settings argument, supplying another type, an absent adapter, or an
adapter without the operation SHALL fail explicitly. The ORM runtime SHALL
NOT keep an authoritative process-local copy or share a snapshot between
Sessions.

#### Scenario: Read and mutate a copied snapshot
- **WHEN** the current Session adapter returns lock duration, attempt count, and user-name addition codes
- **THEN** `GetSettings` returns a detached value exposing all three, and local property mutation does not call the adapter

#### Scenario: Preserve every setting on write
- **WHEN** configuration code changes either reached numeric property and passes the value to `SetSettings`
- **THEN** the adapter receives the current values of both numeric properties and the unchanged user-name addition codes with the current principal

#### Scenario: Keep authentication settings Session-bound
- **WHEN** two Sessions have adapters returning different settings
- **THEN** each manager call uses only its own Session adapter and principal

#### Scenario: Authentication administration is unavailable
- **WHEN** no data adapter is present, the adapter does not implement the operation, or `SetSettings` receives no settings value or a value of another type
- **THEN** the call fails explicitly

### Requirement: Password-strength checking comes from the current information base
The ORM runtime SHALL provide the zero-argument
`ПолучитьПроверкуСложностиПаролейПользователей` /
`GetUserPasswordStrengthCheck` function. It SHALL return the Boolean setting
obtained from the current Session's `DataAccess` with the current principal.
The runtime SHALL NOT substitute a local default, cache the setting across
Sessions, or provide a write operation as part of this slice. An absent adapter
or an adapter without this operation SHALL fail explicitly.

#### Scenario: Read the current setting
- **WHEN** the current Session adapter reports whether password-strength checking is enabled
- **THEN** either language spelling returns that Boolean value

#### Scenario: Keep password-strength settings Session-bound
- **WHEN** two Sessions have adapters reporting different values
- **THEN** each call returns only its own Session adapter's value and supplies that Session's principal

#### Scenario: Password-strength setting is unavailable
- **WHEN** no data adapter is present or its adapter does not implement this operation
- **THEN** the call fails explicitly

### Requirement: Numeric password settings come from the current information base
The ORM runtime SHALL provide the zero-argument getters, in both language
spellings, for user-password maximum effective period, minimum effective
period, reuse limit, and expiration-notification period. Each getter SHALL
return the Number supplied by the current Session's `DataAccess` with the
current principal. The runtime SHALL NOT substitute local defaults or cache
values across Sessions. An absent adapter or unsupported operation SHALL fail
explicitly.

#### Scenario: Read numeric password settings
- **WHEN** the current Session adapter supplies the four numeric settings
- **THEN** each Russian or English getter returns its corresponding Number

#### Scenario: Keep numeric password settings Session-bound
- **WHEN** two Sessions have adapters supplying different settings
- **THEN** each getter uses only its own Session adapter and principal

#### Scenario: Numeric password settings are unavailable
- **WHEN** no data adapter is present or its adapter does not implement these operations
- **THEN** the getter fails explicitly

### Requirement: Password-setting writes stay behind the Session adapter
The ORM runtime SHALL expose the syntax-helper-defined one-argument procedures,
in both language spellings, for setting password-strength checking, minimum
password length, maximum and minimum effective periods, reuse limit, and
expiration-notification period. Each procedure SHALL pass the supplied BSL
value unchanged together with the setting identity and current principal to
the current Session's `DataAccess`. The ORM runtime SHALL NOT infer conversion,
range-validation, authorization, or persistence rules. Missing adapters and
unsupported writes SHALL fail explicitly.

#### Scenario: Delegate a password-setting write
- **WHEN** configuration code calls one of the six procedures with one argument
- **THEN** the current Session adapter receives that setting identity, unchanged value, and current principal exactly once

#### Scenario: Keep password-setting writes Session-bound
- **WHEN** separate Sessions invoke password-setting procedures
- **THEN** each write reaches only its own Session adapter with its own principal

#### Scenario: Reject an unavailable password-setting write
- **WHEN** the current Session has no data adapter or its adapter does not implement writes
- **THEN** the procedure fails explicitly without keeping a process-local value

### Requirement: Formatted strings are immutable base-runtime values
The base runtime SHALL provide `Новый ФорматированнаяСтрока` / `New
FormattedString` in the variadic content form and the five-position formatting
form documented by the Syntax Assistant. The measured runtime SHALL reject an
empty argument list. Content SHALL accept strings,
formatted strings, pictures, and arrays of those values. The value SHALL be
immutable and SHALL preserve its content segments and supplied formatting
values. Plain string conversion, filledness, type identity, and equality SHALL
match the measured platform behavior. Unsupported content types SHALL fail
explicitly.

#### Scenario: Concatenate content
- **WHEN** configuration code constructs a formatted string from strings, nested formatted strings, or an array of supported content
- **THEN** its plain string conversion is the concatenation in source order

#### Scenario: Preserve formatting description
- **WHEN** configuration code uses the five-position formatting constructor
- **THEN** the resulting immutable value retains the supplied content and formatting positions while plain string conversion reflects its content

#### Scenario: Reject unsupported content
- **WHEN** no argument is supplied or a content position or content array contains another value type
- **THEN** construction fails explicitly

### Requirement: Query result iteration is a closed platform enumeration
The base runtime SHALL provide `ОбходРезультатаЗапроса` /
`QueryResultIteration` with the complete Syntax Assistant-defined member set:
`Прямой` / `Linear`, `ПоГруппировкам` / `ByGroups`, and
`ПоГруппировкамСИерархией` / `ByGroupsWithHierarchy`. Both spellings SHALL
resolve to the same values. Their string presentations and type identity SHALL
match the measured platform output.

#### Scenario: Use either spelling
- **WHEN** configuration code reads any query-result iteration member through
  its Russian or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `QueryResultIteration`

### Requirement: Data lock mode is a closed platform enumeration
The base runtime SHALL provide `РежимБлокировкиДанных` / `DataLockMode` with
the complete Syntax Assistant-defined member set: `Разделяемый` / `Shared`
and `Исключительный` / `Exclusive`. Both spellings SHALL resolve to the same
values. Their string presentations and type identity SHALL match the measured
platform output. Exposing these values SHALL NOT add a data-lock manager or
database-locking behavior.

#### Scenario: Use either spelling
- **WHEN** configuration code reads either data-lock mode through its Russian
  or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `DataLockMode`

### Requirement: Reached data-composition template composer is explicit
The ORM runtime SHALL resolve the documented zero-argument
`КомпоновщикМакетаКомпоновкиДанных` / `DataCompositionTemplateComposer`
constructor and its `Выполнить` / `Execute` method with two through seven
arguments so the recorded configuration graph can compile. The constructed
value's type SHALL match the measured platform type. Until data-composition
template generation is implemented, invoking `Execute` SHALL fail explicitly
and SHALL NOT return a partial template, query, or collection.

#### Scenario: Compile template composition
- **WHEN** configuration code constructs the composer and calls `Execute`
  through either language spelling with a documented argument count
- **THEN** the constructor and method resolve

#### Scenario: Invoke composition before the engine exists
- **WHEN** the resolved `Execute` method is invoked
- **THEN** it fails explicitly without returning a partial template

### Requirement: Reached data-composition processor is explicit
The ORM runtime SHALL resolve the documented zero-argument
`ПроцессорКомпоновкиДанных` / `DataCompositionProcessor` constructor and its
complete method surface: `Инициализировать` / `Initialize` with one through
six arguments, `Сбросить` / `Reset` with no arguments, and `Следующий` /
`Next` with no arguments. The constructed value's type SHALL match the
measured platform type. Until the data-composition engine is implemented,
each operation SHALL fail explicitly and SHALL NOT return a partial result.

#### Scenario: Compile processor use
- **WHEN** configuration code constructs the processor and calls any method
  through either language spelling with a documented argument count
- **THEN** the constructor and method resolve

#### Scenario: Invoke processing before the engine exists
- **WHEN** any resolved processor method is invoked
- **THEN** it fails explicitly without returning a partial result

### Requirement: Reached value-collection output processor is explicit
The ORM runtime SHALL resolve the documented zero-argument
`ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений` /
`DataCompositionResultValueCollectionOutputProcessor` constructor. It SHALL
provide mutable `ОтображатьПроцентВывода` / `ShowOutputPercent` with the
measured default, `УстановитьОбъект` / `SetObject` with zero or one argument,
`Вывести` / `Output` with one or two arguments, `ВывестиЭлемент` / `OutputItem`
with one argument, and zero-argument `НачатьВывод` / `BeginOutput` and
`ЗакончитьВывод` / `EndOutput`. Until the data-composition engine is
implemented, every method SHALL fail explicitly and SHALL NOT return or mutate
a partial value collection.

#### Scenario: Construct through either spelling
- **WHEN** configuration code constructs the output processor through its
  Russian or English spelling
- **THEN** it receives the measured value type and property default

#### Scenario: Invoke output before the engine exists
- **WHEN** any resolved output method is invoked
- **THEN** it fails explicitly without returning a partial collection

### Requirement: Explicitly omitted imported arguments preserve their slot
For a qualified call to an exported configuration-module procedure or
function, an explicitly omitted argument position SHALL be passed as
`Undefined` when the corresponding parameter has no default. A missing
trailing required argument SHALL remain a compile error. Parameters with a
declared default SHALL continue to receive that default for both an explicit
omission and a missing trailing position.

#### Scenario: Omit a required middle position
- **WHEN** configuration code calls `Module.Function(First, , Third)` and the
  second exported parameter has no default
- **THEN** the callee receives `Undefined` as its second argument

#### Scenario: Leave off a required trailing position
- **WHEN** configuration code calls an imported function with fewer positions
  than its required parameter list
- **THEN** compilation fails with the existing missing-required-argument error

### Requirement: Password-policy compliance results form a closed platform enumeration
The base runtime SHALL provide `РезультатПроверкиСоответствияПароляПолитике` /
`PasswordPolicyComplianceCheckResult` with the complete Syntax
Assistant-defined member set: `НеСоответствуетТребованиямМинимальнойДлины` /
`DoesNotSatisfyMinLengthRequirements`,
`НеСоответствуетТребованиямОграниченияПовторенияСредиПоследних` /
`DoesNotSatisfyReuseLimitRequirements`,
`НеСоответствуетТребованиямПроверкиРаскрытия` /
`DoesNotSatisfyCompromiseCheckRequirements`, and
`НеСоответствуетТребованиямСложности` /
`DoesNotSatisfyComplexityRequirements`. Both language spellings SHALL resolve
to the same values. Their string presentations and type identity SHALL match
the measured platform output. Exposing these values SHALL NOT implement
password-policy evaluation.

#### Scenario: Use every result through either spelling
- **WHEN** configuration code reads any password-policy compliance result
  through its Russian or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `PasswordPolicyComplianceCheckResult`

### Requirement: Table representations form a closed platform enumeration
The base runtime SHALL provide `ОтображениеТаблицы` / `TableRepresentation`
with the complete Syntax Assistant-defined member set: `Список` / `List`,
`ИерархическийСписок` / `HierarchicalList`, and `Дерево` / `Tree`.
Both language spellings SHALL resolve to the same values. Their string
presentations and type identity SHALL match the measured platform output.
Exposing these values SHALL NOT add a form or client-interface implementation.

#### Scenario: Use every table representation through either spelling
- **WHEN** configuration code reads any table representation through its
  Russian or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `TableRepresentation`

### Requirement: An empty picture is a passive base-runtime value
The base runtime SHALL provide the documented zero-argument `Картинка` /
`Picture` constructor. It SHALL return an empty picture whose type, string
presentation, filledness, and equality with another empty picture match the
measured platform behavior. The value SHALL be usable as an ordinary argument
to configuration code. This slice SHALL NOT claim the file or binary-data
constructors, image decoding, conversion, properties, or methods.

#### Scenario: Construct an empty picture through either spelling
- **WHEN** configuration code constructs `New Picture()` or `Новый Картинка()`
- **THEN** it receives the measured empty `Picture` value without performing I/O

### Requirement: Filter application types form a closed platform enumeration
The base runtime SHALL provide `ТипПримененияОтбораКомпоновкиДанных` /
`DataCompositionFilterApplicationType` with the complete Syntax
Assistant-defined member set: `Элементы` / `Items`, `Иерархия` /
`Hierarchy`, and `ТолькоИерархия` / `HierarchyOnly`. Both language
spellings SHALL resolve to the same values. Their string presentations and
type identity SHALL match the measured platform output. Exposing these values
SHALL NOT add a filter evaluator or data-composition settings model.

#### Scenario: Use every filter application type through either spelling
- **WHEN** configuration code reads any filter application type through its
  Russian or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `DataCompositionFilterApplicationType`

### Requirement: Settings-item view modes form a closed platform enumeration
The base runtime SHALL provide
`РежимОтображенияЭлементаНастройкиКомпоновкиДанных` /
`DataCompositionSettingsItemViewMode` with the complete Syntax
Assistant-defined member set: `Авто` / `Auto`, `Обычный` / `Normal`,
`БыстрыйДоступ` / `QuickAccess`, and `Недоступный` / `Inaccessible`.
Both language spellings SHALL resolve to the same values. Their string
presentations and type identity SHALL match the measured platform output.
Exposing these values SHALL NOT add data-composition settings items.

#### Scenario: Use every settings-item view mode through either spelling
- **WHEN** configuration code reads any settings-item view mode through its
  Russian or English spelling
- **THEN** both expressions yield the same member with the measured string
  presentation and type `DataCompositionSettingsItemViewMode`

### Requirement: A data-composition parameter is a passive base-runtime value
The base runtime SHALL provide the Syntax Assistant-defined
`ПараметрКомпоновкиДанных` / `DataCompositionParameter` constructor with exactly
one String name, matching the measured platform rejection of the zero-argument
form. Its string presentation SHALL preserve that name, its filledness SHALL
match whether the name is empty, and equality SHALL compare names without
regard to case. The Russian and English constructor spellings SHALL create the
same value type. This slice SHALL NOT add parameter collections,
data-composition settings, XML/XDTO serialization, or a composition engine.

#### Scenario: Construct a named parameter through either spelling
- **WHEN** configuration code constructs a named data-composition parameter
  through its Russian or English spelling
- **THEN** it receives the same measured passive value type preserving the
  measured name semantics

#### Scenario: Omit the parameter name
- **WHEN** configuration code invokes the constructor without an argument
- **THEN** compilation rejects the call as the measured platform does

### Requirement: Special characters form the complete documented constant set
The language resolver SHALL provide the complete Syntax Assistant-defined
`Символы` / `Chars` constant set: `ВК` / `CR`, `ПС` / `LF`, `ПФ` / `FF`,
`Таб` / `Tab`, `ВТаб` / `VTab`, and `НПП` / `NBSp`. Each spelling SHALL
resolve to the measured one-character String value. A local or module variable
named `Символы` or `Chars` SHALL retain the existing variable precedence.
The implementation SHALL NOT create a runtime object for this immutable set.

#### Scenario: Read every special character through either spelling
- **WHEN** configuration code reads any member through its Russian or English
  spelling without a shadowing variable
- **THEN** it receives the same measured one-character String

#### Scenario: A variable shadows the constant set
- **WHEN** a local or module variable is named `Символы` or `Chars`
- **THEN** property access uses that variable and does not lower the expression
  to a built-in character

### Requirement: XML string processing is exposed through its global manager
The XML component SHALL provide the Syntax Assistant-defined read-only global
`ОбработкаСтрокиXML` / `XMLStringProcessing` with type
`МенеджерОбработкиСтрокиXML` / `XMLStringProcessingManager`. Its
`НайтиНедопустимыеСимволыXML` / `FindDisallowedXMLCharacters` method SHALL
accept one required String and an optional one-based UTF-16 code-unit starting
position whose default is one. It SHALL return the one-based UTF-16 code-unit
position of the first character disallowed by XML 1.0 and 1.1 at or after that
position, or zero when none is found. Zero and three arguments SHALL be
rejected. Russian and English spellings SHALL resolve to the same manager and
method behavior. The method SHALL share the existing XML character classifier
with the legacy Boolean global function.

#### Scenario: Find a disallowed character through either spelling
- **WHEN** configuration code calls the manager method through its Russian or
  English spelling for a String containing a disallowed character
- **THEN** it receives the same measured position through either spelling

#### Scenario: Start after a disallowed character
- **WHEN** configuration code supplies a starting position after the only
  disallowed character
- **THEN** the method returns zero

### Requirement: Form group type is a complete closed system enumeration
The base runtime SHALL expose the complete eight-member `ВидГруппыФормы` /
`FormGroupType` system enumeration described by the platform Syntax Assistant.
Each Russian or English member spelling SHALL resolve to the same value with
the platform-measured String presentation and member type. This enumeration
SHALL NOT imply form-group objects or a client UI implementation.

#### Scenario: Read the usual group type through either spelling
- **WHEN** configuration code reads `ВидГруппыФормы.ОбычнаяГруппа` or
  `FormGroupType.UsualGroup`
- **THEN** both expressions return the same enum value with the measured
  presentation and type

### Requirement: Display importance is a complete closed system enumeration
The base runtime SHALL expose the six Syntax Assistant-defined members of
`ВажностьПриОтображении` / `DisplayImportance`. Each Russian or English
member spelling SHALL resolve to the same value with the platform-measured
String presentation and member type. This enumeration SHALL NOT imply form
rendering.

#### Scenario: Read a display importance through either spelling
- **WHEN** configuration code reads `ВажностьПриОтображении.Обычная` or
  `DisplayImportance.Usual`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Picture format is a complete closed system enumeration
The base runtime SHALL expose the ten Syntax Assistant-defined members of
`ФорматКартинки` / `PictureFormat`. Each Russian or English member spelling
SHALL resolve to the same value with the platform-measured String
presentation and member type. This enumeration SHALL NOT imply picture
conversion or rendering.

#### Scenario: Read a picture format through either spelling
- **WHEN** configuration code reads `ФорматКартинки.PNG` or
  `PictureFormat.PNG`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Form field type is a complete closed system enumeration
The base runtime SHALL provide every Syntax Assistant-defined member of
`ВидПоляФормы` / `FormFieldType`: HTML document, PDF document, input,
geographical schema, graphical schema, dendrogram, chart, Gantt chart,
progress bar, calendar, picture, label, radio button, period, planner, track
bar, spreadsheet document, text document, check box, and formatted document
fields. Each Russian and English member spelling SHALL resolve to the same
value with the measured `FormFieldType` type and String presentation. This
enumeration SHALL NOT imply that form fields or a client UI exist.

#### Scenario: Read every form field type through either spelling
- **WHEN** configuration code reads any documented form field type through its
  Russian or English spelling
- **THEN** both expressions yield the same member with the measured String
  presentation and type `FormFieldType`

### Requirement: Additional show mode is a complete closed system enumeration
The base runtime SHALL provide both Syntax Assistant-defined members of
`ДополнительныйРежимОтображения` / `AdditionalShowMode`: `НеИспользовать` /
`DontUse` and `Неактуальность` / `Irrelevance`. Each Russian and English member
spelling SHALL resolve to the same value with the measured `AdditionalShowMode`
type and String presentation. This enumeration SHALL NOT imply that state
presentation objects or form rendering exist.

#### Scenario: Read every additional show mode through either spelling
- **WHEN** configuration code reads either additional show mode through its
  Russian or English spelling
- **THEN** both expressions yield the same member with the measured String
  presentation and type `AdditionalShowMode`

### Requirement: Boolean conversion follows the measured scalar contract
The base runtime SHALL provide `Булево` / `Boolean` with exactly one Number,
Boolean, or String argument. Zero SHALL convert to false and every other Number
to true; a Boolean SHALL retain its value; Strings SHALL follow the measured
accepted spellings and rejection behavior. Russian and English function names
SHALL have the same result. Missing or extra arguments and values outside the
documented input types SHALL be rejected.

#### Scenario: Convert the reached numeric expression
- **WHEN** configuration code passes zero or a nonzero Number to either
  spelling of the Boolean conversion
- **THEN** it receives false for zero and true for the nonzero Number

#### Scenario: Reject a value outside the scalar contract
- **WHEN** configuration code passes a value outside Number, Boolean, and the
  measured String spellings
- **THEN** the conversion raises a catchable error

### Requirement: Server TLS-certificate revocation modes cross the HTTP host boundary
The base runtime SHALL provide the complete Syntax Assistant-defined
`РежимПроверкиОтзываTLSСертификатаСервера` /
`ServerTLSCertificateRevocationCheckMode` enumeration: `Авто` / `Auto`,
`НеПроверять` / `DontCheck`, `Нестрогий` / `SoftFail`, and `Строгий` /
`Strict`. Both language spellings SHALL resolve to the same values with the
measured type and String presentations.

The HTTP component SHALL accept the documented optional third argument of
`ЗащищенноеСоединениеOpenSSL` / `OpenSSLSecureConnection` and expose the
measured selected value through its read-only
`РежимПроверкиОтзываTLSСертификатаСервера` /
`ServerTLSCertificateRevocationCheckMode` property. An explicitly supplied
mode SHALL be preserved in the neutral HTTP client configuration passed to the
host. A system adapter that cannot enforce that mode SHALL reject client
creation explicitly and SHALL NOT silently weaken it. Calls that omit the new
argument SHALL retain their existing transport behavior. The read-only
property SHALL expose the measured effective `SoftFail` for an omitted,
explicitly `Undefined`, or `Auto` argument; the other three explicit members
SHALL round-trip unchanged.

#### Scenario: Preserve an explicit strict mode
- **WHEN** configuration code constructs a secure connection with `Строгий`
  and passes it to an HTTP connection
- **THEN** the HTTP host receives `Strict`, or client creation fails explicitly
  if that host cannot enforce it

#### Scenario: Omit the revocation mode
- **WHEN** existing configuration code constructs a secure connection with at
  most the two previously supported arguments
- **THEN** its HTTP transport behavior remains unchanged and the read-only
  property has the measured platform value

### Requirement: Client display information belongs to one foreground Session
`bsl-session` SHALL provide the zero-argument
`ПолучитьИнформациюЭкрановКлиента` / `GetClientDisplaysInformation` function.
For a web-client or thin-client Session it SHALL return a fresh
`ФиксированныйМассив` / `FixedArray` over the explicit display snapshot
supplied when that Session opens. Each element SHALL have the immutable type
`ИнформацияЭкранаКлиента` / `ClientDisplayInformation` and the read-only
numeric `Ширина` / `Width`, `Высота` / `Height`, and `DPI` properties. The
first element SHALL remain the first display supplied by the host.

A foreground Session without the snapshot SHALL fail explicitly when the
function is called. A background Session SHALL return `Undefined`, SHALL NOT
require a snapshot, and SHALL NOT inherit a foreground display snapshot.

#### Scenario: Read the foreground client's primary display
- **WHEN** a foreground Session opens with two display descriptions and calls
  the function through either language spelling
- **THEN** it receives a fixed array of two immutable values in host order with
  their exact width, height, and DPI

#### Scenario: Call from a background Session
- **WHEN** a background Session calls the function
- **THEN** it receives `Undefined` regardless of the parent Session's displays

### Requirement: Guarded COM activation is a runtime capability gap
The base runtime SHALL resolve `Новый COMОбъект` / `New COMObject` with one
required argument and one optional argument. If execution reaches the
constructor, it SHALL raise a catchable explicit unsupported error without
activating an external application or returning a fabricated COM value.

#### Scenario: A guarded COM connection attempt
- **WHEN** configuration code executes a one-argument `Новый COMОбъект` call
  inside `Попытка`
- **THEN** resolution succeeds and the `Исключение` branch handles the runtime
  error

### Requirement: Computer name comes from the executing Session
`bsl-session` SHALL expose the zero-argument `ИмяКомпьютера` / `ComputerName`
function. It SHALL return the explicit network computer name supplied when
the current Session opens, without reading the interpreter process's
hostname. A Session without a supplied name SHALL fail explicitly when the
function is called. A background Session SHALL use its own supplied name
and SHALL NOT inherit its parent's name.

#### Scenario: A background Session reports its own computer
- **WHEN** a foreground Session and its background child receive different
  computer names and both call the function
- **THEN** each call returns the name supplied for that Session

### Requirement: Round mode members work with the existing round builtin
The base runtime SHALL expose the complete two-member
`РежимОкругления` / `RoundMode` system enumeration in both language
spellings. Its members SHALL have the measured type display and string
presentations. The existing `Окр` / `Round` builtin SHALL accept those
members as its third argument while preserving its existing numeric mode
behavior.

#### Scenario: Round a half with either mode
- **WHEN** BSL calls `Окр(1.5, 0, РежимОкругления.Окр15как10)` and
  `Окр(1.5, 0, RoundMode.Round15as20)`
- **THEN** the calls return `1` and `2` respectively

### Requirement: Administrative session enumeration fails explicitly
`bsl-session` SHALL resolve the zero-argument
`ПолучитьСеансыИнформационнойБазы` / `GetInfoBaseSessions` call. Until the
host provides an authorized live session roster, execution SHALL raise a
catchable explicit unsupported error rather than returning a fabricated or
incomplete array.

#### Scenario: A guarded administrative roster query
- **WHEN** configuration code calls the function inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Scheduled-job references do not add a scheduler
The session library SHALL resolve the `РегламентныеЗадания` /
`ScheduledJobs` global. Reading the global SHALL raise a catchable explicit
unsupported error. It SHALL NOT alias `ФоновыеЗадания` or create scheduled
jobs.

#### Scenario: A guarded scheduled-job branch
- **WHEN** a module contains a reference to `РегламентныеЗадания` inside
  `Попытка` and execution reaches that reference
- **THEN** the module compiles and `Исключение` handles the unsupported error

### Requirement: Collaboration-system references fail explicitly
The session library SHALL resolve the `СистемаВзаимодействия` /
`CollaborationSystem` global. Reading it without a collaboration host service
SHALL raise a catchable explicit unsupported error.

#### Scenario: A guarded collaboration-system branch
- **WHEN** configuration code reads the global inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Hour boundaries use the existing date value
The base runtime SHALL provide the one-argument `НачалоЧаса` / `BegOfHour`
and `КонецЧаса` / `EndOfHour` builtins. Each SHALL return the measured
beginning or last whole second of the hour containing the input Date,
including the measured empty-date behavior.

#### Scenario: Find both boundaries of an ordinary hour
- **WHEN** BSL calls both functions with `2024-02-17 13:24:35`
- **THEN** it receives `2024-02-17 13:00:00` and `2024-02-17 13:59:59`

### Requirement: Minute boundaries use the existing date value

The base runtime SHALL provide one-argument `НачалоМинуты` / `BegOfMinute` and `КонецМинуты` / `EndOfMinute` builtins. Their results and aliases SHALL match `tests/conformance/measure/1c/measure-minute-boundaries.platform.txt` from platform 8.3.27.2342, including the empty-date case.

#### Scenario: Find both boundaries of an ordinary minute

- **WHEN** BSL calls both functions with `2024-02-17 13:24:35`
- **THEN** it receives `2024-02-17 13:24:00` and `2024-02-17 13:24:59`

### Requirement: Reached standard OData interface content resolves

The ORM component SHALL recognize zero-argument `ПолучитьСоставСтандартногоИнтерфейсаOData` / `GetStandardODataInterfaceContent`, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until the current information base's configured OData content is available through its data adapter, actual calls SHALL fail explicitly and catchably rather than fabricate an empty or complete metadata-object array.

#### Scenario: Dormant configuration method inspects OData content

- **WHEN** a configuration method calls `ПолучитьСоставСтандартногоИнтерфейсаOData()`
- **THEN** semantic resolution accepts the function name and arity

### Requirement: Registry-backed constructor errors identify the type

When registry-backed semantic resolution rejects an unsupported `New` type, its error SHALL include the rejected type name. The rejection SHALL remain a semantic error. Legacy registry-free resolution SHALL preserve its existing unsupported-constructor error.

#### Scenario: Configuration contains an unsupported constructor

- **WHEN** a configuration method contains `Новый НеизвестныйТип()`
- **THEN** semantic resolution fails and the diagnostic names `НеизвестныйТип`

### Requirement: Reached user-work favorites constructor resolves

The base runtime SHALL recognize `Новый ИзбранноеРаботыПользователя` / `New UserWorkFavorites` with no arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until the mutable favorites collection and item value are implemented, actual construction SHALL fail explicitly and catchably.

#### Scenario: Dormant migration method prepares user favorites

- **WHEN** the method constructs `Новый ИзбранноеРаботыПользователя`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached user-work favorite item constructor resolves

The base runtime SHALL recognize `Новый ЭлементИзбранногоРаботыПользователя` / `New UserWorkFavoritesItem` with no arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until its mutable item properties exist, actual construction SHALL fail explicitly and catchably.

#### Scenario: Dormant migration method prepares a favorite item

- **WHEN** the method constructs `Новый ЭлементИзбранногоРаботыПользователя`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached standard OData interface setter resolves

The ORM component SHALL recognize one-argument `УстановитьСоставСтандартногоИнтерфейсаOData` / `SetStandardODataInterfaceContent` as a procedure, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until the current Session's data adapter can apply the configured selection, actual calls SHALL fail explicitly and catchably rather than report a successful write.

#### Scenario: Dormant migration method updates OData content

- **WHEN** a configuration method calls `УстановитьСоставСтандартногоИнтерфейсаOData(Объекты)`
- **THEN** semantic resolution accepts the procedure name and arity

### Requirement: Reached session-lock surfaces resolve without fabricating state

The base runtime SHALL recognize zero-argument `Новый БлокировкаСеансов` / `New SessionsLock`. The session library SHALL recognize zero-argument `ПолучитьБлокировкуСеансов` / `GetSessionsLock` as a function and one-argument `УстановитьБлокировкуСеансов` / `SetSessionsLock` as a procedure. Dormant configuration methods using them SHALL compile. Until the lock value and current information-base administration adapter exist, actual calls SHALL fail explicitly and catchably rather than return an inactive lock or report an unapplied write.

#### Scenario: Dormant maintenance method prepares a session lock

- **WHEN** a configuration method constructs `Новый БлокировкаСеансов` and calls `УстановитьБлокировкуСеансов(Блокировка)`
- **THEN** semantic resolution accepts the constructor and procedure names and arities

### Requirement: Error message display variants match the platform

The base runtime SHALL expose the complete four-member `ВариантОтображенияСообщенияОбОшибке` / `ErrorMessageDisplayVariant` enumeration. Member aliases, type names and String presentations SHALL match `tests/conformance/measure/1c/measure-error-message-display-variant.platform.txt` from platform 8.3.27.2342, including presentation whitespace.

#### Scenario: Configuration selects an error display variant

- **WHEN** a configuration method reads `ВариантОтображенияСообщенияОбОшибке.Авто`
- **THEN** semantic resolution yields the measured `Auto` value

### Requirement: Reached error-message text construction resolves without a false value

The base runtime SHALL recognize `Новый ТекстыСообщенияОбОшибке` / `New ErrorMessageTexts` with exactly two arguments, as measured in `tests/conformance/measure/1c/measure-error-message-texts.platform.txt` on platform 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until the value object is implemented, actual construction SHALL fail explicitly and catchably instead of returning an ordinary structure.

#### Scenario: Monitoring module prepares paired error texts

- **WHEN** a configuration method constructs `Новый ТекстыСообщенияОбОшибке("без перезапуска", "с перезапуском")`
- **THEN** semantic resolution accepts the constructor and its two-argument arity

### Requirement: Configuration diagnostics identify the failing module

When parsing or resolving a configuration graph fails, the facade SHALL name the failing module and preserve the original phase error through Rust's error source chain. Compiling a standalone module SHALL keep its existing error variant and text.

#### Scenario: A configuration method reads an unresolved identifier

- **WHEN** the resolver rejects a method in named configuration module `Служебный`
- **THEN** the returned error names `Служебный` and exposes the underlying semantic error as its source

### Requirement: A loop variable used as its own collection fails on execution

The configuration resolver SHALL allow a method containing `Для Каждого X Из X` with no earlier binding of `X` to compile, and evaluating that loop SHALL raise a catchable error. Other unknown identifiers SHALL remain build errors. Top-level statements and standalone compilation SHALL retain their existing strict resolution behavior. This mirrors the dormant and caught-call observations in `tests/conformance/measure/1c/measure-foreach-dormant.platform.txt` and `measure-foreach-call-caught.platform.txt` on platform 8.3.27.2342.

#### Scenario: A dormant method uses its loop variable as its collection

- **WHEN** a configuration module declares a method containing `Для Каждого Элемент Из Элемент Цикл` without an earlier assignment and the method is not called
- **THEN** the module loads, while a later call to that method raises an error catchable by the caller

### Requirement: Reached number-based string declension resolves with measured arity

The base runtime SHALL recognize `ПолучитьСклоненияСтрокиПоЧислу` / `GetStringDeclensionsByNumber` with two through six arguments, as observed in `tests/conformance/measure/1c/measure-string-declensions-by-number.platform.txt` on platform 8.3.27.2342. Until morphology is implemented, an actual call SHALL fail explicitly and catchably rather than fabricate declined strings.

#### Scenario: Dormant discount message formats a count

- **WHEN** a configuration method calls `ПолучитьСклоненияСтрокиПоЧислу` with five arguments
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached available-time-zone lookup resolves without a fabricated catalog

The base runtime SHALL recognize zero-argument `ПолучитьДопустимыеЧасовыеПояса` / `GetAvailableTimeZones`, as measured in `tests/conformance/measure/1c/measure-available-time-zones.platform.txt` on platform 8.3.27.2342. Until the host supplies a platform-compatible catalog, an actual call SHALL fail explicitly and catchably instead of returning a fixed or empty array.

#### Scenario: A dormant signature check searches for Moscow

- **WHEN** a configuration method calls `ПолучитьДопустимыеЧасовыеПояса().Найти("Europe/Moscow")`
- **THEN** semantic resolution accepts the global function name and zero-argument arity

### Requirement: Reached Range constructor preserves omitted bounds

The base runtime SHALL recognize `Новый Диапазон` / `New Range` with one or two positional arguments, including an omitted first or second bound. Zero and three arguments SHALL fail. This surface is measured in `tests/conformance/measure/1c/measure-range-constructor.platform.txt` on platform 8.3.27.2342. Until the data-locking consumer can carry a genuine range value, actual construction SHALL raise a catchable explicit unsupported error instead of fabricating a value.

#### Scenario: A dormant lock statement omits the start of a range

- **WHEN** a configuration method contains `Новый Диапазон(, Реквизиты.Период)`
- **THEN** semantic resolution accepts the constructor and omitted first argument without changing the source

### Requirement: Reached command-parameter-use enumeration has measured members

The base runtime SHALL expose `РежимИспользованияПараметраКоманды` / `CommandParameterUseMode` with exactly `Одиночный` / `Single` and `Множественный` / `Multiple`. Member presentations SHALL be `Single` and `Multiple`, and the member type SHALL be `CommandParameterUseMode`, as observed in `tests/conformance/measure/1c/measure-command-parameter-use-mode.platform.txt` on platform 8.3.27.2342.

#### Scenario: A dormant EDI command chooses a single parameter

- **WHEN** a configuration method reads `РежимИспользованияПараметраКоманды.Одиночный`
- **THEN** semantic resolution accepts the member, and the English alias denotes the same value

### Requirement: Reached value-collection-area template constructor resolves

The base runtime SHALL recognize zero-argument `Новый МакетКоллекцииЗначенийОбластиКомпоновкиДанных` / `New DataCompositionAreaTemplateValueCollection`, as observed in `tests/conformance/measure/1c/measure-data-composition-area-template-value-collection.platform.txt` on platform 8.3.27.2342. One argument SHALL fail. Until a genuine data-composition template value is implemented, actual construction SHALL raise a catchable explicit unsupported error instead of returning a fabricated template.

#### Scenario: A dormant collections module prepares a template

- **WHEN** a configuration method contains `Новый МакетКоллекцииЗначенийОбластиКомпоновкиДанных`
- **THEN** semantic resolution accepts the constructor without changing the module source

### Requirement: Reached collection data-composition constructors resolve

The base runtime SHALL recognize the zero-argument constructors `МакетЗаголовкаКоллекцииЗначенийОбластиКомпоновкиДанных` / `DataCompositionAreaTemplateValueCollectionHeader` and `МакетКомпоновкиДанных` / `DataCompositionTemplate`, as measured in `tests/conformance/measure/1c/measure-data-composition-collection-constructors.platform.txt` on platform 8.3.27.2342. One argument SHALL fail for each. Until their typed values are implemented, actual construction SHALL raise a catchable explicit unsupported error instead of returning fabricated templates. The already registered `bsl-orm` processors `ПроцессорКомпоновкиДанных` and `ПроцессорВыводаРезультатаКомпоновкиДанныхВКоллекциюЗначений` SHALL remain the constructors used by the same module.

#### Scenario: A dormant collections method builds its output pipeline

- **WHEN** a configuration method contains these four constructors without arguments
- **THEN** semantic resolution accepts their names and arities without changing the source

### Requirement: Reached automatic form-data-save enumeration has measured members

The base runtime SHALL expose `АвтоматическоеСохранениеДанныхФормыВНастройках` / `AutoSaveFormDataInSettings` with exactly `Использовать` / `Use` and `НеИспользовать` / `DontUse`. Their presentations SHALL be `Use` and `Do not use`, and the member type SHALL be `AutoSaveFormDataInSettings`, as observed in `tests/conformance/measure/1c/measure-auto-save-form-data-in-settings.platform.txt` on platform 8.3.27.2342.

#### Scenario: A dormant form helper checks automatic saving

- **WHEN** a configuration method compares a form property to `АвтоматическоеСохранениеДанныхФормыВНастройках.Использовать`
- **THEN** semantic resolution accepts the member and its English alias denotes the same value

### Requirement: Reached PDF-document-file-type enumeration is complete

The base runtime SHALL expose `ТипФайлаДокументаPDF` / `PDFDocumentFileType` with exactly `PDF`, `PDF_A_1`, `PDF_A_2`, and `PDF_A_3`. Their presentations SHALL be `PDF`, `PDF/A-1`, `PDF/A-2`, and `PDF/A-3`, and their member type SHALL be `PDFDocumentFileType`, as observed in `tests/conformance/measure/1c/measure-pdf-document-file-type.platform.txt` on platform 8.3.27.2342.

#### Scenario: A dormant EDI helper checks PDF/A-1

- **WHEN** a configuration method reads `ТипФайлаДокументаPDF.PDF_A_1`
- **THEN** semantic resolution accepts the member, and the English alias denotes the same value

### Requirement: Reached collaboration application constructors resolve

The session library SHALL recognize `ИдентификаторПриложенияСистемыВзаимодействия` / `CollaborationSystemApplicationID` and `ИдентификаторСообщенияСистемыВзаимодействия` / `CollaborationSystemMessageID` with exactly one String argument, plus zero-argument `КоллекцияИдентификаторовПриложенийСистемыВзаимодействия` / `CollaborationSystemApplicationIDCollection` and `СовместноеИспользованиеПриложенийСистемыВзаимодействия` / `CollaborationSystemApplicationLinks`, as measured in `tests/conformance/measure/1c/measure-collaboration-application-constructors.platform.txt` on platform 8.3.27.2342. The zero-argument application-ID form SHALL fail, as observed in `measure-collaboration-application-id-zero.platform.txt`. Until a collaboration service is available, actual construction SHALL raise a catchable explicit unsupported error rather than fabricating an identifier or collection.

#### Scenario: A dormant discussion integration parses application IDs

- **WHEN** a configuration method contains the one-string application-ID constructor
- **THEN** semantic resolution accepts the name and arity without changing the source

### Requirement: Current client interface variant belongs to the Session

The session library SHALL expose the zero-argument global
`ТекущийВариантИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceCurrentVariant` and return the current Session's
host-supplied `ВариантИнтерфейсаКлиентскогоПриложения` value. The Session
SHALL reject a host-supplied value of another enumeration while opening. An
absent value SHALL raise a catchable error when the global is called. A new
Session, including a background Session, SHALL receive only its own supplied
value; the host MAY supply the parent's value when appropriate. This matches
`tests/conformance/measure/1c/measure-current-client-interface-variant.platform.txt`
on platform 8.3.27.2342, where the thin-client test session and its background
job both observed `Taxi`.

#### Scenario: A configuration checks its current interface

- **WHEN** a host opens a thin-client Session with `Taxi` and a configuration method calls the global
- **THEN** the result equals `ВариантИнтерфейсаКлиентскогоПриложения.Такси`

#### Scenario: A Session has no interface variant

- **WHEN** the global is called without a host-supplied variant
- **THEN** the call raises a catchable error instead of returning a constructor default

### Requirement: Reached input-by-string search mode matches the platform

The base runtime SHALL expose the complete system enumeration
`СпособПоискаСтрокиПриВводеПоСтроке` / `SearchStringModeOnInputByString`
with exactly `Начало` / `Begin` and `ЛюбаяЧасть` / `AnyPart`. Their
presentations SHALL be `At the beginning of line` and `Anywhere in line`,
and their value type SHALL be `SearchStringModeOnInputByString`, as measured in
`tests/conformance/measure/1c/measure-search-string-mode-on-input.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A configuration chooses substring search

- **WHEN** a configuration method reads `СпособПоискаСтрокиПриВводеПоСтроке.ЛюбаяЧасть`
- **THEN** semantic resolution returns the same value as `SearchStringModeOnInputByString.AnyPart`

### Requirement: Reached Base64 buffer encoder matches platform wrapping

The base runtime SHALL expose `ПолучитьBase64СтрокуИзБуфераДвоичныхДанных` /
`GetBase64StringFromBinaryDataBuffer` with exactly one `БуферДвоичныхДанных`
argument and a String result. It SHALL encode the buffer bytes as standard
Base64, insert CRLF after each complete 64-character line except when that
line is the only line, and produce no trailing CRLF after an incomplete line,
as measured in `tests/conformance/measure/1c/measure-base64-buffer-string.platform.txt`
on platform 8.3.27.2342. A value of another type SHALL raise a catchable type
error. The existing binary-data Base64 function SHALL keep its behavior.

#### Scenario: A configuration encodes a full two-line buffer

- **WHEN** the buffer contains 96 ASCII bytes
- **THEN** the result contains two 64-character Base64 lines followed by CRLF

#### Scenario: A configuration passes the wrong type

- **WHEN** a String is passed instead of a binary-data buffer
- **THEN** the function raises a catchable type error

### Requirement: Reached choice-data global resolves with measured arity

The ORM library SHALL recognize `ПолучитьДанныеВыбора` / `GetChoiceData` with
exactly two arguments, a value type and parameters structure, as measured in
`tests/conformance/measure/1c/measure-get-choice-data.platform.txt` on platform
8.3.27.2342. A dormant configuration call SHALL compile. Until the current
principal's metadata and choice data can be read, an actual call SHALL raise a
catchable explicit unsupported error rather than return a fabricated list.

#### Scenario: A pricing module requests catalog choice data

- **WHEN** a configuration method calls `ПолучитьДанныеВыбора` with a catalog reference type and parameter structure
- **THEN** semantic resolution accepts the global function name and two arguments

#### Scenario: Choice data is requested without an implementation

- **WHEN** the function executes through the current ORM library
- **THEN** it raises a catchable error without returning an empty list

### Requirement: Collaboration filter construction fails explicitly

The session library SHALL resolve the zero-argument constructors
`Новый ОтборОбсужденийСистемыВзаимодействия` /
`New CollaborationSystemConversationsFilter` and
`Новый ОтборСообщенийСистемыВзаимодействия` /
`New CollaborationSystemMessagesFilter`. Execution without
a collaboration host service SHALL raise a catchable explicit unsupported
error and SHALL NOT return a fabricated filter object.

#### Scenario: A guarded filter construction
- **WHEN** configuration code constructs the filter inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Collaboration conversation identifier construction fails explicitly
The session library SHALL resolve the one-String-argument constructor
`Новый ИдентификаторОбсужденияСистемыВзаимодействия` /
`New CollaborationSystemConversationID`. Execution without a collaboration
host service SHALL raise a catchable explicit unsupported error and SHALL NOT
return a fabricated identifier value.

#### Scenario: A guarded identifier construction
- **WHEN** configuration code constructs the identifier inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Collaboration conversation context construction fails explicitly
The session library SHALL resolve the one-String-argument constructor
`Новый КонтекстОбсужденияСистемыВзаимодействия` /
`New CollaborationSystemConversationContext`. Execution without a
collaboration host service SHALL raise a catchable explicit unsupported error
and SHALL NOT return a fabricated context object.

#### Scenario: A guarded context construction
- **WHEN** configuration code constructs the context inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Internet mail message construction fails explicitly
The session library SHALL resolve the zero-argument `Новый
ИнтернетПочтовоеСообщение` / `New InternetMailMessage` constructor. Execution
without an implemented mail-message value SHALL raise a catchable explicit
unsupported error and SHALL NOT return a fabricated message.

#### Scenario: A guarded mail-message construction
- **WHEN** configuration code constructs the message inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Internet mail text type is a complete closed system enumeration
The base runtime SHALL expose the four Syntax Assistant-defined members of
`ТипТекстаПочтовогоСообщения` / `InternetMailTextType`. Each Russian or English
member spelling SHALL resolve to the same value with the platform-measured
String presentation and member type. This enumeration SHALL NOT imply mail
delivery or a mail-message value.

#### Scenario: Read an HTML mail text type through either spelling
- **WHEN** configuration code reads `ТипТекстаПочтовогоСообщения.HTML` or
  `InternetMailTextType.HTML`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Internet mail non-ASCII encoding mode is a complete closed system enumeration
The base runtime SHALL expose the three Syntax Assistant-defined members of
`СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения` /
`InternetMailMessageNonASCIISymbolsEncodingMode`. Each Russian or English
member spelling SHALL resolve to the same value with the platform-measured
String presentation and member type. This enumeration SHALL NOT imply mail
encoding or delivery.

#### Scenario: Read the no-encoding mode through either spelling
- **WHEN** configuration code reads
  `СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения.БезКодирования`
  or `InternetMailMessageNonASCIISymbolsEncodingMode.None`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Internet mail message importance is a complete closed system enumeration
The base runtime SHALL expose the five Syntax Assistant-defined members of
`ВажностьИнтернетПочтовогоСообщения` / `InternetMailMessageImportance`.
Each Russian or English member spelling SHALL resolve to the same value
with the platform-measured String presentation and member type. This
enumeration SHALL NOT imply mail-message construction or delivery.

#### Scenario: Read normal mail importance through either spelling
- **WHEN** configuration code reads `ВажностьИнтернетПочтовогоСообщения.Обычная`
  or `InternetMailMessageImportance.Normal`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Item horizontal location exposes its complete measured enumeration
The base runtime SHALL expose the four Syntax Assistant-defined members of
`ГоризонтальноеПоложениеЭлемента` / `ItemHorizontalLocation`. Each Russian or
English member spelling SHALL resolve to the same value with the
platform-measured String presentation and member type. This enumeration SHALL
NOT imply managed-form rendering.

#### Scenario: Read automatic horizontal location through either spelling
- **WHEN** configuration code reads `ГоризонтальноеПоложениеЭлемента.Авто`
  or `ItemHorizontalLocation.Auto`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Button shape representation exposes its complete measured enumeration
The base runtime SHALL expose the four Syntax Assistant-defined members of
`ОтображениеФигурыКнопки` / `ButtonShapeRepresentation`. Each Russian or
English member spelling SHALL resolve to the same value with the
platform-measured String presentation and member type. This enumeration SHALL
NOT imply managed-form rendering.

#### Scenario: Read automatic button shape through either spelling
- **WHEN** configuration code reads `ОтображениеФигурыКнопки.Авто`
  or `ButtonShapeRepresentation.Auto`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Usual group representation exposes its complete measured enumeration
The base runtime SHALL expose the four Syntax Assistant-defined members of
`ОтображениеОбычнойГруппы` / `UsualGroupRepresentation`. Each Russian or
English member spelling SHALL resolve to the same value with the
platform-measured String presentation and member type. This enumeration SHALL
NOT imply managed-form rendering.

#### Scenario: Read normal group marking through either spelling
- **WHEN** configuration code reads `ОтображениеОбычнойГруппы.ОбычноеВыделение`
  or `UsualGroupRepresentation.NormalSeparation`
- **THEN** both expressions return the same value with the measured
  presentation and type

### Requirement: Event log export fails explicitly without a host implementation
The session library SHALL resolve `ВыгрузитьЖурналРегистрации` /
`UnloadEventLog` with the platform-measured one-through-five-argument arity.
Execution without an event-log export service SHALL raise a catchable explicit
unsupported error and SHALL NOT write a file or fabricate exported entries.

#### Scenario: A guarded event log export
- **WHEN** configuration code calls the procedure inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code calls the procedure with zero or six arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Event-log event presentation fails explicitly until localized names are supported
The base runtime SHALL resolve `ПредставлениеСобытияЖурналаРегистрации` /
`EventLogEventPresentation` with the platform-measured one-argument arity.
Execution SHALL raise a catchable explicit unsupported error rather than
return a fabricated event presentation.

#### Scenario: A guarded presentation call
- **WHEN** configuration code calls the function inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code calls the function with zero or two arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Program directory does not claim the interpreter executable path
The base runtime SHALL resolve zero-argument `КаталогПрограммы()` / `BinDir()`.
Without a 1C platform installation directory supplied by the host, execution
SHALL raise a catchable explicit unsupported error rather than return the
directory of the interpreter executable.

#### Scenario: A guarded program-directory call
- **WHEN** configuration code calls `BinDir()` inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Legacy report builder construction fails explicitly
The ORM library SHALL resolve `Новый ПостроительОтчета` /
`New ReportBuilder` with zero or one query-text argument. Until the builder
object model is implemented, execution SHALL raise a catchable explicit
unsupported error and SHALL NOT return a Query or fabricated builder.

#### Scenario: A guarded report builder construction
- **WHEN** configuration code constructs a report builder inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Edit-data locking fails explicitly until the session adapter supports it
The ORM library SHALL resolve `ЗаблокироватьДанныеДляРедактирования` /
`LockDataForEdit` as a procedure with one required and two optional arguments.
Without a Session database adapter for object and record edit locks, execution
SHALL raise a catchable explicit unsupported error and SHALL NOT claim to
have acquired a lock.

#### Scenario: A guarded edit-data lock
- **WHEN** configuration code calls the procedure inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code calls the procedure with zero or four arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Edit-data unlocking fails explicitly until the session adapter supports it
The ORM library SHALL resolve `РазблокироватьДанныеДляРедактирования` /
`UnlockDataForEdit` as a procedure with zero through two arguments.
Without a Session database adapter for object and record edit locks, execution
SHALL raise a catchable explicit unsupported error and SHALL NOT claim to
have released a lock.

#### Scenario: A guarded edit-data unlock
- **WHEN** configuration code calls the procedure inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code calls the procedure with three arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Configuration description construction fails explicitly until CF parsing is supported
The ORM library SHALL resolve `Новый ОписаниеКонфигурации` /
`New ConfigurationDescription` with exactly one argument. Until `.cf` and
`.cfe` binary descriptions are parsed, execution SHALL raise a catchable
explicit unsupported error and SHALL NOT fabricate a description from live
configuration metadata.

#### Scenario: A guarded configuration description construction
- **WHEN** configuration code constructs the description inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code constructs the description with zero or two arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Data source description construction fails explicitly
The ORM library SHALL resolve the one-argument `Новый ОписаниеИсточникаДанных` /
`New DataSourceDescription` constructor. Until its object model is
implemented, execution SHALL raise a catchable explicit unsupported error
and SHALL NOT fabricate a description.

#### Scenario: A guarded data source description construction
- **WHEN** configuration code constructs a description inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Numeric value kind is a closed system enumeration
The base runtime SHALL expose the complete two-member
`ВидЧисловогоЗначения` / `NumericValueType` enumeration. Both Russian and
English member names SHALL resolve to the same values with the measured
type display and string presentations.

#### Scenario: Read both numeric value kinds
- **WHEN** BSL reads `ВидЧисловогоЗначения.Количественное` and
  `NumericValueType.Ordinal`
- **THEN** it receives values presented as `Cardinal` and `Ordinal` with
  type display `NumericValueType`

### Requirement: Number-string pluralization fails explicitly until implemented
The base runtime SHALL resolve `СтрокаСЧислом` / `StringWithNumber` with
three required arguments and an optional fourth argument. Execution SHALL
raise a catchable explicit unsupported error rather than returning a
locale-independent or incomplete plural form.

#### Scenario: A guarded number-string call
- **WHEN** configuration code calls the function inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: String declension fails explicitly until morphology is supported
The base runtime SHALL resolve `ПолучитьСклоненияСтроки` /
`GetStringDeclensions` with one required String and two optional String
arguments. Execution SHALL raise a catchable explicit unsupported error rather
than return a fabricated array of declensions.

#### Scenario: A guarded string declension call
- **WHEN** configuration code calls the function inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

#### Scenario: Reject an impossible argument count
- **WHEN** configuration code calls the function with zero or four arguments
- **THEN** semantic resolution rejects the call before execution

### Requirement: Computer time zone stays separate from the Session zone
The base runtime SHALL expose `ЧасовойПояс()` / `TimeZone()` as a zero-argument
String-valued computer-zone getter. Changing the effective Session time zone
SHALL NOT change its answer. An unidentifiable computer zone SHALL fail
explicitly rather than return a fabricated zone identifier.

#### Scenario: Session zone differs from computer zone
- **WHEN** a State has a Session zone different from its captured computer zone
- **THEN** `TimeZone()` returns the computer zone and `SessionTimeZone()` returns the Session zone

### Requirement: Font construction fails explicitly until the value is supported
The base runtime SHALL resolve `Новый Шрифт` / `New Font` with the
documented zero-argument, description-based, and font-based constructor
shapes. Execution SHALL raise a catchable explicit unsupported error and
SHALL NOT return a fabricated font value.

#### Scenario: A guarded font construction
- **WHEN** configuration code constructs a font inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Unicode title casing fails explicitly until implemented
The base runtime SHALL resolve `ТРег` / `Title` with one argument.
Execution SHALL raise a catchable explicit unsupported error until the
platform's Unicode word-boundary and client-kind rules are implemented.

#### Scenario: A guarded title-case call
- **WHEN** configuration code calls `ТРег` inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Common-template retrieval fails explicitly until available
The ORM library SHALL resolve `ПолучитьОбщийМакет` / `GetCommonTemplate`
with one argument. Until configuration template resources and their BSL value
types are available, execution SHALL raise a catchable explicit unsupported
error and SHALL NOT return a fabricated template.

#### Scenario: A guarded common-template retrieval
- **WHEN** configuration code calls `ПолучитьОбщийМакет` inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Value-tree construction fails explicitly until supported
The base runtime SHALL resolve the documented zero-argument
`Новый ДеревоЗначений` / `New ValueTree` constructor. Execution SHALL raise
a catchable explicit unsupported error and SHALL NOT return a value table or
another fabricated tree substitute.

#### Scenario: A guarded value-tree construction
- **WHEN** configuration code constructs a value tree inside `Попытка`
- **THEN** resolution succeeds and `Исключение` handles the runtime error

### Requirement: Sort direction has both platform members

The base runtime SHALL expose `НаправлениеСортировки` / `SortDirection`
with the two distinct members `Возр` / `Asc` and `Убыв` / `Desc`.
The previously accepted `Ascending` spelling SHALL continue to resolve to
the ascending member without adding a third enumerated value. Their String
presentations SHALL be `Ascending` and `Descending` respectively.

#### Scenario: Descending configuration expression

- **WHEN** configuration code reads `НаправлениеСортировки.Убыв` or
  `SortDirection.Desc`
- **THEN** both expressions resolve to the same descending value
- **AND** that value differs from `НаправлениеСортировки.Возр`

### Requirement: Filter comparison type is a distinct complete enumeration

The base runtime SHALL expose `ВидСравнения` / `ComparisonType` with all
18 members documented by platform 8.3.27. Every Russian member and its
English alias SHALL resolve to the same value with the measured String
presentation. This enumeration SHALL remain distinct from
`ВидСравненияКомпоновкиДанных` / `DataCompositionComparisonType`.

#### Scenario: A configuration uses a filter comparison

- **WHEN** BSL reads `ВидСравнения.Равно` or `ComparisonType.Equal`
- **THEN** both expressions resolve to the same filter-comparison value
- **AND** that value is distinct from
  `ВидСравненияКомпоновкиДанных.Равно`

### Requirement: Convert local time to universal time through the Session zone

The base runtime SHALL provide `УниверсальноеВремя` / `ToUniversalTime`
with one required Date or canonical date String and one optional String-or-
Undefined zone identifier. An omitted or Undefined zone SHALL select the
current State zone. A GMT or known named zone SHALL use its actual offset
for the supplied local time, including the measured fold and gap rules.
The returned Date SHALL preserve fractional ticks. An unknown zone or
Undefined date SHALL raise a catchable error.

#### Scenario: Fixed and named zones

- **WHEN** BSL converts `2024-01-02 12:00:00` from `GMT+3` or
  `Europe/Moscow`
- **THEN** it receives `2024-01-02 09:00:00`

#### Scenario: Ambiguous local time

- **WHEN** BSL converts local `2010-10-31 02:30:00` in `Europe/Moscow`
- **THEN** it receives `2010-10-30 23:30:00`, the later occurrence

### Requirement: Client connection speed belongs to one Session

The base runtime SHALL expose the two distinct members `Низкая` / `Low`
and `Обычная` / `Normal` of `СкоростьКлиентскогоСоединения` /
`ClientConnectionSpeed` with their measured String and type presentations.
The Session library SHALL provide the zero-argument
`ПолучитьСкоростьКлиентскогоСоединения` / `GetClientConnectionSpeed`
function from an explicit speed supplied when that Session opens. A new
Session, including a background Session, SHALL NOT inherit the speed of
another Session. Reading without a supplied speed SHALL fail explicitly.

#### Scenario: Two Sessions of one factory

- **WHEN** two Sessions open with Low and Normal connection speeds
- **THEN** each function call returns its own distinct enum member
- **AND** a third Session with no supplied speed raises a catchable error

### Requirement: Access-right verification uses the current principal's adapter

The ORM library SHALL resolve `ВыполнитьПроверкуПравДоступа` /
`VerifyAccessRights` as a procedure with a required right and configuration
metadata object and an optional standard-attribute path. It SHALL pass the
request to the current Session's data-access adapter. The adapter SHALL either
verify the right and return successfully, or reject it with a catchable error
and record the access-denied event. Without a verifying adapter, the call
SHALL fail explicitly; it SHALL NOT infer a grant from a missing adapter.

The database verifier SHALL obtain the writer from the current Session. A
missing writer or failed journal write SHALL fail explicitly rather than
report successful verification. An allowed check SHALL not emit a denial.
For the retained native configuration-root SaveUserData and catalog Read
controls, denial SHALL use `_$Access$_.AccessDenied` at Information level,
empty comment and Structure data whose Right field contains the canonical
English right name. The root has no metadata payload; an object denial uses
the original validated metadata value. Delivery SHALL be independent of the
current transaction, including when that transaction is rolled back.

The configuration root SHALL be accepted by verification under the same
catalog-ownership and acquired-root-identity checks as ordinary AccessRight.
Both operations SHALL expose the original validated BSL metadata value through
`AccessRightRequest::metadata_value` for the host adapter's journal payload;
this SHALL NOT recreate a metadata object or admit another catalog's value.

#### Scenario: A right is denied

- **WHEN** the current principal lacks the requested right
- **THEN** the procedure raises a catchable error and the adapter records the
  access-denied event

#### Scenario: No verifying adapter

- **WHEN** the Session has no adapter capable of verifying access rights
- **THEN** the procedure raises a catchable error

#### Scenario: Verify a configuration-root right
- **GIVEN** an acquired configuration-root identity and a verifying adapter
- **WHEN** verification receives the current Session's Metadata root
- **THEN** the adapter receives a configuration metadata descriptor with that identity
- **AND** the call is not rejected merely because the argument is the root

#### Scenario: Preserve the validated metadata value
- **WHEN** either access-right operation delegates a current-catalog metadata value
- **THEN** the request exposes that same BSL value without rebuilding it
- **AND** a foreign-catalog root or object fails before adapter dispatch

#### Scenario: Denial survives transaction rollback
- **WHEN** native-equivalent root verification is denied inside a transaction that is later rolled back
- **THEN** the Session writer receives an independent AccessDenied entry with Right=SaveUserData
- **AND** the permission error remains catchable

#### Scenario: Journal delivery is unavailable
- **WHEN** the database verifier has no Session writer or the writer fails
- **THEN** verification fails explicitly
- **AND** it does not report a verified grant or acknowledge a successfully recorded denial

#### Scenario: Privileged root verification is allowed
- **WHEN** the current principal has no SaveUserData grant but effective privileged mode is enabled
- **THEN** verification succeeds without emitting an AccessDenied event

### Requirement: External processor and report managers are explicit capability gaps

The ORM library SHALL resolve the platform's read-only
`ВнешниеОбработки` / `ExternalDataProcessors` and `ВнешниеОтчеты` /
`ExternalReports` globals without treating them as uninitialized local
variables. Until the host supplies external loading, evaluating either
global SHALL raise a catchable error.

#### Scenario: Configuration code refers to external processors

- **WHEN** a compiled module contains a reference to either external manager
- **THEN** compilation succeeds
- **AND** evaluating that reference fails explicitly

### Requirement: Temporary-storage URL recognition matches the platform

The base runtime SHALL expose `ЭтоАдресВременногоХранилища` /
`IsTempStorageURL` as a one-argument boolean function. It SHALL recognize
the platform's `e1cib/tempstorage/<UUID>` path, including a query suffix,
without requiring the address to exist in the current Session. It SHALL
reject malformed UUID paths and values that are not such addresses.

#### Scenario: Well-formed address with no current storage entry

- **WHEN** the function receives `e1cib/tempstorage/11111111-1111-1111-1111-111111111111`
- **THEN** it returns `True`

#### Scenario: Malformed address

- **WHEN** the function receives `e1cib/tempstorage/foo?seanceId=bar`
- **THEN** it returns `False`

### Requirement: Spreadsheet cell area type has its complete measured enum

The base runtime SHALL expose `ТипОбластиЯчеекТабличногоДокумента` /
`SpreadsheetDocumentCellAreaType` with the four measured members Table,
Rows, Columns, and Rectangle, their Russian aliases, and the measured
String and type presentations.

#### Scenario: Table area type

- **WHEN** BSL evaluates `ТипОбластиЯчеекТабличногоДокумента.Таблица`
- **THEN** its String presentation is `Table`
- **AND** its type is `SpreadsheetDocumentCellAreaType`

### Requirement: External component connection type resolves from its documented enum

The base runtime SHALL resolve `ТипПодключенияВнешнейКомпоненты` /
`AddInConnectionType` with the two documented members `Изолированно` /
`Isolated` and `НеИзолированно` / `NotIsolated`. String and type
presentations remain an open platform measurement for the thick-client
context.

#### Scenario: Dormant reference in configuration code

- **WHEN** a module refers to `ТипПодключенияВнешнейКомпоненты.Изолированно`
- **THEN** the module compiles without treating the root as a local variable

### Requirement: StrConcat accepts its optional separator

`СтрСоединить` / `StrConcat` SHALL accept one or two arguments. Omitting
the separator or passing `Undefined` SHALL join the array elements without
a separator; an explicit String separator SHALL retain its existing effect.

#### Scenario: One argument

- **WHEN** BSL calls `СтрСоединить` with only an array containing `А` and `Б`
- **THEN** it returns `АБ`

### Requirement: Period presentation does not fabricate locale-specific text

The base runtime SHALL resolve `ПредставлениеПериода` /
`PeriodPresentation` with two required dates and an optional format
string. Until locale and financial-period formatting are implemented,
executing the function SHALL raise a catchable explicit error.

#### Scenario: A guarded period-presentation call

- **WHEN** configuration code calls `ПредставлениеПериода` inside `Попытка`
- **THEN** the module compiles and `Исключение` catches the runtime error

### Requirement: Number-in-words does not fabricate locale-specific text

The base runtime SHALL resolve `ЧислоПрописью` / `NumberInWords` with one
required number and two optional formatting arguments. Until locale and
counted-object inflection are implemented, executing the function SHALL
raise a catchable explicit error.

#### Scenario: A guarded number-in-words call

- **WHEN** configuration code calls `ЧислоПрописью` inside `Попытка`
- **THEN** the module compiles and `Исключение` catches the runtime error

### Requirement: The global brief error description uses the existing error snapshot

The base runtime SHALL expose `КраткоеПредставлениеОшибки` /
`BriefErrorDescription` as a one-argument function accepting
`ИнформацияОбОшибке`. Its result SHALL be the same brief description
already returned by the error-processing manager for the same snapshot.
An argument of another type SHALL raise a catchable type error.

#### Scenario: One saved error snapshot

- **WHEN** BSL calls the global function and the manager method with the
  same saved `ИнформацияОбОшибке`
- **THEN** their String results are equal

### Requirement: TypeDescription has a zero-argument empty constructor

The base runtime SHALL accept `Новый ОписаниеТипов()` and
`New TypeDescription()` and create a type description containing no
allowed types. Existing constructor forms SHALL keep their behavior.

#### Scenario: An empty type description

- **WHEN** BSL constructs `Новый ОписаниеТипов()`
- **THEN** its type is `TypeDescription`
- **AND** its allowed-type collection is empty

### Requirement: FillChecking has its two measured members

The base runtime SHALL expose `ПроверкаЗаполнения` / `FillChecking`
with `ВыдаватьОшибку` / `ShowError` and `НеПроверять` / `DontCheck`.
Their root, type, and value String presentations SHALL match the
measured platform output.

#### Scenario: Both fill-checking values

- **WHEN** BSL reads both members through their Russian or English names
- **THEN** `ShowError` is presented as `Yes` and `DontCheck` as `No`

### Requirement: FoldersAndItems has its four measured members

The base runtime SHALL expose `ГруппыИЭлементы` / `FoldersAndItems`
with the four members Auto, Folders, FoldersAndItems, and Items and their
Russian aliases. Root, type, and value String presentations SHALL match
the measured platform output.

#### Scenario: All four members

- **WHEN** BSL reads the four members through either language's names
- **THEN** each pair of names refers to the same distinct enum value

### Requirement: All-files mask comes from the current file-system context

The base runtime SHALL resolve zero-argument `ПолучитьМаскуВсеФайлы` /
`GetAllFilesMask`. For a Unix-style file-system context it SHALL return
the measured `*` mask. The Windows-style mask remains an open platform
measurement. The function SHALL use the same host file-system context as
the existing path-separator functions.

#### Scenario: Unix-style file-system context

- **WHEN** BSL calls either name in a State with the process Unix file system
- **THEN** it receives `*`

### Requirement: InternetMailProtocol exposes its measured members

The base runtime SHALL expose `ПротоколИнтернетПочты` /
`InternetMailProtocol` with the IMAP, POP3, and SMTP members. Root,
type, and member String presentations SHALL match the measured platform
output, and both names SHALL resolve to the same enum values.

#### Scenario: All three mail protocols

- **WHEN** BSL reads IMAP, POP3, and SMTP through either root name
- **THEN** each member is presented by its protocol name
- **AND** the root and type are presented as `EnumInternetMailProtocol`
  and `InternetMailProtocol` respectively

### Requirement: InternetMail construction is explicit without mail transport

The base runtime SHALL resolve the documented zero-argument
`Новый ИнтернетПочта()` / `New InternetMail()` constructor. Until a mail
transport and value model are supplied, executing it SHALL raise a
catchable unsupported-capability error rather than fabricate a mail
object.

#### Scenario: A reachable mail constructor

- **WHEN** configuration code contains the zero-argument constructor
- **THEN** semantic resolution succeeds
- **AND** execution raises the explicit error if that branch is taken

### Requirement: InternetMailProfile construction is explicit without mail transport

The base runtime SHALL resolve the documented zero-argument
`Новый ИнтернетПочтовыйПрофиль()` / `New InternetMailProfile()` constructor.
Until its mutable profile value model exists, executing it SHALL raise a
catchable unsupported-capability error.

#### Scenario: A reachable mail-profile constructor

- **WHEN** configuration code contains the zero-argument constructor
- **THEN** semantic resolution succeeds
- **AND** execution raises the explicit error if that branch is taken

### Requirement: InfoBase navigation URL is not fabricated

`bsl-session` SHALL resolve the zero-argument
`ПолучитьНавигационнуюСсылкуИнформационнойБазы` / `GetInfoBaseURL` global
function. Until the host provides an authoritative client navigation URL,
execution SHALL raise an explicit catchable capability error rather than
derive a URL from the database connection string.

#### Scenario: URL is absent from the session host

- **WHEN** configuration code calls either function name in a Session
- **THEN** resolution succeeds and execution reports the missing URL

### Requirement: ToLocalTime uses the current State time zone

The base runtime SHALL expose `МестноеВремя` / `ToLocalTime` with one
required UTC Date and an optional zone name. An omitted or Undefined name
SHALL select the State's time zone. A named zone SHALL use its offset at
the supplied UTC instant. Invalid zones and invalid values SHALL raise
catchable errors. The measured date-string conversion SHALL be accepted.

#### Scenario: Fixed and named zone conversions

- **WHEN** BSL converts a UTC date through `GMT+3` or `Europe/Moscow`
- **THEN** the result matches the real-platform measurement
- **AND** both function names return the same Date

### Requirement: StyleColors is a known platform global

The base runtime SHALL resolve the documented `ЦветаСтиля` /
`StyleColors` global through the existing library-global mechanism.
Until configuration style values and Color objects are available,
evaluating the global SHALL raise a catchable explicit error rather
than return fabricated colors.

#### Scenario: A reachable style-color reference

- **WHEN** configuration code references either global spelling
- **THEN** semantic resolution succeeds
- **AND** execution reports the unavailable style-color model

### Requirement: Legacy Find retains its measured empty-substring result

The base runtime SHALL expose the two-argument `Найти` / `Find` global
function as a case-sensitive substring search. It SHALL reuse the
existing `СтрНайти` search for nonempty substrings while returning the
measured position one for an empty substring, including when both
strings are empty.

#### Scenario: Empty substring

- **WHEN** BSL searches either an empty or nonempty String for `""`
- **THEN** `Найти` returns `1` while the existing `СтрНайти` behavior
  remains unchanged

### Requirement: StyleFonts is a known platform global

The base runtime SHALL resolve `ШрифтыСтиля` / `StyleFonts` through the
existing library-global mechanism. Until configuration style values and
Font objects are available, evaluating it SHALL raise an explicit
catchable error rather than return fabricated fonts.

#### Scenario: A reachable style-font reference

- **WHEN** configuration code references either global spelling
- **THEN** semantic resolution succeeds
- **AND** execution reports the unavailable style-font model

### Requirement: FormAttribute constructor has its documented shape

The base runtime SHALL resolve `Новый РеквизитФормы` /
`New FormAttribute` with two required and three optional arguments.
Until managed-form attributes have a value model, executing the
constructor SHALL raise an explicit catchable error.

#### Scenario: Reachable form-attribute construction

- **WHEN** configuration code constructs a FormAttribute with two to
  five arguments
- **THEN** semantic resolution succeeds
- **AND** execution reports that the value model is unavailable

### Requirement: Color constructor resolves documented forms

The base runtime SHALL resolve `Новый Цвет` / `New Color` with zero or
three arguments, as documented by the platform. Until Color values
have a model, executing either form SHALL raise an explicit catchable
error rather than substitute a number or String.

#### Scenario: RGB color in reachable code

- **WHEN** configuration code constructs a Color from three components
- **THEN** semantic resolution succeeds
- **AND** execution reports the missing Color model

### Requirement: DataCompositionDetailsProcessingAction has seven measured members

The base runtime SHALL expose `ДействиеОбработкиРасшифровкиКомпоновкиДанных` /
`DataCompositionDetailsProcessingAction` with the seven members None,
OpenValue, Filter, ApplyAppearance, DrillDown, Group, and Order and
their documented Russian names. Its root, type, and member String
presentations SHALL match the measured platform output exactly.

#### Scenario: Open-value action presentation

- **WHEN** BSL presents the OpenValue member as a String
- **THEN** the result is the measured `Open value ` including its
  trailing space

### Requirement: DataCompositionSettingsComposer constructor is explicit

The ORM library SHALL resolve the documented zero-argument
`Новый КомпоновщикНастроекКомпоновкиДанных` /
`New DataCompositionSettingsComposer` constructor. Until its settings
model exists, executing it SHALL raise an explicit catchable error
rather than return an unrelated composer object.

#### Scenario: Reachable settings-composer construction

- **WHEN** configuration code contains either constructor spelling
- **THEN** semantic resolution succeeds
- **AND** execution reports the unsupported settings composer

### Requirement: DataCompositionAvailableSettingsSource constructor is explicit

The ORM library SHALL resolve the documented
`Новый ИсточникДоступныхНастроекКомпоновкиДанных` /
`New DataCompositionAvailableSettingsSource` constructor with one required
schema argument and one optional functional-option-parameters argument.
Until the settings-source model exists, executing it SHALL raise an
explicit catchable error.

#### Scenario: Reachable settings-source construction

- **WHEN** configuration code constructs the settings source with one or
  two arguments using either constructor spelling
- **THEN** semantic resolution succeeds
- **AND** execution reports the unsupported settings source

### Requirement: XMLValue converts measured primitive lexical forms

The XML library SHALL resolve global `XMLЗначение` / `XMLValue` with two
required arguments and convert the measured Number, Boolean, String, and
Date lexical forms to values of the requested type. An empty Number
lexical form and an invalid Number lexical form SHALL raise a catchable
error; a single-space Number lexical form SHALL yield zero. Types outside
the measured subset SHALL fail explicitly.

#### Scenario: XMLValue differs from XDTO empty-number parsing

- **WHEN** BSL calls `XMLЗначение(Тип("Число"), "")`
- **THEN** execution raises a catchable error rather than returning zero

### Requirement: WebColors global resolves without fabricated colors

The base runtime SHALL resolve `WebЦвета` / `WebColors` as a global name
after local and module variables. Until Color values have a model,
evaluating the global SHALL raise an explicit catchable error.

#### Scenario: Configuration code names WebColors

- **WHEN** a module reads `WebЦвета.Зеленый`
- **THEN** semantic resolution accepts the global receiver
- **AND** execution reports the missing Color model

### Requirement: TypeDescription copy constructor resolves the documented arity

The base runtime SHALL resolve the six- and seven-argument
`Новый ОписаниеТипов` / `New TypeDescription` copy-constructor forms.
Until type addition, subtraction, and qualifier inheritance are
implemented, executing either form SHALL raise an explicit catchable
error instead of constructing a description from only the first argument.

#### Scenario: Reached six-argument copy form

- **WHEN** configuration code constructs a TypeDescription with six
  arguments
- **THEN** semantic resolution succeeds
- **AND** execution reports the unsupported copy semantics

### Requirement: TypeDescription returns measured defaults without an argument

`ОписаниеТипов.ПривестиЗначение` / `TypeDescription.AdjustValue` SHALL
accept zero or one argument. With no argument, a description containing
only Number, String, Boolean or Date SHALL return that type's measured
default. An empty description or one containing multiple types SHALL
return Undefined. A single unsupported type SHALL fail explicitly.

#### Scenario: Multi-type description has no selected default

- **WHEN** BSL calls `ПривестиЗначение()` on a description containing
  Number and String in either order
- **THEN** it returns Undefined

### Requirement: TextWriter accepts its measured constructor forms

`Новый ЗаписьТекста` / `New TextWriter` SHALL resolve the measured
zero- through five-argument forms. The existing one-argument file writer
behavior SHALL remain unchanged. The zero-argument form SHALL produce an
unopened writer. The two-argument UTF-8 form SHALL write through the
existing file-system capability. Forms whose encoding, separator, or append
semantics are not implemented SHALL fail explicitly when executed.

#### Scenario: Configuration module declares a two-argument writer

- **WHEN** a configuration module contains `Новый ЗаписьТекста(Path, КодировкаТекста.UTF8)`
- **THEN** semantic resolution accepts the constructor
- **AND** execution uses the current State's file-system capability

#### Scenario: Unimplemented writer options

- **WHEN** code executes a writer constructor with an unsupported encoding or additional options
- **THEN** execution raises a catchable error before creating a file

#### Scenario: Excessive writer arguments

- **WHEN** code constructs TextWriter with six arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: MergeFiles resolves without fabricating file output

The base runtime SHALL resolve the measured two-argument procedure
`ОбъединитьФайлы` / `MergeFiles`. Until array and mask merging are
implemented through the State file-system capability, execution SHALL raise
an explicit catchable error and SHALL NOT create an output file.

#### Scenario: Configuration module declares a merge

- **WHEN** a configuration module calls `ОбъединитьФайлы(Parts, OutputPath)`
- **THEN** semantic resolution accepts the procedure
- **AND** execution reports that merging is unsupported without creating output

#### Scenario: Wrong merge arity

- **WHEN** code calls `MergeFiles` with one or three arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: MoveFile resolves without bypassing the session file system

The base runtime SHALL resolve the measured two-argument procedure
`ПереместитьФайл` / `MoveFile`. Until the State file-system capability
supports platform-compatible renaming, execution SHALL raise an explicit
catchable error and SHALL NOT move a file through the process file system.

#### Scenario: Configuration code declares a move

- **WHEN** a configuration module calls `ПереместитьФайл(Source, Destination)`
- **THEN** semantic resolution accepts the procedure
- **AND** execution reports the missing rename capability

#### Scenario: Wrong move arity

- **WHEN** code calls `MoveFile` with one or three arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: CopyFile resolves without fabricating copied output

The base runtime SHALL resolve the measured two-argument procedure
`КопироватьФайл` / `CopyFile`. Until the State file-system capability
supports platform-compatible copying, execution SHALL raise an explicit
catchable error and SHALL NOT copy through the process file system.

#### Scenario: Configuration code declares a copy

- **WHEN** a configuration module calls `КопироватьФайл(Source, Destination)`
- **THEN** semantic resolution accepts the procedure
- **AND** execution leaves both paths untouched and reports the missing operation

#### Scenario: Wrong copy arity

- **WHEN** code calls `CopyFile` with one or three arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: ErrorDescription resolves without stale exception text

The base runtime SHALL resolve the measured zero-argument function
`ОписаниеОшибки` / `ErrorDescription`. The accepted `error-description`
specification SHALL govern execution: the function SHALL return the
current call's active handler description, or
the empty String when no handler is active. It SHALL NOT return a stale
task-wide exception as if it were current.

#### Scenario: Configuration code names ErrorDescription

- **WHEN** configuration code calls `ОписаниеОшибки()` in an exception path
- **THEN** semantic resolution accepts the call
- **AND** execution returns that handler's description according to the
  accepted `error-description` specification

#### Scenario: Wrong error-description arity

- **WHEN** code calls `ErrorDescription(1)`
- **THEN** semantic resolution reports an argument-count error

### Requirement: DataSeparationSafeMode resolves without fabricating a safe-zone state

The Session library SHALL resolve the measured one-argument function
`БезопасныйРежимРазделенияДанных` / `DataSeparationSafeMode`. Until the
host supplies the safe-zone baseline that can affect its result, execution
SHALL raise a catchable error rather than report only the VM's local counter.
The existing setter SHALL retain its behavior.

#### Scenario: Configuration code queries a separator

- **WHEN** a configuration module calls `БезопасныйРежимРазделенияДанных(Name)`
- **THEN** semantic resolution accepts the function
- **AND** execution reports the unavailable safe-zone baseline

#### Scenario: Wrong getter arity

- **WHEN** code calls `DataSeparationSafeMode` with zero or two arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: EraseInfoBaseData resolves without deleting through the read-only host

The ORM library SHALL resolve the documented zero-argument procedure
`УдалитьДанныеИнформационнойБазы` / `EraseInfoBaseData`. Execution in the
read-only database host SHALL raise an explicit catchable error and SHALL
NOT delete data or report success.

#### Scenario: Configuration code declares an infobase erase

- **WHEN** a configuration module calls `УдалитьДанныеИнформационнойБазы()`
- **THEN** semantic resolution accepts the procedure
- **AND** execution reports that erasure is unavailable

#### Scenario: Erase call carries an argument

- **WHEN** code calls `EraseInfoBaseData(Undefined)`
- **THEN** semantic resolution reports an argument-count error

### Requirement: DataHistory global resolves without an unbound history service

The ORM library SHALL resolve the server-available global property
`ИсторияДанных` / `DataHistory`. Until the Session supplies a data-history
service, evaluating the property SHALL raise an explicit catchable error
instead of returning an invented manager or using another Session's state.

#### Scenario: Configuration module refers to data history

- **WHEN** a configuration module contains a call through `ИсторияДанных`
- **THEN** semantic resolution accepts the global property
- **AND** executing that call without a Session data-history service reports its absence

### Requirement: Reached session globals retain their distinct services

The Session library SHALL resolve `ИсторияРаботыПользователя` /
`UserWorkHistory` as a current-user history service and
`ХранилищеВариантовОтчетов` / `ReportsVariantsStorage` as the report-variant
settings store. Until the configuration snapshot supplies its custom-storage
selection, evaluating either property SHALL raise a catchable error instead
of returning a fabricated manager or selecting the standard store by guess.

#### Scenario: Configuration code names the user history

- **WHEN** a configuration module refers to `ИсторияРаботыПользователя`
- **THEN** semantic resolution accepts the property
- **AND** evaluating it without a history service reports its absence

#### Scenario: Configuration code names report-variant settings

- **WHEN** a configuration module uses `ReportsVariantsStorage.Load(...)`
- **THEN** semantic resolution accepts the property
- **AND** execution reports that the selected storage is unavailable

### Requirement: ChoiceParameter constructor resolves with documented arity

The base runtime SHALL resolve the documented two-argument constructor
`ПараметрВыбора` / `ChoiceParameter`. Until its value model is implemented,
execution SHALL raise a catchable error instead of constructing a substitute.

#### Scenario: Configuration code declares a choice parameter

- **WHEN** a configuration module contains `Новый ПараметрВыбора("Отбор.X", Value)`
- **THEN** semantic resolution accepts the constructor
- **AND** executing it reports the missing value model

#### Scenario: Choice parameter lacks a value

- **WHEN** code calls the constructor with one argument
- **THEN** semantic resolution reports an argument-count error
### Requirement: Spreadsheet drawing types match the measured platform enum

The base runtime SHALL resolve all thirteen measured members of
`ТипРисункаТабличногоДокумента` / `SpreadsheetDocumentDrawingType` with
their measured presentations and the shared member type
`SpreadsheetDocumentDrawingType`.

#### Scenario: UNF module names the picture drawing type

- **WHEN** configuration code reads `ТипРисункаТабличногоДокумента.Картинка`
- **THEN** semantic resolution accepts the member
- **AND** its string presentation is `Picture`
- **AND** its type is `SpreadsheetDocumentDrawingType`
- **AND** the already supported `Прямоугольник` has that same measured type

#### Scenario: UNF module names a chart drawing

- **WHEN** configuration code reads `ТипРисункаТабличногоДокумента.Диаграмма`
- **THEN** semantic resolution accepts the member
- **AND** its string presentation is `Chart`
### Requirement: Week-calendar functions resolve without guessing a session calendar

The base runtime SHALL resolve `КонецНедели` / `EndOfWeek` with exactly one
date argument and `НеделяГода` / `WeekOfYear` with exactly one date argument.
Until the Session supplies the week-start setting for its platform context,
execution of either new function SHALL raise a catchable error rather than
apply a calendar observed in a different information base. Existing
standalone `НачалоНедели` behavior SHALL remain unchanged.

#### Scenario: UNF code names week end

- **WHEN** configuration code contains `КонецНедели(Дата(2024, 1, 1))`
- **THEN** semantic resolution accepts the function
- **AND** execution without a week-start setting reports its absence

#### Scenario: Week end lacks its date

- **WHEN** code calls `EndOfWeek()`
- **THEN** semantic resolution reports an argument-count error

#### Scenario: UNF code names a week number

- **WHEN** configuration code contains `НеделяГода(Дата(2024, 1, 1))`
- **THEN** semantic resolution accepts the function
- **AND** execution without a week-start setting reports its absence

#### Scenario: Week number lacks its date

- **WHEN** code calls `WeekOfYear()`
- **THEN** semantic resolution reports an argument-count error
### Requirement: Quarter boundaries match the measured platform dates

The base runtime SHALL resolve `НачалоКвартала` / `BegOfQuarter` and
`КонецКвартала` / `EndOfQuarter`, each with one required date argument.
They SHALL return the first second of the first month and the last whole
second of the last month of that date's quarter.

#### Scenario: Leap-day quarter

- **WHEN** code evaluates both boundaries of 2024-02-29 12:34:56
- **THEN** the start is 2024-01-01 00:00:00
- **AND** the end is 2024-03-31 23:59:59

#### Scenario: Quarter boundary call lacks a date

- **WHEN** code calls `BegOfQuarter()` or `EndOfQuarter()`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Data-composition settings refresh methods match the platform enum

The base runtime SHALL expose the complete `СпособВосстановленияНастроекКомпоновкиДанных` / `DataCompositionSettingsRefreshMethod` system enumeration with `Полное` / `Full` and `ПроверятьДоступность` / `CheckAvailability`. The members SHALL share the measured `DataCompositionSettingsRefreshMethod` type and their measured string presentations.

#### Scenario: UNF names a settings refresh method

- **WHEN** configuration code reads `СпособВосстановленияНастроекКомпоновкиДанных.ПроверятьДоступность`
- **THEN** semantic resolution accepts the member
- **AND** its string presentation is `Check accessibility`
- **AND** its type is `DataCompositionSettingsRefreshMethod`

#### Scenario: Language aliases identify the same values

- **WHEN** code compares either Russian member with its English alias
- **THEN** the values compare equal

### Requirement: Data-composition text output types match the platform enum

The base runtime SHALL expose the complete `ТипВыводаТекстаКомпоновкиДанных` / `DataCompositionTextOutputType` system enumeration with `Авто` / `Auto`, `Выводить` / `Output`, and `НеВыводить` / `DontOutput`. Both language names SHALL identify the same typed members, and string presentations SHALL match the platform 8.3.27 oracle.

#### Scenario: UNF names a text output type

- **WHEN** configuration code reads `ТипВыводаТекстаКомпоновкиДанных.НеВыводить`
- **THEN** semantic resolution accepts the member
- **AND** its type is `DataCompositionTextOutputType`
- **AND** its presentation matches the measured platform value

#### Scenario: Language aliases identify the same members

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Standard-period construction resolves with documented arities

The ORM library SHALL resolve `Новый СтандартныйПериод` / `New StandardPeriod` with zero, one, or two arguments as documented by the installed platform Syntax Assistant. Until a standard-period value model is available, execution SHALL fail explicitly instead of returning a fabricated period.

#### Scenario: UNF constructs a standard period

- **WHEN** configuration code constructs `Новый СтандартныйПериод()`
- **THEN** semantic resolution accepts the constructor
- **AND** execution reports the unavailable value model

#### Scenario: Excess arguments

- **WHEN** code passes three arguments to `Новый StandardPeriod`
- **THEN** semantic resolution reports an argument-count error

### Requirement: View scaling modes match the platform enum

The base runtime SHALL expose the complete `РежимМасштабированияПросмотра` / `ViewScalingMode` system enumeration with `Авто` / `Auto`, `Крупный` / `Large`, and `Обычный` / `Normal`. Both language names SHALL identify the same typed members, and string presentations SHALL match the measured platform 8.3.27 values.

#### Scenario: UNF names a view scaling mode

- **WHEN** configuration code reads `РежимМасштабированияПросмотра.Крупный`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Aliases identify the same values

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Reached data-composition construction fails explicitly without its model

The ORM library SHALL resolve the Syntax Assistant-defined zero-argument constructors `СхемаКомпоновкиДанных` / `DataCompositionSchema` and `ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент` / `DataCompositionResultSpreadsheetDocumentOutputProcessor`. Construction SHALL fail explicitly until their value and output models are available. Extra arguments SHALL fail semantic arity checking.

#### Scenario: UNF names a data-composition schema

- **WHEN** configuration code contains `Новый СхемаКомпоновкиДанных`
- **THEN** semantic resolution accepts it
- **AND** execution reports the missing schema model

#### Scenario: UNF names a spreadsheet output processor

- **WHEN** configuration code contains `Новый ПроцессорВыводаРезультатаКомпоновкиДанныхВТабличныйДокумент`
- **THEN** semantic resolution accepts it
- **AND** execution reports the missing output model

#### Scenario: Excess constructor arguments

- **WHEN** either constructor receives one argument
- **THEN** semantic resolution reports an argument-count error

### Requirement: Data-composition total placement matches the platform enum

The base runtime SHALL expose the complete `РасположениеИтоговКомпоновкиДанных` / `DataCompositionTotalPlacement` system enumeration with `Авто` / `Auto`, `Конец` / `End`, `Начало` / `Begin`, `НачалоИКонец` / `BeginAndEnd`, and `Нет` / `None`. Both language names SHALL identify the same typed members, and their string presentations SHALL match the measured platform 8.3.27 values.

#### Scenario: UNF names total placement

- **WHEN** configuration code reads `РасположениеИтоговКомпоновкиДанных.НачалоИКонец`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same values

- **WHEN** code compares every Russian member with its English alias
- **THEN** every corresponding pair compares equal

### Requirement: Query boundary construction resolves with documented arity

The ORM library SHALL resolve `Новый Граница` / `New Boundary` with one required value and one optional boundary-kind argument, as documented by the platform Syntax Assistant. Until a query-boundary value model exists, execution SHALL fail explicitly.

#### Scenario: UNF names a query boundary

- **WHEN** configuration code constructs `Новый Граница(Дата(2024, 1, 1))`
- **THEN** semantic resolution accepts the constructor
- **AND** execution reports the missing value model

#### Scenario: Boundary call has no value

- **WHEN** code calls `Новый Boundary()`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Picture-size modes match the platform enum

The base runtime SHALL expose the complete eight-member `РазмерКартинки` / `PictureSize` system enumeration documented by the platform Syntax Assistant. Both language names SHALL identify the same typed members, and each string presentation SHALL match the platform 8.3.27 oracle.

#### Scenario: UNF names a picture size

- **WHEN** configuration code reads `РазмерКартинки.РеальныйРазмер`
- **THEN** semantic resolution accepts the member
- **AND** its type is `PictureSize`
- **AND** its presentation is `Actual size`

#### Scenario: All picture-size aliases

- **WHEN** code compares each Russian member with its English alias
- **THEN** all eight pairs compare equal

### Requirement: Spreadsheet-document shift types match the platform enum

The base runtime SHALL expose the complete `ТипСмещенияТабличногоДокумента` / `SpreadsheetDocumentShiftType` system enumeration with `БезСмещения` / `WithoutShift`, `ПоВертикали` / `Vertical`, and `ПоГоризонтали` / `Horizontal`. Both language names SHALL identify the same typed members, and their presentations SHALL match the measured platform 8.3.27 values.

#### Scenario: UNF names a horizontal shift

- **WHEN** configuration code reads `ТипСмещенияТабличногоДокумента.ПоГоризонтали`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same values

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Line construction resolves with documented arity

The base runtime SHALL resolve `Новый Линия` / `New Line` with one required line kind and up to two optional arguments, as documented by the platform Syntax Assistant. Until the line value model exists, execution SHALL fail explicitly instead of returning a placeholder.

#### Scenario: UNF names a line

- **WHEN** configuration code constructs `Новый Линия(Неопределено)`
- **THEN** semantic resolution accepts the constructor
- **AND** execution reports the missing value model

#### Scenario: Line has no kind

- **WHEN** code calls `Новый Line()`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Chart types match the complete platform enum

The base runtime SHALL expose all 47 members of the `ТипДиаграммы` / `ChartType` system enumeration listed by the installed platform Syntax Assistant 8.3.27. Both language names SHALL identify the same typed members, and every string presentation SHALL match the measured platform oracle.

#### Scenario: UNF names a chart type

- **WHEN** configuration code reads a documented member of `ТипДиаграммы`
- **THEN** semantic resolution accepts that member
- **AND** its type is `ChartType`
- **AND** its presentation matches the platform oracle

#### Scenario: Complete alias set

- **WHEN** code compares each of the 47 Russian members with its English alias
- **THEN** every corresponding pair compares equal

### Requirement: Chart-label orientations retain both measured Russian names

The base runtime SHALL expose the complete four-member `ChartLabelsOrientation` system enumeration. Both `ОриентацияПодписейДиаграммы` and the measured `ОриентацияМетокДиаграммы` SHALL identify the same enumeration as `ChartLabelsOrientation`. Member names and presentations SHALL match the platform 8.3.27 oracle.

#### Scenario: UNF uses the older Russian name

- **WHEN** configuration code reads `ОриентацияМетокДиаграммы.Горизонтально`
- **THEN** semantic resolution accepts it
- **AND** the value equals `ОриентацияПодписейДиаграммы.Горизонтально`
- **AND** its type is `ChartLabelsOrientation`

#### Scenario: Complete alias set

- **WHEN** code compares all four members through both Russian names and the English name
- **THEN** every corresponding value compares equal

### Requirement: Boundary types match the platform enum

The base runtime SHALL expose the complete `ВидГраницы` / `BoundaryType` system enumeration with `Включая` / `Including` and `Исключая` / `Excluding`. Both language names SHALL identify the same typed values, and their presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names an inclusive boundary

- **WHEN** configuration code reads `ВидГраницы.Включая`
- **THEN** semantic resolution accepts it
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same values

- **WHEN** code compares both Russian members with their English aliases
- **THEN** each corresponding pair compares equal

### Requirement: Chart series limits match the platform enum

The base runtime SHALL expose the complete `МаксимумСерий` / `MaxSeries` system enumeration with `НеЗадано` / `NotDefined`, `Ограничено` / `Limited`, and `Процент` / `Percent`. Both language names SHALL identify the same typed values, and presentations SHALL match the platform 8.3.27 oracle.

#### Scenario: UNF names a series limit

- **WHEN** configuration code reads `МаксимумСерий.Ограничено`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Series-limit aliases

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Chart-label types match the complete platform enum

The base runtime SHALL expose all 23 members of `ВидПодписейКДиаграмме` / `ChartLabelType` listed by the installed platform Syntax Assistant 8.3.27. Both language names SHALL identify the same typed values, and all presentations SHALL match the platform oracle.

#### Scenario: UNF names a chart-label type

- **WHEN** configuration code reads a documented member of `ВидПодписейКДиаграмме`
- **THEN** semantic resolution accepts that member
- **AND** its type is `ChartLabelType`
- **AND** its presentation matches the platform oracle

#### Scenario: Complete chart-label aliases

- **WHEN** code compares each of the 23 Russian members with its English alias
- **THEN** every corresponding pair compares equal

### Requirement: Chart-line styles match the platform enum

The base runtime SHALL expose all six members of `ТипЛинииДиаграммы` / `ChartLineType` listed by the installed platform Syntax Assistant 8.3.27. Both language names SHALL identify the same typed values, and member presentations SHALL match the measured platform oracle.

#### Scenario: UNF names a chart-line style

- **WHEN** configuration code reads `ТипЛинииДиаграммы.ПунктирТочка`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: All chart-line aliases

- **WHEN** code compares every Russian member with its English alias
- **THEN** all six pairs compare equal

### Requirement: Data-composition user settings construction resolves explicitly

The ORM library SHALL resolve the Syntax Assistant-defined zero-argument `ПользовательскиеНастройкиКомпоновкиДанных` / `DataCompositionUserSettings` constructor. Until its mutable settings value model is available, execution SHALL fail explicitly. An argument SHALL fail semantic arity checking.

#### Scenario: UNF names user settings

- **WHEN** configuration code contains `Новый ПользовательскиеНастройкиКомпоновкиДанных`
- **THEN** semantic resolution accepts the constructor
- **AND** execution reports the unavailable value model

#### Scenario: User settings receives an argument

- **WHEN** code calls `Новый DataCompositionUserSettings(1)`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Form button types match the platform enum

The base runtime SHALL expose the complete `ВидКнопкиФормы` / `FormButtonType` system enumeration with `Гиперссылка` / `Hyperlink`, `ГиперссылкаКоманднойПанели` / `CommandBarHyperlink`, `КнопкаКоманднойПанели` / `CommandBarButton`, and `ОбычнаяКнопка` / `UsualButton`. Both language names SHALL identify the same typed values, and presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names a form button type

- **WHEN** configuration code reads `ВидКнопкиФормы.ОбычнаяКнопка`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same values

- **WHEN** code compares all four Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Picture construction recognizes documented loaded forms

The base runtime SHALL continue to construct the existing empty `Картинка` / `Picture` for zero arguments. It SHALL also resolve the Syntax Assistant-defined one- and two-argument forms for a file name or binary data and an optional transparent-background flag. Until loaded-picture decoding is available, executing a nonempty form SHALL fail explicitly rather than return an empty picture. More than two arguments SHALL fail semantic arity checking.

#### Scenario: Existing empty picture

- **WHEN** code constructs `Новый Картинка()`
- **THEN** it receives the same empty picture as before

#### Scenario: UNF names a loaded picture

- **WHEN** configuration code constructs `Новый Картинка("image.png")`
- **THEN** semantic resolution accepts the call
- **AND** execution reports that picture loading is unavailable

#### Scenario: Excess picture arguments

- **WHEN** code calls `New Picture(1, 2, 3)`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Data-composition sort directions match the platform enum

The base runtime SHALL expose the complete `НаправлениеСортировкиКомпоновкиДанных` / `DataCompositionSortDirection` enumeration with `Возр` / `Asc` and `Убыв` / `Desc`. Both language names SHALL identify the same typed values, and presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names a data-composition sort direction

- **WHEN** configuration code reads `НаправлениеСортировкиКомпоновкиДанных.Возр`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same values

- **WHEN** code compares both Russian members with their English aliases
- **THEN** both pairs compare equal

### Requirement: Data-composition group types match the platform enum

The base runtime SHALL expose the complete `ТипГруппировкиКомпоновкиДанных` / `DataCompositionGroupType` enumeration with `Иерархия` / `Hierarchy`, `ТолькоИерархия` / `HierarchyOnly`, and `Элементы` / `Items`. Both language names SHALL identify the same typed values, and presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names a group type

- **WHEN** configuration code reads `ТипГруппировкиКомпоновкиДанных.Элементы`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same group types

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Data-composition group-field placement matches the platform enum

The base runtime SHALL expose the complete `РасположениеПолейГруппировкиКомпоновкиДанных` / `DataCompositionGroupFieldsPlacement` enumeration with `Вместе` / `Together`, `Отдельно` / `Separately`, and `ОтдельноИТолькоВИтогах` / `SeparatelyAndInTotalsOnly`. Both language names SHALL identify the same typed values, and presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names a group-field placement

- **WHEN** configuration code reads `РасположениеПолейГруппировкиКомпоновкиДанных.Вместе`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same placements

- **WHEN** code compares all three Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Data-composition attribute placement matches the platform enum

The base runtime SHALL expose the complete `РасположениеРеквизитовКомпоновкиДанных` / `DataCompositionAttributesPlacement` enumeration with `Вместе` / `Together`, `ВместеСВладельцем` / `WithOwnerField`, `ВСпециальнойПозиции` / `SpecialPosition`, and `Отдельно` / `Separately`. Both language names SHALL identify the same typed values, and presentations SHALL match the measured platform 8.3.27 oracle.

#### Scenario: UNF names attribute placement

- **WHEN** configuration code reads `РасположениеРеквизитовКомпоновкиДанных.Вместе`
- **THEN** semantic resolution accepts the member
- **AND** its type and presentation match the platform oracle

#### Scenario: Language aliases identify the same attribute placements

- **WHEN** code compares all four Russian members with their English aliases
- **THEN** every corresponding pair compares equal

### Requirement: Reached session-support enumerations match the platform

The base runtime SHALL expose the complete platform enumerations named below. Each enumeration SHALL contain exactly the Russian and English member pairs recorded in `tests/conformance/measure/1c/measure-session-enum-batch.bsl`; root presentations, member types, member presentations, and alias equality SHALL match the direct platform 8.3.27 oracle for that script.

| Russian root | English root |
| --- | --- |
| `ВидДекорацииФормы` | `FormDecorationType` |
| `ВидПереключателя` | `RadioButtonType` |
| `ГруппировкаПодчиненныхЭлементовФормы` | `ChildFormItemsGroup` |
| `ИсторияВыбораПриВводе` | `ChoiceHistoryOnInput` |
| `ОтображениеКнопкиВыбора` | `ChoiceButtonRepresentation` |
| `ОтображениеУправленияОбычнойГруппы` | `UsualGroupControlRepresentation` |
| `ПоведениеОбычнойГруппы` | `UsualGroupBehavior` |
| `ПоложениеЗаголовкаЭлементаФормы` | `FormItemTitleLocation` |
| `ПоложениеКнопкиВКоманднойПанели` | `ButtonLocationInCommandBar` |
| `РасположениеРесурсовКомпоновкиДанных` | `DataCompositionResourcesPlacement` |
| `ТипДополненияПериодаКомпоновкиДанных` | `DataCompositionPeriodAdditionType` |

#### Scenario: UNF references session-support enumeration members

- **WHEN** configuration code reads a member from any listed root
- **THEN** semantic resolution accepts that member with its platform type and presentation

#### Scenario: All measured language aliases agree

- **WHEN** the batch measure script compares each Russian member to its English counterpart
- **THEN** all eleven alias checks pass and every output line matches the platform oracle

### Requirement: Further reached session enumerations match the platform

The base runtime SHALL expose the complete `ГлубинаЦвета` / `ColorDepth`, `ОтображениеПодсказки` / `ToolTipRepresentation`, and `ТипКлиентскогоПриложения` / `ClientApplicationType` system enumerations. Each SHALL contain exactly the Russian and English member pairs recorded in `tests/conformance/measure/1c/measure-next-session-enums.bsl`; root presentations, member types, member presentations, and alias equality SHALL match the direct platform 8.3.27 oracle.

#### Scenario: UNF references the further enums

- **WHEN** configuration code reads a measured member of any of these three enums
- **THEN** semantic resolution accepts its platform type and presentation
- **AND** its Russian and English names identify the same value

### Requirement: Processing-picture constructor resolves without inventing image processing

The base runtime SHALL recognize `ОбрабатываемаяКартинка` / `ProcessingPicture` with zero to three arguments, covering the two constructor forms documented by Syntax Assistant 8.3.27. Until its picture-processing value model exists, execution SHALL fail explicitly. Four or more arguments SHALL fail semantic arity checking.

#### Scenario: UNF constructs a processing picture

- **WHEN** code constructs `Новый ОбрабатываемаяКартинка(Картинка)`
- **THEN** semantic resolution accepts the call
- **AND** execution reports that picture processing is unavailable

#### Scenario: Excess processing-picture arguments

- **WHEN** code passes four arguments to `New ProcessingPicture`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Reached random-number generator constructor resolves explicitly

The base runtime SHALL recognize `ГенераторСлучайныхЧисел` / `RandomNumberGenerator` with zero or one argument, as documented by the installed 8.3.27 Syntax Assistant. Until the generator algorithm and entropy source are implemented, execution SHALL fail with a catchable explicit unsupported error rather than return a fabricated generator. Two or more arguments SHALL fail semantic arity checking.

#### Scenario: A dormant configuration method constructs a generator

- **WHEN** a configuration method contains `Новый ГенераторСлучайныхЧисел(НачальноеЧисло)` but is not called
- **THEN** semantic resolution accepts the method
- **AND** no generator is constructed during module initialization

#### Scenario: A guarded generator construction

- **WHEN** code constructs `New RandomNumberGenerator()` inside an exception handler
- **THEN** the constructor raises an explicit catchable unsupported error

#### Scenario: Excess generator arguments

- **WHEN** code passes two arguments to `New RandomNumberGenerator`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Reached XML reader settings constructor resolves explicitly

The XML component SHALL recognize `ПараметрыЧтенияXML` / `XMLReaderSettings` with zero through eleven arguments, matching the two constructor forms in the installed 8.3.27 Syntax Assistant. Until the XML reader settings value model is implemented, execution SHALL raise a catchable explicit unsupported error instead of returning settings that the reader ignores. Twelve arguments SHALL fail semantic arity checking.

#### Scenario: A dormant configuration method names XML reader settings

- **WHEN** a configuration method contains `Новый ПараметрыЧтенияXML()` but is not called
- **THEN** semantic resolution accepts the method

#### Scenario: Guarded XML reader settings construction

- **WHEN** code constructs `New XMLReaderSettings()` inside an exception handler
- **THEN** execution raises an explicit catchable unsupported error

### Requirement: Use-output system enumeration

The base runtime SHALL expose `ИспользованиеВывода` / `UseOutput` with the three members `Авто` / `Auto`, `Разрешить` / `Enable`, and `Запретить` / `Disable`. Russian and English member names SHALL resolve to the same values, and their type and String presentations SHALL match the recorded platform output.

#### Scenario: UNF references use-output mode

- **WHEN** configuration code reads `ИспользованиеВывода.Авто`
- **THEN** semantic resolution returns the same value as `UseOutput.Auto`

### Requirement: HTTP request construction accepts headers by alias

The `HTTPЗапрос` / `HTTPRequest` constructor SHALL accept an optional second `Соответствие` / `Map` argument containing headers. The request SHALL retain the same map object, so subsequent insertions through either reference are visible through the other. The existing zero- and one-argument forms SHALL retain their behavior. More than two arguments SHALL fail semantic arity checking.

#### Scenario: Construct a request with headers

- **WHEN** code constructs `Новый HTTPЗапрос("/probe", Заголовки)`
- **THEN** its resource address is `/probe`
- **AND** changes through the original map and request property are mutually visible

#### Scenario: Excess request arguments

- **WHEN** code passes three arguments to `New HTTPRequest`
- **THEN** semantic resolution reports an argument-count error

### Requirement: Spreadsheet file-type enumeration exposes the measured complete set

The base runtime SHALL expose all 18 `ТипФайлаТабличногоДокумента` / `SpreadsheetDocumentFileType` members recorded in `tests/conformance/measure/1c/measure-complete-spreadsheet-file-type.bsl`. Both language roots SHALL identify the same member values, and member presentations SHALL match the platform 8.3.27 oracle. The existing root and member-type string presentations SHALL remain unchanged for additive compatibility. Member availability SHALL NOT imply that the spreadsheet exporter implements the corresponding file format.

#### Scenario: UNF references HTML output type

- **WHEN** configuration code reads `ТипФайлаТабличногоДокумента.HTML`
- **THEN** semantic resolution accepts the member
- **AND** its member presentation matches the platform oracle

#### Scenario: All file-type aliases agree

- **WHEN** the measure script compares all 18 Russian and English members
- **THEN** every pair compares equal

### Requirement: Base64 binary-data decoding uses the existing binary primitive

The base runtime SHALL expose `ПолучитьДвоичныеДанныеИзBase64ДвоичныхДанных` / `GetBinaryDataFromBase64BinaryData` with exactly one `ДвоичныеДанные` argument. It SHALL decode the input bytes as Base64 and return `ДвоичныеДанные` according to the direct platform oracle in `tests/conformance/measure/1c/measure-base64-binary-data.bsl`. It SHALL ignore CR and LF bytes. A non-Base64 byte after a complete quartet SHALL terminate decoding successfully; one before a complete quartet SHALL produce empty binary data. A Base64 alphabet byte following a complete padded quartet SHALL make the input invalid and produce empty binary data. The existing string-input variant SHALL retain its behavior.

#### Scenario: Decode binary Base64 input

- **WHEN** a caller passes binary data containing `TWE=`
- **THEN** the returned binary data contains two bytes and re-encodes as `TWE=`

#### Scenario: Reject an invalid binary Base64 prefix

- **WHEN** a caller passes `TW?E=` or `TWE=Z`
- **THEN** the returned binary data is empty

#### Scenario: Terminate after a complete quartet

- **WHEN** a caller passes `TWE=?` or `TWFu `
- **THEN** the returned bytes re-encode as `TWE=` or `TWFu` respectively

#### Scenario: Ignore line separators

- **WHEN** a caller passes CR or LF before or within `TWE=`
- **THEN** the returned bytes re-encode as `TWE=`

#### Scenario: Reject an incorrect argument count

- **WHEN** a caller supplies zero or two arguments
- **THEN** semantic resolution reports an argument-count error

### Requirement: Form vertical alignment and spacing enumerations match the platform

The base runtime SHALL expose the complete four-member `ВертикальноеПоложениеЭлемента` / `ItemVerticalAlign` and six-member `ИнтервалМеждуЭлементамиФормы` / `FormItemSpacing` system enumerations recorded in `tests/conformance/measure/1c/measure-form-alignment-spacing-enums.bsl`. Both language roots SHALL identify the same member values; root presentations, member types, member presentations, and aliases SHALL match the direct platform 8.3.27 oracle.

#### Scenario: UNF references form alignment and spacing

- **WHEN** configuration code reads a member of either enumeration
- **THEN** semantic resolution accepts it and presents its measured value
- **AND** the Russian and English member names compare equal

### Requirement: Client application global belongs to the current Session

The `bsl-session` library SHALL expose `КлиентскоеПриложение` / `ClientApplication` as a global object in a registered configuration Session. Its zero-argument `ТипПриложения()` / `ApplicationType()` method SHALL return the measured `ТипКлиентскогоПриложения.ТонкийКлиент` / `ClientApplicationType.ThinClient` for a thin-client Session and `Неопределено` / `Undefined` for a background-job Session. For a web-client Session it SHALL return `ClientApplicationType.WebClient`; that mapping remains explicitly unmeasured until a real web-client oracle is captured. The object type presentation SHALL be `Client application`.

#### Scenario: Thin client reads its application type

- **WHEN** a thin-client Session evaluates `КлиентскоеПриложение.ТипПриложения()`
- **THEN** it receives `ClientApplicationType.ThinClient`
- **AND** the object's type presentation matches the platform thin-client oracle

#### Scenario: Background job reads its application type

- **WHEN** a background-job Session evaluates `ClientApplication.ApplicationType()`
- **THEN** it receives `Undefined`

#### Scenario: Web client reads its application type

- **WHEN** a web-client Session evaluates `КлиентскоеПриложение.ТипПриложения()`
- **THEN** it receives `ClientApplicationType.WebClient` under the registered unmeasured decision

### Requirement: Value-table copy accepts measured omitted positions

The base resolver SHALL accept an omitted first or second argument of `ТаблицаЗначений.Скопировать` / `ValueTable.Copy` and pass `Неопределено` / `Undefined` to the existing copy implementation for that position. It SHALL preserve the existing behavior of explicit arguments, other closed core methods, and runtime validation for non-table receivers. The result SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-value-table-copy-omissions.bsl`.

#### Scenario: Omit the row filter

- **WHEN** a value table copies with `Таблица.Скопировать(, "А")`
- **THEN** its row and selected column match `Таблица.Скопировать(Неопределено, "А")`

#### Scenario: Omit the column selection

- **WHEN** a value table copies with `Таблица.Скопировать(Неопределено, )`
- **THEN** all columns are retained

### Requirement: XML writer settings accept the complete measured constructor

The existing `ПараметрыЗаписиXML` / `XMLWriterSettings` constructor SHALL accept zero through five arguments in the Syntax Assistant order: encoding, version, node indentation, attribute indentation, and indentation characters. The object SHALL expose the five corresponding read-only properties. Defaults SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-xml-writer-settings-five-args.bsl`: UTF-8, version 1.0, node indentation enabled, attribute indentation disabled, and one tab indentation character. With attribute indentation enabled, a second attribute SHALL start on a new line with indentation one level deeper than its element. Custom indentation characters SHALL indent elements and attributes. With node indentation disabled, attribute indentation SHALL produce no line breaks. The empty string, space, tab, LF, and CR SHALL be accepted as indentation characters; non-whitespace characters SHALL fail construction. Existing zero-through-three-argument behavior and the public three-argument Rust helper SHALL remain unchanged.

#### Scenario: Five-argument settings format XML

- **WHEN** the caller uses `Новый ПараметрыЗаписиXML("UTF-8", "1.0", Истина, Истина, "  ")`
- **THEN** the second attribute and child element are indented exactly as in the platform oracle

#### Scenario: Attribute indentation is disabled

- **WHEN** the fourth argument is `Ложь`
- **THEN** attributes stay on the same line while child elements use the custom indentation

#### Scenario: Node indentation is disabled

- **WHEN** the third argument is `Ложь` and the fourth is `Истина`
- **THEN** no formatting line breaks are emitted

#### Scenario: Invalid indentation characters

- **WHEN** the fifth argument contains a non-whitespace character
- **THEN** construction returns a catchable error

### Requirement: Core Insert preserves omitted positions
The base runtime SHALL accept an omitted first or second position of the core `Вставить` / `Insert` method as an explicit `Неопределено` / `Undefined` value, leaving receiver-specific validation to the existing method implementation.

#### Scenario: Omitted first position
- **WHEN** an Array, Map, or Structure calls `Insert(, value)`
- **THEN** the call follows that receiver's existing validation of an Undefined index or key

#### Scenario: Omitted second position
- **WHEN** an Array, Map, or Structure calls `Insert(key, )`
- **THEN** the inserted value is Undefined

### Requirement: HashFunction exposes the complete measured member set
The base runtime SHALL expose the six `ХешФункция` / `HashFunction` members documented by platform 8.3.27: `CRC32`, `MD5`, `PBKDF2SHA256`, `SHA1`, `SHA256`, and `SHA512`. Russian and English enumeration names SHALL resolve to the same members. Adding members SHALL NOT imply support for computing the newly exposed hash algorithms.

#### Scenario: Newly reached CRC32 member
- **WHEN** configuration code reads `ХешФункция.CRC32`
- **THEN** it receives the same enumeration value as `HashFunction.CRC32`

#### Scenario: Remaining newly exposed members
- **WHEN** code reads `PBKDF2SHA256` or `SHA512` through either enumeration spelling
- **THEN** it receives the corresponding measured member of type `HashFunction`

### Requirement: Current locale code belongs to the current Session

The `bsl-session` library SHALL expose the zero-argument `ТекущийКодЛокализации` / `CurrentLocaleCode` function. It SHALL return exactly the localization code supplied for that Session as a String. If no code was supplied, calling it SHALL fail explicitly. A background Session SHALL use its own supplied code rather than implicitly inheriting the parent's code. The CLI SHALL accept an optional `--locale-code` with `--config-db` and pass it to the opened Session; it SHALL NOT derive the code from the process locale.

#### Scenario: Explicit session localization

- **WHEN** a Session is opened with localization code `ru_RU` and calls either spelling
- **THEN** both calls return the String `ru_RU`

#### Scenario: Missing localization code

- **WHEN** a Session without a supplied localization code calls the function
- **THEN** the call returns an explicit error

#### Scenario: Background localization is separate

- **WHEN** a background Session is opened from a parent with a different supplied code
- **THEN** the background Session returns its own code

### Requirement: Current system language belongs to the current Session

The `bsl-session` library SHALL expose the zero-argument `ТекущийЯзыкСистемы` / `CurrentSystemLanguage` function. It SHALL return exactly the system-language code supplied for that Session as a String and fail explicitly when none was supplied. A background Session SHALL use its own supplied code. The CLI SHALL accept optional `--system-language` only with `--config-db` and pass it to that Session; it SHALL NOT infer it from the process locale or localization code.

#### Scenario: Explicit system language

- **WHEN** a Session is opened with system language `en` and calls either spelling
- **THEN** both calls return the String `en`

#### Scenario: Missing system language

- **WHEN** a Session without a supplied system language calls the function
- **THEN** the call returns an explicit error

#### Scenario: Background system language is separate

- **WHEN** a background Session is opened with a different system language from its parent
- **THEN** it returns its own code

### Requirement: External resources mode is reported for a Session

The `bsl-session` library SHALL expose the zero-argument `ПолучитьРежимВнешнихРесурсов` / `GetExternalResourcesMode` function. It SHALL return the String `D` for the documented default mode unless the host supplies that Session's mode; a supplied mode SHALL be returned for that Session only. The function SHALL NOT read process-global state. The `A` result for an explicitly selected alternative mode is a registered unmeasured decision pending a platform session configured in that mode.

#### Scenario: Default mode

- **WHEN** a Session without an explicit mode calls either spelling
- **THEN** it receives the String `D`

#### Scenario: Host-supplied alternative mode

- **WHEN** one Session is supplied mode `A` and a second Session is not
- **THEN** the first returns `A` and the second returns `D`

### Requirement: Form pages representation enumeration matches the platform

The base runtime SHALL expose all seven members of `ОтображениеСтраницФормы` / `FormPagesRepresentation`: `Авто` / `Auto`, `Нет` / `None`, `Пролистывание` / `Swipe`, `ЗакладкиСверху` / `TabsOnTop`, `ЗакладкиСнизу` / `TabsOnBottom`, `ЗакладкиСлеваГоризонтально` / `TabsOnLeftHorizontal`, and `ЗакладкиСправаГоризонтально` / `TabsOnRightHorizontal`. Member aliases, String presentations, and the `FormPagesRepresentation` type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-form-pages-representation.bsl`.

#### Scenario: Configuration reads a form pages member

- **WHEN** configuration code reads any member through either enumeration spelling
- **THEN** both spellings identify the same member with the measured presentation and type

### Requirement: Formatted document constructor resolves without fabricating its value

The base runtime SHALL recognize the zero-argument `Новый ФорматированныйДокумент` / `New FormattedDocument` constructor documented by Syntax Assistant. A dormant construction in configuration code SHALL compile. Executing the constructor SHALL return an explicit catchable unsupported-value error until its object model is implemented. Additional arguments SHALL fail arity validation.

#### Scenario: Dormant formatted document construction

- **WHEN** an uncalled configuration method contains `Новый ФорматированныйДокумент`
- **THEN** semantic resolution accepts the method

#### Scenario: Guarded execution

- **WHEN** the constructor is executed inside `Попытка` / `Try`
- **THEN** its unsupported-value error is caught

### Requirement: Form button picture location enumeration matches the platform

The base runtime SHALL expose the complete three-member `ПоложениеКартинкиКнопкиФормы` / `FormButtonPictureLocation` enumeration: `Авто` / `Auto`, `Лево` / `Left`, and `Право` / `Right`. Member aliases, String presentations, and the `FormButtonPictureLocation` type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-form-button-picture-location.bsl`.

#### Scenario: Configuration reads a button picture location

- **WHEN** configuration code reads any member through either enumeration spelling
- **THEN** both spellings identify the same measured value

### Requirement: Spreadsheet drawing line type enumeration matches the platform

The base runtime SHALL expose all six members of `ТипЛинииРисункаТабличногоДокумента` / `SpreadsheetDocumentDrawingLineType`: `НетЛинии` / `None`, `Сплошная` / `Solid`, `Пунктир` / `Dashed`, `Точечная` / `Dotted`, `ПунктирТочка` / `DashDotted`, and `ПунктирТочкаТочка` / `DashDottedDotted`. The member aliases, String presentations (including `DotDashDotted` for the final member), and type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-spreadsheet-drawing-line-type.bsl`.

#### Scenario: Configuration reads a drawing line type

- **WHEN** configuration code reads any member through either enumeration spelling
- **THEN** both spellings identify the same measured value and type

### Requirement: Check box type enumeration matches the platform

The base runtime SHALL expose all four members of `ВидФлажка` / `CheckBoxType`: `Авто` / `Auto`, `Флажок` / `CheckBox`, `Выключатель` / `Switch`, and `Тумблер` / `Tumbler`. The aliases, String presentations (including `Checkbox` and `Tumblers`), and `CheckBoxType` type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-checkbox-type.bsl`.

#### Scenario: Configuration reads a check box type

- **WHEN** configuration code reads any member through either enumeration spelling
- **THEN** both spellings identify the same measured value and type

### Requirement: Delivered notification surface resolves without fabricating an object

The base runtime SHALL recognize the zero-argument `Новый ДоставляемоеУведомление` / `New DeliverableNotification` constructor documented by Syntax Assistant. A dormant construction SHALL compile; executing it SHALL raise an explicit catchable unsupported-value error until the notification object model exists. Additional constructor arguments SHALL fail arity validation. The base runtime SHALL also expose the measured two-member `ЗвуковоеОповещение` / `SoundAlert` enumeration, with `Нет` / `None` and `ПоУмолчанию` / `Default`, matching the direct platform oracle in `tests/conformance/measure/1c/measure-sound-alert.bsl`.

#### Scenario: Dormant notification construction

- **WHEN** an uncalled configuration method contains `Новый ДоставляемоеУведомление`
- **THEN** semantic resolution accepts the method

#### Scenario: Guarded notification construction

- **WHEN** the constructor is executed inside `Попытка` / `Try`
- **THEN** its unsupported-value error is caught

#### Scenario: Notification sound values

- **WHEN** configuration code reads either sound member through either enumeration spelling
- **THEN** the aliases, presentations, and `SoundAlert` type match the platform oracle

### Requirement: Notification sending manager is a Session global

The `bsl-session` library SHALL recognize the read-only `ОтправкаДоставляемыхУведомлений` / `DeliverableNotificationSend` global documented by Syntax Assistant. Dormant configuration code referencing it SHALL compile. Until a notification transport is supplied, evaluating the global SHALL fail explicitly and catchably rather than return a fabricated manager.

#### Scenario: Dormant manager reference

- **WHEN** an uncalled configuration method refers to the global
- **THEN** semantic resolution accepts the method

#### Scenario: Guarded manager access

- **WHEN** code evaluates the global inside `Попытка` / `Try`
- **THEN** the unavailable-service error is caught

### Requirement: Through alignment enumeration matches the platform

The base runtime SHALL expose the complete three-member `СквозноеВыравнивание` / `ThroughAlign` enumeration: `Авто` / `Auto`, `Использовать` / `Use`, and `НеИспользовать` / `DontUse`. Member aliases, String presentations (including `Do not use`), and the `ThroughAlign` type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-through-align.bsl`.

#### Scenario: Configuration reads a through-alignment member

- **WHEN** configuration code reads any member through either enumeration spelling
- **THEN** both spellings identify the same measured value and type

### Requirement: Button-shape enumeration

The base runtime SHALL expose `ФигураКнопки` / `ButtonShape` as a distinct three-member enumeration from `ОтображениеФигурыКнопки` / `ButtonShapeRepresentation`. Its `Auto`, `Usual`, and `Oval` members, Russian aliases, String presentations, and type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-button-shape.bsl`.

#### Scenario: A configuration references the button shape

- **WHEN** a configuration module references `ФигураКнопки` or `ButtonShape`
- **THEN** the resolver recognizes the enum root and its measured members without changing the separate button-shape-representation enum

### Requirement: Reached web-service and certificate constructors resolve

The core constructor table SHALL recognize `WSОпределения` / `WSDefinitions` with one through seven arguments, `WSПрокси` / `WSProxy` with four through nine arguments, and `СертификатКриптографии` / `CryptoCertificate` with zero or one argument, as documented by Syntax Assistant 8.3.27.2342. Their names SHALL resolve in dormant configuration methods; invoking them without value models SHALL fail explicitly and catchably.

#### Scenario: A dormant configuration method constructs service values

- **WHEN** an uncalled method contains any of the three constructors with documented arity
- **THEN** semantic resolution accepts the method

#### Scenario: An unsupported constructor is guarded

- **WHEN** any of the three constructors is executed inside `Попытка` / `Try`
- **THEN** its unavailable-value error is caught

### Requirement: Form-data conversion call resolves

The base runtime SHALL recognize `ДанныеФормыВЗначение` / `FormDataToValue` with exactly two arguments, as documented by Syntax Assistant 8.3.27.2342. Until managed-form data conversion is implemented, invoking it SHALL fail explicitly and catchably.

#### Scenario: A dormant method references form-data conversion

- **WHEN** an uncalled configuration method calls the function with two arguments
- **THEN** semantic resolution accepts the method

#### Scenario: Conversion is guarded

- **WHEN** code executes the function inside `Попытка` / `Try`
- **THEN** the unsupported-conversion error is caught

### Requirement: Data-composition filter item group type matches the platform

The base runtime SHALL expose the complete three-member `ТипГруппыЭлементовОтбораКомпоновкиДанных` / `DataCompositionFilterItemsGroupType` enumeration. Its aliases, String presentations, and type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-data-composition-filter-items-group-type.bsl`.

#### Scenario: A configuration references a filter item group type

- **WHEN** configuration code reads a member through either enumeration spelling
- **THEN** the same measured member value and type are returned

### Requirement: Reached key enumeration members match the platform

The base runtime SHALL resolve `Клавиша` / `Key` and the 27 members reached by the UNF configuration: `_1` through `_9`, `Num0` through `Num9`, `NumAdd`, `NumDecimal`, `NumDivide`, `NumMultiply`, `NumSubtract`, `A`, `S`, and `V`. Their aliases, String presentations, and type name SHALL match the direct platform oracle in `tests/conformance/measure/1c/measure-key-reached.bsl`. This requirement does not extend keyboard event handling or the `СочетаниеКлавиш` value model.

#### Scenario: A configuration references a key member

- **WHEN** an uncalled configuration method reads one of the listed members through either root spelling
- **THEN** semantic resolution accepts the member and yields the measured enumeration value

### Requirement: Reached shortcut constructor resolves

The base runtime SHALL recognize `Новый СочетаниеКлавиш` / `New Shortcut` with one through four arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using that shape SHALL compile, while execution without a managed-client shortcut value model SHALL fail explicitly and catchably.

#### Scenario: A dormant method constructs a shortcut

- **WHEN** an uncalled configuration method constructs a shortcut from a key and up to three optional modifier flags
- **THEN** semantic resolution accepts the constructor without creating a fabricated value

### Requirement: Reached control borders resolve

The base runtime SHALL resolve `ТипРамкиЭлементаУправления` / `ControlBorderType` members `БезРамки` / `WithoutBorder`, `Одинарная` / `Single`, and `Подчеркивание` / `Underline` with aliases, String presentations, and type name matching the direct platform oracle in `tests/conformance/measure/1c/measure-control-border-type-reached.bsl`. It SHALL recognize `Новый Рамка` / `New Border` with one or two arguments as documented by Syntax Assistant 8.3.27.2342 and fail explicitly and catchably if construction executes without a Border value model.

#### Scenario: A dormant method builds a border

- **WHEN** an uncalled configuration method constructs a border using any reached border type
- **THEN** the method resolves with the measured enum member and documented constructor arity

### Requirement: Reached color type enum resolves as measured

The base runtime SHALL resolve the five `ВидЦвета` / `ColorType` members reached in the XML-verified UNF source, including their aliases, String presentations, and type name according to the direct platform oracle `tests/conformance/measure/1c/measure-color-type-reached.bsl`.

#### Scenario: A dormant UNF module refers to a color kind

- **WHEN** the module compares a color's `Вид` property with `ВидЦвета.АвтоЦвет`
- **THEN** the name resolves during compilation and the member has the measured platform value

### Requirement: Reached spreadsheet cell line types resolve as measured

The base runtime SHALL resolve the six `ТипЛинииЯчейкиТабличногоДокумента` / `SpreadsheetDocumentCellLineType` members reached in the XML-verified UNF source, including their aliases, String presentations, and type name according to the direct platform oracle `tests/conformance/measure/1c/measure-spreadsheet-cell-line-type-reached.bsl`.

#### Scenario: A dormant UNF module chooses a cell border

- **WHEN** the module reads `ТипЛинииЯчейкиТабличногоДокумента.Сплошная`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: Reached zero-date literal matches the platform

The compiler and runtime SHALL treat the reached BSL literal `'00000000'` and `Дата("00000000")` as the empty Date, according to the direct 1C result recorded in `tests/conformance/measure/1c/measure-zero-date-literal-reached.bsl`. Other invalid civil dates SHALL retain their existing diagnostics.

#### Scenario: A UNF module assigns the zero-date literal

- **WHEN** the module contains `ДатаПоследнейВерсии = '00000000';`
- **THEN** compilation succeeds and the value equals `'00010101'`

### Requirement: Reached data composition details constructor resolves

The base runtime SHALL recognize `Новый ДанныеРасшифровкиКомпоновкиДанных` / `New DataCompositionDetailsData` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Actual construction without a details-data value model SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method prepares report drilldown

- **WHEN** the method constructs `Новый ДанныеРасшифровкиКомпоновкиДанных`
- **THEN** semantic resolution accepts the constructor name and arity

#### Scenario: The constructor is executed without a details-data model

- **WHEN** execution reaches that constructor
- **THEN** it produces a catchable unsupported-operation error

### Requirement: Reached marked-for-deletion search resolves without a false result

The Session library SHALL recognize `НайтиПомеченныеНаУдаление` / `FindMarkedForDeletion` with zero through three optional arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using the omitted-first-argument form SHALL compile. Until a principal-bound scan honoring read rights, RLS, and data separation exists, execution SHALL fail explicitly and catchably rather than return an empty array.

#### Scenario: A dormant UNF method prepares a deletion search

- **WHEN** the method calls `НайтиПомеченныеНаУдаление(, ОтборМетаданных, ИсключенияПоиска)`
- **THEN** semantic resolution accepts the name, omitted argument, and arity

#### Scenario: A search would run without the rights-aware scan

- **WHEN** execution reaches `FindMarkedForDeletion()`
- **THEN** it produces a catchable unsupported-operation error

### Requirement: Reached file-drag mode resolves as measured

The base runtime SHALL resolve both `СпособПеретаскиванияФайлов` / `FileDragMode` members with their aliases, String presentations, and type name matching the direct 1C oracle `tests/conformance/measure/1c/measure-file-drag-mode-reached.bsl`.

#### Scenario: A dormant UNF form helper sets file-drag behavior

- **WHEN** the method reads `СпособПеретаскиванияФайлов.КакСсылкаНаФайл`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: Reached button-group representation resolves as measured

The base runtime SHALL resolve all three `ОтображениеГруппыКнопок` / `ButtonGroupRepresentation` members with their aliases, String presentations, and type name matching the direct 1C oracle `tests/conformance/measure/1c/measure-button-group-representation-reached.bsl`.

#### Scenario: A dormant UNF form helper chooses compact commands

- **WHEN** the method reads `ОтображениеГруппыКнопок.Компактное`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: XML-reached UNF enum members resolve as measured

The base runtime SHALL resolve the 156 distinct enum member expressions recorded in `tests/conformance/measure/1c/measure-unf-reached-enums-batch.bsl`, with their String presentations and type names matching the direct 1C oracle. Russian names and English aliases SHALL refer to the same values. The aliases absent from the installed Syntax Assistant SHALL match the direct alias probe `tests/conformance/measure/1c/measure-unf-enum-alias-gaps.bsl`.

#### Scenario: A dormant UNF module reads a reached form enum

- **WHEN** the module reads `ОтображениеПредупрежденияПриРедактировании.Отображать`
- **THEN** semantic resolution yields the value with the measured type and presentation

#### Scenario: A reached enum is evaluated by its English alias

- **WHEN** the module evaluates `SearchStringLocation.None`
- **THEN** the result equals `ПоложениеСтрокиПоиска.Нет`

### Requirement: Access parameters cannot be fabricated

The Session library SHALL recognize `ПараметрыДоступа` / `AccessParameters` with three required arguments and one optional user-or-role argument, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using the function SHALL compile. Until field-specific principal or role restrictions can be computed from the current Session rights and RLS, execution SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method inspects field access

- **WHEN** the method calls `ПараметрыДоступа("Чтение", ОбъектМетаданных, "Ссылка")`
- **THEN** semantic resolution accepts the documented name and arity

#### Scenario: Rights data is insufficient for an access-parameters result

- **WHEN** execution reaches the function
- **THEN** it produces a catchable error instead of an unrestricted result

### Requirement: Day of year follows the measured platform result

The base runtime SHALL expose `ДеньГода` / `DayOfYear` with one argument. For Date values and the measured 8-, 12-, and 14-digit date strings it SHALL return the one-based ordinal day matching `tests/conformance/measure/1c/measure-day-of-year-reached.bsl`, including day 1 for the empty date. Invalid strings and non-date numbers SHALL fail catchably.

#### Scenario: A dormant UNF module computes the length of a leap year

- **WHEN** the module calls `ДеньГода(Дата(2024, 12, 31))`
- **THEN** the result is `366`

#### Scenario: A string date is accepted through the shared parser

- **WHEN** the module calls `DayOfYear("20241231235959")`
- **THEN** the result is `366`

### Requirement: Value-to-form-data procedure resolves without a fake conversion

The base runtime SHALL recognize `ЗначениеВДанныеФормы` / `ValueToFormData` as a two-argument procedure, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Actual execution without compatible managed-form data values SHALL fail explicitly and catchably rather than report a completed conversion.

#### Scenario: A dormant UNF form helper copies application data

- **WHEN** the method calls `ЗначениеВДанныеФормы(Значение, ДанныеФормы)` as a statement
- **THEN** semantic resolution accepts the name and arity

#### Scenario: The procedure is used as an expression

- **WHEN** a module assigns the result of `ValueToFormData(Значение, ДанныеФормы)`
- **THEN** semantic resolution rejects the procedure-as-function call

### Requirement: Form-data copying cannot report a fabricated result

The base runtime SHALL recognize `КопироватьДанныеФормы` / `CopyFormData` as a two-argument function with a Boolean result, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until compatible managed-form data objects exist, execution SHALL fail explicitly and catchably rather than return either success or incompatibility without a copy.

#### Scenario: A dormant UNF form helper copies two form-data objects

- **WHEN** the method calls `КопироватьДанныеФормы(Источник, Приемник)`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: The host has no managed-form data model

- **WHEN** execution reaches `CopyFormData(Источник, Приемник)`
- **THEN** it produces a catchable unsupported-operation error

### Requirement: Reached Fast Infoset writer constructor resolves

The XML library SHALL recognize `Новый ЗаписьFastInfoset` / `New FastInfosetWriter` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Actual construction without binary Fast Infoset support SHALL fail explicitly and catchably rather than return an XML writer for a different format.

#### Scenario: A dormant UNF method prepares binary XML output

- **WHEN** the method constructs `Новый ЗаписьFastInfoset`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: Binary XML output is not available

- **WHEN** execution reaches `New FastInfosetWriter()`
- **THEN** it produces a catchable unsupported-operation error

### Requirement: Reached Fast Infoset reader constructor resolves

The XML library SHALL recognize `Новый ЧтениеFastInfoset` / `New FastInfosetReader` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Actual construction without binary Fast Infoset decoding SHALL fail explicitly and catchably rather than return an XML reader for a different format.

#### Scenario: A dormant UNF method prepares binary XML input

- **WHEN** the method constructs `Новый ЧтениеFastInfoset`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached global XML read and write calls resolve

The XML library SHALL recognize `ЗаписатьXML` / `WriteXML` as a procedure with the documented two-through-five-argument forms and `ПрочитатьXML` / `ReadXML` as a function with one or two arguments, according to Syntax Assistant 8.3.27.2342. Dormant configuration methods using these calls SHALL compile. Until serialization of their configuration values is supported, execution SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method serializes a value

- **WHEN** the method calls `ЗаписатьXML(Писатель, Значение)` as a statement
- **THEN** semantic resolution accepts the name and arity

#### Scenario: A dormant UNF method deserializes a value

- **WHEN** the method calls `ПрочитатьXML(Читатель)` as an expression
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached query-schema expression constructor resolves

The ORM library SHALL recognize `Новый ВыражениеСхемыЗапроса` / `New QuerySchemaExpression` with one required argument, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using the constructor SHALL compile. Until the query-schema expression object model is supported, execution SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method constructs a query-schema expression

- **WHEN** the method constructs `Новый ВыражениеСхемыЗапроса("&Параметр")`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached object-deletion constructor resolves without a fake deletion

The ORM library SHALL recognize `Новый УдалениеОбъекта` / `New ObjectDeletion` with one required reference argument, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until the database deletion object and its authorized `Write()` operation are supported, construction SHALL fail explicitly and catchably.

#### Scenario: A dormant data-exchange method prepares deletion

- **WHEN** the method constructs `Новый УдалениеОбъекта(Ссылка)`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: A deletion adapter is unavailable

- **WHEN** execution reaches `New ObjectDeletion(Ссылка)`
- **THEN** it produces a catchable error rather than an object that silently skips deletion

### Requirement: Reached global XML readability check resolves

The XML library SHALL recognize global `ВозможностьЧтенияXML` / `CanReadXML` with one XML reader argument, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until the type at the reader's current position can be identified, execution SHALL fail explicitly and catchably rather than return a guessed Boolean.

#### Scenario: A dormant data-exchange method checks XML readability

- **WHEN** the method calls `ВозможностьЧтенияXML(ЧтениеXML)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Hex string conversion follows the measured platform result

The base runtime SHALL expose `ПолучитьДвоичныеДанныеИзHexСтроки` / `GetBinaryDataFromHexString` with one string argument and return `ДвоичныеДанные`. It SHALL discard non-ASCII-hex characters before decoding pairs, return empty binary data when no digits remain, and fail catchably when the remaining number of digits is odd, matching `tests/conformance/measure/1c/measure-binary-data-from-hex-reached.platform.txt`.

#### Scenario: Separators are ignored

- **WHEN** the function receives `"AB CD"`
- **THEN** the returned bytes are `AB CD`

#### Scenario: An odd number of digits remains

- **WHEN** the function receives `"0xFF"`
- **THEN** it fails catchably

#### Scenario: The paired buffer conversion uses the same measured rules

- **WHEN** `ПолучитьБуферДвоичныхДанныхИзHexСтроки("G1G2")` is called
- **THEN** it returns a `БуферДвоичныхДанных` containing the byte `12`

### Requirement: Reached binary configuration metadata constructor resolves

The ORM library SHALL recognize `Новый ОбъектМетаданныхКонфигурация` / `New ConfigurationMetadataObject` with one required `ДвоичныеДанные` argument, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until `.cf` and `.cfe` bytes can be parsed into an independent metadata object, execution SHALL fail explicitly and catchably rather than substitute the current configuration's metadata.

#### Scenario: A dormant service method inspects extension bytes

- **WHEN** the method constructs `Новый ОбъектМетаданныхКонфигурация(Данные)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached type-link constructor resolves

The base runtime SHALL recognize `Новый СвязьПоТипу` / `New TypeLink` with zero through two arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until the target form or metadata path can be represented, actual construction SHALL fail explicitly and catchably rather than create an inert link.

#### Scenario: A dormant property-management method declares a type link

- **WHEN** the method constructs `Новый СвязьПоТипу("Источник.Свойство", 0)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached full-text-search manager global resolves

The Session library SHALL declare `ПолнотекстовыйПоиск` / `FullTextSearch` as a read-only global name, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods referencing it SHALL compile with the existing local and module variable precedence. Until a principal-bound full-text-search adapter is supplied, reading it SHALL fail explicitly and catchably rather than return an empty manager.

#### Scenario: A dormant goods-selection method prepares search

- **WHEN** the method references `ПолнотекстовыйПоиск.СоздатьСписок(...)`
- **THEN** semantic resolution accepts the global name

### Requirement: Server path separator follows the session file system

The Session library SHALL recognize `ПолучитьРазделительПутиСервера` / `GetServerPathSeparator` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. It SHALL return the path separator supplied by the current session's host file system. If no file system is available, the call SHALL fail through the existing missing-capability error.

#### Scenario: A dormant report-delivery method constructs a server path

- **WHEN** the method calls `ПолучитьРазделительПутиСервера()`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: A session calls the function

- **WHEN** the session host file system supplies a path separator
- **THEN** the call returns that separator as a BSL string

### Requirement: Reached FTP connection constructor resolves without a transport

The base runtime SHALL recognize `Новый FTPСоединение` / `New FTPConnection` with one required and up to eight optional arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using the constructor SHALL compile. Until an FTP transport is supplied, actual construction SHALL fail explicitly and catchably rather than return an inert connection.

#### Scenario: A dormant report-delivery method prepares an FTP connection

- **WHEN** the method constructs `Новый FTPСоединение(Сервер, Порт, Логин, Пароль)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached choice-parameter link constructor resolves

The base runtime SHALL recognize `Новый СвязьПараметраВыбора` / `New ChoiceParameterLink` with two required string arguments and one optional linked-value change mode, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using the constructor SHALL compile. Until the managed-form data path and linked-value mode can be represented, actual construction SHALL fail explicitly and catchably rather than return a link that cannot be applied.

#### Scenario: A dormant integration method links a selection filter

- **WHEN** the method constructs `Новый СвязьПараметраВыбора("Отбор.Владелец", ПутьКДанным)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached query builder constructor resolves

The ORM library SHALL recognize `Новый ПостроительЗапроса` / `New QueryBuilder` with zero or one query-text argument, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until the query-builder object model is implemented, actual construction SHALL fail explicitly and catchably rather than return an inert builder or run unfiltered SQL.

#### Scenario: A dormant tax-account method prepares a query builder

- **WHEN** the method constructs `Новый ПостроительЗапроса`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached PDF representation description constructor resolves

The PDF component SHALL recognize zero-argument `Новый ОписаниеОтображаемогоОбъектаPDF` / `New PDFRepresentationObjectDescription`, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until the representation properties are implemented, actual construction SHALL fail explicitly and catchably rather than return an unusable PDF object.

#### Scenario: A dormant file-handling method prepares a PDF representation

- **WHEN** the method constructs `Новый ОписаниеОтображаемогоОбъектаPDF`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached PDF writer constructor resolves

The PDF component SHALL recognize `Новый ЗаписьPDF` / `New PDFWriter` with zero or one argument, as documented by Syntax Assistant 8.3.27.2342 for an uninitialized writer, a file name, or a stream. A dormant configuration method using the constructor SHALL compile. Until the writer's BSL object model is implemented, actual construction SHALL fail explicitly and catchably rather than silently discard PDF output.

#### Scenario: A dormant file-handling method writes PDF to a stream

- **WHEN** the method constructs `Новый ЗаписьPDF(Поток)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached binary-buffer concatenation matches the platform

The base runtime SHALL provide `СоединитьБуферыДвоичныхДанных` / `ConcatBinaryDataBuffers` with one required array of binary-data buffers, an optional separator buffer, and an optional byte order. It SHALL return a new buffer containing the parts in array order with the separator only between adjacent parts. The result SHALL use LittleEndian by default, regardless of source-buffer order, or the explicitly supplied byte order. Invalid separator, part, or byte-order values SHALL fail catchably. These behaviors SHALL match the captured 1C 8.3.27.2342 oracle.
Explicit `Неопределено` for either optional argument SHALL select its default, as measured on the platform.

#### Scenario: A dormant file-handling method joins encoded lines

- **WHEN** the method calls `СоединитьБуферыДвоичныхДанных(СтрокиДвоичныхДанных, Соединитель)`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: Parts, separator, and order match measured output

- **WHEN** the function receives buffers `A1` and `B2` with separator `FF` and BigEndian order
- **THEN** it returns a buffer containing `A1FFB2` with BigEndian order

### Requirement: Reached COM safe-array constructor resolves without COM

The base runtime SHALL recognize `Новый COMSafeArray` with at least one argument and the bytecode-supported maximum arity, reflecting the variable-dimension overloads documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Without a COM host, actual construction SHALL fail explicitly and catchably rather than return an ordinary BSL array pretending to be a COM safe array.

#### Scenario: A dormant OpenOffice conversion method prepares COM arguments

- **WHEN** the method constructs `Новый COMSafeArray("VT_VARIANT", 2)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached licensing-client name function resolves without disclosure

The Session library SHALL recognize zero-argument `ПолучитьИмяКлиентаЛицензирования` / `GetLicensingClientName`, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until an authorized information-base licensing-name adapter is supplied, invocation SHALL fail explicitly and catchably rather than return an empty name or disclose a host-global value across sessions.

#### Scenario: A dormant licensing module reads its client name

- **WHEN** the module calls `ПолучитьИмяКлиентаЛицензирования()`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached configuration ID function resolves without fabrication

The Session library SHALL recognize zero-argument `ПолучитьИдентификаторКонфигурации` / `GetConfigurationID`, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until an authorized platform configuration-ID source is supplied, invocation SHALL fail explicitly and catchably; a metadata GUID, content fingerprint, or arbitrary Base64 value SHALL NOT be substituted for the platform's encrypted identifier.

#### Scenario: A dormant licensing module reads its configuration ID

- **WHEN** the module calls `ПолучитьИдентификаторКонфигурации()`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Licensing-client additional parameter is session-bound

The Session library SHALL recognize zero-argument `ПолучитьДополнительныйПараметрКлиентаЛицензирования` / `GetLicensingClientAdditionalParameter`, as documented by Syntax Assistant 8.3.27.2342. Until an authorized information-base value is supplied for the current session, invocation SHALL fail explicitly and catchably rather than return an empty or process-global value.

#### Scenario: A dormant licensing module reads its additional parameter

- **WHEN** the module calls `ПолучитьДополнительныйПараметрКлиентаЛицензирования()`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Licensing-client settings write is never a no-op

The Session library SHALL recognize `УстановитьНастройкиКлиентаЛицензирования` / `SetLicensingClientParameters` as a procedure with zero through three arguments, as documented by Syntax Assistant 8.3.27.2342. Until an authorized information-base writer is supplied for the current session, invocation SHALL fail explicitly and catchably rather than report success without persisting licensing settings.

#### Scenario: A dormant licensing module writes settings

- **WHEN** the module calls `УстановитьНастройкиКлиентаЛицензирования(Имя, Пароль, Параметр)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Reached global XML type function resolves separately from XMLTypeOf

The XML library SHALL recognize one-argument global `XMLТип` / `XMLType`, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile. Until configuration metadata types can be mapped to their XML data types, invocation SHALL fail explicitly and catchably rather than return `Неопределено` or reuse the value-based `XMLТипЗнч` mapping.

#### Scenario: A dormant document-recognition method inspects a metadata type

- **WHEN** the method calls `XMLТип(ЭтотТипДокумента)`
- **THEN** semantic resolution accepts the name and arity

### Requirement: Available locale codes and presentations require platform data

The Session library SHALL recognize `ПолучитьДопустимыеКодыЛокализации` / `GetAvailableLocaleCodes` with zero arguments and `ПредставлениеКодаЛокализации` / `LocaleCodePresentation` with one argument, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using these calls SHALL compile. Until the host supplies the platform's available locale catalog, execution SHALL fail explicitly and catchably rather than return a guessed list or presentation.

#### Scenario: A dormant UNF method enumerates selectable locales

- **WHEN** the method calls `ПолучитьДопустимыеКодыЛокализации()`
- **THEN** semantic resolution accepts the name and arity

#### Scenario: The platform locale catalog is unavailable

- **WHEN** execution reaches `LocaleCodePresentation("ru_RU")`
- **THEN** it produces a catchable unavailable-data error

### Requirement: Reached HTML reader constructor resolves

The base runtime SHALL recognize `Новый ЧтениеHTML` / `New HTMLReader` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using it SHALL compile; actual construction without HTML parsing support SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method prepares to read HTML

- **WHEN** the method constructs `Новый ЧтениеHTML`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached HTML writer and document batch constructors resolve

The base runtime SHALL recognize `Новый ЗаписьHTML` / `New HTMLWriter` and `Новый ПакетОтображаемыхДокументов` / `New RepresentableDocumentBatch` with zero arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using them SHALL compile. Actual construction while the corresponding output model is absent SHALL fail explicitly and catchably.

#### Scenario: A dormant UNF method prepares document output

- **WHEN** the method constructs either reached type without arguments
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached duplex printing value resolves as measured

The base runtime SHALL resolve `ТипДвустороннейПечати.Нет` / `DuplexPrintingType.None` with its alias, String presentation, and type name matching the direct platform oracle `tests/conformance/measure/1c/measure-duplex-printing-reached.bsl`.

#### Scenario: A dormant UNF method disables duplex printing

- **WHEN** the method reads `ТипДвустороннейПечати.Нет`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: Reached cryptography tools manager resolves

The session library SHALL declare `СредстваКриптографии` / `CryptoTools` as a global property. A dormant configuration method using the manager SHALL compile. Reading it without a cryptography provider SHALL fail explicitly and catchably.

#### Scenario: A dormant method requests crypto module information

- **WHEN** an uncalled UNF method refers to `СредстваКриптографии.ПолучитьИнформациюМодуляКриптографии`
- **THEN** semantic resolution accepts the global manager name

### Requirement: Reached binary-buffer hexadecimal conversion works

The base runtime SHALL provide `ПолучитьHexСтрокуИзБуфераДвоичныхДанных` / `GetHexStringFromBinaryDataBuffer` with one `БуферДвоичныхДанных` argument. It SHALL return the uppercase hexadecimal representation measured by `tests/conformance/measure/1c/measure-hex-buffer-reached.bsl`; a value of another type SHALL cause a catchable type error.

#### Scenario: A configuration formats a binary identifier

- **WHEN** a method passes a binary buffer containing `00`, `AB`, and `FF`
- **THEN** the function returns `00ABFF`

### Requirement: Reached infobase timezone query resolves

The session library SHALL declare `ПолучитьЧасовойПоясИнформационнойБазы` / `GetInfoBaseTimeZone` as a zero-argument function. A dormant configuration method using it SHALL compile. If executed without the infobase timezone setting, it SHALL fail explicitly and catchably rather than return a host or session timezone.

#### Scenario: A dormant UNF method compares timezone settings

- **WHEN** the method calls `ПолучитьЧасовойПоясИнформационнойБазы()`
- **THEN** semantic resolution accepts the documented function

### Requirement: Reached internet mail text processing values resolve

The base runtime SHALL resolve both `ОбработкаТекстаИнтернетПочтовогоСообщения` / `InternetMailTextProcessing` members with aliases, String presentations, and type name matching the direct platform oracle `tests/conformance/measure/1c/measure-internet-mail-text-processing-reached.bsl`.

#### Scenario: A dormant method selects mail text processing

- **WHEN** the method reads `ОбработкаТекстаИнтернетПочтовогоСообщения.НеОбрабатывать`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: Reached cryptography manager constructor resolves

The base runtime SHALL recognize `Новый МенеджерКриптографии` / `New CryptoManager` with zero, one, three, or four arguments, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using any of those forms SHALL compile. Actual construction without a cryptography provider SHALL fail explicitly and catchably; two arguments SHALL remain a semantic error.

#### Scenario: A dormant UNF method creates a cryptography manager

- **WHEN** the method constructs a manager using three module-description arguments
- **THEN** semantic resolution accepts the documented form

### Requirement: Reached certificate store types resolve as measured

The base runtime SHALL resolve the three `ТипХранилищаСертификатовКриптографии` / `CryptoCertificateStoreType` members reached in the XML-verified UNF source, including their aliases, String presentations, and type name according to the direct platform oracle `tests/conformance/measure/1c/measure-crypto-certificate-store-type-reached.bsl`.

#### Scenario: A dormant UNF method selects a certificate store

- **WHEN** the method reads `ТипХранилищаСертификатовКриптографии.КорневыеСертификаты`
- **THEN** semantic resolution yields the measured enumeration value

### Requirement: Reached transferable file description works as measured

The base runtime SHALL construct `ОписаниеПередаваемогоФайла` / `TransferableFileDescription` with zero through two arguments. Its writable `Имя` / `Name` and `Хранение` / `Location` properties SHALL start as empty strings when omitted and SHALL convert numeric assignments to strings. The result SHALL match `tests/conformance/measure/1c/measure-transferable-file-description-reached.platform.txt` from platform 8.3.27.2342.

#### Scenario: Configuration builds a transferable file description

- **WHEN** a configuration constructs the value with a file name and storage location
- **THEN** both properties return those strings
- **AND** later assignments update the same object

### Requirement: Reached word-highlighting function resolves

The base runtime SHALL recognize `СтрНайтиИВыделитьОформлением` / `StrFindAndHighlightByAppearance` with exactly two arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Actual calls SHALL fail explicitly and catchably until the word matching and formatted-string highlighting semantics are implemented.

#### Scenario: Dormant address presentation method highlights a word

- **WHEN** the method calls `СтрНайтиИВыделитьОформлением(Представление, СтрокаПоиска)`
- **THEN** semantic resolution accepts the function name and arity

### Requirement: Reached XSL transformation constructor resolves

The XML component SHALL recognize `Новый ПреобразованиеXSL` / `New XSLTransform` with no arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Actual construction without XSLT support SHALL fail explicitly and catchably.

#### Scenario: Dormant configuration method prepares an XSL transformation

- **WHEN** the method constructs `Новый ПреобразованиеXSL`
- **THEN** semantic resolution accepts the constructor

### Requirement: Reached point-in-time constructor resolves

The base runtime SHALL recognize `Новый МоментВремени` / `New PointInTime` with one required date and one optional reference argument, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using either arity SHALL compile. Actual construction before the point-in-time value and reference ordering are implemented SHALL fail explicitly and catchably.

#### Scenario: Dormant document method prepares a point in time

- **WHEN** the method constructs `Новый МоментВремени(ДатаДокумента, Ссылка)`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached HTTP service response constructor resolves

The HTTP component SHALL recognize `Новый HTTPСервисОтвет` / `New HTTPServiceResponse` with one required status code and up to two optional arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant HTTP-service methods SHALL compile. Actual construction without a service-response value and body API SHALL fail explicitly and catchably.

#### Scenario: Dormant service handler prepares a response

- **WHEN** the method constructs `Новый HTTPСервисОтвет(200)`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached additional authentication settings global resolves

The session library SHALL recognize `ДополнительныеНастройкиАутентификации` / `AdditionalAuthenticationSettings` as a read-only global property. Dormant server methods referring to its manager SHALL compile. Reading the property without authorized infobase settings SHALL fail explicitly and catchably, without exposing or fabricating authentication policy.

#### Scenario: Dormant module reads recovery settings

- **WHEN** the method refers to `ДополнительныеНастройкиАутентификации.ПолучитьНастройкиВосстановленияПароля()`
- **THEN** semantic resolution accepts the global property name

### Requirement: Reached internet mail message flags work as measured

The base runtime SHALL construct `ФлагиИнтернетПочтовогоСообщения` / `InternetMailMessageFlags` with no arguments. All seven properties SHALL initially be `Неопределено`. `Новое` / `New` SHALL be read-only; the other six properties SHALL accept Boolean values and `Неопределено`, and SHALL reject numeric and string assignments. The type presentation and property aliases SHALL match `tests/conformance/measure/1c/measure-internet-mail-message-flags-reached.platform.txt` from platform 8.3.27.2342.

#### Scenario: A configuration prepares IMAP message flags

- **WHEN** it constructs the flags and assigns `Истина` to `Прочитанное`
- **THEN** `Прочитанное` returns `Истина`, while untouched `Новое` remains `Неопределено`

### Requirement: Reached SplitFile function resolves within the State file boundary

The base runtime SHALL recognize `РазделитьФайл` / `SplitFile` with two required arguments and one optional argument, as documented by Syntax Assistant 8.3.27.2342. A dormant configuration method using either form SHALL compile. Until file splitting is implemented through the State file-system capability, actual calls SHALL fail explicitly and catchably without accessing the process file system or reporting a fabricated array of parts.

#### Scenario: Dormant exchange method splits a file

- **WHEN** a configuration method calls `РазделитьФайл(ИмяФайла, РазмерЧасти)`
- **THEN** semantic resolution accepts the function name and arity

#### Scenario: SplitFile is called before file support is available

- **WHEN** a script calls `РазделитьФайл(ИмяФайла, РазмерЧасти, Каталог)`
- **THEN** the call raises a catchable error without creating output parts

### Requirement: Reached TextExtraction constructor resolves

The base runtime SHALL recognize `Новый ИзвлечениеТекста` / `New TextExtraction` with an optional file-name argument, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods SHALL compile. Until a host capability can extract text from arbitrary formats, actual construction SHALL fail explicitly and catchably rather than return fabricated text.

#### Scenario: Dormant file module prepares text extraction

- **WHEN** a configuration method constructs `Новый ИзвлечениеТекста(ПолноеИмяФайла)`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached data-composition filter constructor resolves

The base runtime SHALL recognize `Новый ОтборКомпоновкиДанных` / `New DataCompositionFilter` with no arguments, as documented by Syntax Assistant 8.3.27.2342. Dormant configuration methods using it SHALL compile. Until the mutable data-composition filter model exists, actual construction SHALL fail explicitly and catchably rather than return an empty stand-in.

#### Scenario: Dormant configuration method prepares a filter

- **WHEN** the method constructs `Новый ОтборКомпоновкиДанных`
- **THEN** semantic resolution accepts the constructor name and arity

### Requirement: Reached representable-document batch file type matches the platform

The base runtime SHALL expose the complete system enumeration
`ТипФайлаПакетаОтображаемыхДокументов` /
`RepresentableDocumentBatchFileType` with exactly DOCX, HTML4, HTML5, ODS,
PDF, TXT, XLS, and XLSX. Every member's presentation SHALL be its name and
its value type SHALL be `RepresentableDocumentBatchFileType`, as measured in
`tests/conformance/measure/1c/measure-representable-document-batch-file-type.platform.txt`
on platform 8.3.27.2342. The English enum name SHALL denote the same members.

#### Scenario: A configuration chooses PDF export

- **WHEN** a configuration method reads `ТипФайлаПакетаОтображаемыхДокументов.PDF`
- **THEN** semantic resolution returns the same value as `RepresentableDocumentBatchFileType.PDF`

### Requirement: Reached font type matches the platform

The base runtime SHALL expose the complete system enumeration `ВидШрифта` /
`FontType` with exactly `WindowsШрифт` / `WindowsFont`, `Абсолютный` /
`Absolute`, `АвтоШрифт` / `AutoFont`, and `ЭлементСтиля` / `StyleItem`.
Their presentations SHALL be `Windows font`, `Absolute`, `Auto font`, and
`Style element`, and their value type SHALL be `FontType`, as measured in
`tests/conformance/measure/1c/measure-font-type.platform.txt` on platform
8.3.27.2342.

#### Scenario: A report checks for an absolute font

- **WHEN** a configuration method reads `ВидШрифта.Абсолютный`
- **THEN** semantic resolution returns the same value as `FontType.Absolute`

### Requirement: Reached picture type matches the platform

The base runtime SHALL expose the complete system enumeration `ВидКартинки` /
`PictureType` with exactly `Абсолютная` / `Absolute`, `ИзБиблиотеки` /
`FromLib`, and `Пустая` / `Empty`. Their presentations SHALL be `Absolute`,
`From library`, and `Empty`, and their value type SHALL be `PictureType`, as
measured in `tests/conformance/measure/1c/measure-picture-type.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A picture's empty kind is checked

- **WHEN** a configuration method reads `ВидКартинки.Пустая`
- **THEN** semantic resolution returns the same value as `PictureType.Empty`

### Requirement: Reached full-text-search metadata use matches the platform

The base runtime SHALL expose the complete system enumeration
`ИспользованиеМетаданныхПолнотекстовогоПоиска` / `FullTextSearchMetadataUse`
with exactly `Использовать` / `Use` and `НеИспользовать` / `DontUse`.
Their presentations SHALL be `Use` and `Do not use`, and their value type
SHALL be `FullTextSearchMetadataUse`, as measured in
`tests/conformance/measure/1c/measure-full-text-search-metadata-use.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A full-text-search list disables metadata use

- **WHEN** a configuration method reads `ИспользованиеМетаданныхПолнотекстовогоПоиска.НеИспользовать`
- **THEN** semantic resolution returns the same value as `FullTextSearchMetadataUse.DontUse`

### Requirement: Reached linked-value change mode matches the platform

The base runtime SHALL expose the complete system enumeration
`РежимИзмененияСвязанногоЗначения` / `LinkedValueChangeMode` with exactly
`НеИзменять` / `DontChange` and `Очищать` / `Clear`. Their presentations
SHALL be `Do not change` and `Clear`, and their value type SHALL be
`LinkedValueChangeMode`, as measured in
`tests/conformance/measure/1c/measure-linked-value-change-mode.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A choice link clears its dependent value

- **WHEN** a configuration method reads `РежимИзмененияСвязанногоЗначения.Очищать`
- **THEN** semantic resolution returns the same value as `LinkedValueChangeMode.Clear`

### Requirement: Reached mail-attachment encoding mode matches the platform

The base runtime SHALL expose the complete system enumeration
`СпособКодированияИнтернетПочтовогоВложения` /
`InternetMailAttachmentEncodingMode` with exactly `MIME` / `MIME` and
`UUEncode` / `UUEncode`. Their presentations SHALL be `MIME` and `UUEncode`,
and their value type SHALL be `InternetMailAttachmentEncodingMode`, as
measured in
`tests/conformance/measure/1c/measure-internet-mail-attachment-encoding-mode.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A configuration selects MIME for an attachment

- **WHEN** a configuration method reads `СпособКодированияИнтернетПочтовогоВложения.MIME`
- **THEN** semantic resolution returns the same value as `InternetMailAttachmentEncodingMode.MIME`

### Requirement: Reached data-composition details processor constructor is recognized

The ORM library SHALL recognize `Новый ОбработкаРасшифровкиКомпоновкиДанных`
and `New DataCompositionDetailsProcess` with exactly two arguments, as
measured in
`tests/conformance/measure/1c/measure-data-composition-details-process-constructor.platform.txt`.
It SHALL reject other arities during semantic resolution. Until a
data-composition details processor value is implemented, a valid-arity call
SHALL fail explicitly without returning a fabricated value.

#### Scenario: A dormant report helper is compiled

- **WHEN** a configuration method constructs the processor with details data and an available-settings source
- **THEN** semantic resolution accepts the constructor call
- **AND** executing it without a processor value model returns an explicit unsupported-model error

### Requirement: Reached infobase localization code is supplied by the host

The Session library SHALL expose the zero-argument global function
`КодЛокализацииИнформационнойБазы` / `InfoBaseLocaleCode` returning the
infobase localization code as a string, as measured in
`tests/conformance/measure/1c/measure-infobase-locale-code.platform.txt`.
The value SHALL be supplied explicitly for the current Session and SHALL
remain distinct from `ТекущийКодЛокализации` / `CurrentLocaleCode`. An
actual call without a supplied infobase code SHALL fail explicitly.
The CLI SHALL accept an optional `--infobase-locale-code` only with
`--config-db` and pass it to the Session without guessing a default.

#### Scenario: The infobase and current Session have different locale codes

- **WHEN** the host supplies different infobase and current-session locale codes
- **THEN** each global function returns its corresponding supplied string

#### Scenario: The infobase locale was not supplied

- **WHEN** configuration code calls `InfoBaseLocaleCode()` without that host value
- **THEN** execution returns an explicit missing-host-value error

### Requirement: Reached event-log transaction status matches the platform

The base runtime SHALL expose the complete system enumeration
`СтатусТранзакцииЗаписиЖурналаРегистрации` /
`EventLogEntryTransactionStatus` with exactly `Зафиксирована` / `Committed`,
`НеЗавершена` / `Unfinished`, `НетТранзакции` / `NotApplicable`, and
`Отменена` / `RolledBack`. Their presentations SHALL be `Committed`,
`Not completed`, `No transaction`, and `Rolled back`, respectively, and
their value type SHALL be `EventLogEntryTransactionStatus`, as measured in
`tests/conformance/measure/1c/measure-event-log-entry-transaction-status.platform.txt`
on platform 8.3.27.2342.

#### Scenario: A bank-exchange helper converts a transaction status

- **WHEN** configuration code reads `СтатусТранзакцииЗаписиЖурналаРегистрации.Зафиксирована`
- **THEN** semantic resolution returns the same value as `EventLogEntryTransactionStatus.Committed`

### Requirement: Reached configuration-update description constructor is recognized

The ORM library SHALL recognize `Новый ОписаниеОбновленияКонфигурации` /
`New ConfigurationUpdateDescription` with exactly one argument, as measured
in `tests/conformance/measure/1c/measure-configuration-update-description-constructor.platform.txt`.
It SHALL reject other arities during semantic resolution. Until a CFU
description value is implemented, a valid-arity call SHALL fail explicitly
without returning a fabricated value.

#### Scenario: A dormant update helper is compiled

- **WHEN** a configuration method constructs the description from CFU binary data
- **THEN** semantic resolution accepts the constructor call
- **AND** executing it without a CFU value model returns an explicit unsupported-model error

### Requirement: Acquired subsystem metadata is available during startup

The ORM metadata root SHALL expose `Подсистемы` / `Subsystems` as an immutable collection derived from the acquired configuration. Looking up a subsystem by its folded name SHALL return the corresponding metadata object with its `Имя` / `Name` property and nested `Подсистемы` / `Subsystems` collection. A missing direct property SHALL be an error; `Найти` / `Find` SHALL return `Неопределено`. The adapter SHALL preserve the acquired parent-child hierarchy and SHALL NOT synthesize subsystem names or membership from BSL source.

A subsystem metadata object SHALL expose the acquired read-only boolean `ВключатьВКомандныйИнтерфейс` / `IncludeInCommandInterface`. An absent or unknown serialized value SHALL be a loading error rather than a guessed default.

The existing metadata-object collection SHALL expose zero-argument `Количество()` / `Count()` and return the number of objects in that collection, including the selected level of a nested subsystem collection.

#### Scenario: Startup inspects a nested subsystem

- **WHEN** the acquired configuration contains a parent subsystem and a named child and startup code reads them through `Метаданные.Подсистемы.<parent>.Подсистемы.<child>`
- **THEN** each lookup returns the acquired subsystem metadata object, and its `Имя` is the acquired name

#### Scenario: A subsystem is absent

- **WHEN** startup code calls `Метаданные.Подсистемы.Найти` with a name absent from the collection
- **THEN** the result is `Неопределено` under the existing collection rule

#### Scenario: Startup checks command-interface inclusion

- **WHEN** an acquired subsystem has command-interface inclusion disabled
- **THEN** its metadata object's `ВключатьВКомандныйИнтерфейс` returns `Ложь`

#### Scenario: Startup counts subsystem metadata

- **WHEN** startup code invokes `Количество()` on a subsystem metadata collection with two acquired members
- **THEN** the result is `2`

### Requirement: Information-register record-key signature
The ORM information-register manager SHALL declare `СоздатьКлючЗаписи` and `CreateRecordKey` with one required argument, including in its own manager-module context. Until acquired register-key metadata and native semantics are implemented, execution SHALL return an explicit unsupported-operation error and SHALL NOT fabricate a record key or perform database operations.

#### Scenario: Link original manager source
- **WHEN** an information-register manager module calls `СоздатьКлючЗаписи` with one argument
- **THEN** the existing semantic resolver binds it to the manager receiver method

#### Scenario: Unsupported key creation is explicit
- **WHEN** execution reaches the declared method before record-key construction is implemented
- **THEN** the caller receives the existing runtime error mechanism with an explicit unsupported diagnostic
- **AND** no successful key value or database operation is produced

### Requirement: Accumulation-register record-set signature
The ORM accumulation-register manager SHALL declare `СоздатьНаборЗаписей`
and `CreateRecordSet` with zero arguments, including in its own manager-module
context. Until record-set construction and its required native behavior are
implemented, execution SHALL return an explicit unsupported-operation error.
It SHALL NOT return a fabricated successful record set or perform database
operations.

An existing same-named manager-module export SHALL retain its existing
qualified-call behavior and declared arity.

#### Scenario: Link a dormant update procedure
- **WHEN** an accumulation-register manager module contains an unused procedure
  that calls `СоздатьНаборЗаписей()`
- **THEN** the existing resolver binds the method to that manager receiver

#### Scenario: Execution reaches unsupported record-set creation
- **WHEN** execution invokes the method before record-set behavior is implemented
- **THEN** the caller receives an explicit runtime error
- **AND** no successful value or database operation is produced

#### Scenario: Existing export uses a different arity
- **WHEN** the manager module already exports `СоздатьНаборЗаписей` with
  its own parameter list
- **THEN** a qualified call selects that export and preserves its arity

### Requirement: Host construction of typed result tables
The public Rust value API SHALL allow the host to construct an empty value
table from ordered column names and type descriptions, including repeated
names. The BSL column-addition method SHALL retain its existing duplicate-name
validation. Construction SHALL initialize the table's normal row and schema
state without exposing private runtime types outside the facade.

#### Scenario: Construct a native manager result schema
- **WHEN** the host constructs a table with two typed columns named `Категория`
- **THEN** both columns remain in their supplied order with their supplied types
- **AND** the table can add a row through the existing value API

### Requirement: Acquired accumulation-register catalog settings
The immutable ORM catalog SHALL retain an optional acquired accumulation-register
kind and independently optional complete ordered dimension and resource
collections. Attaching these settings to an absent owner or another metadata
kind SHALL fail. Unknown collections SHALL remain distinct from known empty
collections, and extending a catalog SHALL not mutate a previously cloned metadata object.

#### Scenario: Preserve unknown and empty acquired collections
- **WHEN** the host attaches a known empty dimension collection and an unknown resource collection
- **THEN** catalog access distinguishes the empty dimensions from unknown resources
- **AND** a previously cloned metadata object retains its previous settings

### Requirement: Accumulation-register balance method
The ORM SHALL provide `Остатки` / `Balance` on accumulation-register
managers through the existing manager receiver mechanism, including own
manager-module calls. The method SHALL accept four optional arguments:
moment, filter, dimension names and resource names. It SHALL execute reads
under the current Session principal, rights, RLS and session-parameter
context. Missing data access or unknown required register metadata SHALL
produce an explicit error rather than a fabricated empty successful result.
Existing variable resolution priority SHALL remain unchanged.
Existing same-named manager-module exports SHALL retain their result and arity.

#### Scenario: Preserve an existing manager export
- **GIVEN** a manager module already exports `Остатки` with one required parameter
- **WHEN** the qualified call supplies that parameter
- **THEN** the existing module export executes with its declared arity and result

#### Scenario: Exclude a movement at the date boundary
- **GIVEN** a single incoming movement of 3.25 at 12:00 on 2026-10-06
- **WHEN** the method receives a date at midnight, 11:59:59 or exactly 12:00 that day
- **THEN** the result has no rows
- **WHEN** the date is 12:00:01 or later that day
- **THEN** the result contains the movement's balance of 3.25
- **AND** the date's time is not truncated to midnight

#### Scenario: Include the recorder movement by an explicit boundary
- **GIVEN** one movement with its recorder's point in time
- **WHEN** the method receives that point in time or its exclusive boundary
- **THEN** the recorder's movement is excluded
- **WHEN** the method receives its inclusive boundary
- **THEN** the recorder's movement is included
- **AND** inclusive and exclusive boundaries of the movement date respectively include and exclude that movement

#### Scenario: Read an empty known register
- **GIVEN** an acquired balance register with string dimension `Категория`
  of length 12 and numeric resource `Количество` of precision 15 and scale 2
- **WHEN** the current Session invokes `Остатки()` against a database with no movements
- **THEN** the result is a value table with no rows
- **AND** its columns are `Категория`, `Количество` in metadata order
- **AND** their descriptions are string length 12 and number precision 38, scale 2
- **AND** neither description adds the query's nullable type

#### Scenario: Preserve repeated selected dimensions
- **WHEN** the dimension list is `Категория, Категория`
- **THEN** the result retains two columns both named `Категория`
- **AND** the resource column follows those columns
- **AND** the contract of ordinary query results remains unchanged

#### Scenario: Validate filter and field names
- **WHEN** the filter is an empty structure or a structure with a known dimension
- **THEN** the filter is accepted and its values are applied through query parameters
- **WHEN** the filter is a map or names an unknown dimension
- **THEN** the method fails explicitly
- **WHEN** a selected resource is unknown
- **THEN** the method fails explicitly

#### Scenario: Accept omitted values and case-independent names
- **WHEN** undefined is supplied as moment, filter or dimension list
- **THEN** the method accepts it as an omitted argument
- **WHEN** the existing dimension is named `категория`
- **THEN** the method resolves it without regard to case
- **WHEN** null or a number is supplied as the moment
- **THEN** the method fails explicitly

### Requirement: Reached second-factor authentication template global
The ORM library SHALL resolve `ШаблоныНастроекВторогоФактораАутентификации`
and `SecondAuthenticationFactorSettingsTemplates` using the existing
library-global registry. Local and module variables SHALL retain their
existing priority. Dormant configuration procedures referring to this
platform global SHALL compile. Until a principal-bound platform service
is implemented, evaluating the global SHALL fail explicitly and catchably;
it SHALL NOT return a fabricated empty manager or silently write settings.

#### Scenario: Compile the original dormant template maintenance procedure
- **WHEN** a configuration procedure refers to the global to find or create
  a second-factor settings template
- **THEN** the global name is recognized through the existing component path
- **AND** the procedure can be compiled without executing template operations

#### Scenario: Evaluate without the platform service
- **WHEN** execution evaluates either alias without an implemented service
- **THEN** it receives an explicit catchable runtime error
- **AND** no template values or successful writes are manufactured

#### Scenario: Preserve a module variable with the same name
- **WHEN** a module declares a variable using the global's Russian name
- **THEN** the variable retains its existing priority and value

### Requirement: Reached second-factor authentication setting value
The ORM component SHALL construct `НастройкаВторогоФактораАутентификации`
and `SecondAuthenticationFactorSetting` with zero arguments as an in-memory
value. It SHALL expose writable `ИмяШаблонаНастройки` / `SettingTemplateName`
and `Параметры` / `Parameters`. New values SHALL have an empty name and an
independent empty correspondence. Assigning a correspondence SHALL preserve
its identity and subsequent mutations through either reference. This value
SHALL NOT persist authentication settings or imply support for the separate
platform template service.

#### Scenario: Construct independent setting values
- **WHEN** BSL constructs two values through the Russian and English names
- **THEN** their types agree and the values are distinct
- **AND** each initially has an empty name and empty correspondence
- **AND** modifying the first correspondence does not alter the second

#### Scenario: Retain an assigned correspondence
- **WHEN** BSL assigns a correspondence to `Параметры` and mutates it
- **THEN** those changes are visible through the setting's getter
- **AND** mutations through the getter are visible through the assigned value

#### Scenario: Reject a non-correspondence parameter value
- **WHEN** BSL assigns a structure or number to `Параметры`
- **THEN** the assignment fails

#### Scenario: Clear parameters on a rejected assignment
- **WHEN** a setting with an assigned correspondence receives a structure
  or number as `Параметры` and the script catches the assignment error
- **THEN** the setting's parameters are undefined
- **AND** the formerly assigned correspondence itself retains its contents

#### Scenario: Convert a numeric template name
- **WHEN** BSL assigns a number to `ИмяШаблонаНастройки`
- **THEN** the property holds the ordinary string representation of that number

### Requirement: Reached second-factor settings processing enumeration
The runtime SHALL resolve `ТипОбработкиНастроекВторогоФактораАутентификации` / `SecondAuthenticationFactorSettingsProcessingType` with `НеИспользовать` / `DontUse` and `ИспользоватьСледующуюПриОшибке` / `UseNextIfFailed`. Their string presentations and type name SHALL match the native processing-type fixture. This enumeration SHALL NOT imply support for changing persisted user authentication settings.

#### Scenario: Resolve both aliases and distinct members
- **WHEN** a script reads either member using Russian and English names
- **THEN** the aliases identify the same member
- **AND** the two members are distinct
- **AND** their string presentations and type name match the native receipt

### Requirement: Reached business-process object creation
A business-process manager SHALL expose zero-argument `СоздатьБизнесПроцесс` / `CreateBusinessProcess` through its explicit and own-module receiver contexts. Successful creation SHALL return a distinct, unsaved configuration object with its own module state. For the measured variable-length string-number configuration, initial values SHALL match the native business-process-object fixtures. Creation SHALL NOT persist or start a process. Unknown required metadata SHALL cause an explicit error rather than inferred document settings.

#### Scenario: Create separate unsaved objects
- **WHEN** a supported manager creates two objects using either method alias
- **THEN** both objects have the configuration-specific object type
- **AND** each is new with an empty reference, number and date
- **AND** deletion, started and completed flags are false
- **AND** their additional-property structures are independent

#### Scenario: Resolve the own manager receiver
- **WHEN** its manager module calls `СоздатьБизнесПроцесс()` without qualification
- **THEN** compilation binds the call to that manager's zero-argument method

#### Scenario: Preserve an assigned time
- **WHEN** an unsaved object receives a date including a time
- **THEN** its date retains that time

#### Scenario: Initialize distinct object-module state
- **WHEN** two new business-process objects initialize the measured module
- **THEN** incrementing an exported counter on the first leaves the second unchanged

#### Scenario: Restrict date assignment during the module body
- **WHEN** the measured object-module body reads and attempts to assign Date
- **THEN** reading succeeds and assignment raises a catchable error
- **AND** the completed new object retains its empty date
- **AND** assigning Date after creation succeeds

#### Scenario: Restrict standard property assignments during initialization
- **WHEN** the measured object-module body reads and attempts to assign Number, DeletionMark, Started and Completed
- **THEN** all reads succeed and each assignment raises a catchable error
- **AND** after creation the measured string number and boolean flag assignments succeed
- **AND** local flag assignment does not persist or start the process

### Requirement: Acquired business-process object metadata
The ORM metadata catalog SHALL accept explicitly acquired number settings and complete ordered own attribute and tabular-section descriptions for a business-process owner. Unknown settings and collections SHALL remain distinct from known empty collections. Attaching this metadata SHALL reject missing owners and other metadata kinds, preserve unrelated objects and preexisting metadata-object clones, and SHALL NOT infer values from document metadata.

#### Scenario: Preserve acquired collections and isolation
- **WHEN** a host attaches known empty attributes and unknown tabular sections to a business process while retaining a metadata-object clone
- **THEN** these states remain distinct
- **AND** the retained object clone and unrelated objects remain unchanged

#### Scenario: Reject an invalid owner
- **WHEN** a host attaches business-process metadata to a missing owner or a catalog owner
- **THEN** construction returns an explicit owner error

### Requirement: Configuration-specific business-process object types
Session factories SHALL register business-process object types separately from reference types using `БизнесПроцессОбъект.<Name>` / `BusinessProcessObject.<Name>` aliases. Aliases SHALL identify one type within a configuration; different owners and configuration identities SHALL remain distinct. Registering a type SHALL NOT provide a transferable object-state decoder.

#### Scenario: Resolve object aliases independently of references
- **WHEN** a configured engine evaluates both object aliases and the owner's reference type
- **THEN** the object aliases agree and differ from the reference type

#### Scenario: Keep configuration type identities separate
- **WHEN** catalogs with different configuration identities describe equally named business processes
- **THEN** their object type descriptors remain distinct

### Requirement: Explicit formatting of empty object dates
Formatting an empty date with the reached explicit date patterns SHALL return an empty string, matching the native empty-date-format fixture. Formatting a nonempty date SHALL retain the existing pattern behavior.

#### Scenario: Format an empty date using reached patterns
- **WHEN** a script formats an empty date with the compact timestamp, year, calendar or literal-only native control patterns
- **THEN** each result is empty
- **AND** the nonempty timestamp control retains its date and time

### Requirement: Reached empty-structure business-process filling
The measured unsaved business-process object SHALL accept `Заполнить` / `Fill` with an empty structure when no filling handler is present. Filling SHALL preserve its new status and measured standard property values, including additional properties. Unsupported filling data or handlers SHALL fail explicitly rather than silently report successful filling.

#### Scenario: Fill a fresh empty-collection object
- **WHEN** the measured new business-process object is filled with an empty structure
- **THEN** its date and number remain empty and it remains new

#### Scenario: Preserve changed standard properties
- **WHEN** the native control assigns date, number, flags and additional properties and then fills with an empty structure
- **THEN** all measured assigned values remain unchanged

### Requirement: Measured business-process filling handler invocation
For the measured three-parameter `ОбработкаЗаполнения` handler, Fill SHALL invoke the object's existing module instance with filling data, undefined filling text, and an initially true standard-processing flag. An empty input structure SHALL be passed as undefined; a nonempty structure SHALL retain shared identity. If the handler disables standard processing, Fill SHALL preserve its object and shared-data side effects and skip standard filling. Unsupported handler forms or standard filling paths SHALL fail explicitly.

#### Scenario: Fill with shared nonempty data
- **WHEN** the measured handler mutates the input structure and then replaces its data parameter
- **THEN** the caller observes the mutation but retains its original structure
- **AND** the handler can assign the completed object's date
- **AND** another object's module counter remains unchanged

#### Scenario: Normalize empty filling data
- **WHEN** Fill receives an empty structure with the measured handler
- **THEN** the handler receives undefined data and undefined filling text
- **AND** its standard-processing flag initially is true

#### Scenario: Invoke the same instance repeatedly
- **WHEN** Fill is called twice on the same object
- **THEN** the handler counter advances twice in its existing module state

### Requirement: Standard filling of the measured empty-collection business process
For the measured business process with known empty own collections, standard filling from structure data SHALL leave its standard properties unchanged. This SHALL apply with no handler, with a handler leaving data unchanged, and with the measured handler replacing its data parameter. Read-only property names and incompatible values of standard property names in filling data SHALL not become direct assignments.

#### Scenario: Ignore standard property names in filling data
- **WHEN** the native direct-data control supplies date, number and flags to a fresh or changed object
- **THEN** the object's original standard values remain unchanged

#### Scenario: Retain read-only and incompatible standard values
- **WHEN** the native control supplies Ref, AdditionalProperties and incompatible Date/Number values
- **THEN** filling succeeds without changing the object's additional properties or date

#### Scenario: Preserve values after standard processing remains enabled
- **WHEN** the measured three-parameter handler leaves standard processing enabled
- **THEN** filling completes and retains the native control's standard values

### Requirement: Measured own business-process attributes
The runtime SHALL support explicitly acquired own string attributes of variable length 12 and Boolean attributes in the measured unsaved business-process form. It SHALL expose their initial values during object-module initialization, reject assignment during that phase, and apply existing type adjustment to assignment after creation. Attributes whose acquired FillFromFillingValue is false SHALL retain their values during standard structure filling. Unknown metadata SHALL NOT be treated as an empty collection or a disabled filling flag.

#### Scenario: Native own-field receipts
- **GIVEN** the measured length-11 variable string number, no tabular sections, a variable string-12 attribute and a Boolean attribute with filling disabled
- **WHEN** the own-fields and own-fields-module controls execute
- **THEN** all nine and six respective native output lines match, including truncation, numeric conversions and phase-specific assignment errors

### Requirement: Measured business-process tabular sections
For the measured explicitly acquired business-process form, the runtime SHALL create isolated empty tabular sections and support adding rows, reading row numbers, adjusting the measured string-12 and Boolean fields, numeric indexing, iteration and clearing. Unknown section or row-field metadata SHALL fail explicitly rather than produce an empty schema.

#### Scenario: Complete native section control
- **GIVEN** an acquired business-process section with the two measured ordered primitive fields
- **WHEN** the tabular-section control executes on two new unsaved objects
- **THEN** all eleven native lines match, including isolation and field conversion

#### Scenario: Section access during object-module initialization
- **WHEN** the tabular-section-module control reads, counts, adds and clears through the section in the object module body
- **THEN** those operations succeed, assignment to the section property fails, and all seven native control lines match after creation

#### Scenario: Empty filling preserves existing section rows
- **GIVEN** the measured new object with one assigned section row and no filling handler
- **WHEN** Fill receives an empty structure
- **THEN** row count, values, row number and new-object status remain unchanged as shown by the five-line native control

#### Scenario: A named value table does not fill the measured section
- **GIVEN** no filling handler and a one-element structure naming the measured section with a value table
- **WHEN** Fill is called on a fresh object and an object with one assigned row
- **THEN** all five native lines match: the fresh section stays empty, existing row values remain, and the source table is unchanged

#### Scenario: Filling handler mutates sections and disables standard processing
- **GIVEN** the measured three-parameter filling handler and empty input structure
- **WHEN** the handler adds and assigns one section row and disables standard processing
- **THEN** all eight native lines match, including repeated-call state and isolation of another object

#### Scenario: Standard processing preserves rows added by the measured handler
- **GIVEN** the same measured section handler with standard processing enabled and an empty input structure
- **WHEN** filling is called twice on one object while another stays untouched
- **THEN** all eight native lines match and rows added by the handler remain after standard processing

### Requirement: Business-process section type identity
Business-process sections and rows SHALL have separate configuration-scoped identities and resolve the measured Russian and English aliases through the existing type registry.

#### Scenario: Native aliases distinguish rows and sections
- **WHEN** the tabular-section-types control compares both aliases of the section and row against TypeOf
- **THEN** all five native lines match and row and section identities differ

### Requirement: Acquired business-process checking policy
The catalog SHALL retain a complete ordered checking policy for standard business-process attributes, distinguishing unknown metadata from an acquired empty policy. Existing attribute checking modes SHALL be reused rather than inferred from another owner kind.

#### Scenario: Unknown checking settings
- **GIVEN** missing checking settings for the business-process owner
- **WHEN** host code inspects the policy or requests CheckFilling
- **THEN** unknown settings remain observable and SHALL NOT manufacture successful validation

### Requirement: Measured business-process CheckFilling
The business-process CheckFilling method SHALL accept zero arguments and evaluate acquired checking settings against its own unsaved state. The measured required string behavior SHALL match native results.

#### Scenario: Required string and isolated object state
- **WHEN** the check-filling-required control tests an empty, assigned and cleared required string with the Boolean field checking disabled
- **THEN** all six native lines match, including the untouched second object

#### Scenario: Zero-argument arity
- **WHEN** the check-filling-arity control calls with zero, one and two arguments
- **THEN** zero-argument calls succeed on the measured fully acquired policy and nonzero arity fails as shown by all four native lines

#### Scenario: Measured check message
- **GIVEN** the required string control with English system language and its Russian configuration synonym
- **WHEN** CheckFilling rejects the empty string
- **THEN** one user message has the measured text and attribute field, matching all four native message-control lines

#### Scenario: Checking handler clears the acquired field list
- **GIVEN** the measured required string and two-parameter business-process checking handler
- **WHEN** the handler observes the initially false refusal flag and required-field array, then clears that array
- **THEN** all five native lines match: the field remains in the initial list after assignment, clearing suppresses validation, and another instance is untouched

#### Scenario: Checking handler explicitly refuses after clearing the list
- **GIVEN** the native check-filling-handler-refusal control with a two-parameter handler
- **WHEN** the handler clears the required-field array and sets the refusal flag to true
- **THEN** all five native lines match, including false validation results for both empty and assigned strings and isolated instance observations

#### Scenario: Checking handler replaces its array parameter
- **WHEN** the native check-filling-handler-replace handler assigns a new empty array to its parameter without mutating the original array
- **THEN** validation uses the original required-field list: an empty required string fails, an assigned string succeeds, and all five native lines match

### Requirement: Measured enabled business-process attribute filling
For the measured string-12 and Boolean attributes with acquired FillFromFillingValue=true and known undefined filling defaults, filling without a handler SHALL copy matching structure fields only into unfilled attributes, using existing type adjustment. Already filled attributes SHALL remain unchanged. Undefined input and absent fields SHALL preserve the measured defaults. Unknown filling policy or defaults required by the operation SHALL fail explicitly. For the measured three-parameter handler, standard attribute filling SHALL use its returned data parameter while preserving the caller’s original structure on parameter replacement. Shared structure mutation SHALL remain visible to the caller. Disabling standard processing SHALL retain handler effects and skip standard filling.

#### Scenario: Enabled filling native replay
- **WHEN** fill-enabled runs against the acquired enabled attribute policy without an object handler
- **THEN** all 21 native lines match, including repeated filling, absent fields, undefined data, string truncation and numeric conversion

#### Scenario: Enabled filling after handler
- **WHEN** fill-enabled-handler mutates own attributes, mutates shared data, replaces data or disables standard processing
- **THEN** all 16 native lines match, including replacement data used for the object and unchanged original caller data

#### Scenario: Empty returned filling data
- **WHEN** fill-enabled-handler-empty replaces the handler data parameter with an empty structure or Undefined for fresh and assigned objects
- **THEN** all 16 native lines match: fresh attributes retain their measured empty defaults, assigned attributes retain their values, and the original caller structure remains unchanged

### Requirement: Acquired business-process and accumulation-register runtime metadata
The database configuration adapter SHALL project SDK-acquired business-process number settings, ordered own attributes and tabular sections with their ordered fields, and standard filling-check policy into the existing ORM catalog. It SHALL project acquired accumulation-register kind, ordered dimensions and ordered resources using the existing attribute representation. Known empty collections SHALL remain distinct from unavailable collections. Member types, qualifiers, synonyms, filling flags, filling values and checking policies SHALL use the existing SDK descriptor projection; the adapter SHALL NOT decode Config again or infer missing members.

#### Scenario: Native business-process descriptor
- **WHEN** SDK acquisition supplies the measured enabled-filling business-process descriptor
- **THEN** ORM metadata retains the number settings, ordered attributes, enabled flags, acquired defaults and standard checking policy

#### Scenario: Native accumulation-register descriptors
- **WHEN** SDK acquisition supplies the measured balance and turnover register descriptors
- **THEN** ORM metadata retains each kind, dimension order, resource order and declared type qualifiers
- **AND** the measured empty turnover collections remain known empty

#### Scenario: Unknown collections
- **WHEN** SDK descriptors omit a complete collection
- **THEN** ORM metadata keeps that collection unavailable rather than publishing an empty or partial list

#### Scenario: Acquired qualifiers reach object execution
- **WHEN** the native enabled business-process attributes are projected with their acquired string-12 variable-length qualifiers
- **THEN** the unchanged 21-line native filling control executes through SessionFactory and matches its platform receipt
- **AND** the legacy shorthand qualifiers remain accepted without treating other unmeasured qualifier combinations as supported

#### Scenario: Required-field checking with acquired attributes
- **WHEN** the native required-field business-process metadata is projected with its checking policy and complete acquired string qualifiers
- **THEN** the unchanged six-line required-field control matches the native receipt using explicit configuration and system language resources

### Requirement: Business-process Write declaration and unavailable persistence
The business-process object SHALL declare Записать and Write as a zero-argument method in qualified and own-object-module receiver contexts. Previously callable matching object-module exports SHALL remain callable. Until a real persistence adapter is connected, invocation of the platform method SHALL fail explicitly without changing reference, number, attributes or new-object state, without running write events, and without claiming a database write. This declaration SHALL NOT be treated as implementation of native persistence.

#### Scenario: Unused own-module write call
- **WHEN** a selected business-process object module contains an unused zero-argument bare Записать call
- **THEN** the module compiles through the existing own-receiver method context
- **AND** compilation performs no database operation

#### Scenario: Unavailable persistence
- **WHEN** either method alias is invoked without a connected persistence adapter
- **THEN** invocation returns the explicit unsupported persistence error
- **AND** the object's original reference, number, attributes and new-object state remain unchanged

#### Scenario: Existing exported name
- **WHEN** an object module already exposes an export matching the invoked write name
- **THEN** that export remains callable with its previous behavior

#### Scenario: Excess own-context arguments
- **WHEN** an own-object-module body supplies an argument to either platform write alias
- **THEN** the existing configuration arity policy rejects entry to that method with the ordinary argument-count diagnostic
- **AND** an unused bad method does not prevent valid neighboring methods from running

### Requirement: Measured new business-process Read
For the currently supported unsaved business-process shape, Прочитать and Read SHALL accept zero arguments and reset the measured number, date, Boolean standard fields and own string/Boolean attributes to their measured initial values. The reference SHALL remain empty and the object SHALL remain new. The additional-properties container and object-module state SHALL be preserved. Each tabular-section property SHALL receive a new empty container; aliases to previous sections and their rows SHALL remain usable. Existing matching exported methods SHALL retain their behavior and argument count. Read during object-module initialization SHALL fail explicitly while that phase is unimplemented. This scope SHALL NOT claim stored-object reload without a real persistence adapter.

#### Scenario: Assigned new object native replay
- **WHEN** read-new-assigned and read-new-standard run on SDK-acquired own metadata
- **THEN** all 14 and 22 native lines match, including reset fields and retained additional properties

#### Scenario: Section replacement native replay
- **WHEN** read-new-section runs on the acquired section metadata
- **THEN** all eight native lines match
- **AND** old section aliases retain their rows and additions do not change the object's replacement section

#### Scenario: Object-module state and extra-container identity
- **WHEN** read-new-module runs with its own object module
- **THEN** all five native lines match, including the increasing private counter and mutations through the retained additional-properties alias

#### Scenario: Existing exported name and arity
- **WHEN** an object module already exposes an export matching the invoked read name
- **THEN** that export retains its previous result and arity
- **AND** excess arguments to the platform method follow the existing configuration arity policy


### Requirement: Measured business-process head-task reference
Business-process objects SHALL expose ВедущаяЗадача and HeadTask using the explicitly acquired task-reference type. Valid references of that type SHALL be preserved; the measured undefined and string assignments SHALL produce the typed empty reference. Unsaved Read SHALL reset this property to the typed empty reference. The property SHALL be readable during object-module initialization, while assignment in that phase SHALL fail. Unknown task bindings SHALL NOT be inferred from the source code, names or the presence of a single task in the catalog.

#### Scenario: Assign and reset a task reference
- **GIVEN** the measured business-process and task configuration with an acquired binding
- **WHEN** the initial, non-empty-reference and module-initialization controls are replayed
- **THEN** the directly compared task type, reference values and initialization rules match all native identity-control lines. Original presentation receipts remain retained separately; their text-formatting replay is not claimed.

#### Scenario: Project an explicit storage reference
- **GIVEN** a business-process SchemaStorage HeadTask column with one R target resolving to a task metadata object
- **WHEN** the database adapter constructs the ORM catalog
- **THEN** the configured task identity is preserved without re-parsing Config or extending the SDK.

#### Scenario: Keep unavailable bindings unavailable
- **GIVEN** an absent, ambiguous, malformed or non-task storage target
- **WHEN** head-task metadata is projected
- **THEN** no configured task is invented and a runtime access reports unavailable metadata explicitly.


### Requirement: Task manager reference factories
Task managers SHALL expose ПустаяСсылка/EmptyRef with zero arguments and ПолучитьСсылку/GetRef with zero or one UUID argument through existing configuration reference factories. These methods SHALL produce references of the selected task type without writing a task to the database. Existing matching exported manager methods SHALL remain callable with their existing arity.

#### Scenario: Use task references for a business-process head task
- **GIVEN** a configured task manager
- **WHEN** the native head-task identity controls request empty and explicit-UUID references
- **THEN** the references carry that task type and can be assigned to the bound business process.


### Requirement: Measured business-process modification state
Модифицированность and Modified SHALL accept zero arguments and return the measured per-object modification flag. A new object and an object after unsaved Read SHALL be unmodified. Successful measured standard/own-field assignments, including assignment of the current value, and current-section Add/Clear SHALL mark it modified. Additional properties and non-mutating checking/counting SHALL not change the flag. Successful standard Fill SHALL mark it modified, including the measured empty-data path. A filling handler that disables standard processing without mutating fields SHALL preserve the flag. Old section and row aliases retained before Read SHALL not modify the current object's flag. The method SHALL fail during object-module initialization. Existing same-name exports SHALL retain their behavior and arity.

#### Scenario: Replay field and filling state
- **WHEN** the 21-line native field/control script executes against acquired metadata
- **THEN** every modification flag matches the platform receipt.

#### Scenario: Detach previous section state
- **WHEN** the ten-line native section script retains aliases across Read and mutates both generations
- **THEN** only the current generation changes the current object's flag.

#### Scenario: Check own context and cancellation
- **WHEN** the four-line native module script calls Modified in initialization, own context, cancelled filling and with an extra argument
- **THEN** all native results match and no synthetic modification is introduced by cancelled standard processing.


#### Scenario: Preserve independent objects and accepted conversions
- **WHEN** the seven-line native conversion/isolation control assigns an array to the own Boolean field, fills from the measured structure, assigns the empty head-task reference and creates a second object
- **THEN** actual accepted conversions are preserved, the first object is modified and the second remains unmodified.


### Requirement: Measured business-process data exchange parameters
Business-process objects SHALL expose read-only ОбменДанными/DataExchange using the existing data-exchange parameter object. Load SHALL initially be false and Recipients.AutoFill true. Parameter mutations SHALL not mark the business process modified, SHALL survive Read and SHALL remain visible through previously obtained aliases. Another business-process object SHALL have independent defaults. Access during object-module initialization SHALL fail as measured.

#### Scenario: Preserve parameter state across Read
- **WHEN** the eleven-line native qualified-object control executes
- **THEN** both aliases, parameter defaults, Read persistence, isolation and read-only behavior match its native receipt.

#### Scenario: Reject initialization access
- **WHEN** the four-line separated phase control tries to read and write Load and Recipients.AutoFill before selection
- **THEN** all initialization attempts are rejected and the selected object retains its defaults and unmodified state.


### Requirement: Measured business-process pending reference
Business-process objects SHALL expose ПолучитьСсылкуНового/GetNewObjectRef with zero arguments and УстановитьСсылкуНового/SetNewObjectRef with one argument. The pending reference SHALL initially be an empty reference of the business-process type. Assignment of a valid reference SHALL replace only the pending reference, without changing Ref or Modified. Read SHALL preserve it. Empty references of the owner type SHALL be accepted; the measured undefined value, task reference, string and number SHALL be rejected without replacing the previous reference. Both methods SHALL reject calls during object-module initialization. Existing matching exports SHALL retain their behavior and arity.

#### Scenario: Preserve a pending reference across Read
- **WHEN** the thirteen-line native qualified control assigns, reads and rejects measured values
- **THEN** the pending reference, actual Ref, modification flag, arity and object isolation match its native receipt.

#### Scenario: Use methods in own context after initialization
- **WHEN** the six-line native module control invokes both methods before and after selection
- **THEN** initialization calls are rejected and later own-context calls preserve the assigned reference across Read.

#### Scenario: Reject incompatible input without losing the pending reference
- **WHEN** the nine-line native invalid-input/factory control assigns a task reference, string and number
- **THEN** each rejected input leaves the previous pending reference unchanged and manager reference factory results match the native receipt.

### Requirement: Business-process manager reference factories
Business-process managers SHALL expose ПустаяСсылка/EmptyRef with zero arguments and ПолучитьСсылку/GetRef with zero or one UUID argument through the existing configuration-reference factories, without persisting an object. Matching existing manager exports SHALL remain callable with their original arity.

#### Scenario: Create references for a pending business process
- **WHEN** the native pending-reference controls request empty, explicit-UUID and generated references
- **THEN** they receive references of the selected business-process type and the existing object remains unsaved.


### Requirement: DeleteObjects resolves with explicit read-only rejection

The ORM library SHALL resolve the documented global procedure УдалитьОбъекты/DeleteObjects with one through six arguments. In the current read-only host, every invocation SHALL raise an explicit catchable unavailable-write error without changing supplied values or deleting data. This host policy SHALL NOT be presented as native empty-list behavior or as implemented deletion support.

#### Scenario: Existing configuration calls deletion
- **WHEN** a module contains a two- or three-argument call to either alias
- **THEN** semantic resolution accepts the declared procedure
- **AND** execution in the read-only host raises the explicit unavailable-write error.

#### Scenario: Caller retains output and input
- **WHEN** a caller supplies an array and a populated output table and catches the read-only rejection
- **THEN** both values retain their contents.

#### Scenario: Incorrect argument count
- **WHEN** a module supplies zero or seven arguments
- **THEN** semantic resolution reports an argument-count error.


### Requirement: Empty graphical schema construction

The existing spreadsheet component SHALL provide zero-argument ГрафическаяСхема/GraphicalSchema construction. Each construction SHALL produce a distinct empty schema. Its Russian type and value presentations SHALL match the native Russian-locale control. The read-only ЭлементыГрафическойСхемы/GraphicalSchemaItems property SHALL return an empty graphical-schema-items collection with measured type and value presentations, zero Count/Количество, empty enumeration and undefined Найти/Find for a missing string name. Repeated property getters SHALL compare unequal as measured. This initial empty-state implementation SHALL NOT manufacture populated route maps or claim file, print or item-editing support.

#### Scenario: Native empty-schema control
- **WHEN** the eleven-line empty graphical-schema control is replayed through the facade
- **THEN** it matches the retained Russian-locale native receipt, including distinct collection wrappers and read-only rejection.

#### Scenario: Constructor argument is supplied
- **WHEN** either constructor alias receives an argument
- **THEN** semantic resolution rejects the argument count.


### Requirement: Measured empty graphical-schema lookup rejection

Graphical-schema-items Найти/Find SHALL require exactly one argument as measured on 8.3.27.2342, despite the installed help marking the argument optional. Omitted arguments SHALL fail the argument-count check; undefined and numeric inputs SHALL fail the type check. Failed calls SHALL preserve the assignment target.

#### Scenario: Native invalid lookup control
- **WHEN** the retained invalid-lookup control is replayed
- **THEN** its six status/value lines match the native receipt
- **AND** native context-dependent diagnostic lines remain retained as evidence without a claim of identical runtime wording.


### Requirement: Graphical-schema item picture location enumeration

The base runtime SHALL expose the complete measured ПоложениеКартинкиЭлементаГрафическойСхемы/GraphicalSchemaItemPictureLocation enumeration with Верх/Top, Лево/Left, Низ/Bottom, Право/Right and Центр/Center. Member presentations, member type and bare enumeration presentation/metatype SHALL match the retained Russian-locale native receipt. Names SHALL use the existing folded enumeration lookup.

#### Scenario: Full native picture-location control
- **WHEN** the fourteen-line picture-location script executes through the facade
- **THEN** all lines match the native receipt.

#### Scenario: Folded names and missing member
- **WHEN** code uses another case for the enumeration or member name
- **THEN** it resolves to the same member
- **AND** an undeclared member remains a semantic error.


### Requirement: Graphical-schema shape enumeration

The base runtime SHALL expose the complete measured ФигурыГрафическойСхемы/GraphicalSchemaShapes enumeration: Блок/Block, Документ/Document, Нет/None, Папка/Folder, СкобкиВертикальные/VerticalBrackets, СкобкиГоризонтальные/HorizontalBrackets, СтрелкаВверх/UpArrow, СтрелкаВверхВниз/UpDownArrow, СтрелкаВлево/LeftArrow, СтрелкаВлевоВправо/LeftRightArrow, СтрелкаВниз/DownArrow, СтрелкаВправо/RightArrow, Файл/File and Эллипс/Ellipse. Member presentations, member type, aliases and bare enumeration presentation/metatype SHALL match the native Russian-locale receipt. Resolution SHALL use the existing folded lookup.

#### Scenario: Complete shape control
- **WHEN** the thirty-two-line native shape script executes through the facade
- **THEN** every line matches the retained receipt, including spaced and hyphenated presentations.

#### Scenario: Folded lookup and unknown shape
- **WHEN** a shape name uses another case
- **THEN** it resolves to the same measured member
- **AND** an undeclared member remains a semantic error.


### Requirement: Graphical-schema line and arrow enumerations

The base runtime SHALL expose the complete measured ТипСоединительнойЛинии/ConnectorLineType set (НетЛинии/None, Пунктир/Dashed, ПунктирТочка/DashDotted, ПунктирТочкаТочка/DashDottedDotted, Сплошная/Solid, Точечная/Dotted), ПоложениеТекстаСоединительнойЛинии/ConnectorTextLocation set (ПервыйСегмент/FirstSegment, СерединаЛинии/Middle), and СтильСтрелки/ArrowStyle set (Заполненная/Filled, Незаполненная/Blank, Нет/None). All member presentations, aliases, member types and bare presentation/metatype SHALL match the native Russian-locale receipt through existing folded enumeration resolution.

#### Scenario: Complete line and arrow control
- **WHEN** the thirty-four-line script executes through the facade
- **THEN** every line matches the native receipt, including the non-derived dash-dotted presentation.


### Requirement: New characteristic item predefined flag

A newly created chart-of-characteristic-types object SHALL expose read-only Предопределенный/Predefined as Boolean false. Both names SHALL be available during object-module initialization and through qualified and selected own-module access. Attempts to assign the property SHALL fail without changing its value. This measured new-item behavior SHALL NOT imply loading an existing predefined element.

#### Scenario: Complete native predefined control
- **WHEN** the nine-line characteristic predefined control executes with its original object module
- **THEN** all lines match the native receipt, including initialization reads and rejected writes.

#### Scenario: Separate new elements
- **WHEN** another new item is constructed
- **THEN** its flag remains false regardless of rejected writes to the previous item.

### Requirement: Nonperiodic information-register direct read

The Rust catalog API SHALL accept explicit optional periodicity and complete optional dimension/resource lists for an information register. Missing declarations SHALL remain distinguishable from known empty lists. Attaching these declarations to an absent owner or another metadata kind SHALL fail.

The configuration runtime SHALL expose Получить/Get on independent nonperiodic information-register managers in qualified and own-manager contexts. It SHALL read through the current session's data adapter and access restrictions using the complete acquired dimension and resource declarations. The result SHALL contain resources only, with typed empty values when no record matches. Unknown or incomplete declarations SHALL fail explicitly rather than return a fabricated empty result. Dimension names SHALL be case-insensitive; extra filter keys, including resource names, SHALL be rejected.

#### Scenario: Empty measured register
- **WHEN** the measured independent register with one numeric dimension and one numeric resource is read with a complete filter and no matching record
- **THEN** the result is a structure containing only the numeric resource with value zero
- **AND** omitted and empty filters are rejected.

#### Scenario: Existing measured record
- **WHEN** the native nonempty control is replayed against equivalent acquired declarations and stored data
- **THEN** numeric 7, string "7", the English method and a differently cased dimension name retrieve the measured resource value 13
- **AND** the remaining accepted and rejected filters agree with the saved receipt without converting a rejected read into empty success.

#### Scenario: Numeric dimension conversion
- **GIVEN** the measured Number(5,0) dimension has distinct records at keys 0, 1 and 7
- **WHEN** Get receives Undefined, False, an empty string or an invalid numeric string as its dimension value
- **THEN** it retrieves the record at key 0
- **AND** True and 0.6 retrieve key 1, while number 7 and string "7" retrieve key 7.

### Requirement: Exact-period information-register direct read

For the measured independent Day-periodic register, Получить/Get SHALL accept a date followed by the complete dimension filter and read the exact stored period. It SHALL NOT round the read argument to the register periodicity. An invalid period or missing dimension filter SHALL fail instead of being treated as an unrestricted read.

#### Scenario: Day-periodic exact key
- **GIVEN** the native control stores resource 42 at the period 2024-01-02 00:00:00
- **WHEN** Get reads that exact period with the complete filter
- **THEN** it retrieves 42
- **AND** 12:34:56 and 23:59:59 of the same day, the previous day and the next day return the empty numeric resource zero.

### Requirement: Session-bound constant write capability

Constant managers SHALL expose Установить/Set with exactly one argument in qualified and own-manager contexts. The runtime SHALL delegate the metadata, supplied value and current frozen principal to the current Session's data adapter. An adapter without constant-write capability SHALL fail explicitly; the runtime SHALL NOT store a successful replacement in a process-wide or Session-local cache. The Rust DataAccess API SHALL provide an optional constant-write operation whose default implementation rejects the operation. An existing same-named manager export SHALL retain its result and arity.

#### Scenario: Explicit host writer
- **GIVEN** the current Session has a constant-write adapter
- **WHEN** a constant manager calls Set with one value
- **THEN** that adapter receives the exact constant, value and principal
- **AND** success is returned only after the adapter succeeds.

#### Scenario: Read-only source
- **GIVEN** the current Session's data adapter does not implement constant writing
- **WHEN** Set is executed
- **THEN** the call fails explicitly and does not change the value returned by Get.

### Requirement: Shared point-in-time value

The base runtime SHALL provide the registered МоментВремени/PointInTime constructor and immutable Date/Дата and Ref/Ссылка properties. A date or measured compact date string SHALL initialize the date; an omitted or Undefined reference SHALL remain Undefined, distinct from a typed empty reference. Compare/Сравнить SHALL compare dates first and supported reference keys second, with physical UUID order for the same reference type and acquired storage numbers for measured document reference types. Equality SHALL be by value; primitive less-than SHALL remain rejected. A missing cross-type ordering key SHALL fail explicitly. ObjectProtocol SHALL offer an optional host reference-order key without importing ORM or database types into the runtime; the facade SHALL expose the required API.

#### Scenario: Constructor and immutable date
- **WHEN** a date-only moment is constructed with a date or the measured compact string
- **THEN** Date equals that date and Ref is Undefined
- **AND** attempts to assign Date fail without changing it.

#### Scenario: Equality and Compare
- **WHEN** two moments have identical date and reference values
- **THEN** equality is true and Compare returns zero
- **AND** a later date compares greater, while using less-than on moments fails.

### Requirement: Archive file path restoration enumeration

The runtime SHALL expose РежимВосстановленияПутейФайлаАрхива/ArchiveFileRestoreFilePathsMode with exactly Восстанавливать/Restore and НеВосстанавливать/DontRestore. Their native presentations SHALL be «Восстанавливать пути» and «Не восстанавливать пути». The member type presentation SHALL be ArchiveFileRestoreFilePathsMode and the bare-name type presentation SHALL be EnumArchiveFileRestoreFilePathsMode. Both language spellings SHALL resolve case-insensitively. The new enumeration and its members SHALL remain distinct from РежимВосстановленияПутейФайловZIP/ZIPRestoreFilePathsMode; existing ZIP values SHALL retain their behavior.

#### Scenario: Native archive enumeration identity
- **WHEN** Restore is obtained through either language spelling
- **THEN** the values are equal and have the native presentation and type
- **AND** Restore is not equal to the legacy ZIP Restore value.

#### Scenario: Closed member set
- **WHEN** an unknown archive restoration member is requested dynamically
- **THEN** the expression fails rather than creating another value.

### Requirement: Own data processor instances

DataProcessor managers SHALL expose zero-argument Создать/Create in qualified and own-manager contexts. With complete acquired object metadata, each call SHALL create a distinct configuration object instance, initialize its own object module before use and preserve exported methods and variables through the existing configuration-instance mechanism. The object's metadata SHALL identify its owner, and ЭтотОбъект in its module SHALL denote that instance. Unknown attribute or tabular-section metadata SHALL NOT be represented as a verified empty collection. This requirement does not imply form opening or external processor loading.

#### Scenario: Independent initialized object modules
- **GIVEN** the measured own data processor initializes an exported counter to 7
- **WHEN** two instances are created and the first counter is incremented to 8
- **THEN** the second counter remains 7
- **AND** reading and assigning the exported variable use the corresponding instance.

#### Scenario: Own manager receiver
- **WHEN** an exported manager function calls bare Создать()
- **THEN** it returns a new initialized object of that manager's acquired owner
- **AND** the object's own module sees that object through ЭтотОбъект.


### Requirement: Data processor own fields and tabular sections

Complete acquired data processor attributes and tabular sections SHALL be
represented separately from unknown metadata in the public Rust metadata API.
Creation SHALL initialize measured String/Boolean attributes and section rows
through existing typed-value storage. Values and rows written by the object
module body SHALL remain present after creation. Each instance SHALL own its
values and rows; measured string length qualifiers SHALL apply to attribute
and row assignment. The same existing tabular-section implementation SHALL
serve supported processor sections without changing document behavior.

#### Scenario: Module body keeps own data
- **GIVEN** the measured own module sets text to body, flag to true and adds one row
- **WHEN** Create returns
- **THEN** these values and the row remain present
- **AND** another created instance retains its independent body values.

#### Scenario: Measured string qualifiers
- **GIVEN** an attribute and section field have variable string length 12
- **WHEN** abcdefghijklmnop is assigned
- **THEN** both contain abcdefghijkl
- **AND** a new row starts with empty text and false Boolean.

### Requirement: Exported variables of a configuration instance

CallContext SHALL provide read and write access to exported module variables
of an existing ConfigurationInstance through the current VM service. Names
SHALL resolve case-insensitively; non-exported or unknown variables SHALL fail.
The operation SHALL validate the current Session owner, module identity and
initialized instance before accessing its existing module slots. It SHALL NOT
create a second variable store or execute a synthetic BSL accessor.

#### Scenario: Exported processor counter
- **WHEN** a processor's exported counter is assigned through the object
- **THEN** its exported read function observes the new value
- **AND** another instance retains its own counter.

#### Scenario: Foreign Session memory
- **WHEN** another Session attempts to read or write that instance's exported variable
- **THEN** access fails and the original Session's variable remains unchanged.


### Requirement: Acquired reference type presentation
The ORM metadata host API SHALL allow an explicit acquired presentation for a
configured reference type. The presentation SHALL affect type-to-string output
without changing reference identity or Russian and English qualified lookup
names. Existing objects without this explicit value SHALL retain their current
presentation. The task HeadTask native receipts SHALL remain unchanged.

#### Scenario: Task synonym differs from its identifier
- **GIVEN** a task named ЗамерЗадачи with the acquired presentation Замер задачи
- **WHEN** a business process HeadTask reference type is formatted
- **THEN** the original native receipt displays Замер задачи
- **AND** ЗадачаСсылка.ЗамерЗадачи and TaskRef.ЗамерЗадачи resolve to the same type

#### Scenario: No explicit presentation supplied
- **GIVEN** metadata constructed through the existing host API without an explicit presentation
- **WHEN** its configured reference type is formatted
- **THEN** the existing metadata-name presentation is preserved


#### Scenario: Task presentation from acquired configuration language
- **GIVEN** acquired task synonyms and the configuration primary language code
- **WHEN** the database loader builds reference types
- **THEN** it supplies the synonym matching that language as the task type presentation
- **AND** it retains the metadata name when no matching synonym exists
- **AND** it does not choose a synonym from a different language


### Requirement: Configuration primary language metadata
The configuration metadata object SHALL expose its acquired primary language
through the read-only ОсновнойЯзык / DefaultLanguage property. The returned
language metadata object SHALL retain its acquired language code. Missing host
metadata SHALL fail explicitly rather than fabricate a language.

#### Scenario: Read acquired primary language
- **GIVEN** a configuration bound to an acquired language metadata object
- **WHEN** BSL reads the primary language and its language code
- **THEN** the result matches the native language-selection control


### Requirement: Data processor object template method declaration

A data processor object SHALL declare ПолучитьМакет/GetTemplate with exactly one required argument in both qualified and own-object module contexts. Until template resources are acquired and implemented, invocation SHALL fail explicitly as unsupported instead of returning fabricated template data. This declaration SHALL NOT introduce a global function outside an object context. Existing same-named object-module exports SHALL retain their previous dispatch path and arity.

#### Scenario: Unexecuted original template call
- **WHEN** an object-module procedure contains ПолучитьМакет("Template") without being invoked
- **THEN** compilation succeeds through the existing object receiver method table.

#### Scenario: Template acquisition is unavailable
- **WHEN** either alias is called on a created processor object
- **THEN** invocation reports unsupported template acquisition rather than a missing method or an empty value.

#### Scenario: Argument count
- **WHEN** an own-object call supplies zero or two arguments
- **THEN** the existing semantic arity check rejects it when that body is used, retaining the existing deferred method-arity behavior.


### Requirement: PDF reader constructor declaration

The PDF component SHALL recognize Новый ЧтениеPDF/New PDFReader with zero or one argument, matching the uninitialized, file-name and stream constructor signatures documented in Syntax Assistant 8.3.27.2342. Dormant configuration code SHALL compile. Until the PDFReader object model is implemented, construction SHALL fail explicitly and catchably rather than fabricate a readable document.

#### Scenario: Dormant PDF reader construction
- **WHEN** a configuration method contains Новый ЧтениеPDF(Поток) without executing it
- **THEN** the existing constructor resolver accepts the declaration.

#### Scenario: Explicit unsupported reader
- **WHEN** either alias is constructed with zero or one argument
- **THEN** the operation reports unsupported PDFReader construction and can be caught by BSL exception handling.

#### Scenario: Excess arguments
- **WHEN** PDFReader is constructed with two arguments
- **THEN** the existing constructor arity check rejects it.


### Requirement: Information register slice method declarations

Information register managers SHALL declare СрезПервых/SliceFirst and СрезПоследних/SliceLast with zero to two arguments in qualified and own-manager contexts, matching Syntax Assistant 8.3.27.2342. Until acquired periodic-register metadata and measured slice execution are implemented, invocation SHALL fail explicitly as unsupported instead of fabricating an empty value table. Existing manager-module exports with the same name SHALL retain their previous dispatch path.

#### Scenario: Dormant first-slice call
- **WHEN** a manager module contains СрезПервых(, Отбор) in an unexecuted procedure
- **THEN** the existing receiver-method mechanism resolves it without a new global function.

#### Scenario: Unsupported slice execution
- **WHEN** either alias is invoked with zero, one or two arguments
- **THEN** invocation reports unsupported slice execution and does not return successful fabricated data.

#### Scenario: Existing manager export
- **GIVEN** an existing manager module exports a same-named three-argument function
- **WHEN** configuration code calls that export
- **THEN** the existing export and its arity remain usable.

#### Scenario: Excess slice arguments
- **WHEN** an own-manager platform call supplies three arguments without a same-named export
- **THEN** the existing deferred arity error is preserved.


#### Scenario: Last-slice declaration
- **WHEN** a periodic-register manager module contains СрезПоследних() or SliceLast(Период, Отбор)
- **THEN** the existing receiver table resolves the declaration
- **AND** execution fails explicitly until last-slice acquisition and data semantics are implemented.


### Requirement: Constant value-manager method declaration

Constant managers SHALL declare СоздатьМенеджерЗначения/CreateValueManager with zero arguments in qualified and own-manager contexts, matching Syntax Assistant 8.3.27.2342. Until the value-manager object and its measured operations are implemented, invocation SHALL explicitly report unsupported construction. Existing same-named manager-module exports SHALL preserve their dispatch and arity.

#### Scenario: Dormant own-manager call
- **WHEN** a constant manager contains an unexecuted procedure calling СоздатьМенеджерЗначения()
- **THEN** the existing receiver mechanism resolves the method without adding a global function.

#### Scenario: Explicit unsupported construction
- **WHEN** either alias is invoked without arguments
- **THEN** execution reports unsupported value-manager construction instead of returning a fabricated object.

#### Scenario: Existing export and arity
- **GIVEN** a constant manager exports a same-named one-argument function
- **WHEN** that export is called
- **THEN** its existing behavior and arity are preserved.

#### Scenario: Excess platform arguments
- **WHEN** a platform call supplies one argument without a same-named export
- **THEN** the existing arity check rejects the call.


### Requirement: Catalog object lock declaration

Catalog objects SHALL declare Заблокировать/Lock as a zero-argument procedure in qualified and own-object contexts, matching Syntax Assistant 8.3.27.2342. Until native-compatible object locking is connected, execution SHALL explicitly fail as unsupported without reporting a successful lock. Existing same-named object-module exports SHALL preserve their dispatch and arity.

#### Scenario: Dormant own-object lock
- **WHEN** an object module contains an unexecuted procedure calling Заблокировать()
- **THEN** the existing object receiver resolves the method without a new global function.

#### Scenario: Unsupported lock
- **WHEN** either alias is invoked without arguments
- **THEN** execution reports unsupported object locking and does not establish a local substitute lock.

#### Scenario: Existing export
- **GIVEN** an object module exports a same-named one-argument function
- **WHEN** configuration code calls that function
- **THEN** its existing behavior and arity remain available.

#### Scenario: Excess lock argument
- **WHEN** a qualified platform lock call supplies one argument without a same-named export
- **THEN** the existing runtime method arity check rejects it.


### Requirement: Information register selection declaration

Information register managers SHALL declare Выбрать/Select in qualified and own-manager contexts with a declaration-only argument range of zero to four, covering the periodic and nonperiodic Syntax Assistant 8.3.27.2342 signatures. Until acquired periodicity and native-measured selection execution are implemented, invocation SHALL explicitly fail as unsupported. The declaration SHALL NOT claim that four arguments are valid for a nonperiodic register. Existing same-named exports SHALL preserve their dispatch and arity.

#### Scenario: Dormant own-manager selection
- **WHEN** a manager module contains an unexecuted call Выбрать(,,Отбор,Порядок)
- **THEN** the existing receiver mechanism resolves it without a new global function.

#### Scenario: Unsupported selection execution
- **WHEN** either alias is invoked with zero through four arguments
- **THEN** execution explicitly reports unsupported selection without returning fabricated rows.

#### Scenario: Existing export
- **GIVEN** a manager exports a same-named five-argument function
- **WHEN** configuration code calls that export
- **THEN** its prior dispatch and arity remain available.

#### Scenario: Excess declaration arguments
- **WHEN** an own-manager call supplies five arguments without a same-named export
- **THEN** the existing deferred arity check rejects it.


### Requirement: Report manager template declaration

Report managers SHALL declare ПолучитьМакет/GetTemplate with exactly one argument in qualified and own-manager contexts, matching Syntax Assistant 8.3.27.2342. Until template resources are acquired and supported, invocation SHALL explicitly fail as unsupported without returning fabricated templates. Existing same-named exports and variable lookup SHALL preserve their behavior.

#### Scenario: Own-manager template call
- **WHEN** a report manager contains an unexecuted call ПолучитьМакет("Основной")
- **THEN** the existing receiver mechanism resolves the method without adding a global function.

#### Scenario: Unsupported acquisition
- **WHEN** either alias is invoked with one argument
- **THEN** execution reports unsupported report template acquisition.

#### Scenario: Existing export and variables
- **GIVEN** an existing report manager declares a same-named variable or exports a same-named function with a different arity
- **WHEN** code accesses the variable or calls the export
- **THEN** the previous value, dispatch and arity remain unchanged.

#### Scenario: Invalid platform arity
- **WHEN** an own-manager platform call supplies zero or two arguments without a same-named export
- **THEN** the existing deferred arity check rejects the call.


### Requirement: Data composition period type enumeration

The runtime SHALL expose ТипПериодаКомпоновкиДанных/DataCompositionPeriodType with Основной/Main and Дополнительный/Additional through the existing system enumeration tables. Aliases, value and type presentations, type lookup and equality SHALL match the retained native 8.3.27.2342 control. This enumeration SHALL remain distinct from DataCompositionPeriodAdditionType.

#### Scenario: Native enumeration replay
- **WHEN** the retained control script runs through the facade
- **THEN** its output matches the real platform receipt for both members, aliases, presentations and type identity.

#### Scenario: Distinct enum kinds
- **WHEN** period type and period addition type values are compared by type
- **THEN** their types remain distinct.


### Requirement: Data composition balance type enumeration

The runtime SHALL expose ТипОстаткаКомпоновкиДанных/DataCompositionBalanceType with НачальныйОстаток/OpeningBalance, КонечныйОстаток/ClosingBalance and Нет/None through the existing system enumeration tables. Aliases, presentations, type lookup and equality SHALL match the retained native 8.3.27.2342 control. This kind SHALL remain distinct from DataCompositionPeriodType.

#### Scenario: Native balance-type replay
- **WHEN** the retained control executes through the facade
- **THEN** all native results match for its three members, aliases, presentations and type identity.

#### Scenario: Distinct kinds
- **WHEN** the balance type and period type members are compared by type
- **THEN** their types differ.


### Requirement: Data composition accounting balance type enumeration

The runtime SHALL expose ТипБухгалтерскогоОстаткаКомпоновкиДанных/DataCompositionAccountingBalanceType with Дебет/Debit, Кредит/Credit and Нет/None through the existing system enumeration tables. Presentations, aliases, type lookup and equality SHALL match the retained native 8.3.27.2342 control. This kind SHALL remain distinct from DataCompositionBalanceType.

#### Scenario: Native accounting-balance replay
- **WHEN** the retained control executes through the facade
- **THEN** every native result matches for the three members, aliases, presentations and type identity.

#### Scenario: Distinct balance kinds
- **WHEN** accounting balance and ordinary balance members are compared by type
- **THEN** their types differ.


### Requirement: Data composition field placement enumeration

The runtime SHALL expose РасположениеПоляКомпоновкиДанных/DataCompositionFieldPlacement with Авто/Auto, Вертикально/Vertically, Вместе/Together, Горизонтально/Horizontally and ОтдельнаяКолонка/SpecialColumn through the existing system enumeration tables. Aliases, presentations, type lookup and equality SHALL match the retained native 8.3.27.2342 control. This kind SHALL remain distinct from DataCompositionResourcesPlacement.

#### Scenario: Native field-placement replay
- **WHEN** the retained control executes through the facade
- **THEN** every native result matches for all five members, aliases, presentations and type identity.

#### Scenario: Distinct placement kinds
- **WHEN** field placement and resource placement members are compared by type
- **THEN** their types differ.


### Requirement: Data composition conditional appearance use enumeration

The runtime SHALL expose ИспользованиеУсловногоОформленияКомпоновкиДанных/DataCompositionConditionalAppearanceUse with Использовать/Use and НеИспользовать/DontUse through the existing system enumeration tables. Aliases, presentations, type lookup and equality SHALL match the retained native 8.3.27.2342 control. This kind SHALL remain distinct from DataCompositionFieldPlacement.

#### Scenario: Native conditional-appearance replay
- **WHEN** the retained control executes through the facade
- **THEN** every native result matches for both members, aliases, presentations and type identity.

#### Scenario: Distinct kinds
- **WHEN** conditional appearance use and field placement members are compared by type
- **THEN** their types differ.


### Requirement: Acquired processor object type projection
The configuration loader SHALL map an acquired data processor object type GUID to the existing configuration-specific DataProcessorObject descriptor when projecting declared attribute types. Unknown GUIDs SHALL remain unsupported. This mapping SHALL NOT register an object-state transfer decoder or interpret reference filling payloads as processor objects.

#### Scenario: Acquired identity
- **WHEN** a declared attribute type uses an acquired processor object type GUID
- **THEN** its projected descriptor is the same configuration-specific descriptor used by the processor object.

#### Scenario: No object decoding
- **WHEN** a reference filling payload names a processor object type GUID
- **THEN** the loader does not decode it as a reference or construct a processor object.


### Requirement: Acquired root common attributes
The ORM metadata root SHALL expose read-only ОбщиеРеквизиты/CommonAttributes through its existing metadata collection mechanism. The collection SHALL contain the complete acquired root common-attribute identities in acquired order, without filtering by usage on catalogs. Name lookup, indexed access, Count, Get, Find and iteration SHALL match the retained native control. Unknown membership or a missing acquired member SHALL fail explicitly rather than appear as an empty or partial collection.

#### Scenario: Native collection access
- **WHEN** the retained native control reads the acquired common attribute through both aliases, indexing, Get, Find and iteration
- **THEN** the facade returns the native names, count and missing-name result and rejects assignment to the root property

#### Scenario: Unknown composition
- **WHEN** a host catalog has no complete common-attribute list or a listed identity has no corresponding common-attribute metadata object
- **THEN** reading the root collection fails explicitly

#### Scenario: Acquired composition
- **WHEN** the DB configuration loader projects the known root common-attribute collection
- **THEN** every member is represented as MetadataKind::CommonAttribute, preserving identity, name and root order independently of applicable catalog attributes

### Requirement: Common attribute metadata property enumerations
The ORM SHALL expose CommonAttributeDataSeparation, CommonAttributeAutoUse and CommonAttributeUse through Metadata.ObjectProperties and their Russian aliases. Their containers SHALL retain the native metadata enumerated-property type, and their values SHALL retain distinct native type identities, names and presentations. These property names SHALL NOT become bare global identifiers.

#### Scenario: Native property values
- **WHEN** the retained native control accesses the seven values through Russian and English metadata property names and indexes their containers by member name
- **THEN** the facade reproduces their measured types, presentations and equal identities across aliases

#### Scenario: No extra global names
- **WHEN** code reads a common-attribute property enumeration as a bare global identifier
- **THEN** it remains unresolved, matching the native control

### Requirement: Acquired common attribute automatic use
The ORM SHALL expose read-only АвтоИспользование/AutoUse on common-attribute metadata objects as a CommonAttributeAutoUse value from acquired settings. The database loader SHALL project an explicitly acquired SDK automatic-use setting without deriving it from content membership. Unknown settings SHALL fail explicitly. Other metadata kinds SHALL retain their previous property behavior.

#### Scenario: Known automatic-use settings
- **WHEN** the acquired common attribute has automatic use enabled or disabled
- **THEN** both property aliases return the corresponding measured Use or DontUse identity, independently of its explicit content list

#### Scenario: Unknown automatic-use setting
- **WHEN** a common attribute has no acquired automatic-use setting
- **THEN** reading AutoUse fails explicitly instead of returning DontUse

### Requirement: Explicit common attribute data separation
The ORM SHALL expose read-only РазделениеДанных/DataSeparation on common-attribute metadata objects as the measured CommonAttributeDataSeparation value when its mode is explicitly known. Existing SDK DataSeparationSettings alone SHALL NOT establish either mode: native artifacts demonstrate their presence for a disabled common attribute too. Without an explicit acquired mode the property SHALL remain unknown. A host with an explicitly known mode MAY supply it through the catalog API.

#### Scenario: Settings do not establish the mode
- **WHEN** the existing SDK provides DataSeparationSettings without an explicit mode
- **THEN** the loader does not infer Separate, including for the retained native DontUse artifact

#### Scenario: Absent settings are not a disabled mode
- **WHEN** no explicit data-separation mode is available
- **THEN** reading the property fails explicitly rather than returning DontUse

#### Scenario: Explicit host mode
- **WHEN** the host supplies a known enabled or disabled mode
- **THEN** the property returns the corresponding native enumeration identity and rejects assignment

#### Scenario: Acquired field classification
- **WHEN** exactly one acquired SDK MetadataField matches a common-attribute GUID
- **THEN** the loader projects its explicit data_separator classification as Separate or DontUse, independently of descriptor.separation

#### Scenario: Missing or ambiguous field classification
- **WHEN** no acquired field or multiple acquired fields match the common-attribute GUID
- **THEN** the mode remains unknown rather than being inferred from missing storage or a selected first match


### Requirement: Measured metadata full names

Catalog, common-attribute, constant and functional-option metadata objects SHALL expose the zero-argument
`ПолноеИмя()` / `FullName()` method. It SHALL use the acquired configuration
ScriptVariant: `Catalog` and `CommonAttribute` for English, `Справочник` and
`ОбщийРеквизит` for Russian, followed by a dot and the acquired object name.
Constants and functional options SHALL likewise use the acquired ScriptVariant: `Constant` / `FunctionalOption` for English and `Константа` / `ФункциональнаяОпция` for Russian.
The spelling used to access the object or method SHALL NOT change the result.
An unknown configuration ScriptVariant SHALL cause an explicit runtime error;
no language SHALL be guessed. Other metadata kinds without measured support
SHALL cause an explicit unsupported-operation error rather than an invented name.

#### Scenario: Both script variants replay the platform receipts

- **GIVEN** the catalogs and common attribute from the native full-name controls
- **WHEN** the control runs with each acquired ScriptVariant
- **THEN** the full-name output lines match its corresponding native platform receipt (the separate ScriptVariant presentation canary is outside this method contract)

#### Scenario: Missing configuration language is explicit

- **GIVEN** a catalog without acquired configuration properties
- **WHEN** FullName is called on its metadata object
- **THEN** execution fails explicitly without assuming Russian or English


#### Scenario: Constant and functional-option metadata names

- **GIVEN** acquired constant and functional-option metadata and a known configuration ScriptVariant
- **WHEN** FullName or ПолноеИмя is called using either language of object access
- **THEN** their language-specific qualified names match the corresponding native `measure-metadata-option-full-name` control
- **AND** the method does not read or evaluate the value of either object


### Requirement: Acquired functional-option evaluation

The configured Session data adapter SHALL resolve a functional-option name against
acquired configuration metadata and evaluate its acquired source. The supported
measured sources SHALL include a constant, a catalog attribute and a resource of
an independent nonperiodical information register. An unknown option or unknown
source/binding metadata SHALL cause an explicit runtime error, rather than a
false result, guessed same-named constant or incomplete successful evaluation.
The function SHALL retain the existing Russian/English aliases and folded name
lookup. Evaluation SHALL use the current Session's connection and principal.

#### Scenario: Constant source ignores the parameter structure

- **GIVEN** the acquired constant-bound options from `measure-functional-option-constant`
- **WHEN** the option is read before and after the control's constant writes, including with an unrelated parameter key
- **THEN** its Boolean values match the native receipt, including ignoring that unrelated key
- **AND** no separate option value is manufactured from its name

#### Scenario: Catalog attribute source uses its acquired reference parameter

- **GIVEN** the complete acquired catalog/attribute/parameter bindings from `measure-functional-option-catalog`
- **WHEN** the option is evaluated with a full reference, empty reference, group reference or incomplete parameters
- **THEN** its values and exceptions match the native control
- **AND** a missing matching value returns Boolean false even for the numeric option
- **AND** an inapplicable item-only attribute for the group returns Null

#### Scenario: Two register dimensions constrain the selected resource values

- **GIVEN** the acquired bindings from `measure-functional-option-register` and its three control rows
- **WHEN** both parameters or either single parameter are supplied
- **THEN** the Boolean result is the disjunction of the selected resource values
- **AND** an incomplete parameter set for the numeric resource causes an exception even if one row matches
- **AND** a complete parameter set without a matching row returns Boolean false

### Requirement: Measured privilege boundary of acquired options

For the measured constant, catalog-attribute and independent nonperiodical information-register resource sources, evaluation SHALL honor the acquired
PrivilegedGetMode flag. A false flag SHALL preserve ordinary access checks. A
true flag SHALL allow the option value observed by the native rights control
without changing the Session principal or globally disabling ordinary checks.
An unknown flag SHALL NOT be treated as permission to bypass access checks.
This requirement SHALL NOT imply unmeasured RLS, data-separation or cross-Session
cache semantics.

#### Scenario: Selected denied user reads only the privileged option

- **GIVEN** the selected `FunctionalDenied` principal and metadata/roles from `functional-option-rights-native`
- **WHEN** direct constant reading, the ordinary option, the privileged option and the ordinary option again are evaluated
- **THEN** direct and ordinary reading fail while privileged option reading returns Boolean true
- **AND** ordinary reading still fails afterwards and the effective Read permission remains false

#### Scenario: Denied user reads privileged catalog and register options

- **GIVEN** the owned native source-rights control with one true catalog attribute, one true register resource and the selected `FunctionalDenied` user
- **WHEN** ordinary, privileged and ordinary-again options for each source are evaluated without supplied parameters
- **THEN** ordinary readings fail and privileged readings return Boolean true
- **AND** both effective source Read permissions remain false before and after
- **AND** any invocation-local permission is confined to that acquired source rather than retained by the Session


## ADDED Requirements

### Requirement: Measured functional-option argument defaults
The existing GetFunctionalOption global function SHALL treat an omitted
second argument and an explicit Undefined second argument as an empty
parameter structure. It SHALL accept a mutable Structure and SHALL reject
a Number, String, Array or FixedStructure second argument. The option name
SHALL be a String. This argument normalization SHALL preserve the current
Session principal and data adapter.

#### Scenario: Explicit Undefined parameters match omission
- **GIVEN** the native constant-backed option argument control
- **WHEN** the caller omits the parameters or explicitly passes Undefined
- **THEN** both calls forward an empty parameter list and produce the same option value
- **AND** a Number, String, Array or FixedStructure parameter raises an error


## ADDED Requirements

### Requirement: Immutable acquired functional-option descriptors
The ORM metadata catalog SHALL retain a typed functional-option source,
the separately acquired privileged-get flag and the complete parameter
bindings needed to evaluate that source. Supported sources SHALL distinguish
a constant, a catalog attribute, an information-register resource and known
absence. Unknown source, flag or bindings SHALL remain unknown. Each binding
SHALL retain its canonical parameter name and target configuration identity.
These descriptions SHALL contain no connection or mutable Session state.

#### Scenario: Unknown acquisition is not an empty binding list
- **GIVEN** an option whose source or complete parameter bindings were not acquired
- **WHEN** a consumer reads its immutable description
- **THEN** the missing information remains unknown
- **AND** a verified absent source or verified empty binding list remains distinguishable


## ADDED Requirements

### Requirement: Validated functional-option projection
Projection of an acquired option source SHALL validate its owner kind
and field ownership against acquired metadata. A known invalid owner SHALL
fail projection explicitly. Source-specific parameter bindings SHALL be
complete only when the full root parameter membership and every referenced
parameter Use target are known; an incomplete acquisition SHALL preserve
unknown bindings without publishing a partial list. Constant-backed option
sources SHALL require no parameter bindings.

#### Scenario: Known source targets the wrong owner kind
- **GIVEN** a known constant option source whose target is a catalog object
- **WHEN** the loader projects its acquired description
- **THEN** projection fails explicitly before option evaluation

#### Scenario: Parameter Use is missing from a complete root membership
- **GIVEN** a catalog-backed option and complete root parameter identities
- **AND** one referenced parameter lacks its complete Use targets
- **WHEN** the loader projects the description
- **THEN** its parameter bindings remain unknown rather than partially complete


### Requirement: Measured primitive inverse XML type lookup
The XML library SHALL expose global `ИзXMLТипа` / `FromXMLType` and the same-named method on an existing XDTO serializer. Both SHALL accept either one XMLDataType value or two strings naming the XML local type and namespace. Canonical XML Schema string, decimal, boolean, dateTime and base64Binary SHALL return the corresponding native BSL Type; the measured other Schema names, wrong namespace and absent names SHALL return Undefined. This lookup SHALL NOT use the broader lexical XDTO conversion mapping: integer, double, date, hexBinary and QName have no inverse native mapping in the retained platform receipt. Invalid argument types SHALL fail catchably. Configuration-specific type bindings remain dependent on acquired configuration XML types and are not established by this primitive requirement.

#### Scenario: Replay the native inverse lookup receipt
- **WHEN** the retained `measure-from-xml-type.bsl` control runs through the XML library
- **THEN** its primitive, overload, alias, unknown-type and argument-error output matches the native 8.3.27.2342 receipt

#### Scenario: Serializer uses the canonical mapping
- **GIVEN** a serializer created from an explicit XML schema factory
- **WHEN** it resolves canonical string and decimal XML types through both overloads
- **THEN** it returns the same native Types as the global function


### Requirement: Session factory retains its visible common-module namespace
The Session factory SHALL declare server-visible common modules as roots through the existing configuration-root API, in addition to the session module. Runtime dynamic text can select their names without a static reference. Such a module SHALL NOT become an unknown variable solely because the original session code selects it dynamically. Retained modules SHALL preserve existing lazy initialization and name/variable priority. Unrelated private manager/object modules SHALL remain governed by the existing semantic dependency closure. The generic facade root API SHALL keep its existing explicit-root behavior.

#### Scenario: Common module selected only by runtime text
- **GIVEN** a Session snapshot has a broker using Evaluate on its name argument and an otherwise unreferenced server common module
- **WHEN** the session handler passes that module name to the broker and calls an export through the returned reference
- **THEN** evaluation resolves the existing module in the current Session and the export executes

#### Scenario: Unrelated private manager remains excluded
- **GIVEN** the same configuration has an unreached private manager with a semantic error
- **WHEN** the Session factory is prepared
- **THEN** the private manager remains outside the compilation closure

#### Scenario: Independent sessions
- **WHEN** two Sessions use the same prepared factory and dynamic module lookup
- **THEN** each call uses its own Session parameters and module instances


### Requirement: Deferred unsupported constructors in common modules
With the existing opt-in common-module name-error policy enabled, an unsupported constructor in a common-module method body SHALL NOT prevent an unused module from being retained in the configuration namespace. Accessing that module SHALL raise the existing module-compilation error before its exports execute. Default compilation and Private or Explicit modules SHALL remain strict. This policy SHALL NOT register the unsupported constructor or fabricate its value.

#### Scenario: Unused module with an unsupported constructor
- **GIVEN** a Named or Global module whose neighboring method uses an unsupported constructor
- **WHEN** a separate valid module runs with the policy enabled
- **THEN** its result is unchanged and no constructor executes

#### Scenario: Access to the affected module
- **WHEN** an export of that common module is accessed
- **THEN** the existing module-compilation error names the unsupported constructor
- **AND** the export body does not execute

#### Scenario: Strict contexts
- **WHEN** the policy is disabled or the module is Private or Explicit
- **THEN** the existing semantic unsupported-constructor diagnostic occurs during compilation


### Requirement: Reached Gantt chart interval representation enumeration
The existing system enumeration registry SHALL expose ОтображениеИнтервалаДиаграммыГанта and GanttChartIntervalRepresentation as aliases of one enumeration. Its four members SHALL be Плоский/Flat, Объемный/ThreeDimensional, Ромб/Rhomb and Градиент/Gradient. Lookup SHALL retain existing case folding and variable precedence, and string presentations SHALL match the native measurement. This declaration SHALL NOT implement chart rendering.

#### Scenario: Original report manager compiles
- **WHEN** retained report manager code refers to the reached interval representation
- **THEN** the existing resolver resolves the registered enumeration without source rewriting

#### Scenario: Native member aliases
- **WHEN** each Russian member and its English alias are compared
- **THEN** they designate the same value and expose the measured enumeration type

#### Scenario: Case folding
- **WHEN** the root and member are written in another case
- **THEN** the existing folded lookup yields the same value


### Requirement: Reached chart legend placement enumeration
The existing system enumeration registry SHALL expose РасположениеЛегендыДиаграммы and ChartLegendPlacement as aliases of one enumeration. Its members SHALL be Авто/Auto, Низ/Bottom, Верх/Top, Лево/Left, Право/Right, Нет/None and УказываетсяРасположение/UseCoordinates. Lookup SHALL preserve current folding and variable precedence; root, type and member presentations SHALL match the native receipt. Declaring these values SHALL NOT implement chart rendering.

#### Scenario: Retained report manager namespace
- **WHEN** original report manager code refers to the reached legend enumeration
- **THEN** the existing resolver recognizes its registered name without rewriting source

#### Scenario: Native aliases and presentations
- **WHEN** each Russian member and English alias are compared and formatted
- **THEN** the aliases designate the same member and formatting matches the retained native result

#### Scenario: Folded names
- **WHEN** the root or member uses a different case
- **THEN** the current folded lookup yields the same value


### Requirement: Reached Gantt chart vertical stretching enumeration
The existing enumeration registry SHALL expose РастягиваниеПоВертикалиДиаграммыГанта and GanttChartVerticalStretch as aliases of one enumeration. Its members SHALL be НеРастягивать/None, РастягиватьСтроки/StretchRows and РастягиватьСтрокиИДанные/StretchRowsAndData. Lookup SHALL preserve existing case folding and variable precedence; string representations of the root, type and members SHALL match the native receipt. The declaration SHALL NOT implement chart scaling or rendering.

#### Scenario: Original report manager reference
- **WHEN** retained original report code uses РастягиватьСтрокиИДанные
- **THEN** the current resolver recognizes the existing enumeration declaration mechanism

#### Scenario: Native aliases and representations
- **WHEN** all three Russian members and their English aliases are compared and formatted
- **THEN** alias identity, type and representations match the native control

#### Scenario: Case folding
- **WHEN** a root and member are written in another case
- **THEN** the existing lookup produces the same value


### Requirement: Reached time scale unit enumeration
The existing system enumeration registry SHALL expose ТипЕдиницыШкалыВремени and TimeScaleUnitType as aliases of one enumeration. Its members SHALL be Секунда/Second, Минута/Minute, Час/Hour, День/Day, Неделя/Week, Месяц/Month, Квартал/Quarter and Год/Year. Lookup SHALL preserve current case folding and variable precedence; root, type and member presentations SHALL match the native receipt. This declaration SHALL NOT implement time-scale rendering.

#### Scenario: Original report manager unit references
- **WHEN** retained original report manager code refers to the day, minute or hour units
- **THEN** the existing resolver recognizes the registered enumeration without rewriting source

#### Scenario: Native aliases and presentations
- **WHEN** all eight Russian members and English aliases are compared and formatted
- **THEN** alias identity, type and presentations match the retained native control

#### Scenario: Folded lookup
- **WHEN** the root and member use another case
- **THEN** the current folded lookup returns the same value


### Requirement: Reached time scale day format enumeration
The existing enumeration registry SHALL expose ФорматДняШкалыВремени and TimeScaleDayFormat as aliases of one enumeration. Its members SHALL be ДеньМесяца/MonthDay, ДеньНедели/WeekDay, ДеньМесяцаДеньНедели/MonthDayWeekDay and ДеньНеделиДеньМесяца/WeekDayMonthDay. Lookup SHALL preserve current case folding and variable precedence; root, type and member presentations SHALL match the native receipt. This declaration SHALL NOT implement chart date formatting or rendering.

#### Scenario: Original report manager day-format reference
- **WHEN** retained original report manager code refers to ДеньМесяца
- **THEN** the current resolver recognizes the registered enumeration without source rewriting

#### Scenario: Native aliases and presentations
- **WHEN** all four Russian members and English aliases are compared and formatted
- **THEN** their identity, type and presentations match the retained native control

#### Scenario: Folded lookup
- **WHEN** the root or member uses another case
- **THEN** existing folded lookup yields the same value

### Requirement: Acquired separation parameter bindings remain explicit metadata
The configuration loader SHALL project known common-attribute separation value/use parameter bindings and acquired value types into the ORM metadata catalog without importing the database SDK into the runtime or VM. Unknown bindings and unknown types SHALL remain distinguishable from an explicitly absent binding or a known empty list. Referenced bindings SHALL identify existing SessionParameter metadata objects.

#### Scenario: Acquired bindings identify session parameters
- **GIVEN** acquired configuration metadata binds a separating common attribute to a value parameter and a use parameter
- **WHEN** the loader creates the ORM catalog
- **THEN** the bindings identify those SessionParameter objects and retain the acquired value types

#### Scenario: Unsupported acquired type declaration
- **GIVEN** a separating attribute has an unknown value type declaration
- **WHEN** its parameter initialization is required
- **THEN** the host reports the unknown metadata explicitly rather than inventing a default value

### Requirement: Separation-bound parameters have measured initial values
A fresh configured Session SHALL initialize the supported acquired separation-bound parameters before invoking the original startup handler, matching the retained native control. A Boolean use parameter SHALL start as false, a String value parameter SHALL start as the empty string, and a Numeric value parameter SHALL start as zero. This SHALL NOT initialize ordinary session parameters. Initial values for further types SHALL only be added after native measurement. Background Session parameter inheritance SHALL retain the parent's values instead of replacing them with these fresh-Session defaults.

#### Scenario: String separation-bound parameters and an ordinary parameter
- **GIVEN** a separating common attribute binds a Boolean use parameter and a String value parameter, and the configuration also declares an ordinary Boolean parameter
- **WHEN** a fresh Session begins
- **THEN** the use parameter is false and the value parameter is the empty string without invoking selective initialization
- **AND** reading the ordinary unset parameter invokes the original selective handler and preserves its failure

#### Scenario: Two fresh Sessions remain isolated
- **WHEN** one fresh Session changes its acquired separation-bound values
- **THEN** another fresh Session keeps its own measured initial values

#### Scenario: Background inheritance retains bound values
- **GIVEN** a parent Session has changed its separation-bound parameters
- **WHEN** a background Session receives the parent parameter snapshot
- **THEN** it retains those inherited values while using its separate Session and connection

#### Scenario: Numeric separation-bound parameter
- **GIVEN** an acquired separating attribute binds a Numeric value parameter
- **WHEN** a fresh Session begins
- **THEN** the parameter starts as zero, matching the retained native numeric control

### Requirement: Acquired global conditional separation
The database configuration loader SHALL retain a known global common-attribute ConditionalSeparation binding to its Boolean Constant metadata object. Unknown settings SHALL remain distinguishable from an explicitly absent condition. The runtime and VM SHALL NOT depend on SDK or database types to represent this metadata.

#### Scenario: Acquired Boolean condition
- **GIVEN** the acquired common attribute names a Boolean constant as its global separation condition
- **WHEN** the database loader projects its ORM metadata
- **THEN** the binding retains that constant's identity and does not become a BSL session parameter or a new global identifier

#### Scenario: Unknown condition metadata
- **WHEN** a query requires a condition whose binding, target, or Boolean type is unknown
- **THEN** the host fails explicitly instead of assuming that separation is enabled or disabled

### Requirement: Session queries honor measured conditional separation
The configured data adapter SHALL provide required global condition values to the existing SDK query compiler through the same Session-owned data connection. It SHALL NOT rewrite BSL source or implement separate SQL generation. For the measured Independent String and Numeric controls, a false condition SHALL select only the empty separator area, independently of the supplied use/value parameters. A true condition SHALL retain the measured Independent behavior. Changing the condition through a supported operation in the same Session SHALL affect subsequent query compilation; an initial value SHALL NOT be retained permanently while ignoring that change. Further modes or types SHALL require native measurement before support is claimed.

#### Scenario: False condition selects the empty area
- **GIVEN** the retained native control has records in the empty area and areas A and B, and the condition is false
- **WHEN** a Session queries with a value for area C or with a false use parameter
- **THEN** it returns the empty-area record and does not expose A or B
- **AND** it does not request lazy initialization of use/value parameters that this condition makes unnecessary

#### Scenario: Numeric empty area
- **GIVEN** the measured Numeric control has records in empty area zero and areas one and two
- **WHEN** the condition is false and the supplied parameter names area three
- **THEN** only the empty-area record is returned

#### Scenario: True condition retains the Independent refusal
- **WHEN** the condition is true and the use parameter is false
- **THEN** query compilation preserves the terminal Independent refusal rather than retrying a missing session parameter or dropping its filter

#### Scenario: Condition changes within one Session
- **GIVEN** a supported operation changes the condition from false to true in one Session
- **WHEN** the next query uses a false use parameter
- **THEN** it is refused, matching the retained native control

#### Scenario: Missing condition input identifies a constant
- **WHEN** the SDK compiler requires a condition value not yet supplied
- **THEN** the host resolves the specifically identified constant through that Session's data adapter and retries with a Boolean value
- **AND** this request does not invoke session-parameter initialization under the constant's name

### Requirement: Complete common attribute content metadata
Common-attribute metadata SHALL expose read-only Состав/Content backed by the complete acquired SDK content in acquired order, including Auto and DontUse entries. Explicit applicability settings alone SHALL NOT be presented as the full collection. Unknown or incomplete content SHALL fail explicitly rather than appear empty or be shortened to represented members. Known empty content SHALL remain distinguishable from unknown content.

#### Scenario: Full content includes automatic entries
- **GIVEN** acquired complete content has automatic and explicitly enabled or disabled members
- **WHEN** code enumerates Content or reads Count
- **THEN** every acquired member is retained in its acquired order, independently of effective applicability to object attributes

#### Scenario: Unknown membership
- **WHEN** full content is unknown or a listed member cannot be represented by the host metadata catalog
- **THEN** reading Content fails explicitly without returning an empty or partial collection

### Requirement: Measured common attribute content surface
Content SHALL reproduce the retained native CommonAttributeContent and CommonAttributeContentItem type identities, Count, Find, Contains, iteration, and read-only item properties. Numeric indexing and a Get method SHALL remain unavailable. Find SHALL accept one metadata object or one short name; short names SHALL be compared without case sensitivity, while qualified Russian and English names SHALL NOT be treated as short names. Contains SHALL test metadata identity; String arguments SHALL return false, and the measured invalid Number, Boolean and Undefined arguments SHALL fail. Both Find and Contains SHALL reject missing or extra arguments.

#### Scenario: Non-indexable iteration
- **WHEN** code enumerates Content through ForEach
- **THEN** it receives its native content items using the existing collection protocol
- **AND** direct numeric indexing or Get remains an error

#### Scenario: Native lookup variants
- **WHEN** the retained lookup control searches by represented metadata, a folded short name, qualified names, an empty string, or absent metadata
- **THEN** it reproduces the native found identities or Undefined results
- **AND** Count, Find and Contains remain case-insensitive through their Russian and English aliases

#### Scenario: Item properties and identity
- **WHEN** code reads Метаданные/Metadata, Использование/Use, or УсловноеРазделение/ConditionalSeparation on an item
- **THEN** it returns the represented metadata identity, existing CommonAttributeUse value, and acquired Undefined or Constant metadata identity
- **AND** an item found by metadata equals the same item encountered during enumeration
- **AND** property writes fail

#### Scenario: Type aliases
- **WHEN** code compares TypeOf of the collection or item with its Russian or English Type name
- **THEN** it returns the same measured type identity and presentation

#### Scenario: Unknown item condition
- **WHEN** a represented item's conditional-separation metadata is unknown
- **THEN** reading that property fails explicitly instead of inventing Undefined

### Requirement: Measured Shared String and Numeric conditional separation
For the retained Shared String and Numeric controls, a false global condition SHALL select only the empty separator area regardless of the use/value parameters. A true global condition with a false use parameter SHALL allow the measured unfiltered Shared query. This behavior SHALL remain distinct from the terminal Independent refusal. Each supported measured type SHALL be backed by its own retained native control; other types SHALL NOT be claimed as measured by these controls.

#### Scenario: Shared false condition
- **GIVEN** the retained control has empty, A and B records and its global condition is false
- **WHEN** its value parameter is C or its use parameter is false
- **THEN** the query returns only the empty record

#### Scenario: Shared true condition and false use
- **GIVEN** the same Session changes its global condition to true and use is false
- **WHEN** it queries either retained Shared control
- **THEN** it returns all three records without the Independent refusal

#### Scenario: Shared Numeric empty area
- **GIVEN** the retained Numeric Shared control has empty area zero and areas one and two
- **WHEN** its global condition is false and its value parameter is three
- **THEN** it returns only the empty-area record, including with a false use parameter

### Requirement: Measured shared body with unused ordinary locals
The native-image loader SHALL recover the retained two-empty-guard shared-body control with a five-variable public frame and a four-variable private frame. Both frames have one ordinary reference parameter in slot zero; the shared commands access only that parameter, and the remaining ordinary locals are unused. Recovery SHALL preserve both declarations, parameter names and reference effects through the existing BSL compiler and VM. It SHALL NOT treat this control as evidence for unknown variable flags, other frame shapes or arbitrary overlaps.

#### Scenario: Different ordinary frames
- **WHEN** both entry paths of the retained control execute with argument five
- **THEN** recovered BSL reproduces the native result and updated argument twelve for both paths

#### Scenario: Unmeasured overlapping frame
- **WHEN** a variable descriptor or shared command is outside the retained measured forms
- **THEN** recovery fails explicitly rather than discarding the private declaration or returning a partial module

### Requirement: Frame-scoped labels in a measured overlapping body
The native-image loader SHALL preserve the current method's label table for the retained ordinary-frame overlapping-body control. A shared goto-label instruction SHALL NOT be globally resolved using only its address when the retained public and private label tables name different targets. Recovery SHALL preserve the complete module and both native entry outcomes through the existing BSL compiler and VM. Unknown overlapping frames SHALL remain explicit recovery errors.

#### Scenario: Public and private label targets differ
- **WHEN** the retained control executes its public and private entry paths with argument five
- **THEN** recovered BSL reproduces public result and argument twelve and private result and argument five
- **AND** recovery does not replace both paths with one global jump target

### Requirement: Measured pure private returns from constant-backed slots
The native-image loader SHALL recover the retained pure private-return controls through BSL source and the existing compiler and VM. It SHALL preserve the ordinary baseline result, the measured 1338/0 and 1338/1 numeric constant selections, and each read in the retained four-slot 1338/0,1459/0,3091/0,3027/0 frame. Both the result and unchanged caller argument SHALL match the retained native receipts. This support SHALL NOT imply support for writes, dynamic evaluation, other descriptors or arbitrary overlapping bodies.

#### Scenario: Indexed constant selection and complete four-slot frame
- **GIVEN** the retained numeric private-return and private-frame-read controls
- **WHEN** the complete images are recovered and their drivers execute
- **THEN** results select numeric constants 7, 11 or 23 as measured, all four frame reads return 7, and caller arguments remain 5
- **AND** both private and public declarations remain present

#### Scenario: Unmeasured private body remains unsupported
- **GIVEN** a changed frame descriptor, unsupported constant slot, store or dynamic command in the private body
- **WHEN** the image is recovered
- **THEN** recovery fails explicitly instead of returning a partial or successful substitute module

### Requirement: Measured numeric private-slot writes
The native-image loader SHALL recover the retained numeric private-alias-write controls as complete BSL modules executed by the existing compiler and VM. A private numeric slot initialized from Const[0] SHALL retain its measured local write within the call without changing the caller argument or the caller constant. This support SHALL be limited to the retained single-slot arithmetic body with its measured descriptor and defaults; it SHALL NOT imply arbitrary private-frame writes or absent-default signatures.

#### Scenario: Independent observation after a private write
- **WHEN** the retained direct-observation driver calls the recovered module twice
- **THEN** each call observes private result fourteen, caller constant seven and caller argument five
- **AND** both declarations remain present and the complete native receipt matches

#### Scenario: Changed private write body
- **WHEN** a descriptor, default, constant kind, store target or control-flow edge falls outside the measured private write form
- **THEN** recovery fails explicitly without returning a partial module

### Requirement: Measured private numeric reads without defaults
The native-image loader SHALL recover the retained four-slot numeric private-read control without a private DefPrm table through complete BSL source and the existing compiler and VM. Recovery SHALL preserve the declared one-argument private signatures and all four native receipts for static calls supplying exactly one actual argument. It SHALL NOT generalize this support to absent-default single-slot frames, private procedures, dynamic calls or changed bodies.

#### Scenario: One actual argument and all four slot reads
- **GIVEN** the retained private-frame-no-defaults image
- **WHEN** its complete recovered module executes the retained driver
- **THEN** all four slot reads return seven and each caller argument remains five
- **AND** all eight public and private declarations remain present

#### Scenario: An arity outside the supported source form
- **GIVEN** an absent-default private frame called with zero or two actual arguments
- **WHEN** the image is recovered
- **THEN** recovery fails explicitly rather than inventing defaults, changing the declared signature or dropping an actual argument

### Requirement: Measured zero-address labels in overlapping function frames
The native-image loader SHALL recover the retained three-path zero/nonzero private-label controls as complete BSL modules through the existing compiler and VM. It SHALL preserve the selected frame's label target, including address zero, and SHALL NOT substitute the public frame's nonzero target for the measured private function. Unknown procedure-frame routing SHALL remain an explicit recovery error.

#### Scenario: Three distinguishable private paths
- **GIVEN** the paired zero-label-private-path and nonzero-label-private-path images, differing only in the private label target
- **WHEN** the complete recovered modules execute their retained drivers
- **THEN** their public results and caller arguments are five
- **AND** their private results are respectively eleven and seven with unchanged caller arguments
- **AND** neither private result is the fallthrough sentinel twenty-three

#### Scenario: Unsupported frame routing
- **WHEN** a frame descriptor, entry path or shared body is outside the measured overlapping-function forms
- **THEN** recovery fails explicitly without globally reinterpreting a zero label or discarding the private declaration

### Requirement: Measured overlapping private-procedure effects
The native-image loader SHALL recover the retained zero/nonzero-label-procedure-queue images as complete BSL modules executed by the existing compiler and VM. Recovery SHALL retain the private procedure declaration and preserve its measured frame-specific route message, ordinary caller value and unchanged argument. This support SHALL NOT imply arbitrary private-frame calls, writes, dynamic code or unknown procedure prologues.

#### Scenario: Private procedure route has an observable effect
- **GIVEN** the paired retained procedure-queue controls
- **WHEN** their complete recovered modules execute their retained drivers
- **THEN** the public route is nonzero in both receipts
- **AND** private routes are respectively zero and nonzero
- **AND** all caller values and arguments are five and all nine receipt lines match

#### Scenario: Unsupported private procedure effect
- **WHEN** a private frame writes a slot, invokes an unmeasured callback or contains an unknown entry prologue
- **THEN** recovery fails explicitly without substituting an empty private procedure

### Requirement: Frame-scoped collection loops in recovered overlapping methods

Configuration code acquisition SHALL recover the measured overlapping-method foreach profile through the existing BSL compiler and runtime. A public ordinary one-parameter four-slot frame SHALL retain its own collection, accumulator, element and iterator state when its body continues after the entry of a private four-slot constant-alias method. The private declaration and its observable invocation SHALL remain present. Unsupported iterator descriptors or control-flow envelopes SHALL fail rather than produce a partial module or fabricated result.

#### Scenario: Public collection loop and private read use different frames
- **GIVEN** the complete native scoped-foreach-frame control with all three declarations and their distinct local tables
- **WHEN** its full driver is replayed after recovery
- **THEN** the nonempty array produces sum 6 and count 3, the empty array produces sum 0 and count 0, and the private wrapper returns 7 while leaving its argument at 5
- **AND** all seven retained native output lines match through the ordinary compiler and VM.

#### Scenario: Unmeasured iterator state is rejected
- **WHEN** the measured control is changed to use an unsupported iterator descriptor or an invalid loop cleanup or back edge
- **THEN** acquisition rejects the entire image without replacing the private method or synthesizing an empty successful loop.

### Requirement: Measured private frame in a contextual module
Configuration code acquisition SHALL recover the retained private27-context
module through complete BSL source and the existing compiler and VM. It
SHALL preserve all callable declarations and global/external name bindings,
the measured constant-backed private frame, route messages and unchanged
caller arguments. An unsupported declaration SHALL reject the entire image.

#### Scenario: Context does not change the measured private route
- **GIVEN** the retained private27-context image with Metadata and NStr declarations
- **WHEN** the complete recovered module executes its retained driver
- **THEN** all nine native observations match the retained string-frame18 receipt
- **AND** recovery retains the public wrappers and private procedure.

#### Scenario: A changed private frame is not guessed
- **WHEN** the private signature, descriptor, label routing or constant kind is outside the measured frame forms
- **THEN** acquisition rejects the entire image without omitting a declaration or inventing a successful body.

### Requirement: Measured backward shared targets in ordinary frames
The native-image loader SHALL recover the retained ordinary-backward-shared
module as complete BSL source executed by the existing compiler and VM.
A valid measured frame-local label target SHALL NOT be rejected solely
because its command address precedes the frame entry. Existing checks for
missing label indices, invalid command addresses and unsupported bodies
SHALL remain in force.

#### Scenario: An earlier shared constant body
- **GIVEN** the retained two-function control whose second label target is3 and entry is8
- **WHEN** its complete recovered module executes the retained driver
- **THEN** both exported functions return42 and both native receipt lines match.

#### Scenario: Invalid indices remain unsupported
- **WHEN** a label index is missing or negative, or a target lies beyond the command table
- **THEN** recovery rejects the image rather than redirecting the jump to another frame's label.

### Requirement: Frame-scoped initialization in complete recovered modules
The native-image loader SHALL recover the retained scoped-initializer
module as complete BSL source executed by the existing compiler and VM.
It SHALL preserve initialization, ordinary declarations, reference updates
and private visibility without emitting an additional callable initializer
declaration. An unsupported method or initialization body SHALL still
reject the complete image.

#### Scenario: Initialization precedes preserved ordinary exports
- **GIVEN** the retained scoped-initializer image with rebased empty handlers and literal short-circuit branches
- **WHEN** the full module is recovered and its driver executes
- **THEN** all four native receipt lines match the ordinary-shared-context baseline
- **AND** the original public and private declarations remain present.

#### Scenario: Invalid initialization remains unsupported
- **WHEN** the initializer entry, reachable handler target, short-circuit target or local-slot read is invalid
- **THEN** acquisition rejects the initialization instead of dropping it or returning a partial module.

### Requirement: Conservative cleanup of recovered native source
The native-image loader SHALL remove unconditional jumps to an immediately
following generated label, retaining labels with other incoming jumps. It
SHALL fold `(False = False)` and `(True = True)` to True, and `(False = True)`
and `(Not True)` to False. It SHALL remove unused local declarations and
unread assignments of pure constants while preserving callable signatures,
module storage, calls and potentially failing or effectful expressions.
Cleanup SHALL NOT transform text inside string literals or comments and
SHALL NOT make an unsupported image acceptable through partial recovery.

#### Scenario: Shared targets remain reachable
- **GIVEN** a recovered method containing a fallthrough jump and another incoming jump to its target
- **WHEN** source cleanup runs
- **THEN** the fallthrough jump is removed and the shared label remains.

#### Scenario: Unread constants do not become runtime work
- **GIVEN** an unused local and an unread generated temporary assigned only pure constants
- **WHEN** source cleanup runs
- **THEN** their assignments and unnecessary local declarations are removed
- **AND** method parameters and assignments that perform calls remain unchanged.

#### Scenario: Dynamic code and textual data are preserved
- **GIVEN** a method executing dynamically evaluated code or a literal containing the cleanup patterns
- **WHEN** source cleanup runs
- **THEN** variable cleanup is disabled for the dynamic scope and literal contents remain unchanged.

#### Scenario: Complete native receipts remain equivalent
- **WHEN** complete retained native controls are recovered with cleanup enabled
- **THEN** their complete driver observations match the native receipts through the ordinary compiler and VM.

### Requirement: Measured characteristic manager reference construction
Characteristic managers SHALL expose ПустаяСсылка/EmptyRef with zero arguments and ПолучитьСсылку/GetRef with zero or one UUID argument using the existing session reference factory. EmptyRef SHALL return the selected characteristic reference type with a nil UUID. A supplied UUID SHALL be preserved in a reference of the selected characteristic type; an omitted UUID SHALL generate a nonempty reference without persisting an object. String arguments SHALL be rejected. Matching existing manager exports SHALL retain their existing resolution and arity.

#### Scenario: Characteristic reference identity
- **WHEN** either alias receives the same UUID
- **THEN** the resulting references have the selected characteristic type and equal UUIDs.

#### Scenario: Generated and invalid characteristic references
- **WHEN** no argument is supplied
- **THEN** a nonempty reference is generated without writing an object
- **AND** a string argument is rejected.

#### Scenario: Empty characteristic reference
- **WHEN** either empty-reference alias is called without arguments
- **THEN** it returns a nil UUID reference of the selected characteristic type.

### Requirement: Measured legacy Base64 string input
The original Base64Строка/Base64String builtin SHALL return an empty string for a String argument, as measured on 1C 8.3.27.2342. Its existing binary-data encoding SHALL remain unchanged. This exception SHALL NOT change ПолучитьBase64СтрокуИзДвоичныхДанных/GetBase64StringFromBinaryData or infer behavior for other input types.

#### Scenario: Legacy Base64 with a string
- **WHEN** Base64Строка receives a nonempty String containing an extension hash
- **THEN** it returns an empty String without raising an exception.

### Requirement: Measured configuration extension purpose
The ORM SHALL accept an explicit optional typed extension purpose in an immutable extension description without changing existing snapshot construction. It SHALL expose read-only Назначение/Purpose and scoped Метаданные.СвойстваОбъектов.НазначениеРасширенияКонфигурации / Metadata.ObjectProperties.ConfigurationExtensionPurpose. The three distinct values SHALL support Исправление/Patch, Адаптация/Customization and Дополнение/AddOn with measured presentations and type identity. Unknown acquired purpose SHALL fail explicitly when read and SHALL NOT default from the extension name, activity, version or scope. The purpose namespace SHALL NOT become a global BSL name, and ordinary type lookup by its Russian/English identifier or friendly display name SHALL remain unavailable as measured. Database projection SHALL use only the SDK's acquired typed field once available.

#### Scenario: Explicit purpose and scoped aliases
- **WHEN** a host supplies one of the three typed purposes
- **THEN** either extension property alias returns the corresponding scoped metadata value
- **AND** reading or comparing either language's member aliases preserves identity and presentation.

#### Scenario: Read-only and unknown purpose
- **WHEN** BSL assigns either extension purpose property
- **THEN** assignment is rejected without changing it
- **AND** reading an extension whose purpose is unknown fails explicitly without inventing a value.

#### Scenario: Preserve measured namespace boundary
- **WHEN** BSL requests the bare global purpose name or its ordinary named type
- **THEN** the request remains unavailable
- **AND** access through Metadata.ObjectProperties remains available.

### Requirement: Measured legacy mail authentication enumerations
The runtime SHALL expose СпособPOP3Аутентификации/POP3AuthenticationMode with APOP, CramMD5 and Обычная/General, and СпособSMTPАутентификации/SMTPAuthenticationMode with CramMD5, Login, Plain, БезАутентификации/None and ПоУмолчанию/Default. Both namespace aliases and member aliases SHALL identify the same values. Bare enum and value-type presentations SHALL match the native 8.3.27.2342 control. These declarations SHALL NOT create or authenticate a mail connection.

#### Scenario: Reached authentication constants in original UNF
- **WHEN** original source reads either language's namespace and members
- **THEN** all three POP3 and five SMTP values resolve with native presentations and type identity
- **AND** no network connection is opened.

### Requirement: Measured XML canonicalization enumeration
The runtime SHALL expose ТипКаноническогоXML/XMLCanonicalizationType with the six native canonicalization algorithm constants and their documented Russian/English aliases. Namespace, member and named type identity and string presentations SHALL match the recorded 1C 8.3.27.2342 control. Declaring these values SHALL NOT execute XML canonicalization or change existing XML operations.

#### Scenario: Algorithm constants in original UNF
- **WHEN** original source reads either namespace alias and any of its six algorithm values
- **THEN** the value resolves with its measured aliases, presentation and type identity
- **AND** no XML transformation is performed by the declaration.

### Requirement: Reached DOM XML canonicalization
The XML component SHALL provide the stateless zero-argument ПреобразованиеККаноническомуXML/ConversionToCanonicalXML constructor and its measured type identity. КанонизироватьВСтроку/CanonicalizeToString, КанонизироватьВФайл/CanonicalizeToFile and КанонизироватьВПоток/CanonicalizeToStream SHALL canonicalize the existing DOM document without modifying it. They SHALL support the six measured algorithms, default to КаноническийXML1_1, preserve native attribute/namespace ordering, escaping and comment handling, and operate on the selected node set when a string XPath filter supplies it. Existing DOM and XPath infrastructure SHALL remain shared. File output SHALL use the current host filesystem; stream output SHALL use the existing byte-stream protocol at its current position without closing it. UTF-8 output SHALL have no BOM. Optional filter/prefix inputs SHALL follow the recorded native controls; no unmeasured argument conversion SHALL be inferred.

#### Scenario: Canonical text and aliases
- **WHEN** original source constructs either alias and requests canonical text using a measured algorithm
- **THEN** it receives the native control text and type identity
- **AND** the DOM remains unchanged.

#### Scenario: File and stream output
- **WHEN** canonical output is written to a file or twice to an open memory stream
- **THEN** the file contains the measured UTF-8 bytes without BOM
- **AND** the stream contains both successive outputs at its current position and stays open for the caller.

#### Scenario: XPath selects nodes rather than a complete subtree
- **WHEN** the filter selects only an element without its attributes or namespace nodes
- **THEN** canonical output includes only the selected nodes according to the native control
- **AND** attributes are not added by ordinary subtree serialization.

#### Scenario: An acquired XPathResult does not restrict canonical output
- **GIVEN** a DOM document and its node-set, Number, Boolean or String XPathResult
- **WHEN** the result is supplied as the canonicalization filter
- **THEN** output retains the full document, matching the four native controls

### Requirement: Reached legacy DOM canonicalization
The XML component SHALL expose the measured zero-argument
КаноническаяЗаписьXML/XMLCanonicalizingWriter and
КаноническийDOM/DOMCanonicalization constructors needed by original UNF.
Their joint string-output workflow SHALL support the four legacy algorithms
and reject the two Canonical XML 1.1 algorithms, matching retained native
controls. They SHALL reuse the existing DOM and canonicalization engine.
Writer lifecycle, aliases, arity and node/result selection SHALL follow
additional retained native measurements rather than the incomplete vendor
Canonicalize signature.

#### Scenario: Original legacy string workflow
- **WHEN** the writer is configured for a legacy algorithm and a DOM document is canonicalized into it
- **THEN** closing the writer returns the native canonical text

#### Scenario: Legacy writer rejects version 1.1
- **WHEN** the legacy writer is configured with either Canonical XML 1.1 variant
- **THEN** the operation fails catchably

### Requirement: Reached ZIP OS encoding with UTF8 value
The runtime SHALL expose КодировкаИменФайловВZipФайле.КодировкаОСДополнительноUTF8
and FileNamesEncodingInZipFile.OSEncodingWithUTF8 as the same distinct enum
value, with the retained native presentation and type identity. It SHALL NOT
alias Auto or UTF8. The ZIP writer SHALL NOT silently treat this value as its
existing UTF8-only mode; container behavior must follow retained native evidence.

#### Scenario: Original ZIP encoding expression resolves
- **WHEN** original UNF selects OSEncodingWithUTF8 by either member alias
- **THEN** it receives the native distinct enum value

#### Scenario: Existing member identity remains unchanged
- **WHEN** the new member is compared with Auto or UTF8
- **THEN** both comparisons are false

#### Scenario: Measured Linux archive encoding
- **WHEN** a ZIP writer on Linux uses OSEncodingWithUTF8
- **THEN** both headers retain the measured UTF8 filename bytes and Unicode Path extra field, with the UTF8 flag cleared

#### Scenario: Unmeasured OS codepage is not substituted
- **WHEN** a ZIP writer outside Linux requests OSEncodingWithUTF8
- **THEN** it reports that this encoding mode is unsupported instead of silently writing UTF8-only names

### Requirement: Reached formatted document file type enum
The runtime SHALL expose ТипФайлаФорматированногоДокумента and
FormattedDocumentFileType with ANSITXT, HTML, PDF and TXT members. Each alias
pair SHALL denote the same value, with the retained native member presentation
and type identity. Bare enum access and type lookup SHALL use the existing
system enum mechanism.

#### Scenario: Original formatted document format resolves
- **WHEN** original configuration code references any documented member
- **THEN** resolution returns the corresponding system enum value

#### Scenario: Native aliases and type identity
- **WHEN** the retained native enum control is replayed
- **THEN** member presentations, alias equality and type identity match the platform, allowing the existing interface language difference in type presentation

### Requirement: Document object filling check
Document objects SHALL expose zero-argument ПроверитьЗаполнение and CheckFilling
in qualified and own-object-module contexts. The method SHALL check acquired
fill-checking policies and invoke ОбработкаПроверкиЗаполнения in that object's
instance. It SHALL preserve the measured handler refusal and checked paths.
Unknown policies SHALL NOT be interpreted as DontCheck or replaced by assumed
platform defaults. Existing callable object-module exports SHALL remain callable.

#### Scenario: Native document baseline
- **WHEN** the retained native baseline document is checked in Russian, English or its own module
- **THEN** each call invokes the handler with Date in the checked paths and an empty date fails

#### Scenario: Filled date and handler refusal
- **WHEN** the native baseline document's standard fields are filled
- **THEN** the check succeeds unless the handler sets refusal

#### Scenario: Incomplete acquired checking policy
- **WHEN** required checking policies are unknown
- **THEN** execution reports the missing information rather than returning a fabricated success

#### Scenario: Own document attribute and handler mutation
- **WHEN** a required own string attribute is empty in the retained native document
- **THEN** checking fails and the handler receives Date before the own attribute
- **AND** filling the string or clearing the original checked-path array permits success

#### Scenario: Handler replaces the checked-path array
- **WHEN** the handler assigns a new array to its checked-path parameter
- **THEN** the original checked paths remain in effect as measured on the platform

#### Scenario: Required tabular column
- **WHEN** only a document tabular column has ShowError
- **THEN** an empty section passes, a row with an empty value fails, and a filled row passes
- **AND** the checked path names the section and column as retained in the native control

#### Scenario: Required tabular section
- **WHEN** both the section and a column have ShowError
- **THEN** an empty section fails and the handler receives the section path before its column path
- **AND** clearing the original checked-path array removes both checks

### Requirement: Acquired document checking metadata
The ORM metadata API SHALL preserve an optional complete list of standard
attribute checking policies for a document and an optional checking policy for
each tabular section. Unknown policies SHALL remain distinguishable from a
known empty list or a known DontCheck policy.

#### Scenario: Unknown and known policies remain distinct
- **WHEN** a host supplies no document policies, an empty standard policy list, or DontCheck for a section
- **THEN** the respective metadata accessors preserve each distinct state

### Requirement: Copy a document object
The ORM document object SHALL expose zero-argument Скопировать and Copy methods,
including calls without a receiver from its object module. Copy SHALL create a
new unsaved document of the same type, copy its own attributes and tabular rows,
and leave the source unchanged. The new document SHALL have an empty reference,
number and date, false deletion and posted flags, freshly initialized module
variables, and fresh additional properties. A matching object-module export
SHALL retain the existing dispatch precedence.

#### Scenario: Copy preserves data without sharing tabular rows
- **WHEN** a document with own attributes and tabular rows is copied
- **THEN** the copy contains those values and changing or clearing its rows does not alter the source

#### Scenario: Copy invokes the object event
- **WHEN** the object module declares ПриКопировании
- **THEN** the event executes on the copy after copying values, receives the source object, and sees freshly initialized module variables

### Requirement: Document object exported variables
Document objects SHALL expose exported object-module variables through the
existing configuration-instance variable access mechanism, after resolving
standard properties and acquired own attributes. Names SHALL be folded and
memory SHALL remain isolated between objects and sessions.

#### Scenario: Copy initializes independent exported variables
- **WHEN** the source exported variable is changed before Copy
- **THEN** the copy exposes its module-body initial value and the source retains its changed value

### Requirement: Document object template method declaration
Document objects SHALL declare ПолучитьМакет/GetTemplate with exactly one
required argument in qualified and own-object contexts, as in Syntax Assistant
8.3.27.2342. Until template resources are acquired and supported, invocation
SHALL fail explicitly as unsupported through the existing document template
path. Existing same-named object-module exports SHALL retain their dispatch.

#### Scenario: Compile an unexecuted object template call
- **WHEN** an object-module procedure contains ПолучитьМакет("Template")
- **THEN** compilation resolves the object method without introducing a global function

#### Scenario: Invoke without acquired template support
- **WHEN** that method is invoked without template resource support
- **THEN** execution reports the explicit unsupported error instead of fabricated template data

### Requirement: Imported procedure expression errors occur at invocation
A qualified call to an exported procedure of another configuration module in
an expression SHALL compile. When reached, it SHALL evaluate argument
expressions and then raise the catchable object-procedure-as-function error
before executing the procedure body. An unexecuted branch SHALL NOT raise it.
Unqualified own procedure diagnostics SHALL retain their existing behavior.

#### Scenario: Error preserves argument evaluation and skips the procedure
- **WHEN** an imported procedure is used as an expression with an effectful argument
- **THEN** the argument effect occurs, the procedure body does not run, and the enclosing exception handler catches the error

#### Scenario: Dormant invalid expression does not block initialization
- **WHEN** the imported procedure expression is in a branch that is not executed
- **THEN** the enclosing module can compile and execute without that error

### Requirement: Accounting record type enum
The runtime SHALL expose ВидДвиженияБухгалтерии/AccountingRecordType with
Дебет/Debit and Кредит/Credit members. Their presentations SHALL be Debit and
Credit, member types SHALL present as AccountingRecordType, and the bare enum
type SHALL present as EnumAccountingRecordType, matching the native control.
This declaration SHALL NOT imply accounting-register persistence support.

#### Scenario: Aliases identify the native enum values
- **WHEN** the Russian and English names of either member are evaluated
- **THEN** both identify the same value and its native presentation and type

### Requirement: Characteristic object deletion mark
A newly created characteristic object SHALL expose readable and writable
ПометкаУдаления/DeletionMark as one Boolean property, initially false.
Its module body SHALL be able to read it before selection but SHALL reject
writes in that phase. Selected objects SHALL support writes using the existing
Boolean conversion, with separate values for separate objects. The own-module
receiver context SHALL include both property names. This property SHALL NOT
imply database persistence or SetDeletionMark method support.

#### Scenario: Module body and selected object match native phases
- **WHEN** an object body reads and attempts to write the deletion mark, then a caller changes it after creation
- **THEN** the body reads false and catches a write error, while the caller's selected-object write succeeds

#### Scenario: Aliases and isolation preserve one instance value
- **WHEN** the Russian or English property is changed on one created object
- **THEN** both names observe that value and a second object retains false

### Requirement: Get XML type from the reader position
The XML component SHALL expose ПолучитьXMLТип/GetXMLType with one required XML
reader argument and return the existing XMLDataType value or Undefined. Its
reader movement, namespace resolution and xsi:type/xsi:nil handling SHALL match
the retained native controls. Initial state and whitespace SHALL advance to the
appropriate element; ordinary text or an end element SHALL return Undefined.
The function SHALL reuse the existing XML reader and XMLDataType representations.

#### Scenario: Resolve explicit type namespaces
- **WHEN** xsi:type names a declared prefix or an unprefixed name in the default namespace
- **THEN** the result contains the native local name and resolved namespace URI

#### Scenario: Undefined types preserve native refusal
- **WHEN** xsi:type uses an undeclared prefix, xsi:nil is true or 1, or the reader is on ordinary text
- **THEN** the function returns Undefined

#### Scenario: Infer element type after reader movement
- **WHEN** the reader is initial or on whitespace before an element
- **THEN** it advances as in the native control and derives the element type, including the XSD builtin-name rule

### Requirement: Load custom settings through their configuration handler
The ORM SHALL expose Загрузить/Load on acquired custom settings storage managers
with one required object key and three optional parameters: settings key,
settings description and user name. Calls in the storage's own manager module
SHALL use the same receiver method. The operation SHALL invoke the acquired
ОбработкаЗагрузки/LoadProcessing handler and return its final settings parameter.
Handler lookup SHALL be case-insensitive, prefer ОбработкаЗагрузки when both
names exist, and otherwise accept LoadProcessing. This priority SHALL not
depend on ScriptVariant or declaration order. Missing-handler diagnostics
SHALL retain the existing Russian-name fallback.
This SHALL execute original application code rather than substitute an empty
standard storage or fabricated setting. Omitted optional arguments SHALL match
retained native controls. Handler errors SHALL propagate to the caller. An uncaught
BSL exception, including explicit `ВызватьИсключение` and division by zero,
inside `ОбработкаЗагрузки`
SHALL terminate execution with a typed critical-configuration error visible to
the host, bypassing enclosing BSL exception handlers. The original cause SHALL
remain available. The owning facade State SHALL reject subsequent execution
entries, including pollable execution and calls on existing module instances.
A fresh State of the same Engine SHALL remain usable. This requirement does
not reclassify exceptions outside that handler, cancellation or module-compilation errors.
The host entry policy SHALL apply critical promotion only after target
preparation succeeds. It SHALL preserve the existing type and catchability of
preparation errors rather than promote them to critical handler failures.
This host API rule SHALL NOT imply that a 1C settings-manager module permits
module-body operators.

#### Scenario: Native defaults and explicit user
- **WHEN** the retained native probe loads an object with omitted optional
  arguments or with an explicit settings key and user
- **THEN** the handler inputs and returned settings match the native output

#### Scenario: Own manager context
- **WHEN** an exported function of a settings storage manager calls Load
  without qualifying the receiver
- **THEN** its own storage handler supplies the result

#### Scenario: Raised load-handler error terminates the owning State
- **WHEN** the acquired load handler explicitly raises an exception inside an enclosing BSL try/catch
- **THEN** the host receives a typed critical-configuration error containing the original cause
- **AND** the BSL catch block and subsequent statements do not execute
- **AND** later execution entries on the same State fail before executing BSL

#### Scenario: English custom load handler
- **WHEN** a manager defines only LoadProcessing, including another letter case
- **THEN** Load invokes that handler and returns its final settings parameter

#### Scenario: Both handler names are defined
- **WHEN** a manager defines both handler names in either order
- **THEN** Load invokes ОбработкаЗагрузки for Russian and English ScriptVariant

#### Scenario: Arithmetic handler failure is critical
- **WHEN** the acquired load handler divides by zero inside an enclosing BSL try/catch
- **THEN** the host receives a typed critical-configuration error with the arithmetic cause
- **AND** the BSL catch and continuation do not execute

#### Scenario: Host target preparation preserves the original error
- **GIVEN** a host-defined module recipe whose preparation returns an ordinary Raised error
- **WHEN** the host requests critical entry into its handler
- **THEN** preparation retains the original Raised result rather than becoming a critical handler failure
- **AND** unrelated later host execution is not rejected by the critical-State latch
- **AND** this synthetic host-module case is not a claim about valid 1C manager source

#### Scenario: Critical failure does not poison another State
- **WHEN** a custom load handler critically fails in one State
- **THEN** a fresh State of the same Engine can execute a normal load
- **AND** ordinary BSL exceptions outside that handler remain catchable

#### Scenario: Dynamic expression compilation fails inside the handler body
- **GIVEN** the retained LoadProcessing body calling Evaluate with an unknown function inside its String expression
- **WHEN** the storage invokes that handler and dynamic expression compilation fails
- **THEN** the host receives a critical-configuration error retaining the dynamic-expression cause, the outer BSL catch does not run and State reentry fails
- **AND** this does not classify preparation failures of the manager module as handler execution errors

### Requirement: Create an acquired report object
The ORM SHALL expose Создать/Create with zero arguments on acquired report
managers, including their own manager-module context. Every successful call
SHALL create a distinct object with its acquired ReportObject type, independent
module-variable memory and independent tabular rows. Original object-module
body code SHALL run once for that instance; exported methods and ЭтотОбъект
SHALL use the same instance. The retained native report controls SHALL match
for complete string/boolean field metadata. Unknown own-attribute or tabular
section composition SHALL cause an explicit error instead of a fabricated
empty object. Existing same-named manager exports SHALL retain their dispatch.

#### Scenario: Native instance lifecycle
- **WHEN** the retained native report control creates two objects and mutates
  one object's module variable and tabular rows
- **THEN** the body, receiver identity, own-field values and independent state
  match the native controls

#### Scenario: Own manager Create
- **WHEN** an exported report-manager function calls Create without a receiver
- **THEN** it creates an object of its own acquired report

#### Scenario: Unknown field composition
- **WHEN** the report's full own-attribute or tabular-section collection is unknown
- **THEN** creation fails explicitly and does not invent empty collections

### Requirement: Report settings composer is declared explicitly

Report object modules SHALL resolve the standard read-only property
`КомпоновщикНастроек` / `SettingsComposer` through their own receiver context.
Until the settings model is implemented, reading the property SHALL raise
the existing explicit unsupported-settings-model error rather than return
an empty or unrelated object. This declaration SHALL NOT be reported as
implemented settings-composer execution support.

#### Scenario: Dormant report handler

- **WHEN** a report handler contains a standard SettingsComposer reference
- **THEN** the original handler compiles without replacing its source
- **AND** a handler that is not called does not read the property

#### Scenario: Reachable unsupported settings read

- **WHEN** a report reads either property spelling before the settings model exists
- **THEN** the existing unsupported-settings-model error is catchable

#### Scenario: Read-only report property

- **WHEN** code assigns either property spelling
- **THEN** assignment is rejected as read-only

### Requirement: Report data composition schema is declared explicitly

Report object modules SHALL resolve the standard read/write property
`СхемаКомпоновкиДанных` / `DataCompositionSchema` through their own receiver.
Until acquisition and the schema model are implemented, reading or assigning
this property SHALL raise the existing explicit unsupported-schema-model
error. The property SHALL NOT be declared read-only or return fabricated
absence when the actual report schema is unknown.

#### Scenario: Dormant schema handler

- **WHEN** original report code refers to either standard schema property name
- **THEN** the module compiles without rewriting its source

#### Scenario: Unsupported schema access

- **WHEN** a report reads or writes the schema property before its model exists
- **THEN** the existing unsupported-schema-model error is catchable
- **AND** the property descriptor records read/write availability

### Requirement: Acquired characteristic predefined manager names

The consumer SHALL project acquired predefined names of a chart of
characteristic types into its manager properties and own-module context.
A predefined GUID SHALL NOT substitute for the database record UUID.
Resolution SHALL use the current session data adapter and validate the
returned reference owner, kind and configuration identity, as for catalogs.
Local and module variable priorities SHALL remain unchanged.

#### Scenario: Native manager context

- **WHEN** a characteristic manager contains a bare acquired predefined name
- **THEN** compilation resolves it through the own manager
- **AND** execution returns the same reference as the qualified manager property

#### Scenario: Explicit local shadow

- **WHEN** a function explicitly declares a local variable with that name
- **THEN** the local value is returned without querying the predefined record

#### Scenario: Global predefined lookup

- **WHEN** code calls PredefinedValue with a characteristic predefined path
- **THEN** the same manager property and current session adapter resolve it

#### Scenario: Read-only and incompatible result

- **WHEN** code assigns an acquired predefined property or the adapter returns an incompatible reference
- **THEN** the operation fails explicitly

### Requirement: Native text position relative to picture enum

The runtime SHALL expose `ПоложениеТекстаОтносительноКартинки` /
`TextPositionRelativeToPicture` with Auto, OnTop, Top, Left, Bottom and
Right and their documented Russian names. Member, member-type, bare
container and container-type presentations SHALL match the retained
native platform control exactly. Both type names SHALL resolve identically.

#### Scenario: Native enum control

- **WHEN** original BSL reads any of the six members through either alias
- **THEN** it receives the same corresponding enum value
- **AND** String presentations match the measured nine-line control

### Requirement: Document manager lookup by acquired attribute

The ORM SHALL expose ДокументМенеджер.НайтиПоРеквизиту / DocumentManager.FindByAttribute with exactly two required arguments, including an unqualified call inside the own manager module. It SHALL resolve the acquired own attribute name without case sensitivity, pass the current session principal and request to its data adapter, return a typed empty reference for no match, and reject incompatible adapter references. Missing attribute composition SHALL fail explicitly. The database adapter SHALL reuse its existing rights/RLS-aware query path and acquired field types for document lookup, without catalog parent or owner filters.

#### Scenario: Native own-manager lookup and missing record
- **GIVEN** the retained thirteen-control native document lookup measurement and complete own attributes
- **WHEN** the same lookup calls execute through a session with a data adapter
- **THEN** Russian, English, own-manager and case-varied attribute calls return the measured document reference, missing matches return the typed empty reference, and invalid attributes or arity fail
- **AND** changing the session principal or adapter does not change another session's state

#### Scenario: Unknown or incompatible lookup data
- **WHEN** document own attributes are unknown, the adapter fails, or its result belongs to another metadata owner, kind or configuration
- **THEN** lookup fails explicitly instead of returning an invented document reference

### Requirement: Reached database data size function binding

The configuration runtime SHALL declare ПолучитьРазмерДанныхБазыДанных / GetDatabaseDataSize as a global function with zero to three optional arguments, so original dormant handlers can bind the vendor API. Until the principal-bound database size operation is implemented and checked against native data, execution SHALL fail explicitly and SHALL NOT return a fabricated size, a whole-table physical size, or an unfiltered aggregate.

#### Scenario: Original dormant register handler
- **WHEN** an information-register manager declares the original call with an omitted first argument and an array of metadata objects as its second argument
- **THEN** the module binds through the existing ORM component function table
- **AND** executing the call fails explicitly while data-size support is incomplete

#### Scenario: Finite optional argument contract
- **WHEN** Russian, English or case-varied names use zero, one, two or three arguments
- **THEN** those calls bind with the same finite optional signature
- **AND** a fourth argument fails before execution

### Requirement: Configuration WS reference global

The ORM library SHALL bind `WSСсылки` / `WSReferences` through its existing registered global-name path, preserving local and module variable priority and standalone behavior without the library. Unknown WS configuration composition SHALL fail explicitly when the global executes, rather than fabricate an empty manager, a named reference or a functional SOAP proxy. Binding a dormant original source expression SHALL NOT be reported as implementing WS reference execution.

#### Scenario: Dormant original reference expression
- **GIVEN** a configuration module containing the retained UNF reference to WSСборОтчетностиРосстата
- **WHEN** the module is compiled without invoking that function
- **THEN** the WSReferences global is bound without source rewriting

#### Scenario: WS acquisition is not implemented
- **GIVEN** no supported acquired WS reference composition
- **WHEN** the global is evaluated
- **THEN** execution reports an explicit unsupported WS configuration error

#### Scenario: A variable shadows the global
- **GIVEN** a local or module variable with the same Russian or English name
- **WHEN** the identifier is resolved
- **THEN** it resolves to that variable through the existing resolver order

### Requirement: Catalog object new code event

The configuration code snapshot SHALL retain whether an object's final module source came from the base configuration or an extension. Database loading SHALL mark actual extension source replacement, including adopted objects whose module name and owner still identify the base object, and preserve base origin when no replacement source exists. ORM object-module binding SHALL retain this origin. Origin SHALL NOT be inferred from a logical module name, owner identity or extension membership alone.

#### Scenario: Adopted object receives extension source
- **GIVEN** a base object's module and an extension with replacement source
- **WHEN** the loader replaces the module under its existing logical identity
- **THEN** its final source is marked as extension source and ORM binding retains that mark

#### Scenario: Extension has no replacement source
- **GIVEN** a base object's module and an extension without module source for that object
- **WHEN** the final snapshot and ORM binding are constructed
- **THEN** the unchanged module retains base source origin

Catalog objects SHALL expose `УстановитьНовыйКод` / `SetNewCode` with zero or one prefix argument, including the own-module receiver context. A selected object's base configuration event SHALL receive initially true standard processing and the supplied prefix, or an empty string when omitted. By-reference changes SHALL determine cancellation and the prefix used for allocation. Cancellation SHALL retain the event's assigned code. Base configuration event execution SHALL suppress safe mode for the event and restore the caller's effective modes on success and error. This exception SHALL NOT be applied to extension handlers. Calling before object selection SHALL fail. Standard allocation SHALL use the current Session's principal-bound adapter and existing code assignment validation; missing allocation support SHALL fail explicitly rather than inventing a code.

#### Scenario: Cancel generation in the base configuration event
- **GIVEN** a selected catalog object with the retained native event
- **WHEN** SetNewCode receives `P-` and the event cancels standard processing
- **THEN** the event receives true and `P-` and the assigned `manual` code remains

#### Scenario: English event spelling
- **GIVEN** the retained native module declaring `OnSetNewCode`
- **WHEN** SetNewCode invokes the selected object's event
- **THEN** cancellation, final prefix, initial parameters and base-event safe-mode restoration reproduce the same seven native controls as the Russian spelling

#### Scenario: One-parameter numeric-code event
- **GIVEN** a numeric-code catalog with the retained one-parameter event
- **WHEN** SetNewCode is called with no prefix or with the retained string prefix
- **THEN** the event receives initially true standard processing and its cancellation preserves code 42 without requiring an allocation adapter
- **AND** all four native numeric-event lines replay

#### Scenario: Change the allocation prefix
- **GIVEN** an event that changes the prefix to `H-` without cancellation
- **WHEN** the caller omits the prefix
- **THEN** the event initially receives an empty string and the current Session adapter receives `H-`
- **AND** its result is assigned through the existing code setter

#### Scenario: Restore safe mode after a base event
- **GIVEN** a caller with safe mode enabled
- **WHEN** its base configuration event executes or raises an exception
- **THEN** the event observes safe mode disabled and the caller's effective safe mode remains enabled after return or propagation

For the retained own-extension catalog control, event entry SHALL use the
acquired extension safe-mode setting rather than treating source origin as a
sufficient execution policy. A false setting SHALL run the event outside safe
mode; a true setting SHALL run it in safe mode even when the caller has enabled
safe-mode suppression. The caller's effective safe mode and suppression state
SHALL remain unchanged after return. Unknown extension settings SHALL NOT be
silently interpreted as false. This rule for an own-extension event SHALL NOT
be generalized to an adopted module's interceptors from source origin alone.

#### Scenario: Own extension setting controls event entry
- **GIVEN** the retained own-extension catalog event and its acquired Boolean safe-mode setting
- **WHEN** callers with safe mode disabled or enabled invoke SetNewCode
- **THEN** the event observes the extension's setting in both cases
- **AND** the caller retains its prior effective mode

#### Scenario: Own extension event and caller suppression
- **GIVEN** the retained own-extension catalog event and a caller that suppresses safe mode
- **WHEN** the extension's acquired safe-mode setting is true
- **THEN** the event still observes safe mode enabled
- **AND** the caller's suppression state remains enabled after the event returns
- **AND** the caller's parent frame retains its original state after the caller returns

#### Scenario: No allocator is supplied
- **GIVEN** standard processing remains enabled and the Session has no supported code allocator
- **WHEN** SetNewCode executes
- **THEN** it reports an explicit unsupported allocation error without a local counter or fabricated code

### Requirement: Catalog object hierarchy level

Catalog objects SHALL expose Уровень / Level with no arguments, including an own-module unqualified call. For a selected object the method SHALL use its current in-memory parent and the existing principal-bound query adapter for acquired ancestor references, without modifying records or using another session's state. Native root, one-parent, two-parent, reset and missing-reference controls SHALL replay. Calling the method from the module body before element selection SHALL fail as measured. Unknown hierarchy settings, malformed ancestor results and cyclic adapter graphs SHALL fail explicitly. The unmeasured treatment of an ancestor hidden by RLS SHALL remain in the platform question registry until a native restricted-principal control resolves it.

#### Scenario: Native selected-object levels and body guard
- **GIVEN** the retained native catalog level controls
- **WHEN** a selected object's current parent changes between empty, a root folder and a nested folder
- **THEN** levels are 0, 1 and 2 respectively, with aliases and own-module context agreeing
- **AND** the own module body cannot query the level before element selection

#### Scenario: Session adapter and hierarchy completeness
- **WHEN** an ancestor read uses the current session adapter
- **THEN** its principal and bound reference remain specific to that session
- **AND** unknown hierarchy, incompatible values or a cycle are errors rather than invented levels

#### Scenario: Preexisting object export named Level
- **GIVEN** a previously supported object-module export named Уровень or Level with its own arity
- **WHEN** the new platform method is declared
- **THEN** external calls preserve that export's behavior and argument contract, following the existing additive object-method policy


### Requirement: Hosts construct event-log entries through the public API
The Session crate SHALL allow a host adapter to construct an event-log entry
with an explicit event name, level, metadata value, data value, comment and
transaction mode. The existing read-only accessors SHALL expose those values
unchanged. Construction SHALL neither deliver the entry nor select a Session
writer: delivery remains the responsibility of the caller using the writer
bound to the current Session.

#### Scenario: An adapter constructs an entry for its Session writer
- **WHEN** a host constructs an entry with explicit values and passes it to its Session writer
- **THEN** the writer receives those same values through the existing entry accessors
- **AND** constructing another entry does not itself invoke any writer
