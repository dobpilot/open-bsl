## ADDED Requirements

### Requirement: Reusable session factory
`bsl-session` SHALL provide a factory that combines one compiled configuration code snapshot with one immutable ORM metadata catalog. Factory construction SHALL reject a different configuration identity or a manager module linked to an absent metadata object. It SHALL perform no per-session `Config` fingerprint computation.

#### Scenario: Compatible code and metadata
- **WHEN** code and metadata identify the same configuration and every linked object exists
- **THEN** the factory is created once and may open multiple isolated sessions

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

### Requirement: Role availability
The ORM library SHALL provide `РольДоступна` and `IsInRole` with one argument. A string role name or metadata role object SHALL answer whether that role belongs to the current principal. An unknown role, an invalid value, or an invalid argument type SHALL raise an error.

#### Scenario: Assigned role
- **WHEN** code asks for a role assigned to the session principal
- **THEN** the function returns `Истина`

#### Scenario: Invalid role name
- **WHEN** code asks for a role absent from the configuration
- **THEN** the call raises an error

### Requirement: Additive database CLI mode
`bsl-cli` SHALL enable configuration-session execution only when `--config-db` is present. Without that option every existing command, script invocation, argument, and REPL behavior SHALL remain unchanged. `--principal` SHALL be accepted only with `--config-db`.

#### Scenario: Ordinary script
- **WHEN** the CLI is invoked with an existing script command and no `--config-db`
- **THEN** it follows the existing execution path without loading a configuration or opening a database session

#### Scenario: Configuration test script
- **WHEN** the CLI is invoked with `--config-db`, a valid principal, and a test script
- **THEN** it starts the configuration session before executing that script in its context
