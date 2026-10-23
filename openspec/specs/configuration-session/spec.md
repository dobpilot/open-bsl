# configuration-session Specification

## Purpose
Define the acquired configuration facts and isolated runtime resources required to initialize and execute a configuration Session without inferred platform defaults.

## Requirements

### Requirement: Acquired extension description completeness
The ORM catalog SHALL retain a complete provider-neutral description of every
acquired database extension: identity, name, version, hash, safe-mode settings,
distributed-infobase use, activity, scope and platform order. Applied and
disabled session collections SHALL be derived from explicit acquired activity.
No missing property SHALL be replaced with an inferred platform default.

#### Scenario: Active and inactive acquired extensions
- **WHEN** acquisition returns one active and one inactive extension with complete properties
- **THEN** both appear in the database collection
- **AND** the active extension appears only in the applied collection and the inactive extension only in the disabled collection

#### Scenario: Incomplete extension description
- **WHEN** acquisition omits a property required by the public extension object
- **THEN** catalog construction fails explicitly instead of fabricating a value

### Requirement: Reached server administration constructor declaration
The session library SHALL declare АдминистрированиеСервера and ServerAdministration as aliases of the same constructor accepting zero through two arguments. Compilation SHALL use the existing component registry and SHALL NOT execute a constructor merely because an unused function mentions it. While ras transport is unavailable, invoking this constructor SHALL fail explicitly and SHALL NOT return a fabricated administration object or claim a network connection.

#### Scenario: Unused constructor in a common module
- **GIVEN** a registered session library and a common module containing an unused function constructing АдминистрированиеСервера
- **WHEN** a separate valid export runs
- **THEN** compilation and the valid export SHALL succeed without constructing a ras object

#### Scenario: Aliases and optional arguments
- **WHEN** a program contains either constructor name with zero, one or two arguments
- **THEN** semantic compilation SHALL accept the declaration
- **AND** actual invocation without implemented ras transport SHALL fail explicitly

#### Scenario: Excess arguments
- **WHEN** either constructor is supplied three arguments
- **THEN** semantic compilation SHALL reject the arity through the existing constructor diagnostic

### Requirement: Missing imported methods fail at common-module method entry
With the existing common-module name-error policy enabled, an unresolved
method call to a known imported common module in a common-module function or
procedure body SHALL NOT prevent valid neighboring methods from running.
Calling the affected method SHALL raise a catchable error before its body
or call arguments execute. Valid neighboring methods SHALL remain usable.

#### Scenario: Native missing-method receipt
- **WHEN** a valid parent export is called before and after an export that references a missing child method
- **THEN** both valid calls return their native result
- **AND** the affected call raises a catchable error without either body or argument markers

#### Scenario: Strict default
- **WHEN** the common-module opt-in is disabled
- **THEN** the same unresolved imported method retains its existing semantic error

#### Scenario: Other source contexts
- **WHEN** an object, manager or explicit session module contains the same unresolved imported call
- **THEN** its existing strict diagnostics remain unchanged

#### Scenario: Unrelated unknown names
- **WHEN** a common module references an unknown bare function or variable
- **THEN** its existing diagnostics and deferred whole-module policy remain unchanged

### Requirement: Reusable session factory
`bsl-session` SHALL provide a factory that combines one compiled configuration code snapshot with one immutable ORM metadata catalog. Factory construction SHALL reject a different configuration identity or a manager module linked to an absent metadata object. It SHALL perform no per-session `Config` fingerprint computation.

The factory SHALL compile the session startup graph in the server context. It SHALL include server common modules and SHALL exclude common modules available only in client or external-connection contexts. Duplicate global exports SHALL be checked only among modules selected into that server graph.

The session crate SHALL obtain the standard platform components through the `open-bsl` facade and SHALL NOT depend directly on their implementation crates.

The `open-bsl` facade SHALL allow a host to name compilation roots for a configuration recipe. For in-memory recipes, roots SHALL retain the existing transitive static body closure. For source-backed catalogs, roots and explicitly declared imports SHALL prepare their own bodies; ordinary static references SHALL require immutable declarations and SHALL NOT by themselves prepare target bodies. An empty root selection SHALL preserve whole-catalog compilation. An unknown root SHALL fail engine construction. Global-export conflicts SHALL be checked before unreachable bodies are omitted.

