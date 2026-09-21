## ADDED Requirements

### Requirement: Reusable session factory
`bsl-session` SHALL provide a factory that combines one compiled configuration code snapshot with one immutable ORM metadata catalog. Factory construction SHALL reject a different configuration identity or a manager module linked to an absent metadata object. It SHALL perform no per-session `Config` fingerprint computation.

#### Scenario: Compatible code and metadata
- **WHEN** code and metadata identify the same configuration and every linked object exists
- **THEN** the factory is created once and may open multiple isolated sessions

#### Scenario: Matching snapshots from different providers
- **WHEN** a code snapshot and an ORM metadata catalog come from different physical providers but identify the same configuration and linked objects
- **THEN** factory construction applies the same compatibility checks
- **AND** it does not require the providers to match

#### Scenario: Incompatible manager owner
- **WHEN** a code snapshot links a manager module to an object absent from the metadata catalog
- **THEN** factory construction fails before any session starts

### Requirement: Failure-atomic session startup
Every session SHALL own a separate database connection, open-bsl State, module instances, parameter container, and principal snapshot. Startup SHALL invoke the configuration session handler with `Неопределено`. A session SHALL be returned only after that call succeeds.

#### Scenario: Successful startup
- **WHEN** the startup handler returns normally
- **THEN** the caller receives a session whose module and parameter state persists for later calls

#### Scenario: Startup failure
- **WHEN** connection setup or the startup handler fails
- **THEN** no usable session is returned

### Requirement: Explicit selectable principal
A non-empty `v8users` catalog SHALL expose only users with at least one enabled authentication method as selectable principals. Selection SHALL accept a case-insensitive name or exact GUID. A single selectable user MAY be chosen automatically; multiple selectable users SHALL require an explicit choice. An empty user catalog SHALL produce the platform empty user with the configuration's main roles. A principal description SHALL remain unchanged for the lifetime of its session.

#### Scenario: User cannot authenticate
- **WHEN** a caller selects an existing user with no enabled authentication method
- **THEN** session creation fails as a forbidden principal rather than an unknown name

#### Scenario: Several selectable users
- **WHEN** the CLI starts a configuration session without `--principal` and several users are selectable
- **THEN** it fails and lists the selectable names

### Requirement: Session launch mode
`bsl-session` SHALL expose `ТекущийРежимЗапуска` and `CurrentRunMode` with no arguments. A web-client or thin-client session SHALL return `РежимЗапускаКлиентскогоПриложения.УправляемоеПриложение`. A background-job session SHALL return `Неопределено`, as measured on platform 8.3.27.2342.

#### Scenario: Managed client session
- **WHEN** configuration code calls `ТекущийРежимЗапуска()` in a web-client or thin-client session
- **THEN** it receives `РежимЗапускаКлиентскогоПриложения.УправляемоеПриложение`

#### Scenario: Background session
- **WHEN** configuration code calls `ТекущийРежимЗапуска()` in a background-job session
- **THEN** it receives `Неопределено`

### Requirement: Platform version snapshot
`SessionFactory` SHALL require one non-empty host-supplied platform version and SHALL reuse it for every session it opens. `bsl-session` SHALL provide the zero-argument `СистемнаяИнформация` and `SystemInfo` constructors. Their `ВерсияПриложения` and `AppVersion` read-only property SHALL return the factory's platform version.

#### Scenario: Version is available during startup
- **WHEN** the session handler constructs `СистемнаяИнформация` during startup
- **THEN** `ВерсияПриложения` returns the version supplied to its factory

#### Scenario: Missing version
- **WHEN** a caller tries to construct a session factory with an empty platform version
- **THEN** factory construction fails before any session starts

### Requirement: Role availability
The ORM library SHALL provide `РольДоступна` and `IsInRole` with one argument. A string role name or metadata role object SHALL answer whether that role belongs to the current principal. An unknown role, an invalid value, or an invalid argument type SHALL raise an error.

#### Scenario: Assigned role
- **WHEN** code asks for a role assigned to the session principal
- **THEN** the function returns `Истина`

#### Scenario: Invalid role name
- **WHEN** code asks for a role absent from the configuration
- **THEN** the call raises an error

### Requirement: Configuration extension snapshot
The ORM library SHALL provide the global `РасширенияКонфигурации` and `ConfigurationExtensions` manager. Its `Получить` and `Get` method SHALL accept an optional filter structure containing `УникальныйИдентификатор`/`UUID` and/or `Имя`/`Name`, plus an optional `ИсточникРасширенийКонфигурации`/`ConfigurationExtensionsSource`. The source enum SHALL contain `БазаДанных`/`Database`, `СеансАктивные`/`SessionApplied`, and `СеансОтключенные`/`SessionDisabled`; the default SHALL be `БазаДанных`/`Database`. Returned extension objects SHALL expose the properties needed by the surveyed startup paths, including identity, version, hash, active and safety flags, distributed-infobase use, and scope. The scope enum SHALL contain `ИнформационнаяБаза`/`InfoBase` and `РазделениеДанных`/`DataSeparation`.

#### Scenario: Filter the applied session extensions
- **WHEN** code requests `СеансАктивные` with a filter naming one applied extension
- **THEN** it receives an array containing only the matching extension snapshot

#### Scenario: Complete configuration without extensions
- **WHEN** acquisition explicitly records a complete empty extension view
- **THEN** `Получить()` returns an empty array

#### Scenario: Extension information is unavailable
- **WHEN** the ORM catalog has no complete extension view for the session
- **THEN** `Получить()` raises an explicit capability error instead of returning an empty array

### Requirement: Session extension application issues
`РасширенияКонфигурации.ПолучитьИнформациюОПроблемахПримененияВСеансе()` and `ConfigurationExtensions.GetSessionApplicationIssuesInformation()` SHALL return the application issues recorded for the current session. Each issue SHALL expose its related extension, description, and severity. An explicitly complete empty issue snapshot SHALL return an empty array; unavailable issue information SHALL raise an explicit capability error.

#### Scenario: Session has one application issue
- **WHEN** acquisition records an issue for an extension used by the session
- **THEN** the method returns an issue whose `Расширение`/`Extension` property refers to that extension snapshot

### Requirement: Additive database CLI mode
`bsl-cli` SHALL enable configuration-session execution only when `--config-db` is present. Without that option every existing command, script invocation, argument, and REPL behavior SHALL remain unchanged. `--principal` SHALL be accepted only with `--config-db`.

#### Scenario: Ordinary script
- **WHEN** the CLI is invoked with an existing script command and no `--config-db`
- **THEN** it follows the existing execution path without loading a configuration or opening a database session

#### Scenario: Configuration test script
- **WHEN** the CLI is invoked with `--config-db`, a valid principal, and a test script
- **THEN** it starts the configuration session before executing that script in its context
