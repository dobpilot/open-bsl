## ADDED Requirements

### Requirement: Reusable session factory
`bsl-session` SHALL provide a factory that combines one compiled configuration code snapshot with one immutable ORM metadata catalog. Factory construction SHALL reject a different configuration identity or a manager module linked to an absent metadata object. It SHALL perform no per-session `Config` fingerprint computation.

The factory SHALL compile the session startup graph in the server context. It SHALL include server common modules and SHALL exclude common modules available only in client or external-connection contexts. Duplicate global exports SHALL be checked only among modules selected into that server graph.

The session crate SHALL obtain the standard platform components through the `open-bsl` facade and SHALL NOT depend directly on their implementation crates.

The `open-bsl` facade SHALL allow a host to name compilation roots for a configuration recipe. When roots are present, engine construction SHALL compile the transitive static module closure reported by the existing semantic resolver and SHALL retain explicitly declared imports. An empty root selection SHALL preserve whole-catalog compilation. An unknown root SHALL fail engine construction. Global-export conflicts SHALL be checked before unreachable bodies are omitted.

#### Scenario: Unreachable server module has an unresolved body
- **WHEN** the selected server catalog contains a module outside the session root's static closure whose body has an unresolved name
- **THEN** session factory construction succeeds without resolving or generating code for that body

#### Scenario: Reachable module dependency
- **WHEN** the session root calls an export of another selected server module
- **THEN** that module and its transitive static dependencies are compiled into the session catalog

#### Scenario: Unknown compilation root
- **WHEN** a host names a compilation root absent from the recipe
- **THEN** engine construction fails with a configuration error naming that root

#### Scenario: Existing facade component in a server module
- **WHEN** a selected server module constructs a type already provided by a standard `open-bsl` component
- **THEN** factory construction resolves that constructor through the facade registry
- **AND** `bsl-session` introduces no direct dependency on the component crate

#### Scenario: Client and server globals share an export name
- **WHEN** a client-only global common module and a server global common module export the same folded name
- **THEN** factory construction succeeds
- **AND** server session code resolves the name to the server module

#### Scenario: Two server globals share an export name
- **WHEN** two server global common modules export the same folded name
- **THEN** factory construction fails with the existing duplicate-global diagnostic

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

### Requirement: Database acquisition adapter
`bsl-config-db` SHALL derive the source-neutral code snapshot and immutable ORM metadata catalog from one acquired database configuration. Both results SHALL carry the same configuration identity. Catalog construction SHALL preserve the configuration objects, roles, session parameters, root properties, and complete extension snapshot required by session startup, and SHALL happen once before a reusable `SessionFactory` is built.

The adapter SHALL convert information-base users into `PrincipalCandidate` values using `InfoBaseUser::can_authenticate()` as the authentication decision and preserving the user's stable identifier, name, and assigned role identifiers. `bsl-orm` and `bsl-session` SHALL NOT parse database storage formats.

#### Scenario: One acquisition supplies factory inputs
- **WHEN** a database configuration is loaded for session execution
- **THEN** its code snapshot and ORM catalog are derived from that acquisition
- **AND** `SessionFactory` accepts their matching configuration identities

#### Scenario: Selectable database user
- **WHEN** an acquired user has an enabled authentication method and assigned roles
- **THEN** the adapter produces a selectable principal candidate with the same identifier, name, and role identifiers

#### Scenario: User without authentication
- **WHEN** an acquired user has no enabled authentication method
- **THEN** the adapter preserves the candidate but marks it as not selectable
- **AND** it does not infer eligibility from administrative or display flags

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

### Requirement: Configuration compatibility mode
The immutable ORM catalog SHALL store the configuration compatibility mode. `Метаданные.РежимСовместимости` and `Metadata.CompatibilityMode` SHALL return that value with the measured `Compatibility mode` value type. `Метаданные.СвойстваОбъектов.РежимСовместимости` and `Metadata.ObjectProperties.CompatibilityMode` SHALL return a value whose measured type is `Metadata object enumerated property` and whose presentation is `MetadataObjectsPropertyEnumed`; that value SHALL provide case-insensitive indexed access to the compatibility-mode member names documented for the supported platform version. The ORM library SHALL NOT declare a separate bare global named `РежимСовместимости`.

#### Scenario: Read the configured mode
- **WHEN** the catalog records the `DontUse` compatibility mode
- **THEN** `Строка(ТипЗнч(Метаданные.РежимСовместимости))` returns `Compatibility mode`
- **AND** `Строка(Метаданные.РежимСовместимости)` returns `DontUse`

#### Scenario: Compare through metadata object properties
- **WHEN** code indexes `Метаданные.СвойстваОбъектов.РежимСовместимости` by the documented name of the configured mode in either letter case
- **THEN** it receives the same compatibility-mode value
- **AND** the indexed property's type and presentation match the measured platform values

#### Scenario: Bare name remains unavailable
- **WHEN** no other library declares a global named `РежимСовместимости`
- **THEN** a bare reference retains the existing undefined-variable diagnostic
- **AND** `Тип("РежимСовместимости")` and `Тип("CompatibilityMode")` remain unavailable

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

The database form SHALL be `--config-db <postgres|mssql>` together with
`--db-host`, `--db-name`, `--db-user`, and `--platform-version`. `--db-port`
MAY override the provider default. Database passwords SHALL come from
`PGPASSWORD` or `MSSQL_PASSWORD` and SHALL NOT be accepted as command-line
arguments. PostgreSQL plaintext and an unverified SQL Server certificate
SHALL require explicit provider-specific opt-in flags.

#### Scenario: Ordinary script
- **WHEN** the CLI is invoked with an existing script command and no `--config-db`
- **THEN** it follows the existing execution path without loading a configuration or opening a database session

#### Scenario: Configuration test script
- **WHEN** the CLI is invoked with `--config-db`, a valid principal, and a test script
- **THEN** it starts the configuration session before executing that script in its context

#### Scenario: Password on the command line
- **WHEN** a database-mode invocation supplies a password-shaped option
- **THEN** argument validation rejects it before a connection is attempted

#### Scenario: Principal without database mode
- **WHEN** an invocation supplies `--principal` without `--config-db`
- **THEN** argument validation rejects it without changing ordinary script execution