#### Scenario: Unreachable server module has an unresolved body
- **WHEN** the selected server catalog contains a module outside the session root's static closure whose body has an unresolved name
- **THEN** session factory construction succeeds without resolving or generating code for that body

#### Scenario: Reachable module dependency
- **WHEN** an in-memory recipe session root calls an export of another selected server module
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

The host SHALL invoke the compiled session module's handler even when the
procedure has no `Экспорт` declaration. Initial and selective parameter
initialization SHALL use the same session-owned catalog module instance. This
host entry SHALL NOT make the procedure callable as an export from another
BSL module. The `open-bsl` facade SHALL expose a host-only call to that
session-owned configuration handler.

#### Scenario: Non-exported platform session handler
- **WHEN** a configuration session module declares `УстановкаПараметровСеанса` without `Экспорт`
- **THEN** session startup calls it with `Неопределено` and retains its parameter writes
- **AND** an ordinary BSL module still cannot call that procedure as an export

#### Scenario: Selective initialization reenters the same private handler
- **WHEN** the startup handler reads an unset session parameter
- **THEN** the parameter container invokes that handler again with the required name through the same module instance
- **AND** no `Экспорт` declaration is required for either invocation

#### Scenario: Successful startup
- **WHEN** the startup handler returns normally
- **THEN** the caller receives a session whose module and parameter state persists for later calls

#### Scenario: Startup failure
- **WHEN** connection setup or the startup handler fails
- **THEN** no usable session is returned

### Requirement: Database acquisition adapter
`bsl-config-db` SHALL derive the source-neutral code snapshot and immutable ORM metadata catalog from one acquired database configuration. Both results SHALL carry the same configuration identity. Catalog construction SHALL preserve the configuration objects, roles, session parameters, root properties, and complete extension snapshot required by session startup, and SHALL happen once before a reusable `SessionFactory` is built.

The adapter SHALL convert information-base users into `PrincipalCandidate` values using `InfoBaseUser::can_authenticate()` as the authentication decision and preserving the user's stable identifier, name, and assigned role identifiers. `bsl-orm` and `bsl-session` SHALL NOT parse database storage formats.

For session startup, the adapter SHALL select the `Config` resources needed
by its code and ORM projections after reading the configuration root. It
SHALL NOT retain unrelated `Config` resources merely to discard them after
acquisition.

#### Scenario: One acquisition supplies factory inputs
- **WHEN** a database configuration is loaded for session execution
- **THEN** its code snapshot and ORM catalog are derived from that acquisition
- **AND** `SessionFactory` accepts their matching configuration identities

#### Scenario: Large unrelated resources
- **WHEN** `Config` contains forms or templates that neither factory input reads
- **THEN** the startup acquisition does not retain their compressed bytes
- **AND** the code and ORM projections still receive every resource they require

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
SHALL require the explicit provider-specific `--db-plaintext` and
`--db-trust-server-certificate` flags respectively. The former SHALL be
rejected for SQL Server and the latter for PostgreSQL.

#### Scenario: Ordinary script
- **WHEN** the CLI is invoked with an existing script command and no `--config-db`
- **THEN** it follows the existing execution path without loading a configuration or opening a database session

#### Scenario: Configuration test script
- **WHEN** the CLI is invoked with `--config-db`, a valid principal, and a test script
- **THEN** it starts the configuration session before executing that script in its context

#### Scenario: Configuration test script arguments
- **WHEN** the CLI is invoked with `--config-db` and arguments after the script path
- **THEN** that Session's `АргументыКоманднойСтроки` contains those arguments in order

#### Scenario: Password on the command line
- **WHEN** a database-mode invocation supplies a password-shaped option
- **THEN** argument validation rejects it before a connection is attempted

#### Scenario: Principal without database mode
- **WHEN** an invocation supplies `--principal` without `--config-db`
- **THEN** argument validation rejects it without changing ordinary script execution

### Requirement: Source-neutral incremental execution catalog
The bytecode boundary SHALL expose a source-neutral catalog interface that discovers stable module identities without materializing module code and retrieves executable programs on demand. The VM SHALL allocate execution caches and module state for an unprepared module only when it is reached. Existing complete configuration images SHALL remain usable through the same execution entry points without a database or frontend dependency in the VM.

#### Scenario: Attach an indexed catalog
- **WHEN** execution attaches a catalog with prepared and unprepared modules
- **THEN** attachment does not request executable code for unprepared modules
- **AND** the first actual access prepares the requested program before executing its instructions

### Requirement: Dependency-directed configured session preparation
Source-backed configured startup SHALL prepare explicit session roots and supplied entry sources without recursively resolving, compiling or verifying bodies referenced only by unused code. Declaration discovery SHALL NOT imply preparation of a named module body. Existing name precedence and global declaration conflict detection SHALL remain unchanged.

#### Scenario: Many unused server modules
- **WHEN** a source-backed configuration contains a session root, a dependency chain executed during startup and unrelated server modules
- **THEN** only explicit roots and bodies actually accessed by startup undergo resolution, code generation and verification
- **AND** declaration discovery and static references alone do not prepare unused bodies

#### Scenario: Unused method references a child
- **WHEN** a valid export is called from a parent whose unused sibling references a child containing an invalid constructor
- **THEN** parent construction and the valid call succeed without acquiring or compiling the child body
- **AND** a later actual child call prepares the child and reports its existing compilation failure

#### Scenario: Keep declaration conflicts explicit
- **WHEN** the declaration index contains conflicting global exports
- **THEN** factory preparation reports the conflict without resolving unrelated bodies

#### Scenario: Explicit imports remain roots
- **WHEN** a source-backed root explicitly declares an import
- **THEN** the explicit imported body retains its existing preparation policy

### Requirement: Prepare runtime-discovered dependencies at first access
A source-backed configured Session SHALL prepare each requested module body through the existing BSL pipeline before executing it. Cross-module links SHALL be checked against immutable declarations without executing declaration-only structures. A real prepared program SHALL agree with its declarations and pass the existing image and link validation before publication. Preparation SHALL preserve module identities, existing live state and the native compilation failure policy. Immutable code SHALL be reusable by Sessions of the same factory; mutable state SHALL remain isolated.

#### Scenario: Runtime common-module name
- **WHEN** a valid module name is passed through a parameter into `Вычислить`
- **THEN** its actual access prepares and executes that module
- **AND** unrelated and merely referenced unused bodies remain unprepared

#### Scenario: Inconsistent declaration and body
- **WHEN** a source supplies export declarations inconsistent with the subsequently compiled program
- **THEN** preparation fails before publishing or executing that program

#### Scenario: Reuse in an isolated Session
- **WHEN** a second Session of the same factory calls a previously prepared module
- **THEN** its immutable prepared code is reused
- **AND** its variables, initialization state and session parameters remain isolated

#### Scenario: Late code after runtime field-name growth
- **WHEN** a Session has more than 65,536 runtime field names and reaches a valid module whose own member operands fit the bytecode format
- **THEN** the module remains usable without a limit from accumulated Session names
- **AND** structures retain their fields and spelling across static and dynamic calls

### Requirement: Metadata details follow actual dependencies
Configured loading SHALL discover object identities and collection membership without eagerly materializing every detailed ORM object. It SHALL materialize details and their dependencies when required by startup or actual access. Complete metadata collection discovery and runtime string lookup SHALL remain available. Authorization, separation and session initialization inputs SHALL NOT be omitted solely because they are not explicit source identifiers.

#### Scenario: Runtime metadata lookup
- **WHEN** executing code looks up an indexed object by a runtime-computed name
- **THEN** that object's required details are materialized and returned through the ordinary metadata API
- **AND** unrelated details are not materialized

#### Scenario: Collection enumeration
- **WHEN** executing code enumerates a metadata collection
- **THEN** it observes the collection's complete declared membership rather than only previously materialized objects

#### Scenario: Acquire contexts of a reached dependency group
- **WHEN** the dependency walk has reached several modules whose receivers require metadata
- **THEN** their provider can acquire the required owner details as one group and reuse the resulting snapshot during individual context resolution
- **AND** owners of modules outside the reached dependencies are not acquired

#### Scenario: Discover a tabular-section type without its attributes
- **WHEN** execution requests the existing type alias of an indexed tabular section before accessing its owner's details
- **THEN** it receives the same type identity as whole-catalog loading
- **AND** discovery does not require materializing the section's attribute descriptions

### Requirement: Selective acquisition avoids hidden whole-configuration work
The configured database startup path SHALL acquire bootstrap/discovery data and requested module or metadata resources through the database adapter without an implicit full source or detailed-metadata decode. Requested reads SHALL retain existing provider limits and error handling. Standalone CLI operation and explicitly whole-catalog APIs SHALL remain available.

#### Scenario: Count unused resources
- **WHEN** a counting provider exposes unused module bodies and detailed metadata resources outside the reached dependencies
- **THEN** configured startup does not request or decode those bodies and details
- **AND** first access requests the newly needed resources without a whole-configuration fallback

#### Scenario: Ordinary script
- **WHEN** the CLI is launched without database configuration
- **THEN** its existing loading and execution behavior is unchanged

#### Scenario: Inspect a protected global declaration
- **WHEN** global exports of a protected module are discovered with full body decompilation enabled
- **THEN** discovery reads declarations without recovering executable bodies
- **AND** method identities remain the same when the module is prepared at first access

### Requirement: Reached chart label location enum
The runtime SHALL expose ПоложениеПодписейКДиаграмме.КрайАвто and ChartLabelLocation.EdgeAuto as the same system enumeration value. It SHALL also expose ПоложениеПодписейКДиаграмме.КрайВнутри and ChartLabelLocation.EdgeInside as a second distinct value with presentation `Edge inside`. The existing EdgeAuto presentation SHALL remain `Edge auto`.

#### Scenario: Original BUH manager declaration
- **WHEN** original manager code uses ПоложениеПодписейКДиаграмме.КрайАвто
- **THEN** the expression resolves without undefined-variable errors
- **AND** its value equals ChartLabelLocation.EdgeAuto

#### Scenario: Type aliases
- **WHEN** either declared type name is passed to Type
- **THEN** it equals TypeOf of the reached member

#### Scenario: Subsequently reached inside-edge label
- **WHEN** original BUH manager code uses ПоложениеПодписейКДиаграмме.КрайВнутри or ChartLabelLocation.EdgeInside with any letter case
- **THEN** both resolve to the same value and differ from EdgeAuto
- **AND** its type is ChartLabelLocation and its presentation is `Edge inside`

#### Scenario: Unsupported guessed alias
- **WHEN** code uses ChartLabelLocation.InsideEdge
- **THEN** normal unresolved-member diagnostics are retained

#### Scenario: Primitive label ordering
- **WHEN** code compares EdgeAuto and EdgeInside with the primitive less-than operator
- **THEN** execution reports the existing unsupported comparison error

### Requirement: Reached chart scale label location enum
The runtime SHALL expose ПоложениеПодписейШкалыДиаграммы.Нет and ChartScaleLabelLocation.None as the same system enumeration value.

#### Scenario: Following original BUH declaration
- **WHEN** the same original report manager uses ПоложениеПодписейШкалыДиаграммы.Нет
- **THEN** the expression resolves and equals ChartScaleLabelLocation.None
- **AND** both declared type names resolve to its type

### Requirement: Explicitly acquired empty configuration default interface
The ORM metadata root SHALL declare read-only ОсновнойИнтерфейс / DefaultInterface through existing case-insensitive property dispatch. An explicitly acquired nil default-interface UUID SHALL return Undefined. Unacquired data and non-nil interface metadata that is not implemented SHALL fail explicitly, never masquerading as an absent interface.

#### Scenario: Original user-description expression
- **GIVEN** an acquired configuration with a nil default-interface UUID
- **WHEN** the original user-description expression tests Metadata.DefaultInterface against Undefined
- **THEN** it selects the empty-name branch without reading an interface name
- **AND** Russian, English and mixed-case reads return the same Undefined value

#### Scenario: Read-only property
- **WHEN** BSL assigns either property alias
- **THEN** the existing read-only property error is raised

#### Scenario: Unknown and unsupported presence
- **WHEN** the host has not acquired the default-interface UUID or has acquired a non-nil UUID without implemented interface metadata
- **THEN** reading either alias fails explicitly
- **AND** a different catalog with proven absence remains independently readable

### Requirement: Default-interface acquisition preserves unknown data
The configuration adapter SHALL project the default-interface UUID only from a natively established root profile using the existing SDK representation. Unknown layouts, missing fields or malformed UUID values SHALL remain unacquired. The immutable ORM catalog SHALL accept an explicitly supplied acquired UUID without process-wide state or new runtime dependencies.

#### Scenario: Native empty and configured profiles
- **WHEN** retained native empty and configured-interface root resources are projected
- **THEN** their acquired UUIDs equal the corresponding native XML absence or Interface UUID

#### Scenario: Malformed or unknown profile
- **WHEN** the root profile or default-interface field is missing, malformed or unsupported
- **THEN** no absence value is fabricated

### Requirement: CLI registration string for database Sessions
A --config-db invocation SHALL accept optional --infobase-connection-string in separated and equals forms and pass the nonempty string unchanged to its SessionFactory. CLI SHALL NOT derive it from SQL connection parameters. Omission SHALL preserve current behavior. Empty or duplicate values and use without --config-db SHALL fail before a database connection is opened.

#### Scenario: Preserve supplied registration
- **WHEN** a configured invocation supplies a registration string
- **THEN** its SessionFactory receives the exact value without parsing or normalization

#### Scenario: Keep standalone behavior
- **WHEN** a standalone invocation omits the option
- **THEN** its existing execution behavior is unchanged

#### Scenario: Reject invalid option use
- **WHEN** the option is empty, duplicate, or supplied without --config-db
- **THEN** parsing fails without connecting or consuming the invocation

### Requirement: Query selection indexing by column name
Query selections SHALL accept string column names as indices using case-insensitive lookup while preserving existing numeric indexing.

#### Scenario: Named column
- **WHEN** a positioned selection is indexed by its column name in a different case
- **THEN** the value equals the value obtained with the corresponding numeric index

#### Scenario: Missing column
- **WHEN** a positioned selection is indexed by an unknown column name
- **THEN** the operation raises an error

### Requirement: Reached configuration root access rights
AccessRight SHALL accept the current acquired Metadata root for the reached SaveUserData and Administration rights, including Russian names. Its existing host request SHALL identify the acquired configuration GUID and name. An unsupported or inapplicable root right SHALL raise even in a privileged frame.

#### Scenario: Original UNF settings access
- **WHEN** original code checks SaveUserData on Metadata
- **THEN** the request targets the acquired configuration root instead of raising a metadata type error
- **AND** the session's role grants determine the ordinary current-principal result

#### Scenario: Validated privileged current principal
- **WHEN** a valid supported root right is checked for the current principal in a privileged frame
- **THEN** the result is true
- **AND** an unknown or inapplicable right still raises

### Requirement: Explicit root right subjects retain their role grants
An explicit role SHALL be evaluated directly in both modes. An explicit user SHALL require the current principal's Administration right outside privileged mode, including an explicit current user. Privileged mode SHALL bypass that administrative gate but SHALL NOT replace the explicit subject's grants with true. Missing acquired user or role data SHALL fail explicitly.

#### Scenario: Explicit denied role
- **WHEN** a denied role is checked in a privileged frame
- **THEN** the result remains false

#### Scenario: Explicit current user without Administration
- **WHEN** a non-administrative principal explicitly supplies its own user description
- **THEN** ordinary mode raises an administrative-access error
- **AND** privileged mode evaluates that user's actual role grants

#### Scenario: Missing rights resource
- **WHEN** an explicit subject needs an unread or unreadable role
- **THEN** checking raises instead of reporting a grant or a refusal

### Requirement: Root metadata identity is distinct from the catalog namespace
The host SHALL be able to supply the configuration metadata object's identity independently of the existing catalog namespace identity. Acquisition SHALL preserve the native root identity used by role resources without changing the namespace or its type declarations. Missing root metadata identity SHALL fail a root right check explicitly.

#### Scenario: Different identities in a native configuration
- **WHEN** the catalog namespace and configuration metadata GUID differ
- **THEN** the access request uses the configuration metadata GUID
- **AND** the catalog namespace and existing type identities remain unchanged

#### Scenario: Missing acquired root identity
- **WHEN** a host omits the configuration metadata identity
- **THEN** a root right check raises instead of using the catalog namespace as a rights target

### Requirement: Flat BSL array query parameters
The database adapter SHALL accept flat Array parameters whose elements have supported scalar encodings and SHALL preserve their element order, reference owner identities and scalar encodings through the existing SDK list carrier. Empty arrays SHALL be supported for IN predicates. Binding SHALL capture the current array values at execution without replacing the BSL parameter object.

#### Scenario: Original reference IN list
- **WHEN** original code passes an array of catalog references to an IN parameter
- **THEN** binding uses those references instead of raising an unsupported Array error

#### Scenario: Empty IN list
- **WHEN** an empty array is bound to an IN predicate
- **THEN** execution returns no matching rows

#### Scenario: Live parameter and execution snapshot
- **WHEN** the application appends to its array after SetParameter and executes the query
- **THEN** the execution snapshot includes the append
- **AND** a previously captured host parameter snapshot is unchanged

### Requirement: Automatic session parameter initialization context
Automatic initial and selective session parameter initialization SHALL enter the acquired SessionModule handler with privileged mode requested for that invocation. The caller's existing mode state and authenticated identity SHALL remain unchanged after normal return or an error. Existing effective safe-mode suppression SHALL remain authoritative. The ordinary generic host handler default SHALL remain unchanged.

#### Scenario: Initial automatic entry
- **WHEN** a session invokes its original initialization handler with Undefined
- **THEN** that invocation observes privileged mode in the measured ordinary managed-application context
- **AND** subsequent ordinary session execution observes ordinary mode

#### Scenario: Selective automatic entry
- **GIVEN** an unset declared session parameter and an ordinary caller
- **WHEN** reading the parameter invokes the original selective handler
- **THEN** that handler observes privileged mode in the measured ordinary context
- **AND** the caller remains in ordinary mode after receiving the initialized value

#### Scenario: Exceptional return
- **WHEN** automatic initialization raises an error
- **THEN** the error propagates through existing session initialization handling
- **AND** the entry privilege does not remain in the caller or another session

#### Scenario: Generic handler default
- **WHEN** a host invokes an unrelated configuration handler without selecting privileged entry
- **THEN** the existing inherited security context applies
- **AND** its module or procedure spelling does not implicitly enable privilege

### Requirement: ORM metadata acquisition defers query finalization
Materializing reached ORM metadata SHALL acquire and validate the normalized
runtime facts it needs without constructing a complete query snapshot merely
for ORM projection. Query metadata finalization SHALL occur when a complete
snapshot is explicitly requested or query preparation requires it. Runtime
facts SHALL NOT masquerade as a complete MetadataSnapshot.

#### Scenario: Runtime object access before a query
- **WHEN** startup reaches a new ORM metadata object without preparing a query
- **THEN** its normalized facts and required dependencies are acquired
- **AND** no query fingerprint, query index or query resolution report is built solely for that object access

#### Scenario: Query after runtime metadata growth
- **WHEN** a query or explicit snapshot request follows acquisition of new runtime facts
- **THEN** a complete immutable snapshot is finalized for the current acquired composition
- **AND** its facts, diagnostics and fingerprint match the existing strict full-snapshot resolver
- **AND** a previously prepared query retains its original snapshot identity

#### Scenario: Invalid runtime dependency
- **WHEN** an ORM object's required named type, reference, separator or extension fact cannot be resolved
- **THEN** acquisition obtains the dependency or reports the existing applicable failure
- **AND** it does not publish an incomplete runtime fact set as successful

#### Scenario: Reuse without new acquisition
- **WHEN** repeated ORM access or snapshot requests add no resources
- **THEN** the corresponding immutable runtime facts or full snapshot are reused
- **AND** mutable state remains isolated between Sessions
