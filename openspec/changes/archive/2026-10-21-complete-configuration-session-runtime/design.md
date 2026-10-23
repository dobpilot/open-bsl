# Design

## Reached WS references

Original UNF binding reaches WSReferences in a dormant register-manager helper. Five retained native controls prove both global spellings, the WSReferencesManager type identity, its English `WS-reference manager` presentation, unknown-member refusal and absence of Count. Vendor 8.3.27.2342 pages describe configuration-defined read-only named properties, GetWSDefinitions and CreateWSProxy with three required and five optional arguments. Existing core WSDefinitions/WSProxy constructors already explicitly refuse execution; do not create a second interpreter or pretend these declarations implement WSDL/SOAP.

First bind the finite global through the existing ORM descriptor with explicit unsupported execution while acquisition is absent. Keep full values, retained service definitions and proxy transport work unchecked. Acquisition belongs to open-sdbl and bsl-config-db; runtime values belong to ORM and reuse existing XML/XDTO, streams and host network capability. No root/VM database dependency, fake empty configuration, invented WS identity or remote download during configuration acquisition is introduced.

The reached information-register Get operation uses the existing session-bound
query adapter, not another SQL generator. SDK 0.9.9 identifies individual
dimension/resource roles but does not promise complete collections or register
periodicity. The required acquisition contract is recorded in
`docs/plans/open-sdbl-information-register-metadata-request.md`; the ORM must
not infer completeness from the fields that happened to decode. Native empty
and nonempty controls preserve 22 lines, including rejected filters. A further
14-line control stores distinguishable resources for numeric keys 0, 1 and 7.
For the measured Number(5,0) dimension, Undefined, False, empty and invalid
strings select key 0; True and 0.6 select key 1; number 7 and string "7" select
key 7. Conversion should reuse the existing typed adjustment mechanism after
verifying it against these observations; other dimension types are not inferred.
The separate Day-register control preserves 11 lines. Native record writing
normalizes the stored date to midnight, while Get compares the supplied date
exactly: another time on the same day returns an empty resource. The read path
must therefore not reuse write-time periodicity rounding. Other periodicities
and PointInTime arguments remain outside the measured scope.
The ORM catalog first accepts explicit host-supplied settings independently of
SDK acquisition, following the existing accumulation-register metadata API.
Initial direct reading supports the measured Number(5,0) dimensions and
Number(15,0) resources with Nonperiodic or Day periodicity; unsupported types
fail explicitly. This is an implementation stage, not completion of the
original configuration startup or the SDK projection task.

Compile-only demo next reaches Constant.ИспользоватьОтметкиВремени.Set.
Its original manager conditionally changes a stored Boolean constant. The
installed 8.3.27 help specifies a single arbitrary required value. The current
SDK DatabaseSession query path is explicitly read-only, so it must not be
repurposed into an unvalidated SQL writer. DataAccess receives an optional
constant-write operation with an explicit rejecting default. A host writer owns
type conversion, write rights, handlers, transactions and persistence; forwarding
the call does not claim those features in the database adapter. Real database
writing remains an open task if authorized startup executes this branch.

The next reached document method is PointInTime. Native controls establish
snapshot date/reference properties, value equality and the Compare method;
the less-than operator rejects these nonprimitive values. A typed empty
document reference differs from an omitted reference (Undefined). Eighteen
further lines cover same-kind GUID pairs but do not establish precedence
between UUID groups or different metadata kinds. The existing deliberate
unsupported core constructor must not be replaced by a date-only value.
Reference snapshots and value equality already exist in ObjectProtocol;
the implementation should reuse that boundary rather than make the runtime
depend on ORM or duplicate configuration-reference parsing.
Ten additional native group-precedence comparisons match the existing
uuid::to_1c_bytes lexicographic order. A focused runtime test verifies this
against the receipt. Cross-kind reference ordering remains unmeasured.

Further controls establish constructor handling and three document-reference
types ordered by acquired storage number. This evidence permits replacing the
deliberate unsupported constructor with a real immutable runtime value;
unsupported query binding still fails explicitly in the data adapter. The value
stores the date, reference and optional host-supplied ordering key. ObjectProtocol
exposes a small reference key (canonical UUID and optional acquired storage
number), preserving crate boundaries. Missing cross-type keys never fall back
to metadata names or GUIDs. Other metadata-class ordering remains unsupported
until measured; no generic lexicographic type-name ordering is introduced.

UNF reaches a manager module through `Справочники.ВерсииРасширений`, while
the rooted graph currently follows only imported-module links. The facade will
accept host associations between a registered component global, a property
name and a private module. The semantic resolver records the dependency after
resolving the receiver to that component factory, so variable shadowing still
applies. The existing deferred-module closure includes the target and its
ordinary dependencies; the expression remains an ordinary property read.
`bsl-session` supplies these associations from the immutable ORM catalog.
No BSL source scanner, rewritten call, or extra startup root is introduced.

The demo startup graph reaches dormant `WindowsClientCertificate` and
`WindowsCertificationAuthorityCertificates` constructors. The platform
8.3.27.2342 syntax helper documents a zero-argument authority-certificate
constructor. Both Windows store families remain explicitly unsupported
at execution without a host adapter.

The demo startup graph reaches a dormant `WindowsClientCertificate`
constructor. The HTTP component already owns its type descriptor but omitted
the constructor because extracting a Windows private key is unsupported. The
documented zero- and one-argument forms are now resolvable; both retain an
explicit execution error. This removes the compile barrier without inventing
a certificate, a private key, or a Windows-store implementation.

The facade exposes one narrow reentrant operation through the component call context: invoke an export of a module already linked into the current configuration and instantiated in the current State. The VM remains the only executor. The nested call preserves `ЭтотОбъект`, by-reference arguments, host services, the current session's common modules, and the BSL call stack. Facade types required by `bsl-orm` and `bsl-session` are re-exported through `open-bsl`; those crates do not import private interpreter crates.

A configuration manager object links its metadata object identity to one manager module key and one session module instance. Dispatch checks a static platform method implemented by `bsl-orm`, then an exported method of the linked BSL manager module, then reports an unknown-method error. The same manager object is passed as `ЭтотОбъект` to its BSL module.

Session parameters keep explicit unset, initializing, and set states. The startup call receives `Неопределено`. Reading an unset parameter synchronously calls the session handler with an array containing its name. A different unset parameter may initialize recursively; reading a name already on the initialization stack fails immediately. Returning without setting the requested value fails. Failed initialization restores only the requested unset state: values written before the error and successfully initialized nested parameters remain. `Очистить()` returns parameters to unset. These rules are the measured 8.3.27 behavior recorded in the research document.

Before compiling or executing a data read, `bsl-orm` uses the current session parameter values and the selected principal's roles. A restriction that needs an unset parameter triggers the same lazy initialization path. Missing role data, an unexpanded restriction, or a failed parameter initialization denies execution rather than widening access. Ordinary queries and `ВЫБРАТЬ РАЗРЕШЕННЫЕ` retain the distinct access behavior supplied by `open-sdbl`.

Prepare every query in `RestrictionMode::Restricted` to collect every table
read and obtain a decision for it. The current adapter rejects both denied
and filtered ordinary selections before SQL. This is a temporary unsupported
execution boundary, not native ordinary-query semantics: the independent
`accumulation-field-access/query-rls-allowed.platform.txt` control proves that
an ordinary balance query succeeds when its selected records satisfy RLS,
whereas `query-rls-queries.platform.txt` proves failure for forbidden records
and an empty successful `SELECT ALLOWED` result on the same data. The SDK
must supply the execution information needed to distinguish these cases;
the adapter must not invent SQL guards or rewrite ordinary selections as
allowed selections. The upstream request is recorded in
`docs/plans/open-sdbl-ordinary-rls-execution-request.md`. These controls do
not establish arbitrary join, aggregation or batch semantics.
`SELECT ALLOWED` may use the decision as a row filter. The SDBL lexer, not text search, identifies each
`SELECT` followed immediately by its `ALLOWED` modifier. Counting `ALLOWED`
elsewhere could mistake an ordinary selection for a permitted one. Until the
compiler exposes per-statement access decisions, a batch mixing allowed and
ordinary selections with a non-unrestricted decision fails explicitly:
deduplicated targets cannot
prove which statement would consume the filter. Compilation itself uses
the restricted mode so no undecided table can slip into SQL.

The concrete database adapter runs inside synchronous BSL calls and owns one
`BlockingDatabaseSession` per Session. `open-sdbl-db` 0.9.3 exposes
`ensure_users` and `ensure_rights` on that blocking wrapper, delegating to
the asynchronous access functions under its current runtime with the same
closed-session and async-context errors. The adapter must not duplicate
`v8users` decoding or role-resource SQL. It
retains an `AccessStore` for the selected principal and uses the existing
`Prepared` restriction requests and decisions; an absent or unreadable role
denies the read rather than producing an unrestricted query.
`Prepared::field_usage()` also reports fields read in projections, filters,
joins, grouping, and ordering. The adapter calls the new
`check_user_field_access` on the complete request before SQL; a table-level
read decision alone is insufficient. Version 0.9.3 proves grants only for
custom attributes with an exact one-step member GUID and explicit `Read=true`.
Standard fields and tabular sections fail closed until their role mapping is
measured and supported upstream. This is a known execution limit, not a
reason to skip the field check.
For a constant read, the ORM object's stable identifier must equal the GUID
that the SDBL snapshot resolves for the same constant name. A configuration
extension can otherwise replace a logical object while the query still reads
the base object's table by name. Mismatches fail before SQL rather than return
a value of the wrong configuration object.

The CLI currently closes its acquisition connection after loading users and
rights, then opens a separate blocking connection for the Session without
attaching a `DataAccess`; this is the concrete task-5 gap. The Session
connection must have one shared owner (`Rc<RefCell<BlockingDatabaseSession>>`)
held by both `SessionResources.connection` and the `bsl-config-db` adapter.
The factory-input loader returns the acquired `MetadataSnapshot` by move
alongside code and ORM catalog, so query compilation does not clone the
possibly large snapshot or reread the configuration.
The already loaded `AccessStore` and selected user move into that adapter;
neither a second session connection nor a process-global store is introduced.
Before borrowing the connection for SQL, the adapter resolves every required
session parameter through `CallContext`, then checks the selected principal,
all role resources, and the prepared query's access decisions. Missing
rights or an unsupported value conversion fails before SQL execution. A
background Session receives a different shared owner and a fresh adapter
for its explicitly opened connection.

One upstream API detail is still missing for lazy session parameters. The
`open-sdbl` restriction expander has a structured
`RestrictionError::MissingParameter(name)`, but `open-sdbl-db::user_decisions`
and `user_restrictions` currently stringify it into `DbError::Data`. The
query compiler also groups absent and unused parameters in
`QueryDiagnosticKind::Parameter` without exposing the missing name. The
database adapter must not parse those messages or eagerly initialize every
known session parameter. Request a structured missing-session-parameter
result from those existing APIs; the adapter can then initialize exactly
that name through `SessionRuntime.parameters().get(name, context)`, add its
value to the SDBL session map, and retry at most once per known parameter.
Unrecognized names, cycles, and any other errors remain fail-closed.
`SessionParameters::get` becomes public to this provider adapter; it is the
same method already used by BSL property reads, so the adapter does not
implement a second initialization state machine.

The first query slice follows the surface measured on platform 8.3.27.2342: one mutable query value owns its text and folded parameter names, while execution crosses the existing Session-owned `DataAccess` boundary. The adapter returns neutral named rows; `bsl-orm` owns the BSL `РезультатЗапроса`, linear selection, and `ТаблицаЗначений` projection. This keeps query syntax, SQL provider details, rights, and RLS in the future `bsl-config-db` adapter while keeping BSL object behavior in `bsl-orm`. Only direct traversal is implemented in this slice. A caller that requests grouped traversal receives an explicit unsupported-operation error instead of a linear result with different semantics.

The information-base connection string is host data in `SessionRuntime`, not a reconstruction from the opaque connection object. The host knows the registration string and its session kind, while `bsl-session` must neither parse provider credentials nor expose the database adapter. A factory without this optional host value remains usable for code that does not request the capability; calling the BSL function then fails explicitly. A separately built background factory carries its own server-side registration string and does not inherit a hidden process-global value.

The information-base connection number belongs to a concrete `Session`, unlike the factory-level registration string. Every `SessionFactory::open*` operation therefore accepts the positive number together with the owned opaque connection and stores it in `SessionRuntime`. The runtime returns that exact host-assigned number; it does not derive one from an address or reuse the parent's number for a background Session. Requiring the number at open time also prevents configuration startup from observing an incomplete connection context. The CLI starts exactly one Session per process with `--config-db` and assigns it the local host number `1`. This is a deliberate host identity for the CLI, not a claim that `open-sdbl` read the 1C platform's connection number from the database. A host that opens multiple Sessions must assign distinct numbers itself.

The information-base session number is a separate platform identity. The
8.3.27 platform probe `measure-infobase-session-number.bsl` confirms a positive
Number and equality with the current session object's `НомерСеанса` property.
`SessionResources` accepts it explicitly as an optional positive value; the
global function and property fail if it is absent. A direct SQL connection
does not itself receive a 1C session number, so neither the database backend
PID nor the connection number is silently substituted. A background Session
receives its own resources value and inherits no session number.

The next UNF graph barrier is the closed `ОтображениеКнопки` /
`ButtonRepresentation` system enumeration. The installed Syntax Assistant
page `objects/catalog2/catalog277/ButtonRepresentation.html` lists exactly
Auto, Picture, PictureAndText, and Text. The 8.3.27 probe
`measure-button-representation.bsl` confirms the type display, all four
presentations, and Russian/English alias equality. It enters the base
runtime's existing enum and type tables; no Session state is involved.

The active-connection list is another host-supplied Session resource. The public `SessionResources` value groups the owned opaque connection, its number, the optional data adapter, and the optional authorized connection snapshot, avoiding one `SessionFactory::open_*` overload for every resource combination. The host obtains and authorizes the snapshot for the selected principal before startup; `bsl-session` validates that the concrete Session connection occurs in it and exposes immutable BSL connection values. Absence, denied access, or an inconsistent snapshot fails explicitly. The runtime neither discovers process connections nor substitutes an incomplete list containing only itself. The user-selected CLI scope is all real Session instances of the current open-bsl process; its current single-Session lifecycle produces a complete singleton snapshot, authorized against acquired database role rights. Native 1C connections are outside that scope. The coherent CLI refinement is specified in `supply-cli-host-connection-snapshot`.

`bsl-orm` owns the configuration-facing `ХранилищеЗначения` and `СжатиеДанных` objects. It reuses the facade's bounded serialized-value graph instead of adding another value walker. The graph receives a detached-capture mode for this use: recursion to an object already on the active path remains bounded, while a repeated reference reached after its first traversal is serialized again. This reproduces the measured platform behavior in which construction snapshots the input, every `Получить()` materializes a fresh value, and two source aliases become independent restored values. The compression descriptor is validated and retained as constructor intent; this in-memory runtime does not fabricate a compressed database representation.

`ПолучитьТекущийСеансИнформационнойБазы()` returns a session object backed only by the current `SessionRuntime`. A foreground session returns `Неопределено` from `ПолучитьФоновоеЗадание()`. Opening a background Session requires the caller to pass the exact `Модуль.Метод` target of that job; the runtime returns a read-only current-job view with that measured `ИмяМетода`. The view uses the existing core `ФоновоеЗадание` type descriptor rather than declaring a competing platform type. Other job properties and live operations continue to belong to the existing background-job service and are not fabricated by the session layer.

Security-mode lifetime belongs to VM call frames rather than `SessionRuntime`. The facade call context therefore exposes a narrow security-mode control implemented by the existing VM. Each VM task stores its own effective counters indexed by frame depth; ordinary return and exception unwinding trim counters together with frames. A nested component-to-module execution receives an effective snapshot as inherited state but cannot decrement the caller's counters. `bsl-session` declares the boolean safe- and privileged-mode functions plus `УстановитьОтключениеБезопасногоРежима` / `SetSafeModeDisabled` and `ПолучитьОтключениеБезопасногоРежима` / `GetSafeModeDisabled`, delegating them to that control. Safe-mode disabling stores both a balanced counter and a latch in the current frame: `Ложь` balances the counter but the measured platform keeps safe mode suppressed until that frame exits. The getter reads only whether the current frame's balanced counter is nonzero, so it returns false in a nested frame and after balancing even while the latch still suppresses safe mode. The latch suppresses the effective safe-mode query without deleting safe-mode enables. Named profiles are rejected explicitly because accepting an arbitrary string without a host profile catalog would silently bypass platform policy.

`УстановитьБезопасныйРежимРазделенияДанных` / `SetDataSeparationSafeMode` uses the same VM-owned frame lifetime, with an independent counter for each folded common-attribute name. The host supplies the finite set of data-separation common attributes in `SessionResources`; an absent list or unknown name fails before the counter changes. This keeps validation tied to the concrete configuration while the VM remains unaware of metadata and databases. `CallContext` exposes the effective per-name query so the existing Session-bound data adapters can enforce it on database operations. A nested synchronous component-to-module execution inherits the effective names, and a new Session starts with no active names. The existing `SECURITY.ASYNC_INHERITANCE` open question also covers the snapshot inherited by a separate async VM task. The documented valid-attribute counter semantics are retained under `НЕ ИЗМЕРЕНО(SECURITY.DATA_SEPARATION_MODE)` until the measurement configuration contains a data-separation common attribute.

A background job is a new Session with an explicitly supplied selectable principal and a separate database connection and module state. Its parameter container starts as a copy of the parent session parameters; subsequent mutations are isolated. There are no scheduled jobs in this scope.

The reached `РасписаниеРегламентногоЗадания` value does not add scheduled jobs to that scope. It is an in-memory serializable schedule description which configuration code constructs and edits even when no scheduled-job manager is used. `bsl-session` owns this passive value because it describes session-side platform behavior and has no configuration metadata or database identity. Its constructor, defaults and mutable property surface are measured by `measure-job-schedule.bsl`. The pure `ТребуетсяВыполнение` calculation remains outside this slice until a startup path actually calls it; inventing a partial scheduler predicate would be less safe than an explicit unsupported error.

The next reached bare value is the platform system enumeration
`АвтоРегистрацияИзменений` / `AutoChangeRecord`. The Syntax Assistant for
8.3.27.2342 identifies its complete two-member surface, and
`measure-auto-change-record.bsl` measures the member presentations, value type,
and English aliases on that platform build. It therefore belongs in the
existing base-runtime system-enumeration table; no exchange-plan persistence or
change-recording operation is implied.

A named common module is also visible under its own name when its exports call
another export. `measure-module-self-reference.bsl` proves this on platform
8.3.27.2342. The facade therefore includes the functions of the current named
or global module in the same automatically built import table used for other
named modules. The change deliberately does not expose the module's exported
variables through that self entry: an attempted extension of the measurement
fixture did not compile on the platform, so it provides no evidence for such a
contract.
That is a resolver visibility edge, not an initialization dependency: adding a
self edge to the explicit initialization graph would create a false cycle.
The same entry also lets the existing deferred bare-module diagnostic apply
when a module reads its own bare name as a value.

UNF next reaches `ValueList.Add(Value, Presentation, , Picture)`. `ValueList`
is constructed by the base-runtime library but, unlike legacy core values, it
declares its public methods in an `ObjectMembersDescriptor`. Constructor
provenance alone must not route such an object through the closed
`BuiltinMethod` table: the existing constructor-to-type mapping is the
authoritative distinction. Descriptor-backed base-runtime objects therefore
use the same linked component-method path as descriptor-backed objects from
any other library. The omission and its resulting `Check` value are measured
by `measure-value-list.bsl`; no second method implementation is added.

The next reached call, `ПолучитьПолноеИмяПредопределенногоЗначения` /
`GetPredefinedValueFullName`, is the documented inverse of the existing
`ПредопределенноеЗначение` / `PredefinedValue`. It therefore belongs to the
same ORM library and uses no database operation: a system-enumeration value is
rendered from the canonical English enum and member names measured on the
platform, and
a configuration-enumeration reference is resolved back to its owner and member
inside the immutable catalog already retained by that value. Other value kinds
fail explicitly. The system-enumeration round trip is measured by
`measure-predefined-value.bsl`; the canonical configuration-enumeration output
remains marked `НЕ ИЗМЕРЕНО` until a platform fixture with a predefined
configuration enumeration is available.

UNF then reaches the one-argument object-reference form of
`ПолучитьНавигационнуюСсылку` / `GetURL`. The Syntax Assistant documents a
single overload family of one through four positions for object references,
attributes, report settings, lists, processors, and commands. Navigation-link
construction depends on the concrete applied reference and, for some forms,
external navigation-data storage. `bsl-orm` therefore preserves the complete
argument vector and delegates it to the current Session's `DataAccess`; it does
not derive an `e1cib` string from display text or metadata alone. The adapter
default fails explicitly. This keeps the reached one-argument form and later
overloads on one contract without implementing an unrelated navigation host in
the runtime.

The next reached UNF compiler barrier is the platform function `ПолучитьФункциональнуюОпцию` / `GetFunctionalOption`. Its result can come from a constant, a catalog attribute, or an information-register resource and depends on the current data-area parameters. `bsl-orm` therefore validates and copies the documented name and optional Structure into an immutable request, then delegates evaluation through the Session-owned `DataAccess`. It does not infer a value from the immutable metadata catalog or cache it across Sessions. The adapter default fails explicitly, so an adapter cannot accidentally report a disabled option when it lacks functional-option support. The arity and parameter types come from the local platform 8.3.27 Syntax Assistant; the concrete database adapter remains part of the general data-access task.

UNF then reaches `МенеджерВременныхТаблиц` / `TempTablesManager`. This value belongs to `bsl-orm` because it scopes temporary tables created by `Запрос` on the concrete Session connection. Construction asks the current `DataAccess` for an opaque handle, the mutable Query property retains the manager object, and query execution includes that handle in `QueryRequest`. `Закрыть` / `Close` delegates destruction to the same adapter. The object retains the adapter instance that created it, so it cannot be attached to a Query from another Session. The provider owns temporary-table state, closed-handle behavior, and cleanup when the connection closes; `bsl-orm` does not emulate them in memory. Platform 8.3.27 permits a second close, assignment of the closed manager, and a Query that does not use temporary tables, while a subsequent `ПОМЕСТИТЬ` through that manager fails. These results are recorded by `measure-temp-tables-manager.bsl`; consequently the ORM layer does not impose a broader local closed flag. The zero-argument constructor, read/write Query property, and zero-argument close method also match the local Syntax Assistant.

The 8.3.27.2342 platform probe `measure-temp-tables-results.bsl` additionally
shows that a final `ПОМЕСТИТЬ` returns a QueryResult with one `Count` row,
while a final `УНИЧТОЖИТЬ` returns Undefined. Keep the existing
`DataAccess::execute_query` contract for adapters already implementing it and
add a defaulted result method that returns `Some` for those adapters; the
database adapter overrides it to return `None` for a final destroy. In that
adapter, each opaque handle owns an `open-sdbl::TempTablesManager` of captured
rows, not a second SQL connection or physical table. Clone its bounded state
for compilation and commit it only after the generated read completes
successfully. The adapter names the measured one-column placement result
`Count`. A measured
`ДОБАВИТЬ` of one row returns `Count = 1`, while the table then contains two
rows; `Count` is the number added by that statement, not the new total. A user-defined
column of the same Russian name in a final ordinary SELECT stays unchanged.
A closed handle remains known and idempotently
closable. The adapter compiles a query on a fresh temporary manager; an
unchanged manager means a plain query may use the closed handle, while a
definition or append changes that state and fails before SQL. A read or
destroy of a removed table fails during compilation. This avoids mistaking
contextual keywords used as column names for temporary-table operations. A nested session
parameter initialization can execute another query, so compilation does not
hold a mutable borrow of the manager across that call. If the manager changes
during nested initialization, the outer query fails before SQL rather than
overwriting the newer state.
The platform probe also changes a typed ValueTable source after placement:
the source becomes `2` while the placed table retains `1`, proving a snapshot
for that source. The adapter converts a ValueTable parameter into the existing
`open-sdbl::ParameterValue::Table`, copying typed scalar columns and current
rows at execution time. The neutral `TypeId` is re-exported by `open-bsl` so
the adapter can inspect column constraints without depending on `bsl-rt`.
Untyped, multi-type, and unsupported columns fail explicitly. The generated
placement callback reads the generated SELECT once and retains its typed
rows. Later mutation of the BSL table does not change the placed rows.
The platform probe also changes a persisted catalog row after placement. A
direct query then returns `After`, while the placed table still returns
`Before`. Thus placement materializes rows at execution time for both
ValueTable and database sources. `open-sdbl` 0.9.4 preflights the batch, calls
the adapter once per placement, and retains typed rows in the manager; later
queries use those rows without rereading the source. The adapter streams each
placement through `query_each`, caps the captured raw rows at 8 MiB, and
rejects cell kinds that cannot be represented as typed SDBL parameters. It
returns the measured `Count` result after the manager revision is committed.

The event log is another Session resource. `ЗаписьЖурналаРегистрации` forwards an immutable entry to a host-supplied writer stored with that Session; it does not write files or database tables inside `bsl-session`. A background Session receives its own writer through its own resources, so neither the connection nor the logging authority is inherited accidentally. Absence is an error because silently dropping a startup diagnostic would make an unsuccessful or restricted path appear successful. The enum metatypes, member presentations, and one-, five-, and six-argument calls were measured on platform 8.3.27.2342 by `measure-event-log.bsl`; the documented defaults are `Примечание` and `Независимая`.

Managed-client startup also reaches the standard settings stores and home-page settings. Their physical persistence, user selection, and authorization remain host responsibilities. `SessionResources` therefore receives one explicit client-settings adapter for the concrete Session connection, and every adapter call receives the frozen selected `Principal`. The BSL objects validate names, argument shapes, and home-page value types, then delegate reads and writes; they do not keep a process-global map or return an invented empty value when the adapter is absent. `Выбрать` materializes the adapter's immutable row snapshot as a session-owned cursor instead of exposing provider objects. A separately opened background Session does not inherit this adapter implicitly.

The minimum surface follows the local 8.3.27 syntax helper: the common,
system, form-data, report-user and dynamic-list-user globals have the standard
settings-storage manager type when the corresponding configuration property
does not select a custom metadata storage. The measured startup configurations
use the standard manager for the latter two globals; a custom configuration
storage remains an ORM manager selected from configuration metadata. `Загрузить`
accepts one through four arguments and `Сохранить` accepts three through five
with an optional second position. The reached surface also delegates
`ПолучитьСписок`, `ПолучитьОписание`, `Удалить` and `Выбрать` through the same
Session adapter. A returned list of setting keys is materialized as the base
runtime's existing `СписокЗначений`; this needs a Rust factory for that existing
BSL value, not another list implementation. Each store is distinguished by an
explicit storage kind, so no second persistence path is introduced.
`НастройкиНачальнойСтраницы` and `СоставФормНачальнойСтраницы` are zero-argument
constructors. The former gets and sets the current form composition; the latter
exposes read-only `ЛеваяКолонка` and `ПраваяКолонка` array properties. The
adapter enforces `СохранениеДанныхПользователя` and
`АдминистрированиеДанных` where the documented operation requires them.

The recorded UNF startup graph reaches `МенеджерПользователейИнформационнойБазы.ТекущийПользователь()`, so the ORM component exposes the manager as a library global and returns an immutable user description copied from the `Principal` already frozen in `SessionData`. The first slice exposes only the `Имя` / `Name` property that the recorded startup reads. It neither re-reads `v8users` nor exposes mutation, authentication settings, or another Session's user.

The same graph reads the platform global `Перечисления` / `Enums`. Enumeration ownership remains in `bsl-orm`: `bsl-config-db` projects enumeration descriptors into the immutable metadata catalog, and the existing generic manager root resolves a folded configuration name. This adds no system enum and no second resolver path; the library global uses the same declared-global mechanism as catalogs, information registers, and constants. Enumeration values are added only when the executing startup path reaches them and their acquired descriptor data is available.

The reached `ПредопределенноеЗначение` / `PredefinedValue` call remains an ORM operation because configuration values are identified by the same immutable catalog. `bsl-config-db` copies the already resolved owner, value GUID, and symbolic name from `open-sdbl`'s acquired `MetadataSnapshot`; it does not decode `Config` a second time. `bsl-orm` parses only the documented full path, resolves configuration enumeration members through that catalog, and represents the result as an immutable reference value. The system-enumeration form delegates to the existing base-runtime enum lookup table, avoiding a second list of platform enums. Unknown paths and non-string arguments fail explicitly. The initial slice implements the configuration-enumeration form reached by UNF and the measured system-enumeration form; catalog and chart predefined references wait until startup reaches them because their BSL reference behavior needs a wider object surface. `measure-predefined-value.bsl` records the one-argument contract, system-enum identity, and error behavior on platform 8.3.27; the local Syntax Assistant supplies the complete documented path grammar.

`ПланыОбмена` / `ExchangePlans` follows that same catalog-backed manager-root path. `MetadataKind::ExchangePlan` and its Config class mapping already exist, so the change adds only the missing library global and root metadata collection name. It does not implement exchange registration, node persistence, or transport. Standard root-manager methods found during compilation remain subject to runtime reachability and receive a separate host-data contract only if startup actually invokes them.

The graph also reads configuration-defined members of `БиблиотекаКартинок` / `PictureLib`. `bsl-config-db` projects common-picture descriptors into the same catalog, while `bsl-orm` returns an opaque picture value identified by its metadata object. Startup only passes these values into other object properties, so the first slice does not decode image resources or implement picture conversion methods. The type name, display, equality, and filled state follow the real-platform measurement in `measure-picture-library.bsl`; unknown and platform-standard members absent from the catalog fail explicitly instead of producing invented content.

The startup graph reads eight members of `ТипФайлаТабличногоДокумента` as data but does not ask the spreadsheet component to write the unsupported formats. Enum membership therefore belongs to the base runtime and is independent of writer-format support. The spreadsheet writer retains its explicit rejection of every enum member for which no encoder exists. `measure-spreadsheet-file-types.bsl` records the previously absent reached values rather than expanding the runtime to all platform members speculatively.

`КонфигурацияИзменена()` is configuration data rather than a process or Session flag. `bsl-orm` stores an optional frozen boolean beside the root configuration description; absence remains distinguishable and is an execution error. The database adapter will fill it from the whole-configuration acquisition once `open-sdbl-db` exposes the comparison of the main configuration with the database configuration. It must not infer `Ложь` merely because only `Config` was acquired. The real-platform call shape and return type are recorded by `measure-configuration-changed.bsl`.

`ОбновитьПовторноИспользуемыеЗначения()` belongs to the configuration Session surface. The current interpreter has no reusable-return-value cache: every BSL function executes on every call. Clearing an absent cache is therefore a successful no-op with the same observable postcondition as the platform operation, rather than an unsupported-operation error. If reusable function caching is introduced later, its Session-owned store must become the implementation behind this procedure. Both language names are exercised by `measure-refresh-reusable-values.bsl`.

`КонфигурацияБазыДанныхИзмененаДинамически()` describes an update that occurred during one user's work, so it belongs to `SessionRuntime`, unlike the factory-wide main-versus-database comparison behind `КонфигурацияИзменена()`. `SessionResources` carries an optional explicit boolean. Absence fails instead of fabricating `Ложь`; a host that observes an update can supply a new value according to its own Session lifecycle. Opening a background Session consumes a new resources value and does not copy this flag from its parent. Only the existing parameter-container copy remains inherited. The call shape, type, initial false value, and language aliases are recorded by `measure-database-configuration-changed-dynamically.bsl`.

Error-processing user settings can vary by user and database, so `bsl-session` receives an immutable current-user snapshot in `SessionResources`; it does not substitute the documented defaults when the host has not acquired the values. The reached manager delegates brief and detailed descriptions to the existing `ErrorInfo` implementation, avoiding a second formatter. The initial settings object is read-only because startup only reads its sending mode and service address. Named-user lookup, mutation, and the rest of the settings surface remain outside this slice. `measure-error-processing.bsl` records the two delegated methods, the reached properties, and the closed sending-mode enum.

The reached `БлокировкаДанных` object belongs to `bsl-orm` because its lock spaces name configuration data and acquisition must use the same concrete Session connection as queries and constants. The object only stages names locally. `Заблокировать()` delegates them through an additive default method on `DataAccess`; the default returns an explicit unsupported error, preserving existing adapters. The provider owns transaction lifetime and release. An in-process mutex would not exclude another 1C or gateway process and is therefore not an implementation of this platform operation. `measure-data-lock-surface.bsl` records the constructor, item types, and language aliases; the Syntax Assistant defines the reached one-string `Добавить` and zero-argument `Заблокировать` calls.

The immediately surrounding startup path uses the zero-argument begin, commit, and rollback procedures and later queries `ТранзакцияАктивна()` / `TransactionActive()`. They extend the same `DataAccess` boundary with additive default methods that fail explicitly, because only the connection provider can keep a database transaction and its managed locks aligned. `bsl-orm` does not maintain a parallel transaction depth or process-local rollback log. The provider remains authoritative for nesting and transaction state, including the query result. `measure-transaction-surface.bsl` records inactive, active, committed and rolled-back states and both language names.

The graph also reaches `МонопольныйРежим()` / `ExclusiveMode()` and the one-argument setter. Exclusive ownership is database and data-area state tied to the concrete Session connection, so both operations extend the same `DataAccess` boundary with additive fail-closed defaults. `bsl-orm` owns only BSL argument validation and does not keep a boolean shadow of a provider lock. The optional `ExclusiveModeParameters` form is outside the reached startup slice. `measure-exclusive-mode.bsl` records the boolean getter and the one-argument transition through both language names on platform 8.3.27.

The next compiler barrier is the global `СервисыИнтеграции` / `IntegrationServices` manager. The local Syntax Assistant defines two zero-argument manager methods and one dynamic property per configured service. Both processing and service resolution depend on the current information base, principal, and Session connection, so additive fail-closed methods on `DataAccess` own those operations. `bsl-orm` supplies only the global object, method arity, and dynamic-property dispatch. It does not decode non-tabular integration-service metadata a second time and does not keep a process-global job runner. The current `open-sdbl` metadata snapshot intentionally describes database-backed objects and cannot enumerate integration services; a provider that needs dynamic service access must expose that configuration metadata through its adapter. The exact side effects and permission failures of starting or stopping processing remain an open platform measurement rather than an inferred promise.

UNF next reaches the information-base predefined-data update mode. This setting is shared platform state but every operation is authorized and executed through the current Session, so `bsl-orm` owns the closed BSL enum and delegates the getter and setter through additive fail-closed `DataAccess` methods. It does not cache a shadow mode in `SessionData`. `measure-infobase-predefined-data-update.bsl` records all three enum members, the getter, changes in both directions, and English aliases on platform 8.3.27. That measurement also overrides the Syntax Assistant's optional-parameter notation: the zero-argument setter fails, so the public descriptor requires exactly one argument.

The following reached zero-argument procedure initializes predefined data in the current information-base area. Record creation, data-separator validation, and automatic-update policy require the same concrete provider and Session context, so `ИнициализироватьПредопределенныеДанные` / `InitializePredefinedData` is another additive fail-closed `DataAccess` operation. `bsl-orm` does not create records from its immutable metadata catalog. Both language names complete successfully in the throwaway platform database used by `measure-initialize-predefined-data.bsl`.

The reached `ОписаниеНастроек` / `SettingsDescription` value belongs to `bsl-session` beside the standard settings-storage managers that consume it. It is a small independent mutable value and needs no host capability: four properties are stored directly, while the read-only additional-properties slot owns a fresh standard mutable `Структура`. `measure-settings-description.bsl` records the defaults, writes, English aliases, type names, and the failure when replacing the read-only structure.

`ТекущаяДатаСеанса()` / `CurrentSessionDate()` is a platform global function, so it extends the existing base-runtime builtin table rather than the Session component library. It consumes the same per-State `Clock` and `TimeZone` already held by `HostEnv`: the host resolves the documented Session, information-base, and server time-zone priority before building the State, and the runtime converts the clock instant with that effective zone. It does not change the existing deliberately compatible behavior of `ТекущаяДата()`. `measure-current-session-date.bsl` records a `+03:00` Session result, equality with `ТекущаяДата`, the Date type, and the English alias on platform 8.3.27.

The next UNF compiler barrier is the platform value `КвалификаторыСтроки` / `StringQualifiers`. It has no Session, database, or configuration ownership, so it belongs beside the existing date qualifier in the base-runtime component registry. The value stores only its measured length and closed allowed-length enum and exposes them through read-only properties; it does not widen the currently supported qualified `ОписаниеТипов` constructor until startup reaches that composition. `measure-string-qualifiers.bsl` records defaults, explicit values, property mutability, type names, the complete two-member enum, and the observed length normalization (`-1` becomes `0`, `1.5` becomes `1`) on platform 8.3.27.

After that value is available, UNF reaches the direct three-argument `ОписаниеТипов` / `TypeDescription` form: type names, an omitted number qualifier, and a string qualifier. The existing one-argument bytecode path remains unchanged. The existing component constructor broadens only to the direct two-through-five-argument family; the separate source-description constructor with up to seven arguments remains outside this reached slice. Platform 8.3.27 accepts a date-qualifier value in the nominal string-qualifier position, so the runtime accepts supported qualifier objects by their own type across the optional qualifier slots instead of inventing stricter positional rejection. The runtime's established narrow type-description representation continues to retain type identities only, so serializing or applying string-qualifier conversion is not promised by this compiler-unblocking slice. `measure-qualified-type-description.bsl` records the reached constructor, retained type membership, language aliases, and that cross-position acceptance.

The next UNF barrier is the global `XMLТипЗнч` / `XMLTypeOf`. This belongs to `bsl-xml`, which already owns the measured primitive-to-XML-Schema mapping used by XDTO serialization. The implementation reuses that mapping and introduces the small immutable `ТипДанныхXML` / `XMLDataType` value there; it does not duplicate XML rules in the base runtime or Session crates. Values without a measured mapping return `Неопределено`. Configuration-defined XDTO types remain the responsibility of the configuration factory and are outside this primitive startup slice. `measure-xml-type-of.bsl` records the primitive names, namespace, equality, properties, constructor, aliases, and unmapped behavior on platform 8.3.27.

The compiled UNF graph also reads the platform global `СериализаторXDTO` / `XDTOSerializer`. `bsl-xml` already implements that serializer and its measured constructor, so the global is another library-declared zero-argument value factory beside the existing configuration `ФабрикаXDTO`. It uses that same configuration factory rather than creating an unrelated empty model. Until a host supplies configuration XDTO packages, reading either configuration global fails with the existing explicit configuration-factory error; compilation still recognizes the platform name, and standalone behavior does not pretend that configuration types are available.

The same serialization helper supplies `НазначениеТипаXML.Явное` to the global serializer. `НазначениеТипаXML` / `XMLTypeAssignment` is a closed platform enumeration independent of configuration and Session state, so it extends the existing base-runtime system-enum table. `measure-xml-type-assignment.bsl` records both members, their presentations, type display, and English aliases on platform 8.3.27.2074.

UNF next constructs `ОписаниеЗащитыОтОпасныхДействий` / `UnsafeOperationProtectionDescription` and immediately assigns its only property. The installed 8.3.27.2074 Syntax Assistant defines a zero-argument constructor and the read-write Boolean `ПредупреждатьОбОпасныхДействиях` / `UnsafeOperationWarnings` property (`platform-syntax-help:syntax-context:objects/catalog63/UnsafeOperationProtectionDescription.html`). This passive value belongs to `bsl-session`, beside the safe-mode surface that consumes Session security policy. It carries configuration data only and does not mutate VM security modes or disable host checks. `measure-unsafe-operation-protection.bsl` records the initial `Истина`, mutation through both language names, and the type names on platform 8.3.27.

The same startup path reads `ОтправкаЭлементаДанных.Игнорировать` / `DataItemSend.Ignore`. This is a closed platform enumeration independent of a Session or configuration catalog, so it belongs to the existing base-runtime enum table. The implementation includes the complete three-member set measured on platform 8.3.27 by `measure-data-item-send.bsl`: `Авто` / `Auto`, `Удалить` / `Delete`, and `Игнорировать` / `Ignore`. It does not add exchange-plan transport behavior.

The next UNF compiler barrier is the global `СтрСравнить` / `StrCompare` function. It is a platform string operation independent of configuration, Session, and database state, so it extends the existing base-runtime builtin table. Platform 8.3.27 converts non-string arguments through the ordinary String representation, so the VM performs the same existing `bsl-format` conversion used by `Строка` before the base runtime compares the resulting strings. The implementation returns only `-1`, `0`, or `1`; it does not introduce a configurable collation service or reuse identifier folding as an unmeasured substitute for platform string ordering. `measure-str-compare.bsl` records conversion, equality across case, ordering, empty strings, Cyrillic letters, `Е` versus `Ё`, and both language names.

The following reached global is `ТекущийЯзык` / `CurrentLanguage`. The selected interface language is part of one opened Session: the host resolves the authenticated user's selected language and falls back to the configuration's primary language before startup. The Session stores that immutable identity and the global does not reread user or configuration tables on every call. A background Session receives its own explicit snapshot with its separately opened resources. Platform 8.3.27 returns the corresponding configuration Language metadata object in the measured managed Session; the documented String variant is outside this reached slice. The ORM catalog therefore acquires Language objects and the primary-language identity from the same configuration snapshot. Missing or inconsistent language data fails Session startup when configuration code requests the function instead of silently choosing a process locale. `measure-current-language.bsl` records the metadata-object form, its name, the primary-language identity, and both language names.

The reached zero-argument `ПолучитьПроверкуСложностиПаролейПользователей` / `GetUserPasswordStrengthCheck` function reads another information-base administration setting. It remains a `bsl-orm` operation beside the already reached minimum-password-length and authentication-lock settings. Each call delegates to the current Session's `DataAccess` with its frozen principal; the runtime keeps no process-local default or cross-Session cache. This slice exposes only the read operation reached by configuration startup. Changing the setting remains outside this change.

The same startup graph contains four zero-argument numeric getters for maximum and minimum password lifetime, password reuse limit, and expiration-notification period. They follow the same boundary: `bsl-orm` owns the BSL names and values, while the current Session's `DataAccess` reads the information-base settings for the frozen principal. They are grouped because their platform surface and isolation contract are identical. The matching write operations remain outside this read slice until execution reaches them.

The conservative graph also reaches the six matching one-argument write procedures. Their names, aliases, and arity come from the 8.3.27 Syntax Assistant and their call shapes are present in the acquired graph. A platform write was deliberately not performed on the test database. `bsl-orm` therefore performs no inferred conversion or range validation: it passes the BSL value, setting identity, current principal, and call context unchanged to `DataAccess`. The concrete database adapter owns validation, authorization, and persistence. Artificial adapters verify routing without changing a real information base.

The next compilation barrier is the platform value type `ФорматированнаяСтрока` / `FormattedString`. It belongs to the base runtime rather than the Session or ORM because configuration code constructs and passes it without a database resource. The first slice preserves immutable content segments and the five-argument formatting description while exposing the measured plain-text conversion, filledness, and value equality. It accepts the Syntax Assistant's variadic content form and the formatting overload without adding UI rendering or mutable methods.

The following compilation barrier is the closed platform enumeration
`ОбходРезультатаЗапроса` / `QueryResultIteration`. It also belongs to the base
runtime: query-result methods consume it, while no member depends on a
configuration or database connection. The Syntax Assistant defines all three
members, and a platform probe records both spellings, their presentations and
the member type. Adding that finite table is sufficient here; query execution
remains the ORM adapter's responsibility.

The next UNF compiler barrier is the closed platform enumeration `РежимПроведенияДокумента` / `DocumentPostingMode`. It is independent of Session and database state, so it extends the existing single enum table in `bsl-rt` with the complete measured member set. This slice only makes the platform values available to original configuration code; it does not add document objects, posting, register writes, or transaction policy. `measure-document-posting-mode.bsl` records the two members, their presentations, type, and English aliases on platform 8.3.27.

The adjacent startup barrier is the closed `РежимЗаписиДокумента` / `DocumentWriteMode` enumeration. It follows the same ownership rule and extends the same table with the complete measured write, posting, and undo-posting members. The values let configuration handlers compile but do not create a document persistence API. `measure-document-write-mode.bsl` records all members, presentations, the member type, and English aliases on platform 8.3.27.

The next reached operation is `НайтиПоСсылкам` / `FindByRef`. Search scope, accessible metadata, data separation and rights all depend on the current Session connection, so `bsl-orm` validates the four array-or-Undefined arguments and delegates a request to an additive fail-closed `DataAccess` method. The adapter returns the same neutral tabular rows used by Query, and `bsl-orm` materializes a standard ValueTable. It does not scan the immutable metadata catalog, invent process-local references, or bypass RLS. `measure-find-by-ref.bsl` records arity, accepted omitted arguments, aliases, argument validation, and the four-column empty result (`Ref`, `Data`, `Metadata`, `DataSeparation`) on platform 8.3.27.

The following UNF compiler barrier is the platform string function `СтрЧислоВхождений` / `StrOccurrenceCount`. It is independent of configuration, Session, and database state, so it extends the existing base-runtime builtin table and the existing UTF-16 string representation. Platform 8.3.27 converts both arguments through the ordinary String representation, then counts non-overlapping, case-sensitive matches and returns zero for an empty substring. The VM therefore reuses its existing `bsl-format` conversion path and passes two runtime strings to a small UTF-16 operation; it does not infer conversion from Rust's string API or introduce another text representation. `measure-str-occurrence-count.bsl` records overlapping candidates, an empty substring, case, converted values, the English alias, and the result type.

The next compiler barrier is the closed `ТипПлатформы` / `PlatformType` system enumeration. The installed 8.3.27.2074 Syntax Assistant lists all seventeen values in `platform-syntax-help:syntax-context:objects/catalog2/catalog2088/PlatformType.html`. They are constants independent of the current host, Session, or database, so the complete measured set extends the existing single enum table in `bsl-rt`. This does not add a current-platform detector or make the interpreter claim the operating system named by any member. `measure-platform-type.bsl` records every presentation, the member type, and the English aliases on platform 8.3.27.

The following UNF barrier is `СмещениеСтандартногоВремени` / `StandardTimeOffset`. The acquired graph uses all documented arities and both literal and computed zone identifiers, so a fixed-offset compiler placeholder would not execute the original code. The operation extends the existing per-State `TimeZone` capability with standard-offset and named-zone lookup while preserving defaults for existing host implementations. `SystemTimeZone` resolves IANA identifiers from the same TZif data already parsed in-tree, and GMT identifiers need no file. No process-global `TZ` mutation, external time-zone crate, or Session cache is introduced. The omitted zone uses the State's current zone, and the omitted UTC date uses the State clock. Platform 8.3.27 also accepts the same canonical digit String as `Дата(...)` for the UTC argument, so the implementation reuses that parser rather than adding a second date syntax. An unknown named zone or another argument type fails explicitly. The installed Syntax Assistant contract is `platform-syntax-help:syntax-context:objects/Global context/methods/catalog1762/StandardTimeOffset4028.html`; `measure-standard-time-offset.bsl` records arity, fixed and named zones, daylight-independent results, types, aliases, conversion, and invalid inputs.

The adjacent reached getter is `ЧасовойПоясСеанса` / `SessionTimeZone`. The identifier belongs to the same per-State `TimeZone` capability as its offsets: existing custom implementations receive an additive optional method, the system implementation reads the configured local-zone identity, and a fixed zone returns an equivalent documented GMT identifier. The getter fails explicitly when a host supplies offsets without an identity. It does not add `УстановитьЧасовойПоясСеанса`, mutate process-global `TZ`, or let one State change another. The installed Syntax Assistant contract is `platform-syntax-help:syntax-context:objects/Global context/methods/catalog33/SessionTimeZone4021.html`; `measure-session-time-zone.bsl` records the zero-argument shape, String result, English alias, and isolated platform Session value.

The next UNF compiler barrier is the configuration-defined `Документы` / `Documents` manager root. Document descriptors and manager-module links are already acquired into the immutable catalog, and the generic manager value already provides platform-first and export-second dispatch. This slice only projects `MetadataKind::Document` through that existing path and through `Метаданные.Документы` / `Metadata.Documents`; it adds no document reference, object, persistence, posting, or register-write behavior. Unknown document names and unknown manager members retain the existing explicit manager errors.

The following barrier is the configuration-defined `ЖурналыДокументов` / `DocumentJournals` manager root. Database acquisition already classifies these descriptors as `MetadataKind::DocumentJournal` and links their manager modules. The runtime therefore projects that existing kind through the same generic manager and metadata-collection paths. It does not add journal selection, filtering, or form behavior.

UNF then reaches the configuration-defined `Отчеты` / `Reports` manager root. Acquired `MetadataKind::Report` descriptors already carry their manager-module links, so the same generic manager and metadata-collection paths expose them. This projection adds neither report-form opening nor a data-composition engine.

The next reached root is `Обработки` / `DataProcessors`. Acquired `MetadataKind::DataProcessor` descriptors and manager-module links use the same generic projections. This slice does not create data-processor objects or open their forms.

The repeated barriers establish that configuration startup expects every database-backed manager root already represented by the acquired catalog, rather than only the first root encountered by compilation. The remaining closed set is `РегистрыНакопления`, `РегистрыБухгалтерии`, `РегистрыРасчета`, `ПланыСчетов`, `ПланыВидовХарактеристик`, `ПланыВидовРасчета`, `БизнесПроцессы`, `Задачи`, and `ХранилищаНастроек`, with their standard English aliases. They use the same generic manager and metadata-collection paths. This completes projection of the kinds already acquired by `bsl-config-db`; it does not add persistence, register writes, workflow execution, or settings storage operations.

UNF next reaches `Последовательности` / `Sequences`. Unlike the preceding roots, the ORM catalog did not yet represent this kind even though `open-sdbl` already resolves its live database identity as `MetadataKind::Sequence`. `bsl-config-db` therefore maps the measured configuration class `bc587f20-35d2-11d6-a3c7-0050bae0a776` into a new ORM `MetadataKind::Sequence`, while the ORM projects it through the same generic manager and metadata collection. This slice does not implement sequence-boundary registration or mutation.

The next compiler barrier is the closed platform enumeration `ДопустимыйЗнак` / `AllowedSign`. It is independent of configuration, Session, and database state, so it extends the existing single enum table in `bsl-rt`. `measure-allowed-sign.bsl` records both members, their presentations, the member type, and English aliases on platform 8.3.27.

UNF then constructs `КвалификаторыЧисла` / `NumberQualifiers`. This passive value belongs beside the existing date and string qualifiers in `bsl-rt`: it stores two normalized nonnegative integer counts and an `AllowedSign`, and exposes three read-only properties. Platform 8.3.27 truncates fractional counts, clamps negative counts to zero, and caps fractional digits at total digits. This slice makes the qualifier available to original configuration code but does not yet widen `ОписаниеТипов` with number-qualifier semantics.

The next reached value is the corresponding `ПолучениеЭлементаДанных.Игнорировать` / `DataItemReceive.Ignore`. It follows the same ownership rule and includes the complete measured three-member set: `Авто` / `Auto`, `Принять` / `Accept`, and `Игнорировать` / `Ignore`. `measure-data-item-receive.bsl` records their presentations, aliases, and member type. These two enum additions still do not implement exchange-plan transport behavior.

`ПравоДоступа` / `AccessRight` evaluates configuration role data for a metadata object and therefore crosses the Session-owned `DataAccess` boundary. `bsl-orm` validates the BSL arguments and converts the optional subject into the frozen current principal, an explicit information-base user snapshot, or a configuration role identity. The adapter remains authoritative for the effective rights hierarchy and for the administrative permission required by an explicit subject. An omitted or `Неопределено` subject denotes the current Session principal. The fourth argument is retained as an optional standard attribute or standard tabular-section path. Privileged mode returns true after argument validation, as documented by the platform, without asking an adapter that represents ordinary rights. Absence of an adapter fails explicitly. The reached two- and three-argument calls and both language aliases are recorded by `measure-access-right.bsl`; the Syntax Assistant defines the full two-through-four-argument surface.

The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog33/AccessRight693.html`
says the result also depends on rights of the parent metadata object and the
role's independent-child-rights setting. Supplying the third argument requires
administrative rights even when it denotes the current user. The current
`open-sdbl` `RoleRights::grants` checks one object's stored right and its
`setForNewObjects` default; it does not resolve parent hierarchy or the
explicit-subject authorization. The concrete adapter therefore keeps
`AccessRight` unsupported until an upstream effective-right API can establish
those facts from the acquired configuration and roles. Calling `grants`
directly would turn a partial role-resource read into a platform claim.

`ИмяПользователя()` / `UserName()` returns the name from the same immutable `Principal` snapshot as `МенеджерПользователейИнформационнойБазы.ТекущийПользователь()`. It neither rereads `v8users` nor keeps a process-global current user. A Session opened for the empty platform user returns the empty string. `measure-user-name.bsl` records the zero-argument shape, string type, empty-user value, and language aliases.

The startup code also reaches the standard `Новый Соответствие(ФиксированноеСоответствие)` / `New Map(FixedMap)` constructor. This extends the existing native collection constructor instead of introducing a component type. The resulting mutable map copies keys, values, and insertion order; later replacement in the copy does not change the fixed source. Explicit `Неопределено` and a mutable map are type errors. `measure-map-copy-constructor.bsl` records these rules and both language spellings. The bytecode representation carries the optional source register on the existing map-construction operation and advances its text format version because older readers cannot interpret that operand.

The next UNF barrier is the read-only global `ПолитикиПаролейПользователей` / `UserPasswordPolicies`. The installed 8.3.27.2342 Syntax Assistant defines its manager type, the optional string argument of `НайтиПоИмени` / `FindByName`, the `Неопределено` result for an unknown name, and the numeric `МинимальнаяДлинаПаролей` / `PasswordMinLength` property. Policy records are information-base administrative data and may change independently of the immutable configuration catalog, so lookup is delegated to the `DataAccess` attached to the current Session together with its principal and call context. The adapter returns a copied snapshot containing only the reached property. `bsl-orm` neither caches it across Sessions nor adds the unreached mutation and password-validation surface.

The same startup path also uses the older zero-argument `ПолучитьМинимальнуюДлинуПаролейПользователей` / `GetUserPasswordMinLength` global function. The installed Syntax Assistant defines a numeric result representing the information-base-wide limit. It crosses the same Session `DataAccess` boundary as named-policy lookup because the value is administrative database state, not configuration metadata. The reached slice is read-only; the corresponding setter is not added merely because it appears beside the getter in platform help.

The conservative graph then constructs `ГенераторСлучайныхПаролей` / `RandomPasswordGenerator` and calls `СлучайныйПароль` / `RandomPassword`. `measure-random-password-generator.bsl` records the zero-argument constructor, zero-through-two-argument method, truncation and length boundaries, result type, and aliases on platform 8.3.27.2342. The existing `RandomSource` capability deliberately promises only sixteen bytes suitable for UUID construction and its process default has a non-cryptographic fallback when `/dev/urandom` is unavailable. Treating it as a password-grade source would silently weaken authentication. This slice therefore registers the exact object surface so the original graph compiles but rejects generation explicitly until a separate audited cryptographic host capability is designed. It does not return a predictable placeholder or silently ignore the memorable-password flag.

The next graph barrier is the passive `ИспользованиеСобытияЖурналаРегистрации` / `EventLogEventUse` value. `measure-event-log-event-use.bsl` records its zero-through-two-argument constructor, defaults, aliases, mutable Boolean `Использование` / `Use`, and arbitrary `ОписаниеИспользования` / `UseDescription`. Numeric assignment to `Use` follows the runtime's already measured Boolean conversion used by language conditions, while the constructor requires an actual Boolean. The value belongs to `bsl-session` beside the existing event-log writer surface but does not itself read or mutate event-log configuration.

The same configuration path reads and conditionally writes the use setting of one named event. Those operations extend the existing Session-owned `EventLogWriter` boundary with additive fail-closed methods because the concrete platform or database adapter owns event-log administration and authorization. `bsl-session` validates the event name and converts between the detached BSL value and a neutral copied snapshot; it does not keep a process-local setting or make `ЗаписьЖурналаРегистрации` consult a shadow value. The read surface and aliases are measured by `measure-event-log-event-use-operations.bsl`; the setter is exercised by writing back the value just read so the probe leaves the test base unchanged.

The following graph barrier is the passive `НастройкиКлиентскогоПриложения` / `ClientSettings` value. It is a detached mutable description, not the Session-owned client-settings storage adapter: constructing or changing it performs no I/O and shares no state between Sessions. `measure-client-settings.bsl` records its zero-argument constructor, four mutable properties, defaults, both language aliases, and the complete two-member sets of `РежимОткрытияФормПриложения` / `ApplicationFormsOpenningMode` and `ВариантИнтерфейсаКлиентскогоПриложения` / `ClientApplicationInterfaceVariant` on platform 8.3.27.2342. Both closed enumerations therefore extend the base runtime's single enum table, while the value itself belongs to `bsl-session` with the other client-session descriptions.

The next value is `НастройкиКомандногоИнтерфейса` / `CommandInterfaceSettings`. The installed 8.3.27.2342 Syntax Assistant defines a zero-argument serializable value with one mutable `ОтображениеПанелиРазделов` / `SectionsPanelRepresentation` property (`platform-syntax-help:syntax-context:objects/catalog1649/catalog1920/CommandInterfaceSettings.html` and `.../properties/SectionsPanelRepresentation9544.html`). The enumeration page `platform-syntax-help:syntax-context:objects/catalog2/catalog2088/SectionsPanelRepresentation.html` defines all five members. `measure-command-interface-settings.bsl` records the default, representations, member type, aliases, mutation, and rejection of an unrelated enum. Like `ClientSettings`, this is a detached value passed to the existing settings-storage adapter; it does not add another storage path or shared state.

The next reached expression is one connected value graph: `НастройкиИнтерфейсаКлиентскогоПриложения` / `ClientApplicationInterfaceSettings`, `НастройкиСоставаИнтерфейсаКлиентскогоПриложения` / `ClientApplicationInterfaceContentSettings`, its four position groups, nested `ГруппаНастройкиСоставаИнтерфейсаКлиентскогоПриложения` / `ClientApplicationInterfaceContentSettingsGroup`, and the named `ЭлементНастройкиСоставаИнтерфейсаКлиентскогоПриложения` / `ClientApplicationInterfaceContentSettingsItem`. The installed Syntax Assistant defines the four constructors, read-only position and name properties, `Добавить` / `Add`, and `УстановитьСостав` / `SetContent`; the measured probe also records `Количество` / `Count` and `Получить` / `Get` so insertion can be checked without relying on display text. The values are one detached graph: `SetContent` retains the explicitly supplied graph and `GetContent` returns it. The platform's initial `GetContent` is not universally empty: it is derived from configuration and user interface settings, and the measurement base already returns two top panels. This slice has no source for that host state, so `GetContent` before the first explicit `SetContent` fails instead of fabricating an empty composition. The optional omitted `SetContent` argument also fails explicitly; the reached configuration always supplies a composition. Full collection editing, iteration, positional indexing, panel-name validation, and configuration-derived defaults remain outside this reached slice.

The next compiler barrier is the passive `ОписаниеИспользованияСобытияДоступЖурналаРегистрации` / `EventLogAccessEventUseDescription` value. The installed Syntax Assistant defines zero- and optional-string construction plus mutable `Объект` / `Object`, `ПоляДоступа` / `AccessFields`, and `ПоляРегистрации` / `LoggedFields` properties. `measure-event-log-access-event-use-description.bsl` records an empty object name, two distinct empty arrays, mutation, both constructor forms and language spellings, and strict string/array property types on platform 8.3.27.2342. This value only describes later event-log administration; construction and mutation perform no I/O and do not extend the existing `EventLogWriter` boundary. The similarly named access-denied value is not constructed by the reached graph and is not added with it.

The graph then calls `ПолучитьИспользованиеЖурналаРегистрации` / `GetEventLogUsing` and searches its result for the information level. The installed Syntax Assistant defines a zero-argument function returning the array of enabled `УровеньЖурналаРегистрации` / `EventLogLevel` members. This is mutable information-base administration state, so `EventLogWriter` receives one additive read method with the existing explicit unsupported default. `bsl-session` converts its neutral `Vec<EventLogLevel>` snapshot into a fresh BSL array for every call. It does not cache the list, infer it from events written by this process, or return the four levels observed in the measurement base as a universal default. `measure-event-log-using.bsl` records the return type, English platform spelling, and the measured base's four enabled members.

The next graph barrier is the read-only global `БлокировкаАутентификации` / `AuthenticationLock`. The installed Syntax Assistant defines it as `МенеджерБлокировкиАутентификацииПользователейИнформационнойБазы` / `InfoBaseUserAuthenticationLockManager`, with reached `ПолучитьНастройки` / `GetSettings` and `УстановитьНастройки` / `SetSettings` methods. Its settings value has three mutable properties: numeric lock duration, numeric maximum unsuccessful-attempt count, and string user-name addition codes. The startup graph touches the first two, but the neutral snapshot carries all three so a read-modify-write does not erase the third. This is mutable information-base administrative state and belongs beside password policy operations on the current Session's `DataAccess`; neither the manager nor `bsl-orm` keeps a process-local authoritative copy. `measure-authentication-lock-settings.bsl` records both type names, all property types, aliases, local mutation, and the platform's declared-property conversions on 8.3.27.2342. The measurement remains read-only; the `SetSettings` signature and optional parameter come from the installed Syntax Assistant, while persistence is tested against an artificial adapter. Calling `SetSettings` without a value fails explicitly because no host default-source contract exists in this slice.

The same startup graph constructs `ПолеКомпоновкиДанных` / `DataCompositionField` and `СтандартнаяДатаНачала` / `StandardBeginningDate`, and reads their closed data-composition enums. These platform values have no session, database, or configuration ownership, so their measured constructors and enum behavior belong to the base `bsl-rt` component registry. Field equality folds path case while its display preserves the supplied spelling. `ObjectProtocol` therefore receives an optional canonical value-hash key; value objects that do not provide one retain the existing display-based hash. This keeps `Соответствие` consistent with equality without changing existing object behavior. Both closed enums use the existing single enum table and include their complete measured member sets rather than partial types dependent on one configuration. The recorded startup uses only the one-argument relative-variant constructor of the standard beginning date and never reads or writes its properties, so this slice stores that variant without adding a calendar evaluator or clock access to the component context. This slice does not introduce a data-composition engine or settings model.

Implementation proceeds in two acceptance slices. The first runs the complete startup handler with `ТребуемыеПараметры = Неопределено`. The second requests the explicitly selected session parameters used by the `demo`, `buh`, and `unf` evidence. Passing the first slice does not claim support for every dynamically registered BSP handler.

After the passive job-schedule value was added, the next UNF compiler barrier
is `ПодключитьВнешнююКомпоненту` / `AttachAddIn`. The installed 8.3.27.2342
Syntax Assistant defines a one-argument COM identifier form and a two-through-
four-argument location-and-name form (`platform-syntax-help:syntax-context:objects/Global context/methods/catalog42/AttachAddIn697.html`). The recorded graph contains two- and four-argument calls. Loading native process code is a host capability outside this Session slice, so `bsl-session` registers the documented call family but fails every invocation explicitly until such a loader is supplied. It does not turn an absent loader into `Ложь`, which would make configuration code treat an unattempted attachment as a platform result.

After that function is resolved, the UNF graph reaches `Новый СхемаЗапроса` / `New QuerySchema`. The installed 8.3.27.2342 Syntax Assistant defines only the zero-argument constructor (`platform-syntax-help:syntax-context:objects/catalog213/catalog2171/QuerySchema/ctors/ctor228.html`). The value belongs to `bsl-orm` beside `Запрос`, but implementing its full editable query object model is outside the session-startup slice. The ORM library therefore registers the exact constructor and fails invocation explicitly until that model exists. It does not return `Запрос` or an empty dynamic object whose members would silently claim platform behavior.

The following UNF barrier is `ЧастиДаты.Дата` / `DateFractions.Date`. The runtime already contained only the previously measured `ДатаВремя` member. The installed Syntax Assistant lists the complete closed set as `Время`, `Дата`, and `ДатаВремя`; a platform-only probe records the two missing presentations, their shared type, and both English aliases before the enum table is extended. This is a completion of the existing system enumeration, with no date conversion behavior added.

UNF then exposes a gap in the archived dynamic-Structure change: its platform probe measured missing and excess values only with a computed field list, while the literal fast path retained open-bsl's old exact-count rejection. A separate platform probe covers the same two cases with a literal. Exact-count and key-only literals retain the shaped bytecode path. A literal with a mismatched count reuses `NewDynamicStructure`, which already evaluates arguments left to right, supplies `Неопределено`, ignores evaluated excess values, and avoids a second constructor implementation.

UNF's conservative module closure next contains a server common module that passes another common module's bare name as a value. A dedicated platform common module proves that the configuration loads, while first access to the invalid module ends a background job in `ЗавершеноАварийно` with `ИнформацияОбОшибке`; the bare name is not a module-reference value. open-bsl therefore keeps its existing eager parse and resolution but records this one imported-alias error on the containing module and prepends an unconditional raise to that module's initialization chunk. This uses the existing lazy `ModuleState` lifecycle: an untouched module does not block Session startup, while first access fails before any module body or export runs. Ordinary unknown variables remain semantic build errors, and no new module-reference bytecode value is introduced.

The later reached UNF path is different: `ОбщегоНазначения.ОбщийМодуль(Имя)` first checks metadata, then runs `Вычислить(Имя)` with `Имя = "МультиязычностьСервер"` and expects the `ОбщийМодуль` value. The dynamic resolver previously rejected that alias before execution. Static bare-source failure remains measured and unchanged. A dynamic fragment resolves an imported alias to the existing Session-owned `ModuleReference` identity and initializes the target module through the existing lazy lifecycle before returning that identity. One bytecode operation loads the reference by catalog module ID; no second reference implementation or host-global cache is introduced. Unknown dynamic names and local/module-variable priority stay unchanged. A direct 1C 8.3.27 background-job probe with a valid target common module completed after `Вычислить("ЗамерыФоновыхЗаданийДвусторонние")`, type comparison, and an exported-method call; its output is `measure-dynamic-common-module-value.platform.txt`.

The next UNF execution passes dynamic name resolution and checks `ТипЗнч(Модуль) <> Тип("ОбщийМодуль")`; the core type catalog previously knew only the host-only `МодульBSL` descriptor. Catalog instances receive a second descriptor while standalone `ЭтотОбъект` keeps `МодульBSL`. Both descriptors use the existing module-reference dispatch; no second object or module state is introduced. The same 1C 8.3.27 probe measured both `Тип("ОбщийМодуль")` and `Тип("CommonModule")` as equal to the reference type; `Строка(ТипЗнч(...))` returned `Common module`, not the Russian lookup name, so the descriptor uses that exact display text.

The live UNF run then reaches `Константа.ОсновнойЯзык` and open-sdbl 0.9.4 rejects its `Projection` field with `no role grants its Read right`. The selected user `Администратор` has `ПолныеПрава`; read-only `open-sdbl` inspection reports object rights granted by default and `attributes by default: yes`. The upstream `access-rights` spec explicitly refuses to infer a field grant from `setForAttributesByDefault` until measured on 1C. This is an upstream proof-boundary decision, so this change does not bypass it in the CLI or ORM. The requested upstream follow-up measures the default and explicit-denial cases, changes that contract if confirmed, then re-runs the same principal through startup.

The next UNF compiler barrier is the closed platform enumeration
`РежимБлокировкиДанных` / `DataLockMode`. The installed 8.3.27.2342 Syntax
Assistant defines exactly `Разделяемый` / `Shared` and `Исключительный` /
`Exclusive`. The values do not carry Session or database state, so the
platform-only probe records their presentations, aliases, and common type
before they extend the base runtime's single enum table. This slice exposes
only the values; it adds neither a data-lock manager nor locking behavior.

The following graph barrier constructs
`КомпоновщикМакетаКомпоновкиДанных` / `DataCompositionTemplateComposer` and
calls its sole method `Выполнить` / `Execute`. The installed Syntax Assistant
defines a zero-argument constructor and a two-through-seven-argument method.
Actual template generation requires the data-composition object model and
engine, neither of which exists in this startup slice. `bsl-orm` therefore
registers the measured value type and exact reached surface so the original
configuration graph compiles, while every execution attempt fails explicitly.
It does not manufacture an empty template whose query and parameter members
would falsely look executable.

The next barrier is `ПроцессорКомпоновкиДанных` / `DataCompositionProcessor`.
The graph initializes it with one through four arguments and advances it with
`Следующий`; the installed Syntax Assistant gives the complete compact method
set: `Initialize` with one through six arguments plus zero-argument `Reset`
and `Next`. It follows the same boundary as the template composer: `bsl-orm`
registers the measured value and signatures, while operations fail explicitly
until one data-composition engine can implement both objects coherently.

The graph next reaches the value-collection output processor. Unlike the
template composer and processor, it also has one local mutable Boolean
property. Its default and type are measured on the platform; changing that
property remains local and does not pretend to execute output. The five
documented methods are registered with their exact arities and fail explicitly
at the same missing-engine boundary.

The next graph barrier is an exported common-module function whose second
parameter is required, while a qualified call explicitly leaves that middle
position empty and supplies a later parameter. This is distinct from calling
with too few positions. The platform-only common-module probe records that the
empty slot arrives as `Undefined`. The imported-call resolver therefore emits
an explicit undefined value only for `Some(None)` without a declared default;
`None` beyond the written argument list keeps the existing error, and local
calls keep their separately measured behavior.

The following barrier is the closed platform enumeration
`РезультатПроверкиСоответствияПароляПолитике` /
`PasswordPolicyComplianceCheckResult`. The installed 8.3.27.2342 Syntax
Assistant defines the complete four-member set, including the compromise-check
result added in 8.3.26. The values carry neither Session nor database state, so
the platform-only probe records every presentation, the common value type, and
both language spellings before the existing base-runtime enumeration table is
extended. This does not implement password validation or the password-policy
manager that produces arrays of these values.

The next reached value is the closed `ОтображениеТаблицы` /
`TableRepresentation` enumeration. Although the graph uses only list and tree,
the installed Syntax Assistant defines one compact three-member set, including
hierarchical list. A platform-only probe records every presentation, aliases,
and the common type before all three extend the same base-runtime table. This
adds no form object or client UI implementation.

The following graph barrier is the documented zero-argument `Картинка` /
`Picture` constructor, which creates an empty picture. The reached value is a
passive placeholder passed through interface descriptions; it reads no file
and invokes no picture operation. The base runtime therefore adds only this
measured empty value and constructor. File- and binary-data constructors,
image decoding, conversion, properties, and methods remain outside the slice
rather than being approximated by the empty value.

The next reached value is the closed
`ТипПримененияОтбораКомпоновкиДанных` /
`DataCompositionFilterApplicationType` enumeration. The installed Syntax
Assistant defines exactly items, hierarchy, and hierarchy-only. These values
carry no data-composition engine state, so a platform-only probe records the
complete set, presentations, aliases, and type before the existing
base-runtime enumeration table is extended. No filter evaluator or settings
object is implied.

The following barrier is another closed data-composition value,
`РежимОтображенияЭлементаНастройкиКомпоновкиДанных` /
`DataCompositionSettingsItemViewMode`. The installed Syntax Assistant defines
four members: auto, normal, quick access, and inaccessible. The same
platform-only measurement and base-runtime ownership rule applies; exposing
the values does not implement the settings items that use them.

The next reached constructor is `ПараметрКомпоновкиДанных` /
`DataCompositionParameter`. The installed 8.3.27.2342 Syntax Assistant defines
one String name, labels it optional, and lists no properties or methods on the
value itself. The real platform nevertheless rejects the zero-argument form;
`measure-data-composition-parameter.bsl` records that through `Evaluate` and
records the reached one-argument value's type, string conversion, filledness,
case-insensitive equality, and English spelling. Like the existing
`DataCompositionField`, this is an immutable base-runtime identifier rather
than a Session resource or a data-composition engine. This slice implements
only the measured one-argument behavior and does not add parameter collections,
settings, XML/XDTO serialization, or composition execution.

The next root-graph failure occurs at `Символы.Таб` / `Chars.Tab`.
The resolver already lowers line feed and carriage return to measured String
literals, but the Syntax Assistant defines one compact six-member set: carriage
return, line feed, form feed, horizontal tab, vertical tab, and non-breaking
space. The existing `measure-chars.bsl` probe is extended to all six Russian
and English spellings before the same compile-time lowering table is completed.
No runtime object, mutable character table, or locale dependency is introduced.

The following root-graph failure is the Syntax Assistant-defined read-only
global `ОбработкаСтрокиXML` / `XMLStringProcessing`, whose type is
`МенеджерОбработкиСтрокиXML` / `XMLStringProcessingManager`. The reached method
`НайтиНедопустимыеСимволыXML` / `FindDisallowedXMLCharacters` returns the
position of an XML 1.0/1.1-disallowed character. The installed 8.3.27.2342 help
is internally incomplete: its rendered signature has no parameters while its
description names the input String and a starting position. A platform-only
probe therefore fixes the arity, default and position unit before
implementation. The manager belongs to `bsl-xml`; its method reuses the XML
character classifier already used by the legacy Boolean global function, so
this slice does not introduce a second XML-validity implementation.

`measure-xml-string-processing-manager.bsl` establishes one required String
and one optional starting position, with one as the default. Positions are
one-based UTF-16 code-unit positions: a disallowed code unit after one emoji is
reported at three. Starting at the disallowed code unit finds it; starting at
the following position returns zero. Zero and three arguments are rejected,
and both English names have the same result.

The next root-graph failure is the closed `ВидПоляФормы` / `FormFieldType`
system enumeration at `ПолеТабличногоДокумента` /
`SpreadsheetDocumentField`. The installed 8.3.27.2342 Syntax Assistant lists
twenty members: HTML document, PDF document, input, geographical schema,
graphical schema, dendrogram, chart, Gantt chart, progress bar, calendar,
picture, label, radio button, period, planner, track bar, spreadsheet document,
text document, check box, and formatted document fields. A platform-only probe
records every Russian and English member, type, equality, and String
presentation before the complete closed set is added to the base runtime. This
does not add form objects or a client UI implementation.

The rooted UNF graph also reads `ВидГруппыФормы` / `FormGroupType`.
The installed 8.3.27.2342 Syntax Assistant lists eight members: button group,
column group, command bar, context menu, usual group, popup, page, and pages.
The base-runtime system-enum table gains that closed set after a platform-only
probe records its presentations, member type, and both language spellings.
This does not create form groups or form rendering.

The same graph reads the six-member `ВажностьПриОтображении` /
`DisplayImportance` system enumeration. The installed 8.3.27.2342 Syntax
Assistant identifies Auto, High, Low, Usual, VeryHigh, and VeryLow as its
complete set and permits server use. A platform-only probe records member
presentations, type, and aliases before the base-runtime enum table is
extended. The values do not add a form-rendering service.

The next rooted barrier is `ФорматКартинки` / `PictureFormat`. The installed
8.3.27.2342 Syntax Assistant defines ten members: BMP, EMF, GIF, Icon,
JPEG, PNG, SVG, TIFF, WMF, and UnknownFormat. They belong to the existing
closed system-enum table, not to the image value or a renderer. A platform-only
probe records their presentations, type, and both spellings before the table
is extended.

The next reached closed enumeration is `ДополнительныйРежимОтображения` /
`AdditionalShowMode`. The installed Syntax Assistant defines exactly
`НеИспользовать` / `DontUse` and `Неактуальность` / `Irrelevance`. A
platform-only probe records both spellings, type, equality, and String
presentation before those two values are added to the base-runtime enum table.
No `ОтображениеСостояния` object or form rendering is introduced.

The following graph failure is the documented scalar conversion
`Булево` / `Boolean`, reached with a Number. The Syntax Assistant defines
exactly one Number, Boolean, or String argument and states that zero becomes
false while every other Number becomes true; it does not specify the accepted
String spellings. A platform-only probe therefore records numeric, Boolean,
Russian and English String inputs, rejected strings, aliases, and arity before
the conversion is added to the existing base-runtime builtin table.

`measure-boolean-conversion.bsl` establishes that String conversion trims
surrounding whitespace and compares without case: `Истина`, `True`, and `Yes`
are true; `Ложь`, `False`, and `No` are false. Russian `Да` / `Нет`, numeric
strings, and the empty String are rejected. Boolean values pass through;
negative and positive nonzero Numbers are true. Zero and two arguments and
`Undefined` are rejected, and the English function alias behaves identically.

The next reached constructor call supplies the third
`ЗащищенноеСоединениеOpenSSL` / `OpenSSLSecureConnection` argument. The
installed Syntax Assistant defines it as the closed
`РежимПроверкиОтзываTLSСертификатаСервера` /
`ServerTLSCertificateRevocationCheckMode` enumeration with `Авто` / `Auto`,
`НеПроверять` / `DontCheck`, `Нестрогий` / `SoftFail`, and `Строгий` /
`Strict`, and exposes the selected value through a read-only property. A
platform-only probe records the default, each explicit value, aliases, type
checking, and arity before implementation.

The BSL HTTP object retains the selected value and the neutral
`HttpClientConfig` carries an explicitly supplied mode to the host factory.
The existing zero-through-two-argument calls keep their existing transport
behavior. The system `reqwest`/rustls adapter cannot fetch or apply the
platform's certificate-revocation policy, so it rejects an explicitly
supplied mode it cannot enforce with `Unsupported`; it never silently treats
`Auto`, `SoftFail`, or `Strict` as `DontCheck`. This preserves the old path,
lets another host implement the policy, and avoids claiming security behavior
the adapter does not provide. No external dependency is added.

`measure-tls-revocation-mode.bsl` establishes the four member presentations
and the read-only property's normalization. An omitted or explicit
`Undefined` argument yields `SoftFail`; in the measured platform environment,
an explicit `Auto` also yields `SoftFail`. `DontCheck`, `SoftFail`, and
`Strict` round-trip unchanged. The neutral host field preserves whether a
mode was supplied and preserves `Auto` as the caller's policy request even
though the BSL property exposes the measured effective `SoftFail`. A Number
in the third position and a fourth argument are rejected.

The next reached global is `ПолучитьИнформациюЭкрановКлиента` /
`GetClientDisplaysInformation`. The installed 8.3.27.2342 Syntax Assistant
defines a zero-argument function returning a `FixedArray` of immutable
`ИнформацияЭкранаКлиента` / `ClientDisplayInformation` values with numeric
`Width`, `Height`, and `DPI`; it returns `Undefined` when the Session has no
client, including a background job. Display geometry belongs to the concrete
client connection, so `SessionResources` accepts one owned snapshot and
`SessionRuntime` keeps it per Session. A foreground Session without a supplied
snapshot fails explicitly if the function is executed rather than inventing a
screen. A live background-job probe in `measure-client-displays-background.bsl`
confirmed `Undefined`. A background Session returns `Undefined` and does not inherit the
parent's display snapshot. The facade exposes one fixed-array construction
helper so `bsl-session` continues to depend only on `open-bsl`.

The rooted graph next reaches `Новый COMОбъект` in exception-guarded
cross-application code. The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog63/COMObject/ctors/ctor35.html` defines one required ProgID
string and one optional server-name string. COM activation is outside this
host's capabilities and unavailable on its Linux platform. The base runtime
recognizes the constructor and its arity, but execution raises a catchable
explicit unsupported error. It does not fabricate a COM value or connect to
an external application. This deliberate capability gap lets the existing
configuration error handling run without treating guarded code as a resolver
failure.

The graph also formats diagnostics with `ИмяКомпьютера` / `ComputerName`.
The installed Syntax Assistant page
`objects/Global context/methods/catalog33/ComputerName36.html` defines a
zero-argument string result, and `measure-computer-name.bsl` confirms a
non-empty string and identical Russian/English calls on the test platform.
The name belongs to the machine executing the concrete Session. The host
passes it as an optional owned `SessionResources` value; a call without it
fails explicitly, because using the interpreter process's hostname would
silently substitute the wrong machine in a remote Session. A background
Session receives its own value rather than copying the parent's.

The next reached expression passes `РежимОкругления` / `RoundMode` members to
the existing `Окр` builtin. The installed Syntax Assistant lists exactly two
members, and `measure-round-mode.bsl` records both presentations, the
`RoundMode` type display, English aliases, and half-round results. The closed
enum enters the base runtime's existing enum table. `Окр` maps these two
values onto its already measured half-down and half-up arithmetic; its
existing numeric third-argument behavior remains unchanged.

The graph also contains administrative calls to
`ПолучитьСеансыИнформационнойБазы` / `GetInfoBaseSessions`. The installed
Syntax Assistant page
`objects/Global context/methods/catalog8/GetInfoBaseSessions3672.html`
defines a zero-argument array result and requires the `ActiveUsers` right.
The existing current-Session object does not represent the server's live
session roster, and the current database adapter has no authorized roster
source. The resolver recognizes the call, but execution fails explicitly
until an authorized host roster adapter is specified and supplied. Returning
the current Session as a one-element list would lie about both authorization
and the live server state.

The next rooted module references the platform global
`РегламентныеЗадания` / `ScheduledJobs`. Scheduled jobs are outside the
accepted runtime scope: only explicit background jobs are implemented. The
session library declares this global so modules containing guarded or
unreached scheduled-job code compile, but reading it raises an explicit
catchable unsupported error. No scheduled-job manager, scheduler, or
background-job alias is fabricated.

The rooted module graph also names `СистемаВзаимодействия` /
`CollaborationSystem`, a separate platform service not supplied by this
Session host. The session library declares the name for compilation and
raises a catchable unsupported error on read. It does not fabricate a
collaboration-system manager or a successful disabled state.

The same unreached collaboration code constructs
`ОтборОбсужденийСистемыВзаимодействия` /
`CollaborationSystemConversationsFilter` and
`ОтборСообщенийСистемыВзаимодействия` /
`CollaborationSystemMessagesFilter`. The installed 8.3.27.2342 Syntax
Assistant pages `objects/catalog63/catalog2454/CollaborationSystemConversationsFilter/ctors/ctor269.html`
and `objects/catalog63/catalog2454/CollaborationSystemMessagesFilter/ctors/ctor270.html`
define zero-argument constructors. Because this Session has no
collaboration service and no supported filter behavior, the constructor is
recognized but raises the same catchable explicit capability gap at
execution. No filter-shaped object is returned.

The rooted graph also constructs `ИдентификаторОбсужденияСистемыВзаимодействия` /
`CollaborationSystemConversationID` from one String. The installed
8.3.27.2342 Syntax Assistant page
`objects/catalog63/catalog2454/CollaborationSystemConversationID/ctors/ctor276.html`
defines that arity and says the constructor does not validate the string.
This host has no collaboration-system service or identifier value model. The
session library recognizes the constructor so guarded configuration branches
compile, then raises the same catchable explicit capability gap if executed;
it does not return a String pretending to be a conversation identifier.

The same unreached graph constructs `КонтекстОбсужденияСистемыВзаимодействия` /
`CollaborationSystemConversationContext` from one navigation-link String. The
installed 8.3.27.2342 Syntax Assistant page
`objects/catalog63/catalog2454/CollaborationSystemConversationContext/ctors/ctor267.html`
defines that arity. It follows the same explicit unsupported boundary: a
navigation-link String is not returned as a fabricated context object.

The rooted graph also contains `Новый ИнтернетПочтовоеСообщение` /
`New InternetMailMessage`. The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog63/catalog2442/catalog766/InternetMailMessage/ctors/ctor_Auto.html`
defines its zero-argument constructor. Mail-message values and SMTP delivery
are outside this Session startup slice. The session library recognizes the
constructor so the dormant utility branch compiles, but execution raises a
catchable explicit unsupported error rather than returning a fabricated mail
message. If startup reaches that branch, it fails visibly.

The same utility branch reads the closed `ТипТекстаПочтовогоСообщения` /
`InternetMailTextType` enumeration. The installed 8.3.27.2342 Syntax
Assistant defines HTML, CustomText, PlainText, and RichText as the complete
set and permits server use. A platform-only probe records presentations,
member type, and aliases before these values enter the base-runtime enum
table. The enum does not add mail-message construction or delivery.

The next rooted reference is `СпособКодированияНеASCIIСимволовИнтернетПочтовогоСообщения` /
`InternetMailMessageNonASCIISymbolsEncodingMode`. The installed 8.3.27.2342
Syntax Assistant lists exactly MIME, QuotedPrintable, and `БезКодирования` /
`None`, and permits server use. A platform-only probe records their
presentations, member type, and aliases before they enter the same closed
system-enum table. These values do not encode or send mail by themselves.

The same rooted mail utility branch reads `ВажностьИнтернетПочтовогоСообщения` /
`InternetMailMessageImportance`. The installed 8.3.27.2342 Syntax Assistant
lists High, Highest, Lowest, Low, and Normal as its complete server-available
set. A platform-only probe records presentations, type, and aliases before
the closed base-runtime enum table is extended. The value does not implement
mail-message construction or delivery.

The next rooted form-setting branch reads the closed
`ГоризонтальноеПоложениеЭлемента` / `ItemHorizontalLocation` enumeration.
The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog277/ItemHorizontalLocation.html`
lists Auto, Left, Right, and Center as the complete server-available set.
`measure-item-horizontal-location.bsl` confirms each presentation, the value
type, and English aliases. It extends the base runtime's existing enum table;
no form behavior is implied.

The next rooted form branch reads `ОтображениеФигурыКнопки` /
`ButtonShapeRepresentation`. The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog277/ButtonShapeRepresentation.html`
lists Auto, Always, None, and WhenActive as the complete server-available
set. `measure-button-shape-representation.bsl` confirms their presentations,
value type, and English aliases. This extends the existing base-runtime enum
table without implementing managed-form drawing.

The following rooted branch reads `ОтображениеОбычнойГруппы` /
`UsualGroupRepresentation`. The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog277/UsualGroupRepresentation.html`
lists None, NormalSeparation, StrongSeparation, and WeakSeparation as the
complete server-available set. `measure-usual-group-representation.bsl`
confirms the platform's English presentations, type, and aliases. This adds
only values to the existing enum table, not form-group rendering.

The next rooted reference is the global `ВыгрузитьЖурналРегистрации` /
`UnloadEventLog` procedure. A platform 8.3.27.2342 probe calls it through
`Выполнить` with an invalid `Неопределено` receiver, so no export or file
write occurs. Zero and six arguments fail arity validation; every count from
one through five reaches receiver type validation. The session library
declares this measured 1..5 surface for dormant utility code but raises a
catchable explicit unsupported error on execution. It neither reads the host
event log nor exports a synthetic one.

The next reached event-log utility calls `ПредставлениеСобытияЖурналаРегистрации` /
`EventLogEventPresentation`. The installed Syntax Assistant search does not
index its page, but the official 1C 8.2 documentation supplement confirms the
method exists. A direct 8.3.27.2342 platform probe
`measure-event-log-event-presentation.bsl` confirms exactly one argument and
the English alias: zero and two fail arity checking, while one reaches type
validation. The runtime registers that surface but raises a catchable explicit
unsupported error instead of guessing the localized event-name catalog.

The following rooted global is `КаталогПрограммы()` / `BinDir()`. The
8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog33/BinDir40.html`
defines a zero-argument String naming the directory of the 1C platform
executables. The directory of `bsl-cli` is not that value. Until the host
provides a 1C platform installation path, the base runtime recognizes the
call but raises a catchable explicit unsupported error; it does not return the
interpreter executable's directory.

The following rooted barrier is the legacy `ПостроительОтчета` /
`ReportBuilder` constructor. The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog63/catalog609/ReportBuilder/ctors/ctor56.html` defines an
optional query-text String (zero or one argument). The ORM library owns this
data-facing surface beside `Запрос`. Because the legacy builder's query,
settings, and output model are not implemented, construction raises a
catchable explicit unsupported error; no Query value is substituted for a
ReportBuilder. A startup path that executes this branch therefore fails
visibly.

The related `ОписаниеИсточникаДанных` / `DataSourceDescription` constructor
is also present in the rooted graph. The installed 8.3.27.2342 Syntax
Assistant page `objects/catalog125/catalog177/DataSourceDescription/ctors/ctor79.html`
defines one required data-source argument. Its source adaptation and member
model belong to `bsl-orm`, but are not implemented in this startup slice.
The ORM library therefore recognizes the constructor and fails explicitly
if called, without fabricating a description from a query result or table.

The next rooted constructor is `Новый ОписаниеКонфигурации` /
`New ConfigurationDescription`. The installed 8.3.27.2342 Syntax Assistant
page `platform-syntax-help:syntax-context:objects/catalog63/catalog272/ConfigurationDescription/ctors/ctor282.html`
requires one BinaryData value containing a `.cf` or `.cfe` file. Parsing that
container is separate from acquiring live metadata or executing the startup
handler. The ORM library recognizes the exact constructor shape but raises a
catchable explicit unsupported error until the file parser and value model
exist; it does not create a description from the current live catalog.

The next rooted global is `ЗаблокироватьДанныеДляРедактирования` /
`LockDataForEdit`. The installed 8.3.27.2342 Syntax Assistant defines a
required database object reference or record key, an optional Base64 data
version, and an optional form UUID. `measure-lock-data-for-edit-arity.bsl`
confirms one through three arguments and the English alias without acquiring
a lock: an `Undefined` key fails type validation after arity checking. This
operation has different identity and lifetime rules from the existing
named-space `DataLockRequest`. Until the Session database adapter implements
record/edit locks, the ORM library registers the measured procedure but fails
execution explicitly. It does not simulate a lock in process memory or route
the call to `DataAccess::lock_data`.

The paired `РазблокироватьДанныеДляРедактирования` /
`UnlockDataForEdit` has zero through two arguments. Its optional key and
form UUID select which edit locks to release, so it shares the same missing
Session adapter, not the named-space `DataAccess::lock_data` operation.
`measure-unlock-data-for-edit-arity.bsl` confirms all three accepted arities
and the English alias on platform 8.3.27.2342 without acquiring a lock.
The ORM library registers the procedure but fails explicitly until the
adapter exists; it must not report a successful release it did not perform.

The next missing global calls are the pure date boundaries
`НачалоЧаса` / `BegOfHour` and `КонецЧаса` / `EndOfHour`. They belong beside
the existing `DateBoundary` selectors in the base runtime, not to Session
data. The installed Syntax Assistant defines one required Date argument for
each. `measure-hour-boundaries.bsl` records both aliases and the ordinary
and empty-date results; in particular the end of the hour containing the
empty date is `0001-01-01 00:59:59`, not the empty date itself.

The rooted graph next reads `ВидЧисловогоЗначения` /
`NumericValueType`. The installed Syntax Assistant lists exactly
`Количественное` / `Cardinal` and `Порядковое` / `Ordinal`; the platform
probe `measure-numeric-value-type.bsl` confirms their presentations and
`NumericValueType` display. This is a closed, Session-independent system
enumeration and extends the base runtime's existing single enum table.

The graph names the platform function `СтрокаСЧислом` /
`StringWithNumber`. The installed Syntax Assistant page
`objects/Global context/methods/catalog20/StringWithNumber5273.html`
defines three required arguments, an optional number-format string, and
language-specific plural categories. The existing formatter has no such
plural-category engine, and implementing only Russian cases would silently
give wrong results for other Session locales. The base runtime recognizes
the documented arity but raises a catchable explicit unsupported error on
execution until a separately measured localization implementation exists.

The next rooted UNF reference is `ПолучитьСклоненияСтроки` /
`GetStringDeclensions`. The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog4838/GetStringDeclensions5452.html`
defines one required String and two optional description/format Strings, and
returns an array of possible forms. Declension depends on a Russian morphology
engine and Session localization; the base runtime has neither. It recognizes
the documented one-through-three-argument surface and fails explicitly when
called, without fabricating the original string as a declension result.

The following rooted call is `ЧасовойПояс()` / `TimeZone()`. The installed
8.3.27.2342 Syntax Assistant names the computer's zone, not the Session zone.
`measure-computer-time-zone.bsl` confirms that changing the Session zone to
`GMT+0` leaves the computer result at `Europe/Moscow`. `HostEnv::process`
therefore captures the system zone identifier separately from its effective
Session `TimeZone`; replacing the latter through the existing State builder
does not rewrite the computer identifier. The builtin returns that captured
identifier and fails explicitly if the host system has no identifiable zone.

The next conservative-closure module constructs `Шрифт` / `Font` in UI
formatting code. The installed Syntax Assistant defines a default constructor,
seven optional description fields, and a variant based on an existing font
with seven optional overrides (`objects/catalog63/catalog1896/Font/ctors/ctor34.html`,
`ctor8.html`, and `ctor9.html`). Font rendering and style inheritance are not
part of Session startup. The base runtime recognizes these shapes so the
module can compile, but invocation raises a catchable explicit unsupported
error. It neither invents a font object nor changes the configuration's
error handling.

The closure also calls `ТРег` / `Title`. The installed Syntax Assistant page
`objects/Global context/methods/catalog4838/Title2783.html` requires one
string and defines Unicode word-boundary casing, with Latin-only conversion
in a web client. The workspace has scalar uppercase/lowercase operations but
no equivalent word-boundary implementation. The builtin is registered with
the documented arity and raises a catchable explicit unsupported error on
execution, avoiding an incorrect ASCII or whitespace-only conversion.

The next graph failure is `ПолучитьОбщийМакет` / `GetCommonTemplate`, used
throughout reporting code. The installed Syntax Assistant page
`objects/Global context/methods/catalog978/GetCommonTemplate376.html`
requires one template name or metadata object and returns a template value
whose type depends on the resource. `bsl-config-db` currently acquires code
and ORM metadata, not the template resource set or its value decoders.
`bsl-orm` owns this configuration-bound global, recognizes its call shape,
and raises a catchable explicit unsupported error when called. Returning a
blank text or spreadsheet document would claim that a real template was
loaded and could alter application behavior.

The UNF graph then constructs `ДеревоЗначений` / `ValueTree`. The installed
Syntax Assistant page `objects/catalog234/catalog264/ValueTree/ctors/ctor_Auto.html`
defines the zero-argument constructor; the value has hierarchical rows and
columns, unlike the existing flat `ValueTable`. Registering the constructor
with an explicit catchable error permits dormant tree code to resolve without
claiming that a flat table implements tree semantics. If startup executes
this call, the failure remains visible and the tree model must be implemented
before the scenario can pass.

The next graph barrier is the `НаправлениеСортировки.Убыв` /
`SortDirection.Desc` member of an existing base-runtime enumeration. The
runtime previously carried only `Возр` with the non-platform English alias
`Ascending`. The installed Syntax Assistant page
`objects/catalog2/catalog2088/SortDirection.html` lists exactly `Возр` /
`Asc` and `Убыв` / `Desc`; `measure-sort-direction.bsl` confirms both aliases
and distinct values and the `Ascending` / `Descending` string presentations
on platform 8.3.27. The table gains the missing member
and makes `Asc` canonical. A narrow lookup alias retains the previously
accepted `Ascending` spelling for additive compatibility; it does not become
a duplicate enumeration member.

The following graph barrier is the separate system enumeration
`ВидСравнения` / `ComparisonType`, not the already implemented data-composition
comparison enumeration. The installed Syntax Assistant page
`objects/catalog2/catalog2088/ComparisonType.html` lists 18 members.
`measure-comparison-type.bsl` checks every Russian/English pair and its String
presentation, plus the root and member type names, on platform 8.3.27.
The base runtime adds those values to its
existing system-enum table; no filter object or comparison operator is
implemented by this addition.

The next UNF compiler barrier is `УниверсальноеВремя` / `ToUniversalTime`.
The installed Syntax Assistant page
`objects/Global context/methods/catalog1762/ToUniversalTime4025.html`
defines one required local Date and one optional String-or-Undefined zone.
The platform probe `measure-to-universal-time.bsl` additionally shows that a
canonical date String is accepted, that an unknown named zone and Undefined
date fail, and that the existing local-time ambiguity rule applies to both
transition directions. The base runtime reuses `BslDate::parse_digits`,
`TimeZone::offset_for_local`, and the current Session zone. A new
`TimeZone::named_offset_for_local` method defaults to the existing GMT parser;
`SystemTimeZone` reads the same TZif table already used by
`StandardTimeOffset`. The conversion shifts the original Date in whole
seconds, preserving fractional ticks. There is no second date model or time
zone dependency.

The next reached global, `ПолучитьСкоростьКлиентскогоСоединения` /
`GetClientConnectionSpeed`, returns the two-member
`СкоростьКлиентскогоСоединения` / `ClientConnectionSpeed` system enum.
The installed Syntax Assistant pages
`objects/Global context/methods/catalog33/GetClientConnectionSpeed3894.html`
and `objects/catalog2/catalog2088/ClientConnectionSpeed.html` define the
method and values. `measure-client-connection-speed.bsl` confirms both
aliases, String and type presentations, and `Normal` in the measured managed
test Session. Speed is a property of one client connection, so the host
supplies a typed value with each `SessionResources`; no generic `Normal`
default is inferred for a web or thin client. A separate background Session
receives only its own resources. Missing speed fails explicitly when read.

Existing core constructors keep the resolver's original core-receiver path:
in particular, `ФиксированнаяСтруктура.Свойство` writes its second, output
argument there, and `ДвоичныеДанные.Размер` remains a closed call.
`СписокЗначений` is the old constructor routed through its component method
table because `Добавить` accepts explicit omitted positions. `ФоновыеЗадания`
keeps the old core-receiver path: `ОжидатьЗавершения` must retain its
procedure-in-expression diagnostic. Only the manager `Выполнить` call with
omitted positions substitutes `Undefined`, which its existing method already
treats as absent. The receiver proof retains the manager identity through
local variables without altering other manager methods. Newly registered
core value types use their component tables.
The existing `fixed_collections_are_shallow_immutable_snapshots_in_every_mode`
test and `a_registered_core_constructor_still_proves_a_core_receiver` are
regression gates; they pass at `HEAD` and caught the initial all-component
classification. The measured value-list test catches the opposite,
over-broad core classification. `legacy_wait_is_a_procedure_even_in_nested_and_dynamic_expressions`
guards the background manager's old route, while the reached `Выполнить`
call guards its omitted positions.
The reached `ВыполнитьПроверкуПравДоступа` /
`VerifyAccessRights` procedure uses the same `AccessRightRequest` shape as
`ПравоДоступа`, but delegates the operation to `DataAccess::verify_access_rights`.
The callback owns both the authorization decision and the access-denied event:
the ORM library cannot safely synthesize the platform's security log entry
from a boolean check. Its default implementation fails explicitly. The
platform syntax helper at
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog33/VerifyAccessRights4155.html`
(8.3.27.2342) specifies two required arguments, one optional standard
attribute, and the access-denied event on refusal. This does not invent a
grant when the host has no authorization adapter.

`ВнешниеОбработки` / `ExternalDataProcessors` is a platform global
property, not a configuration catalog member. The installed 8.3.27.2342
syntax helper documents an external-processor manager with Connect, GetForm,
and Create at
`platform-syntax-help:syntax-context:objects/Global context/properties/ExternalDataProcessors1718.html`
and
`platform-syntax-help:syntax-context:objects/catalog125/catalog224/ExternalDataProcessorsManager.html`.
The paired `ВнешниеОтчеты` / `ExternalReports` property is documented at
`platform-syntax-help:syntax-context:objects/Global context/properties/ExternalReports6125.html`
by the same installed syntax helper.
External processor loading is outside this Session startup slice. The ORM
library declares the global through its existing descriptor mechanism and
fails on evaluation, so the compiler can link dormant references without
inventing a manager or loading files.

The base runtime already formats and parses its own temporary-storage
addresses in `temp_storage.rs`. The real-platform probe
`measure-is-temp-storage-url.bsl` shows that
`ЭтоАдресВременногоХранилища` recognizes a syntactically valid UUID path
even without a Session entry or `seanceId`, accepts arbitrary query suffixes,
and returns False for malformed UUID paths, `Undefined`, and numbers. Keep
the recognizer next to the existing parser, but do not reuse `parse_address`,
which intentionally requires a `seanceId` for storage operations.

The installed 8.3.27.2342 syntax helper at
`platform-syntax-help:syntax-context:objects/catalog2/catalog258/SpreadsheetDocumentCellAreaType.html`
lists exactly four spreadsheet-cell area type members. The real-platform
probe `measure-spreadsheet-cell-area-type.bsl` fixes their root name, value
names, type name, and Russian/English alias identity. Add them to the
existing exhaustive enum table rather than creating a separate spreadsheet
enum implementation.

`ТипПодключенияВнешнейКомпоненты` / `AddInConnectionType` has only two
members in the 8.3.27.2342 syntax helper at
`platform-syntax-help:syntax-context:objects/catalog2/catalog2088/AddInConnectionType.html`.
The helper limits it to the thick client. The managed-form measurement runner
timed out when it referenced the enum directly, so its String and type
presentations are not measured; register an open question instead of claiming
that the documented English names are a platform oracle. This enum is needed
to compile dormant code in the reachable UNF graph.

The syntax helper at
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog4838/StrConcat4533.html`
defines an optional `StrConcat` separator. The real-platform probe
`measure-str-concat-optional-separator.bsl` confirms that omitted,
`Undefined`, and empty separators all join without an inserted character.
Change only the builtin's arity range and its separator handling; the
resolver already supplies `Undefined` for an omitted optional position.

The reached `ПредставлениеПериода` / `PeriodPresentation` function takes
two required dates and an optional format string according to installed
Syntax Assistant 8.3.27.2342 at
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog20/PeriodPresentation1044.html`.
Its output depends on locale, period boundaries, and the financial-period
flag. Register its name and arity in the existing builtin table, but raise
an explicit runtime error until those presentations are measured and
implemented. This lets dormant source compile without returning plausible
but wrong text if execution reaches the call.

The reached `ЧислоПрописью` / `NumberInWords` function has one required
number and two optional formatting arguments in installed Syntax Assistant
8.3.27.2342 at
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog20/NumberInWords714.html`.
The full spelling depends on locale, fractional digits, and counted-object
forms. Register the documented arity in the builtin table and fail
explicitly at runtime until those forms are measured and implemented.

`КраткоеПредставлениеОшибки` / `BriefErrorDescription` is documented as
a one-argument global function at
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog27/BriefErrorDescription3328.html`
(8.3.27.2342). The platform probe `measure-error-processing.bsl` already
records equality between that global and the manager method on a saved
error. The runtime already has `brief_error_description` for the manager;
wire the global builtin to the same function, as the existing detailed
description builtin does.

The zero-argument `Новый ОписаниеТипов()` form uses the all-optional
copy constructor documented at
`platform-syntax-help:syntax-context:objects/catalog63/catalog316/TypeDescription/ctors/ctor15.html`.
The real-platform probe `measure-empty-type-description.bsl` returns a
`Type descriptions` value with zero allowed types. Extend the existing
core constructor descriptor to admit zero arguments and use the same
empty-name path that already creates an empty `BslObject::TypeDescription`;
do not add a parallel value representation.

`ПроверкаЗаполнения` / `FillChecking` is the two-member system enum in
installed Syntax Assistant 8.3.27.2342 at
`platform-syntax-help:syntax-context:objects/catalog2/catalog2088/FillChecking.html`.
The real-platform probe `measure-fill-checking.bsl` fixes root and type
names, alias identity, and the non-obvious String presentations `Yes` and
`No`. Add both members to the existing enum table; do not infer their
display from the English member names.

The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog277/FoldersAndItems.html`
lists four members of `ГруппыИЭлементы` / `FoldersAndItems`.
`measure-folders-and-items.bsl` confirms their root, type, String
presentations, and Russian/English alias identity on the real platform.
They use the existing exhaustive system-enum table.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog805/GetAllFilesMask4321.html`
defines a zero-argument, OS-dependent string result. The real-platform
probe `measure-get-all-files-mask.bsl` returns `*` on the local Unix
installation for both names. Route this function through the existing
file-system capability and its `path_separator` rather than the process
OS, so a State's host context stays authoritative. A Windows-style
separator selects `*.*` from the Syntax Assistant example, with a
`НЕ ИЗМЕРЕНО` marker until checked on Windows 1C.

The installed 8.3.27.2342 Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog2086/InternetMailProtocol.html`
lists IMAP, POP3, and SMTP as the complete `ПротоколИнтернетПочты` /
`InternetMailProtocol` enum. The real-platform probe
`measure-internet-mail-protocol.bsl` confirms all three presentations,
the root and type presentations, and alias identity. Add it to the
existing exhaustive system-enum table.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog63/catalog2442/catalog766/InternetMail/ctors/ctor_Auto.html`
documents the zero-argument `InternetMail` constructor reached in the
configuration graph. No mail transport or mail value exists in this
workspace. Register only that constructor shape in the existing core
constructor table, and make execution fail explicitly until a transport
is designed. This does not claim to model a working mail object.

The companion page
`platform-syntax-help:syntax-context:objects/catalog63/catalog2442/catalog766/InternetMailProfile/ctors/ctor_Auto.html`
documents the zero-argument mail-profile constructor also reached by
the graph. Its mutable property model is not present, so resolve the
shape but fail explicitly upon execution instead of returning an
unusable placeholder.

`platform-syntax-help:syntax-context:objects/Global context/methods/catalog1762/GetInfoBaseURL3974.html`
defines the zero-argument information-base navigation URL and distinguishes
file, server, and web-client forms. A SQL connection string does not identify
the client launch URL, so the Session library resolves the reached call but
returns an explicit error until an authoritative host URL is available.

The real-platform probe `measure-to-local-time.bsl` confirms the
`МестноеВремя` / `ToLocalTime` inverse of the existing
`УниверсальноеВремя` conversion for fixed GMT and IANA zones, default
zone selection, date-string coercion, and catchable invalid-zone/type
errors. Add a named-zone offset-at-UTC query to the existing `TimeZone`
trait, using its GMT default and the existing system tzdata table. Then
perform the inverse shift in the current State's environment; do not add
a second time-zone parser.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog274/StyleColors.html`
defines `ЦветаСтиля` / `StyleColors` as a server-available global whose
members are Color values. Neither Color nor configuration style values
exist in this runtime. Register the reached global in the existing core
library-global table and fail evaluation explicitly; introducing a
placeholder style object would mask missing configuration data.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog1762/Find919.html`
documents the deprecated, case-sensitive two-argument `Найти` / `Find`.
`measure-legacy-find.bsl` confirms that its nonempty search agrees with
the existing `СтрНайти` machinery, but the empty substring yields one
rather than zero. Add a separate builtin name with exact arity two and
delegate nonempty searches to the existing implementation.

The companion Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog274/StyleFonts.html`
defines `ШрифтыСтиля` / `StyleFonts` as a server-available global whose
members are Font values. Add it beside `StyleColors` through the same
core library-global table. The runtime has no configuration style data
or Font value model, so evaluation fails explicitly.

`platform-syntax-help:syntax-context:objects/catalog1649/catalog1890/FormAttribute/ctors/ctor144.html`
documents `Новый РеквизитФормы` / `New FormAttribute` with required name
and TypeDescription plus three optional form-specific fields. It is
reached during server graph compilation, but there is no managed-form
attribute value model. Register the exact constructor arity in the
core table and fail execution explicitly.

The installed Syntax Assistant pages
`platform-syntax-help:syntax-context:objects/catalog63/catalog1896/Color/ctors/ctor33.html`
and `platform-syntax-help:syntax-context:objects/catalog63/catalog1896/Color/ctors/ctor7.html`
define zero-argument and three-component `Цвет` / `Color` construction.
The runtime has no Color value type. Resolve exactly those arities via
the existing core constructor table, but fail execution explicitly
instead of returning an unrelated value.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog2/catalog1073/DataCompositionDetailsProcessingAction.html`
lists seven members of the details-processing action enum. The real
platform probe `measure-data-composition-details-processing-action.bsl`
confirms both aliases and all String presentations. In particular,
`OpenValue` presents as `Open value ` with a trailing space. Preserve
the exact bytes in the existing exhaustive enum table.

The installed Syntax Assistant page
`platform-syntax-help:syntax-context:objects/catalog63/catalog1064/catalog1077/DataCompositionSettingsComposer/ctors/ctor_Auto.html`
defines the zero-argument settings composer reached by the graph. Keep
it in `bsl-orm` beside the existing template and processor surfaces.
The settings model is absent, so the constructor fails explicitly
instead of borrowing the unrelated template-composer value.

The next reached constructor is documented at
`platform-syntax-help:syntax-context:objects/catalog63/catalog1064/catalog1077/DataCompositionAvailableSettingsSource/ctors/ctor115.html`.
`DataCompositionAvailableSettingsSource` requires a composition schema and
accepts optional functional-option parameters. Register its documented
arity in `bsl-orm`, but fail explicitly until the settings-source value
model exists. This keeps graph compilation possible without fabricating
settings derived from a schema we cannot process.

The next rooted UNF compile names global `XMLЗначение` / `XMLValue`.
The installed Syntax Assistant entry
`platform-syntax-help:syntax-context:objects/Global context/methods/catalog1566/XMLValue1568.html`
defines two required arguments. The real platform probe
`measure-xml-value.bsl` confirms number, Boolean, String, Date, invalid
number, alias, and the difference between an empty global-number lexical
form (error) and a single-space form (zero). XDTO's existing empty-number
parser also yields zero. Register the
function in `bsl-xml`, reuse the XDTO primitive lexical parser for the
measured nonempty forms, and keep the empty-number guard local to this
global function. Other target types remain explicit errors pending a
platform probe; do not claim the whole documented return-type set.

The next rooted UNF compile reads `WebЦвета` / `WebColors`. The installed
Syntax Assistant root
`platform-syntax-help:syntax-context:objects/catalog274/WebColors.html`
defines it as a global set of Color values available on the server. The
runtime has no Color value model, so register it beside `StyleColors` in
the existing core library-global table and fail evaluation explicitly.
Local and module variables retain their existing priority.

The following rooted UNF compile reaches the six-argument copy form of
`Новый ОписаниеТипов`. Syntax Assistant
`platform-syntax-help:syntax-context:objects/catalog63/catalog316/TypeDescription/ctors/ctor15.html`
documents up to seven arguments for constructing from an existing
TypeDescription, distinct from the at-most-five-argument type-list form.
Extend the existing core constructor's accepted arity to seven, but fail
the six- and seven-argument executions explicitly until add/subtract
types and qualifier inheritance are implemented. Do not silently pass
only the first argument to the narrow current representation.

The next UNF compile reaches zero-argument
`ОписаниеТипов.ПривестиЗначение()` / `TypeDescription.AdjustValue()`.
Syntax Assistant
`platform-syntax-help:syntax-context:objects/catalog63/catalog316/TypeDescription/methods/AdjustValue1221.html`
marks its value argument optional. The real platform probe
`measure-type-description-default.bsl` shows defaults for single Number,
String, Boolean and Date types; an empty or multi-type description yields
Undefined, independent of type order. Change the existing method's
arity to zero or one and add the measured zero-argument path, leaving
the existing one-argument conversion path intact. A single type outside
the measured primitive set remains an explicit error.

The next rooted UNF compile reaches two-argument `Новый ЗаписьТекста` /
`New TextWriter`. The real 8.3.27.2342 probe
`measure-text-writer-arity.bsl` shows that the default zero-argument
constructor and the file-name forms through five arguments are accepted;
six arguments are rejected. Syntax Assistant lists the five file-name
arguments and separately names an uninitialized-object constructor.
Keep the existing one-argument bytecode and file behavior unchanged.
Register the rare zero- and multi-argument forms in the core constructor
table, rather than enlarging the bytecode and VM dispatch for code not yet
reached at runtime. Zero arguments produce an unopened writer value; the
two-argument UTF-8 form uses the existing file-opening path. Other
file-name forms fail explicitly until their encoding, separator, and append
semantics are implemented. They must never accept arguments and silently
write default UTF-8 output.

The following rooted UNF compile reaches global `ОбъединитьФайлы` /
`MergeFiles`. Syntax Assistant describes two exactly-two-argument forms:
an array of part names or a search mask, followed by an output path. The
8.3.27.2342 probe `measure-merge-files-arity.bsl` confirms the arity and
English alias. Register it as a core procedure with an explicit runtime
error until both forms can use the State-owned file-system capability with
their measured ordering and failure semantics. Compilation must not imply
that the host has merged files.

The next rooted UNF compile reaches `ПереместитьФайл` / `MoveFile`.
Syntax Assistant and the real 8.3.27.2342 probe
`measure-move-file-arity.bsl` agree on two required paths and the English
alias. The current State-owned `FileSystem` does not expose rename, and a
direct `std::fs::rename` would bypass its isolation. Register the procedure
with an explicit runtime error until a file-system rename capability and
its platform failure behavior are implemented.

The following rooted UNF compile reaches `КопироватьФайл` / `CopyFile`.
Syntax Assistant describes two required paths, overwriting a destination,
and optional URI schemes. The 8.3.27.2342 probe
`measure-copy-file-arity.bsl` confirms the exact arity and English alias.
The current file-system capability has no platform-compatible copy
operation, particularly for URI sources. Resolve the call through the core
procedure table and fail explicitly before touching either file until that
boundary is implemented.

The next rooted UNF compile reaches `ОписаниеОшибки()` /
`ErrorDescription()`. The real 8.3.27.2342 probe
`measure-error-description.bsl` shows an empty string outside a handler,
the current error inside it, and restoration of the outer error after a
nested handler. The VM's existing `current_exception` is one task-wide
value: it remains after a handler and is overwritten by an inner error.
Returning it here would knowingly violate both measured cases. Resolve the
zero-argument function through the core table with an explicit runtime
error until handler-scoped error state is implemented in the VM; do not
return stale or nested-wrong diagnostics.

This interim refusal was superseded by the accepted and archived
`2026-10-08-implement-handler-scoped-error-description` change. The current
contract is `openspec/specs/error-description/spec.md`; the active delta is
aligned with that contract rather than reintroducing the earlier refusal.
This is a documentation reconciliation, with no new API or runtime behavior.

The next rooted UNF compile reaches
`БезопасныйРежимРазделенияДанных` / `DataSeparationSafeMode`.
Syntax Assistant specifies one common-attribute name and says the answer
also incorporates `safe="true"` from `default.vrd`. The 8.3.27.2342 probe
`measure-data-separation-safe-mode-getter.bsl` confirms one required
argument, the English alias, and an error for an unknown separator. The
current Session carries configured separator names and VM frame counters,
but no authoritative `default.vrd` baseline. Returning the counter alone
could falsely report `False` for an active safe zone. Register the getter
with its measured arity and a catchable execution error until the host
supplies the missing baseline; do not infer it from a connection or session
kind. The existing setter remains unchanged.

The following rooted UNF compile reaches the destructive global
`УдалитьДанныеИнформационнойБазы()` / `EraseInfoBaseData()`.
Syntax Assistant documents the zero-argument operation and its deletion
of the current infobase or data area. The platform probe
`measure-erase-infobase-data-arity.bsl` executes only one-argument forms,
which the platform rejects at compilation; it intentionally never invokes
the destructive zero-argument form. This database mutation belongs to
`bsl-orm`, not the neutral core. Register the zero-argument procedure in
the ORM with an explicit runtime error. The current database adapter is
read-only and must never translate this call into SQL or report success.

The next rooted UNF compile reaches `ИсторияДанных` in a module procedure
that calls `УдалитьВерсии`, `ПолучитьНастройки`, and `УстановитьНастройки`.
The installed Syntax Assistant 8.3.27.2342 identifies `ИсторияДанных` as a
read-only global property of `МенеджерИсторииДанных` available on the server
(`platform-syntax-help:syntax-context:objects/Global context/properties/DataHistory11202.html`).
The current Session has no version-history service, and two reached methods
would mutate history. Register the global name in `bsl-orm` so the original
module compiles, but fail evaluation explicitly until a Session-bound service
exists. Do not fabricate an empty manager or route mutations through the
read-only SQL connection.
The reached `UserWorkHistory` property belongs to the Session because its
platform use is tied to the current user's SaveUserData right. Until a
history service is available, evaluating it fails explicitly. The distinct
`ReportsVariantsStorage` property can select a standard or a configured
custom store. The acquired configuration snapshot does not yet expose that
selection, so evaluation fails explicitly; routing it to the standard
client-settings adapter would claim the wrong store for some configurations.
It is not conflated with `ReportsUserSettingsStorage`. The reached `ChoiceParameter`
constructor belongs to the base runtime because its documented value is
detached from configuration metadata. Its exact two-argument signature is
registered now, and constructing it fails explicitly until the object model
exists. These signatures and contexts come from the installed 1C 8.3.27.2342
Syntax Assistant pages `UserWorkHistory7232.html`,
`ReportsVariantsStorage7459.html`, and `ChoiceParameter/ctors/ctor133.html`.
The rooted UNF diagnostic reached
`ТипРисункаТабличногоДокумента.Картинка`, then `Диаграмма`. Rather than
repeat a full database acquisition for every next member, a direct platform
measurement on 8.3.27.2342 recorded all thirteen Syntax Assistant-listed
members, their presentations, the common member type, and the English
`Picture` alias in
`tests/conformance/measure/1c/measure-drawing-kind.platform.txt`.
The measurement also corrects the old `Прямоугольник` presentation and type
display while preserving its lookup names. Extend only the existing enum and
type-display tables.
The next rooted UNF compile reached `КонецНедели`. The existing
`DateBoundaryOf` mechanism covers its one-argument signature, but the
calendar cannot be chosen from one measurement. The disposable file-base
measurement on 8.3.27.2342 at
`tests/conformance/measure/1c/measure-end-of-week.platform.txt` shows a
Sunday-to-Saturday week. The earlier live server oracle
`tests/conformance/measure/filesystem/file-date-components.platform.txt`
shows `НачалоНедели(2020-01-15)` returning Monday 2020-01-13, not Sunday
2020-01-12. A regional or information-base setting is a plausible inference,
not a verified input contract. Preserve existing `НачалоНедели` behavior and
resolve `КонецНедели` with a catchable unavailable-calendar error until the
Session can supply that setting. The `DATE.WEEKDAY_NUMBERING` oracle `1 7`
is independent of the start-of-week choice and can move to measured anchors.
The next rooted UNF compile reached `НачалоКвартала`. The installed Syntax
Assistant 8.3.27.2342 documents both quarter-boundary functions with one
date argument. Unlike week boundaries, their observed behavior does not
depend on the infobase's first-day-of-week setting. The direct platform
measurement at
`tests/conformance/measure/1c/measure-quarter-boundaries.platform.txt`
covers all four quarters, leap day, and the maximum calendar date. Add both
to the existing `DateBoundaryOf` path and preserve whole-second results.
The following rooted UNF compile reached `НеделяГода`. Syntax Assistant
defines a one-argument `WeekOfYear`, but the two observed week-start
calendars make a numeric result unsafe to choose without Session context.
Use the existing `DatePartOf` dispatch to resolve the name and report the
same missing calendar setting as `EndOfWeek`; do not change other date parts.
The next rooted UNF compile reached the closed
`СпособВосстановленияНастроекКомпоновкиДанных` /
`DataCompositionSettingsRefreshMethod` system enumeration. The installed
Syntax Assistant 8.3.27.2342 lists exactly `Full` and `CheckAvailability`.
The platform probe
`tests/conformance/measure/1c/measure-data-composition-settings-refresh-method.platform.txt`
records their string presentations, common member type, and English alias
equality. Extend the existing base-runtime enum and type tables; no
configuration catalog or new resolver branch is needed.
The following rooted UNF compile reached `ТипВыводаТекстаКомпоновкиДанных` /
`DataCompositionTextOutputType`. The Syntax Assistant lists exactly `Auto`,
`Output`, and `DontOutput`; the direct platform probe
`tests/conformance/measure/1c/measure-data-composition-text-output-type.platform.txt`
records their presentations, type, and alias equality. Add this second
closed enum through the same base-runtime tables.
The next rooted UNF compile reaches `Новый СтандартныйПериод`. The installed
Syntax Assistant defines three constructors (zero arguments, one period
variant, or two dates). The runtime does not yet have the corresponding
period-variant enumeration or mutable value model. Register only the documented
constructor arities in the existing ORM constructor table and return an
explicit runtime error until a startup path actually needs the value. This
does not invent period dates or conflate it with the unrelated standard
beginning-date variant.
The next recorded UNF global is the closed `РежимМасштабированияПросмотра` /
`ViewScalingMode` system enumeration. The installed Syntax Assistant lists
exactly `Auto`, `Large`, and `Normal`; the direct platform probe
`tests/conformance/measure/1c/measure-view-scaling-mode.platform.txt`
records their type, presentations, and alias equality. Use the existing
base-runtime enum table without adding a library global.
The recorded UNF graph then names two more data-composition constructors:
`СхемаКомпоновкиДанных` and the spreadsheet-document result output
processor. The installed Syntax Assistant defines only their zero-argument
forms. They belong beside the existing ORM data-composition constructors.
Until the schema and output models are needed by an executing startup path,
register those forms with explicit runtime errors rather than adding partial
objects that appear executable.
The next recorded UNF global is the five-member closed
`РасположениеИтоговКомпоновкиДанных` / `DataCompositionTotalPlacement`
system enumeration. Syntax Assistant 8.3.27.2342 defines the full list;
the direct platform probe
`tests/conformance/measure/1c/measure-data-composition-total-placement.platform.txt`
records its member type, presentations, and aliases. Add it to the existing
base-runtime enum table.
With the reached names resolved, the diagnostic UNF compile next stops at
`Новый Картинка` with one argument. The existing base-runtime constructor
creates an empty picture only. Syntax Assistant 8.3.27.2342 documents a
required filename or binary-data source plus an optional transparent-background
flag for loaded pictures. Widen only that constructor's arity to zero through
two: preserve the measured empty form exactly, and reject nonempty forms at
runtime until image loading and decoding are implemented. In particular,
do not silently return the empty singleton for an image source.
The next recorded constructor is `Граница` / `Boundary`, used as a query
parameter value. Syntax Assistant 8.3.27.2342 defines one required value
and one optional boundary-kind argument. Register that arity in ORM, beside
its query constructor, and fail explicitly until query-boundary values can be
represented. Do not manufacture an inclusive/exclusive behavior from an
unimplemented value.
The next recorded UNF global is `РазмерКартинки` / `PictureSize`. Syntax
Assistant 8.3.27.2342 lists eight members; the direct platform probe
`tests/conformance/measure/1c/measure-picture-size.platform.txt` records
their distinct presentations, shared member type, and all English aliases.
Use the existing base-runtime enum table; do not make this a configuration
metadata object.
The following recorded UNF global is the closed
`ТипСмещенияТабличногоДокумента` / `SpreadsheetDocumentShiftType`
enumeration. Syntax Assistant 8.3.27.2342 lists three members; the direct
platform probe
`tests/conformance/measure/1c/measure-spreadsheet-document-shift-type.platform.txt`
records presentations, type, and aliases. Extend the same base-runtime
table.
The next recorded constructor is the general platform `Линия` / `Line`
value. Syntax Assistant 8.3.27.2342 defines one required line-kind argument,
an optional width, and an optional cell-line gap. It belongs in the existing
base-runtime constructor table next to `Цвет` and `Шрифт`; the spreadsheet
library only owns a separate internal `Line` model for MXL rendering and
must not expose it as a complete BSL value. Resolve one through three
arguments and fail explicitly until that value model exists.
The next recorded UNF global is the large closed `ТипДиаграммы` / `ChartType`
system enumeration. Syntax Assistant 8.3.27.2342 lists 47 members. A single
direct platform probe, `tests/conformance/measure/1c/measure-chart-type.bsl`,
records the presentation and English alias equality of every member, so
the base-runtime table can be extended atomically. The enum is a platform
value, not a configuration catalog object or chart renderer.
UNF also names `ОриентацияМетокДиаграммы`, which the installed Syntax
Assistant does not index as a root. A direct platform probe first showed
that the name exists; a second probe
`tests/conformance/measure/1c/measure-chart-label-orientation-name.platform.txt`
confirmed it is the same four-member enumeration as the documented
`ОриентацияПодписейДиаграммы` / `ChartLabelsOrientation`, with the same
type and presentations. Keep both Russian names in one base-runtime enum
lookup table; do not treat the old name as a configuration global.
The actual UNF compile after `ChartType` reaches `ВидГраницы` /
`BoundaryType` before chart-label orientation. Syntax Assistant 8.3.27.2342
defines exactly `Including` and `Excluding`; the platform probe
`tests/conformance/measure/1c/measure-boundary-type.platform.txt` records
their type, presentations, and aliases. Use the base-runtime enum table
alongside the already recognized query-boundary constructor.
The recorded chart modules also name the closed `МаксимумСерий` / `MaxSeries`
and `ВидПодписейКДиаграмме` / `ChartLabelType` system enumerations. The
installed Syntax Assistant lists three and 23 members respectively. The
direct platform probes `measure-max-series.bsl` and
`measure-chart-label-type.bsl` record every member's presentation and alias
equality. Add both through the existing base-runtime enum table; these are
platform values independent of a configuration catalog or chart renderer.
The next actual UNF compile after `BoundaryType` reaches the closed
`ТипЛинииДиаграммы` / `ChartLineType` enumeration. Syntax Assistant
8.3.27.2342 lists six styles; the direct platform probe
`tests/conformance/measure/1c/measure-chart-line-type.platform.txt`
records their type, presentations, and aliases. Add it to the same
base-runtime enum table; it does not implement the separate mutable `Line`
value.
The earlier rooted diagnostic recorded `Новый
ПользовательскиеНастройкиКомпоновкиДанных` as the remaining unsupported
constructor after the other reached names. Syntax Assistant 8.3.27.2342
defines a zero-argument constructor and a mutable settings model. Register
the exact constructor in ORM with an explicit runtime error until a startup
path requires the actual graph; a fabricated empty settings collection
would hide unavailable behavior.
The actual UNF compile after recognizing the user-settings constructor
reaches the closed `ВидКнопкиФормы` / `FormButtonType` enumeration. Syntax
Assistant 8.3.27.2342 lists four members; the direct platform probe
`tests/conformance/measure/1c/measure-form-button-type.platform.txt`
records their presentations, type, and aliases. Add it to the existing
base-runtime enum table.
The next actual UNF compile after the loaded-picture arity fix reaches
`НаправлениеСортировкиКомпоновкиДанных` / `DataCompositionSortDirection`.
This is a distinct platform enum from `НаправлениеСортировки` /
`SortDirection`; the installed Syntax Assistant lists exactly `Asc` and
`Desc`, and the direct platform probe
`measure-data-composition-sort-direction.platform.txt` records its type,
presentations, and aliases. Add it through the base-runtime enum table.
The following actual UNF compile reaches `ТипГруппировкиКомпоновкиДанных` /
`DataCompositionGroupType`. Syntax Assistant lists three members, and the
direct platform probe `measure-data-composition-group-type.platform.txt`
records the English presentation `Elements` for `Items` and `Hierarchy only`
for `HierarchyOnly`. Add the closed enum through the same table.
The next actual UNF compile reaches `РасположениеПолейГруппировкиКомпоновкиДанных` /
`DataCompositionGroupFieldsPlacement`. Syntax Assistant lists three members;
the platform probe `measure-data-composition-group-fields-placement.platform.txt`
records their presentations and aliases. Use the base-runtime enum table.
The following actual UNF compile reaches `РасположениеРеквизитовКомпоновкиДанных` /
`DataCompositionAttributesPlacement`. Syntax Assistant lists four members;
the platform probe `measure-data-composition-attributes-placement.platform.txt`
records their presentations, including `With owner` for `WithOwnerField`.
Use the same enum table rather than a configuration-specific object.
The diagnostic UNF compile logs unresolved globals while preserving the
production resolver sources, then stops at the first non-name error. It found
32 occurrences of 11 distinct platform enum roots, followed by the
two-argument `FixedStructure` constructor. Syntax Assistant 8.3.27.2342
provides all 11 complete member inventories. One direct platform probe,
`measure-session-enum-batch.bsl`, records all 56 members, their types,
presentations, and alias equality in 89 output lines. Add the closed enums
through the existing base-runtime table; do not add another resolver path.
The diagnostic UNF pass with only the `FixedStructure` arity temporarily
relaxed next found `ГлубинаЦвета`, `ОтображениеПодсказки`, and
`ТипКлиентскогоПриложения`, as well as the processing-picture constructor and
the context-dependent `КлиентскоеПриложение` global. The direct platform probe
`measure-next-session-enums.bsl` records all 20 enum members and their
aliases. Add the three closed enums through the existing table. Register
`ProcessingPicture` beside `Picture` with the union of its two documented
constructor arities and an explicit error; implementing image processing or
returning an empty picture would claim unsupported behavior. Handle the
`КлиентскоеПриложение` global separately in the Session layer because its
observable methods depend on the selected session kind.
The following UNF compile barrier is the documented optional headers map of
`HTTPЗапрос` / `HTTPRequest`. The direct platform probe
`measure-http-request-headers-constructor.bsl` shows two-way aliasing with the
source `Соответствие`. The existing property setter already retains that map
by reference, so extend the constructor arity to two and pass its second
argument through the same map validation; preserve zero- and one-argument
behavior and avoid a new HTTP object model.

The first ordinary CLI start against the live UNF database after the SDBL 0.9.4
adapter reached `Новый ГенераторСлучайныхЧисел` in the rooted configuration
graph. The installed 8.3.27.2342 Syntax Assistant pages
`objects/catalog63/RandomNumberGenerator/ctors/ctor110.html` and
`ctor111.html` document the seeded and default constructors. Register their
zero-or-one-argument surface in the existing base-runtime constructor table.
The current startup evidence does not show that the constructor executes, and
the platform-specific seeded sequence has not been measured. Execution must
therefore fail explicitly until that separate behavior is implemented; an
invented random generator would make guarded startup paths appear successful.

The following compile-only diagnostic pass reached
`ПараметрыЧтенияXML` / `XMLReaderSettings` and then
`ИспользованиеВывода` / `UseOutput`. The installed 8.3.27.2342 Syntax
Assistant gives an empty XML settings constructor and a form with eleven
optional positions (`objects/catalog63/catalog565/XMLReaderSettings/ctors/ctor200.html`
and `ctor104.html`). Register that syntax in the existing XML component but
fail explicitly on execution until the reader consumes a real settings value.
`UseOutput` is a three-member platform enumeration documented at
`objects/catalog2/catalog277/UseOutput.html`; measure its presentations on
the platform before adding rows to the core enumeration table. The
diagnostic pass substitutes unknown constructors only while compiling and
stops before Session creation; none of its substituted values is executed or
retained in the product.
The following diagnostic UNF compile reaches
`ТипФайлаТабличногоДокумента.HTML`. The existing enum table carried nine
members and an old comment said the platform had fourteen. Installed Syntax
Assistant 8.3.27.2342 lists eighteen, and the direct probe
`measure-complete-spreadsheet-file-type.bsl` confirms all presentations and
aliases. Complete the existing enum table, correcting the comment, without
changing which formats `bsl-spreadsheet` can encode. The platform's measured
root string is `EnumSpreadsheetDocumentFileType`, but the existing runtime
returns `ПеречислениеТипФайлаТабличногоДокумента`. Keep that old root display
to honor the additive behavior constraint; the member probe intentionally
tests the 18 members without changing that legacy string.
The existing member type also displays as `ТипФайлаТабличногоДокумента`
where the platform displays `SpreadsheetDocumentFileType`; preserve this
legacy display for the same reason. The measured probe checks member values
and aliases only.

The next diagnostic UNF pass reaches
`ПолучитьДвоичныеДанныеИзBase64ДвоичныхДанных`. Syntax Assistant 8.3.27
specifies one binary-data argument. The direct platform probe
`measure-base64-binary-data.bsl` distinguishes whitespace positions: CR/LF
are ignored anywhere; tab, space, and non-ASCII bytes before a complete
quartet yield empty data but terminate successfully after one. The same
termination applies to punctuation and an extra `=`, while a Base64 alphabet
byte after the quartet continues parsing and can invalidate the input.
Normalize only these byte-level rules, then reuse the existing Base64 decoder
for the completed prefix. Keep the existing string variant unchanged.

The diagnostic UNF graph also references `ВертикальноеПоложениеЭлемента`
and `ИнтервалМеждуЭлементамиФормы`. Installed Syntax Assistant enumerates
their complete four- and six-member sets. The direct platform probe
`measure-form-alignment-spacing-enums.bsl` confirms both roots, member types,
presentations (including `One and half`), and all aliases. Add them through
the existing closed system-enum table, with no separate form model.

`КлиентскоеПриложение` is a Session-owned global object, not a constant enum
value. A direct platform probe using `1cv8c` recorded object type presentation
`Client application`, `ApplicationType() = ClientApplicationType.ThinClient`,
and `Undefined` from a real background job. The regular `1cv8` probe reports
`ThickClient`, but that Session kind is outside the current host contract.
Register a small object through the existing library-global and component
method tables. Keep its method result bound to the current Session kind;
background jobs retain their own result. A web-client probe is not available
from this local file-infobase runner, so the natural `WebClient` mapping is a
registered `НЕ ИЗМЕРЕНО` decision rather than claimed platform evidence.

The following diagnostic UNF graph stops at an omitted argument of a closed
`Скопировать` method. The direct probe
`measure-value-table-copy-omissions.bsl` shows that a value-table copy with
the first position omitted matches explicit `Undefined`, and an omitted
second position retains all columns. The existing runtime `Copy` path already
consumes `Undefined` for optional positions. Narrow the resolver exception to
the `Copy` core method and map omitted positions through its existing
builtin-argument path; do not loosen other closed methods or add a second copy
implementation.

UNF subsequently reaches a five-argument `XMLWriterSettings` constructor.
Syntax Assistant documents the fourth attribute-indent flag and fifth
indentation-character string. The direct platform probe
`measure-xml-writer-settings-five-args.bsl` records all five properties,
defaults, exact element and second-attribute formatting, and rejection of a
non-whitespace indent string. It also accepts an empty string and each of
space, tab, CR, and LF. Preserve the public `XmlWriterSettings` Rust
struct and the exported three-argument helper for existing consumers. Store
the two new options in the private settings object and writer layout, and
extend the existing XML writer's newline and attribute paths. This keeps a
single serializer and avoids changing the Rust struct literals already used
by the workspace tests.

The next diagnostic UNF compile reaches a call in an otherwise unused common
module export with two actual parameters while the target export declares one.
Platform 8.3.27.2342 `measure-module-arity-lazy.bsl` proves that valid exports
of the same module execute, while calling either an invalid self-qualified or
imported call raises `Too many actual parameters` at that method. The same
probe also shows `Not enough actual parameters` at the invalid method for
omitted required trailing arguments in local and imported calls. For the
configuration catalog only, semantic resolution replaces a method body that
fails with `ArgumentCountMismatch` or `MissingRequiredArgument` by a single
explicit raise. The ordinary
script and entry resolver stays strict, as do all other semantic failures.
This uses the existing method chunk and catchable runtime error; it does not
relax the target function signature or add a VM dispatch path.

A separate platform 8.3.27.2342 probe placed an unknown variable in a
sibling export and called only the valid export. The platform reported
`Variable is not defined` for the sibling method while accessing the module;
the runner timed out on the resulting modal error. This is negative evidence
against extending method-level deferral from arity failures to unknown names.
The timeout does not establish the exact compilation point or catchability.
The existing `ordinary_unknown_name_remains_a_build_error_with_named_modules`
test asserts an earlier failure than this observation suggests, so changing
unknown-name timing requires an explicit compatibility decision and cannot be
folded into the measured arity change.

The next UNF compiler barrier is an omitted `Insert` position on a statically
known core receiver. `measure-insert-omissions.bsl` on platform 8.3.27.2342
shows that the missing position becomes `Undefined`: Array and Structure
reject it as an index/name, Map accepts it as a key, and an omitted value is
stored as `Undefined` for all three. The resolver already maps omissions to
`Undefined` for two narrowly measured core methods. Add only `Insert` to
that branch and keep the runtime receiver dispatch unchanged.

The following UNF graph barrier reads `ХешФункция.CRC32`. Installed Syntax
Assistant 8.3.27.2342 page
`platform-syntax-help:syntax-context:objects/catalog2/catalog2088/HashFunction.html`
lists six members, while the existing table has only three. The direct
`measure-hash-function-enum.bsl` probe confirms presentations, aliases, and
member types for CRC32, PBKDF2SHA256, and SHA512. Add all three to the
existing closed enum table without changing the three old members. The
`bsl-crypto` constructor remains explicit about unsupported algorithms;
enumerating a member does not fabricate a hash implementation.

The next UNF graph barrier calls `ТекущийКодЛокализации` in a common-module
export. Installed Syntax Assistant 8.3.27.2342 describes it as a zero-argument
function returning the current Session's localization code. The direct
`measure-current-locale-code.bsl` probe returned `en_US_POSIX` for both
language spellings and String type even with process `LANG=ru_RU.UTF-8`.
Therefore the process locale is not a sound default. Carry an optional code in
`SessionResources` and `SessionRuntime`; the function returns it verbatim or
fails explicitly if absent. The CLI accepts optional `--locale-code` only in
`--config-db` mode and does not infer it. A background Session receives its
own resource value; unlike session parameters, localization is not copied
from the parent. This preserves isolation and adds no environment lookup.

The next UNF graph barrier calls `ТекущийЯзыкСистемы`. Installed Syntax
Assistant 8.3.27.2342 documents a zero-argument String result. Direct
`measure-current-system-language.bsl` returned `en` for both spellings while
`CurrentLocaleCode()` returned `en_US_POSIX` in the same environment. Keep
system language as a separate optional Session resource and CLI flag; do not
infer it from the localization code or process environment. Background jobs
receive their own resource value. This adds no global state or extra database
read.

The next UNF graph barrier calls `ПолучитьРежимВнешнихРесурсов`.
Syntax Assistant 8.3.27.2342 documents a zero-argument String result:
`D` is the default mode and `A` is the alternative. The direct
`measure-external-resources-mode.bsl` probe returned `D` for both spellings
with String type. Return documented default `D` without additional host
configuration, and allow a SessionResources override for a host that knows an
alternative mode. Do not add a CLI flag before a concrete CLI use case; the
current CLI represents the default mode. The override remains Session-owned.
The public override accepts only the two documented modes as an enum, so a
host cannot accidentally supply an unsupported code.
The alternative-mode result has `НЕ ИЗМЕРЕНО(SESSION.EXTERNAL_RESOURCES_ALTERNATIVE_MODE)`
until a platform Session configured in mode `A` can be measured directly.

The next UNF graph barrier is the `ОтображениеСтраницФормы` root. Installed
Syntax Assistant 8.3.27.2342 lists seven members. The direct
`measure-form-pages-representation.bsl` probe confirms all Russian/English
member aliases, type `FormPagesRepresentation`, and exact English
presentations including `Swipe (touchscreen)` and horizontal tab labels.
Add members to the existing closed enum table and its TypeId mapping; do not
introduce a form object or change name priority.

The next UNF graph barrier is `Новый ФорматированныйДокумент` in an
otherwise unused method. Installed Syntax Assistant 8.3.27.2342 lists only
the default zero-argument constructor and a substantial object model. Register
that constructor in the base runtime with an explicit catchable error until
the value model exists; this lets dormant code compile without inventing a
formatted document representation.

The following UNF graph barrier is `ПоложениеКартинкиКнопкиФормы`.
Installed Syntax Assistant 8.3.27.2342 lists three members, and the direct
`measure-form-button-picture-location.bsl` probe confirms both language
spellings, English presentations, and the type name. Add the closed enum to
the existing table and TypeId mapping; no form-button object is needed.

The next UNF graph barrier is `ТипЛинииРисункаТабличногоДокумента`.
Installed Syntax Assistant 8.3.27.2342 lists six members. The direct
`measure-spreadsheet-drawing-line-type.bsl` probe confirms all aliases,
type name, and the surprising final presentation `DotDashDotted` for
`DashDottedDotted`. Add only the enum table and TypeId mapping; no drawing
model is implied.

The next UNF graph barrier is `ВидФлажка`. Installed Syntax Assistant
8.3.27.2342 lists four members. The direct `measure-checkbox-type.bsl`
probe confirms aliases, type name, and unexpected presentations `Checkbox`
and `Tumblers`. Add only the closed enum table and TypeId mapping.

A temporary diagnostic compile, stopped before opening a Session, found
`Новый ДоставляемоеУведомление` and then the `ЗвуковоеОповещение` root.
Installed Syntax Assistant 8.3.27.2342 lists only the default notification
constructor and two sound members. The direct `measure-sound-alert.bsl` probe
confirmed aliases, presentations, and `SoundAlert` type. Register the
constructor with an explicit catchable error, and add the enum through the
existing table. The diagnostic branches are removed after the run.

The following UNF graph barrier is the read-only global
`ОтправкаДоставляемыхУведомлений` / `DeliverableNotificationSend`.
Installed Syntax Assistant 8.3.27.2342 gives it a server-available sending
manager type. No notification transport is supplied by the current host, so
register a Session global using the existing zero-argument function pattern
and raise an explicit catchable error when accessed. This does not fabricate
an inert manager.

The next UNF graph barrier is `СквозноеВыравнивание` / `ThroughAlign`.
Installed Syntax Assistant 8.3.27.2342 lists three members. The direct
`measure-through-align.bsl` probe confirms aliases, type, and the `Do not
use` presentation. Add it to the existing closed enum table and TypeId map.

The following UNF compile finds `ФигураКнопки` / `ButtonShape`. Syntax
Assistant 8.3.27.2342 defines it separately from `ОтображениеФигурыКнопки` /
`ButtonShapeRepresentation`, with `Auto`, `Usual`, and `Oval` members. Measure
its Russian aliases and String presentations on the platform, then add only
the closed enumeration through the same base-runtime table. Do not conflate
the two types or introduce managed-form rendering behavior.

The direct platform probe confirmed the Russian `Обычная` and `Овал`
spellings and the `Standard` and `Rounded` presentations. The next live UNF
compile stopped on an unsupported `Новый` type; the ordinary diagnostic does
not include the type name, so inspect the documented constructor set before
adding further support.

The diagnostic compile identified `WSОпределения`, `WSПрокси`, and
`СертификатКриптографии` among the next constructor names. Installed Syntax
Assistant 8.3.27.2342 specifies one-through-seven arguments for WSDefinitions,
four-through-nine for WSProxy, and zero or one for CryptoCertificate. All
three are server-available, but their live value models and transports are
outside the current startup slice. Recognize their names and arities in the
existing core constructor table, and fail catchably if executed.

The next live UNF compile reaches `ДанныеФормыВЗначение` /
`FormDataToValue`. Syntax Assistant 8.3.27.2342 documents exactly two
required arguments: form data and the requested type. The base runtime has
neither managed-form data values nor a conversion model. Add only the global
builtin name and arity, with an explicit catchable error on execution, using
the existing builtin table; do not return the input value as if converted.

The following live UNF compile reaches `ТипГруппыЭлементовОтбораКомпоновкиДанных` /
`DataCompositionFilterItemsGroupType`. Syntax Assistant 8.3.27.2342 lists
three members (`AndGroup`, `OrGroup`, `NotGroup`). Measure their Russian
spellings and presentations on the platform and add the closed enum through
the existing table only; this does not add a data-composition filter engine.
The direct platform probe confirmed `ГруппаИ`, `ГруппаИли`, and `ГруппаНе`
with `AND group`, `OR group`, and `NOT group` presentations. The next live
UNF compile reaches the separate `Клавиша` root; its members require a
separate measurement before any implementation.
Syntax Assistant groups the `Клавиша` / `Key` values into ranges such as
digits, letters, function keys, and numeric-keypad keys. The broad type is
larger than the three-member enums above; inspect the actual reached member
uses and measure those before adding a value table. Do not assume that an
integer key code or keyboard event can substitute for a platform enum value.
The platform XML export of UNF was compared against the decoded source graph.
The reached enum references are exactly `_1` through `_9`, `Num0` through
`Num9`, `NumAdd`, `NumDecimal`, `NumDivide`, `NumMultiply`, `NumSubtract`, and
`A`, `S`, `V`. Occurrences of `Alt`, `Shift`, `Ctrl`, `Имя`, and `Доступность`
are properties of application variables with longer names, not members of
the global enum. Installed Russian Syntax Assistant 8.3.27.2342 confirms
the alphanumeric and numeric-keypad names verbatim; measure presentations
and type on the platform before adding the 27 reached values.
The direct platform probe confirms all 27 presentations are their member
spellings and the type name is `Key`. The same UNF source constructs
`СочетаниеКлавиш` / `Shortcut` with a key and up to three modifier flags.
Russian Syntax Assistant 8.3.27.2342 documents one required and three
optional arguments and lists only client availability. Register the shape
to compile dormant configuration methods, but fail catchably at execution
until a managed-client shortcut value exists; do not pretend it is a server
keyboard event.
The ordinary UNF compile after both additions moved on to another unsupported
`Новый` type. The existing generic diagnostic does not name it; a temporary
diagnostic run is needed to identify the next constructor without guessing.
That diagnostic identified `Рамка` / `Border` and was removed. The checked
UNF XML source uses one or two arguments and three members of
`ТипРамкиЭлементаУправления` / `ControlBorderType`: `БезРамки` /
`WithoutBorder`, `Одинарная` / `Single`, and `Подчеркивание` / `Underline`.
Russian Syntax Assistant 8.3.27.2342 documents one required border type and
an optional width. Measure the reached enum members directly; register the
constructor shape with an explicit catchable error until Border values are
implemented. This does not create a managed-form control model.

The next ordinary UNF run reaches `ВидЦвета` / `ColorType`. The XML-verified
sources refer to `Абсолютный`, `ЭлементСтиля`, `WebЦвет`, `WindowsЦвет`, and
`АвтоЦвет`. Russian Syntax Assistant 8.3.27.2342 lists their corresponding
aliases as `Absolute`, `StyleItem`, `WebColor`, `WindowsColor`, and `AutoColor`.
Measure their presentations and type directly before adding the enum table.

After `ColorType`, UNF reaches `ТипЛинииЯчейкиТабличногоДокумента` /
`SpreadsheetDocumentCellLineType`. The XML-verified code names six values:
`БольшойПунктир`, `Двойная`, `НетЛинии`, `РедкийПунктир`, `Сплошная`, and
`Точечная`. Syntax Assistant 8.3.27.2342 lists their English aliases as
`LargeDashed`, `Double`, `None`, `ThinDashed`, `Solid`, and `Dotted`. It also
lists `ЧастыйПунктир` / `ThickDashed`, which this UNF code does not reach.
Measure the reached values directly before adding them to the enum table.

The following UNF run reaches `ТипХранилищаСертификатовКриптографии` /
`CryptoCertificateStoreType`. XML-verified code uses `КорневыеСертификаты`,
`ПерсональныеСертификаты`, and `СертификатыПолучателей`. Syntax Assistant
8.3.27.2342 names their English aliases `RootCertificates`,
`PersonalCertificates`, and `RecipientCertificates`. It lists a fourth
member not reached in this source. Measure the reached members before
registration; no certificate store access is implied by recognizing the enum.

The next UNF run reaches the global property `СредстваКриптографии` /
`CryptoTools`. The XML-verified modules call its synchronous and callback
forms of `ПолучитьИнформациюМодуляКриптографии`; Syntax Assistant 8.3.27.2342
identifies `CryptoTools` as a global manager. Register the global name through
the existing session library and make an actual read fail catchably until a
cryptography provider exists. This matches the existing explicit-failure
manager policy and does not invent module information or access credentials.

The next diagnostic UNF run identifies `Новый МенеджерКриптографии` /
`New CryptoManager` as the constructor behind the otherwise generic error.
The diagnostic branch is removed after identification. XML-verified source
uses zero and three arguments; Syntax Assistant 8.3.27.2342 documents forms
with zero, one, three, and four arguments. The existing `Arity::one_of`
expresses this without accepting the absent two-argument shape. Register the
constructor name and all documented arities, but actual execution fails
catchably until a cryptography provider exists.

The ordinary UNF run after `CryptoManager` reaches
`ПолучитьHexСтрокуИзБуфераДвоичныхДанных` /
`GetHexStringFromBinaryDataBuffer`. Syntax Assistant 8.3.27.2342 documents a
single required `BinaryDataBuffer` argument and a hexadecimal String result.
The base runtime already has `binary_buffer_bytes` and `encoding::encode_hex`
for binary data, so this is a direct reuse of those paths, not another
conversion algorithm. Measure a small buffer on the platform before adding
the function.

The next UNF run reaches `ПолучитьЧасовойПоясИнформационнойБазы` /
`GetInfoBaseTimeZone`. Syntax Assistant 8.3.27.2342 documents a zero-argument
function returning the infobase's configured zone identifier or Undefined.
This is distinct from the host computer and session zones already carried by
`HostEnv`; neither the current configuration snapshot nor the database
session provides the infobase setting. Register the name in the session
library, but fail catchably if it executes rather than substitute a host zone
or guess Undefined. Supplying the actual setting is separate acquisition work
if the startup handler reaches this call at runtime.

After `GetInfoBaseTimeZone`, UNF reaches `ОбработкаТекстаИнтернетПочтовогоСообщения` /
`InternetMailTextProcessing`. The XML-verified source uses `НеОбрабатывать`
and `Обрабатывать`, and Syntax Assistant 8.3.27.2342 names their aliases
`DontProcess` and `Process`. Measure their presentations and type directly;
the enum itself does not implement mail delivery.

The next diagnostic UNF run identifies `Новый ЧтениеHTML` / `New HTMLReader`
as the constructor behind the generic error. The diagnostic branch is removed
after identification. XML-verified UNF sources use the empty constructor;
Syntax Assistant 8.3.27.2342 documents only that form. Register the name in
the base constructor table so dormant methods compile. Actual construction
fails explicitly until an HTML reader and its DOM parsing behavior exist;
an empty fake object would hide a missing parser.

A temporary diagnostic continuation of the UNF resolver identifies two more
reached constructors, `ЗаписьHTML` / `HTMLWriter` and
`ПакетОтображаемыхДокументов` / `RepresentableDocumentBatch`, before the next
missing enum. The diagnostic substitution is removed after discovery.
Syntax Assistant 8.3.27.2342 documents only an empty constructor for each.
Register their shapes and fail explicitly at execution while HTML writing
and document-batch output are absent. The next reached enum member is
`ТипДвустороннейПечати.Нет` / `DuplexPrintingType.None`; measure its String
presentation and type directly before registration.

The broader diagnostic pass reaches the literal `'00000000'` in a single
UNF common module after collecting unresolved API names. The lexer accepts
the eight-digit shape, while `BslDate::parse_digits` rejects its civil date.
The 1C platform is authoritative here: probe this exact literal with
`Вычислить` so a platform rejection remains catchable, then add only the
observed exception or value without loosening other invalid dates. The same
parser handles `Дата("00000000")`; the direct probe confirms that both forms
are the empty date before the shared parser is changed.

After the zero-date correction, the first ordinary UNF compilation failure is
`Новый ДанныеРасшифровкиКомпоновкиДанных` / `New DataCompositionDetailsData`.
All eleven occurrences in the XML-verified source use the empty constructor,
and Syntax Assistant 8.3.27.2342 documents only that form. Register it in the
existing base constructor table. The value carries settings and a collection
of details items, neither of which has a runtime model; an attempted
construction therefore fails explicitly and catchably instead of returning a
fabricated empty value. The temporary name-printing diagnostic used to find
the first failure is removed before implementation.

The next ordinary UNF compilation failure is the global
`НайтиПомеченныеНаУдаление` / `FindMarkedForDeletion`. Syntax Assistant
8.3.27.2342 documents zero through three optional arguments and a result
filtered by the current user's read rights, RLS, and data separation. The
XML-verified UNF call omits the first argument and supplies the other two.
Declare the function in the Session library so the dormant method compiles,
including that omitted-argument form. Until the principal-bound scan exists,
execution fails explicitly; returning an empty array would misreport data
visibility and allow deletion workflows to proceed on a false result.

After that function resolves, ordinary UNF compilation reaches
`СпособПеретаскиванияФайлов.КакСсылкаНаФайл` / `FileDragMode.AsFileRef`
in a dormant managed-form helper. Syntax Assistant 8.3.27.2342 lists two
members. The direct 1C probe confirms both aliases, the String presentations
`As file reference` and `As file`, and the `FileDragMode` type name. Add this
small complete enumeration through the existing enum table; it does not
implement file dragging or managed-form behavior.

The following ordinary UNF run reaches
`ОтображениеГруппыКнопок.Компактное` / `ButtonGroupRepresentation.Compact`.
Syntax Assistant lists three values; the direct platform probe confirms their
aliases, `Auto` / `Standard` / `Compact` presentations, and the type name.
Register the complete measured enum without implementing command-panel UI.

The earlier broad diagnostic inventory exposed many more enum names in dormant
UNF modules. Scan the XML-verified decoded sources for direct member access,
excluding global managers, and measure the resulting 156 distinct expressions
from 47 enum types in one direct platform run. All expressions succeed on 1C
8.3.27.2342. Syntax Assistant provides 154 English member aliases; its local
archive omits `SearchStringLocation.None` and
`FormattedDocumentItemType.Text`, so a separate direct platform probe verifies
both. Register only these reached members through the existing enum table and
its type registry. This broadens compilation without fabricating unrelated
members or implementing managed-form, cryptography, or query-schema behavior.

After the measured enum batch, ordinary UNF compilation reaches
`ПараметрыДоступа` / `AccessParameters`. Syntax Assistant 8.3.27.2342 defines
three required arguments and an optional user-or-role argument. The function
returns field-specific access restrictions for the current principal or the
explicit role; a Boolean or empty object would give an unsafe answer. Declare
the function in the Session library with its documented arity, but reject
execution explicitly until the principal-bound rights and RLS model supports
this result. This is part of the existing rights work, not a new permission
shortcut.

After `AccessParameters`, ordinary UNF compilation reaches `ДеньГода` /
`DayOfYear`. The direct 1C probe establishes one-based ordinal days for
leap and common years, day 1 for the empty date, acceptance of 8-, 12-, and
14-digit date strings, and catchable errors for an invalid string or number.
Extend the existing `DatePartOf` selector and reuse `BslDate::parse_digits` for
the measured string forms. The ordinal is the difference between the date's
day boundary and its year boundary, plus one. This adds no separate calendar
algorithm or VM path.

After `DayOfYear`, ordinary UNF compilation reaches the procedure
`ЗначениеВДанныеФормы` / `ValueToFormData`. Syntax Assistant 8.3.27.2342
documents exactly two arguments: an application value and compatible mutable
form data. The reverse `FormDataToValue` already uses the base builtin table
and fails explicitly while managed-form data values are absent. Add the
procedure beside it with the same explicit failure; do not fabricate a form
object or pretend that a conversion occurred. Its procedure classification
also prevents use as an expression.

The next ordinary UNF failure is the related `КопироватьДанныеФормы` /
`CopyFormData`. Syntax Assistant documents exactly two mutable form-data
arguments and a Boolean result indicating structural compatibility. Unlike
`ValueToFormData`, this is a function. Register it beside the existing form
conversion builtins, but fail explicitly until compatible managed-form data
objects exist. Returning either Boolean without examining real structures
would misreport whether the copy happened.

After `CopyFormData`, ordinary UNF compilation reaches `Новый
ЗаписьFastInfoset` / `New FastInfosetWriter`. XML-verified UNF sources use
only the empty form, and Syntax Assistant 8.3.27.2342 documents that form.
The XML component owns the neighboring `XMLWriter` constructor, so register
this name there with the next constructor code. Actual construction fails
explicitly until binary Fast Infoset output exists; returning an XMLWriter
would silently produce the wrong format. The component's existing constructor
code-density test changes its expected endpoint from 15 to 16 only to account
for the new public constructor, preserving the density assertion.
The same XML-verified sources also construct `ЧтениеFastInfoset` /
`FastInfosetReader` with no arguments; Syntax Assistant documents exactly
that form. Register the paired reader in the XML component with the next
code and the same explicit failure while binary decoding is absent. The
code-density expectation then ends at 17 for the two additions.

The writer-only live run then reaches global `ЗаписатьXML` / `WriteXML`, and
the same XML-verified source set also calls global `ПрочитатьXML` / `ReadXML`.
Syntax Assistant documents three `WriteXML` forms spanning two through five
arguments and a one-or-two-argument `ReadXML` function. These belong in the
existing XML library. Register their names and documented arities, but reject
execution until serialization of configuration values and metadata objects is
supported. The XML library function-code density endpoint advances from 8
to 10; the invariant itself is unchanged.

The next ordinary UNF failure is `ПолучитьДопустимыеКодыЛокализации` /
`GetAvailableLocaleCodes`. The same XML source set calls
`ПредставлениеКодаЛокализации` / `LocaleCodePresentation`. Syntax Assistant
defines zero and one argument respectively; both results reflect the
platform's available locales and localized language names, not merely the
Session's selected locale. Declare the names in the Session library with
explicit unavailable-data errors until a host-backed locale catalog exists.
Returning a singleton current locale or the input code would fabricate
platform data.

The next live UNF compile reaches `Новый ВыражениеСхемыЗапроса` /
`New QuerySchemaExpression`. The XML-verified sources call it with one
argument, and Syntax Assistant 8.3.27.2342 documents a required string
expression. Register it in `bsl-orm` beside the existing `QuerySchema`
constructor. The object model is still absent, so construction fails
explicitly and catchably instead of yielding an inert expression object.

The following live compile reaches `Новый УдалениеОбъекта` /
`New ObjectDeletion` in the server-side data-exchange module. Syntax Assistant
8.3.27.2342 documents exactly one required object reference; the actual
deletion occurs only when `Write()` is invoked. Register the constructor in
`bsl-orm`, which owns database objects. Until an authorized deletion adapter
exists, construction fails catchably; creating a no-op object would allow
configuration code to report an exchange deletion as completed.

The next live compile reaches the global `ВозможностьЧтенияXML` /
`CanReadXML` in the same server-side data-exchange module. Syntax Assistant
8.3.27.2342 defines one XML reader argument and a Boolean answer determined
from the XML type at the reader's current position. This is a separate global
from `XDTOSerializer.CanReadXML`. Register it in the existing XML library.
The current reader model does not yet supply the full value-type mapping, so
actual execution fails catchably rather than returning false for an unread
or unsupported type.

The next live UNF compile reaches the global `ПолучитьДвоичныеДанныеИзHexСтроки` /
`GetBinaryDataFromHexString`. A direct 1C 8.3.27.2342 measurement in
`measure-binary-data-from-hex-reached.bsl` shows that the platform discards
non-ASCII-hex characters, decodes the remaining digits pairwise, and rejects
an odd number of remaining digits. Thus `"AB CD"` yields `ABCD`, `"GG"`
yields empty binary data, and `"0xFF"` fails because the filtered `0FF` is
odd. Reuse the base runtime's binary-data value and share the digit decoder
with its existing URL-percent parser; no second Hex implementation belongs
in the ORM or binary-buffer component.

The following live UNF compile reaches the paired
`ПолучитьБуферДвоичныхДанныхИзHexСтроки` /
`GetBinaryDataBufferFromHexString`. Syntax Assistant documents one required
string and a mutable binary buffer result. The same platform measurement
confirms the buffer variant has the identical character filtering and odd-digit
error behavior, so it shares `encoding::decode_hex` and changes only the
returned value type.

The next live UNF compile reaches `Новый ОбъектМетаданныхКонфигурация` /
`New ConfigurationMetadataObject` in the service-technology module. Syntax
Assistant 8.3.27.2342 documents a required `ДвоичныеДанные` argument
containing a `.cf` or `.cfe` file. The current ORM metadata value represents
the acquired configuration; it cannot stand in for an arbitrary binary
configuration file. Register the constructor in `bsl-orm` for dormant method
resolution and fail actual construction explicitly until that file format has
its own parser.

The next live UNF compile reaches `Новый СвязьПоТипу` / `New TypeLink`
in the property-management module. Syntax Assistant 8.3.27.2342 documents
two optional arguments (a form/metadata data path and a link-item number).
The current base runtime already keeps `FormAttribute` construction explicit
and unavailable until the managed-form model exists. Register `TypeLink` in
the same base constructor table with its documented zero-through-two arity,
but fail actual construction catchably rather than create a link whose target
the host cannot resolve.

The next live UNF compile reaches the bare global `ПолнотекстовыйПоиск` /
`FullTextSearch` in a goods-selection module. Syntax Assistant 8.3.27.2342
defines it as a read-only global property returning the full-text-search
manager. Declare it through the existing Session-library global descriptor,
so it keeps the resolver's established variable precedence. Until the
current principal's indexed search and rights are supplied by the Session
data adapter, reading the global fails catchably rather than returning an
empty or permissive manager.

The next live UNF compile reaches `ПолучитьРазделительПутиСервера` /
`GetServerPathSeparator` in report delivery. Syntax Assistant 8.3.27.2342
documents a zero-argument server function returning the worker's operating
system path separator. Expose it from the Session library using the current
state's existing `FileSystem::path_separator` capability. This keeps an
injected file system consistent with the path separator used by other BSL
file functions and does not infer it from the process operating system.
The real 1C 8.3.27.2342 server measurement on Linux returned `/` for both
Russian and English names. Plain CLI intentionally does not register the
Session library without `--config-db`, so its direct script output is not an
oracle for this session-specific call.

The following live UNF compile reaches `Новый FTPСоединение` /
`New FTPConnection` in the same report-delivery module. Syntax Assistant
8.3.27.2342 documents a required server argument and eight optional
connection parameters. The core constructor table is the existing path for
`Новый` resolution; register the documented arity there, but fail actual
construction explicitly until the host supplies an FTP transport. Creating
a dummy connection would defer a false success to a later network operation.

The next live UNF compile reaches `Новый СвязьПараметраВыбора` /
`New ChoiceParameterLink` in an integration module. Syntax Assistant
8.3.27.2342 defines two required strings and one optional linked-value
change mode; the XML source uses the two-argument form. As with the reached
`TypeLink`, register the constructor in the core table and reject actual
construction until the managed-form data path and mode are modelled. A
placeholder link would falsely imply that selection filters are wired.

The next live UNF compile reaches `Новый ПостроительЗапроса` /
`New QueryBuilder` in the tax-account module. Both reached XML calls omit the
optional query text; Syntax Assistant 8.3.27.2342 documents zero or one
argument. Keep this data-access constructor in `bsl-orm`, beside the existing
`Query` and `ReportBuilder` constructors. Until the query-builder model
exists, actual creation fails explicitly; substituting `Query` would lose the
builder's selection and filter semantics.

The next live UNF compile reaches the zero-argument
`Новый ОписаниеОтображаемогоОбъектаPDF` /
`New PDFRepresentationObjectDescription` in a file-handling module. Syntax
Assistant 8.3.27.2342 documents that exact form. Register it in the existing
`bsl-pdf` component; until its representation properties exist, creation
fails explicitly. Its constructor-code density test grows mechanically from
two entries to three.

The following live UNF compile reaches `Новый ЗаписьPDF(Поток)` /
`New PDFWriter(Stream)` in the same module. Syntax Assistant 8.3.27.2342
documents zero-argument uninitialized creation and one-argument file-name
or stream creation. Register arity zero through one in `bsl-pdf`, while
failing actual creation until the BSL writer object and its lifecycle
methods are implemented. The crate's existing low-level PDF writer does not
provide that object contract. The constructor-code density test expands
from three entries to four.

The next live UNF compile reaches the global
`СоединитьБуферыДвоичныхДанных` / `ConcatBinaryDataBuffers`. Syntax
Assistant 8.3.27.2342 documents an array of buffers, optional separator,
and optional byte order. A direct 1C measurement captured eleven cases:
two parts, separator placement, empty array, alias, explicit and default
order, a BigEndian first part, and invalid argument types. The result order
defaults to LittleEndian even when the first part is BigEndian. Implement
this as a base-runtime builtin reusing the existing binary-buffer storage;
compare the entire platform output with the CLI rather than inferring bytes
from the interpreter.
The VM pads omitted optional arguments with `Undefined`; the platform
measurement also shows that explicitly passing `Undefined` selects the
default separator and byte order. The implementation therefore handles
both cases identically without changing VM calling conventions.

The next live UNF compile reaches `Новый COMSafeArray("VT_VARIANT", 2)` in
an OpenOffice conversion branch. Syntax Assistant 8.3.27.2342 documents
copy, array, element-type, and variable-dimension overloads. The resolver
supports at most a byte-sized argument count; register one through that
limit in the core constructor table, leaving overload validation to a
future COM component. Actual construction fails explicitly without a COM
host. Replacing it with a BSL array would silently break COM marshaling.

The next live UNF compile reaches `ПолучитьИмяКлиентаЛицензирования()` /
`GetLicensingClientName()` in a licensing module. Syntax Assistant
8.3.27.2342 says it reads an information-base value, returns a string,
and is available only to administrators. Place resolution in the Session
library rather than the base runtime, so any future adapter must bind the
value and authorization to the current principal and connection. Until
then, invocation fails explicitly; an empty string would be a fabricated
licensing identity.

The next live UNF compile reaches `ПолучитьИдентификаторКонфигурации()` /
`GetConfigurationID()` in the same licensing module. Syntax Assistant
8.3.27.2342 defines a server-only, administrator-only encrypted Base64
identifier derived by the platform from configuration metadata. Neither
the ORM metadata GUID nor `CONFIG_FINGERPRINT` has that contract. Resolve
the function through Session, with an explicit runtime error until an
authorized platform value is available for the current connection.

The same reached XML module also calls
`ПолучитьДополнительныйПараметрКлиентаЛицензирования` /
`GetLicensingClientAdditionalParameter` and
`УстановитьНастройкиКлиентаЛицензирования` /
`SetLicensingClientParameters`. Syntax Assistant 8.3.27.2342 documents a
zero-argument administrator-only getter and an administrator-only procedure
with three optional arguments. Declare both in Session, alongside the
licensing-client name and configuration ID. Until a principal-bound DB
adapter can read and persist these values, calls fail explicitly; a no-op
writer would falsely claim to update licensing credentials.

The next live UNF compile reaches global `XMLТип(Тип)` / `XMLType(Type)` in
a document-recognition module. Syntax Assistant 8.3.27.2342 defines one
required BSL type argument and returns `XMLDataType` or `Undefined` when
there is no mapping. The XML crate's `XMLТипЗнч` / `XMLTypeOf` maps a value,
and its current primitive mapping cannot determine the XML type of a
configuration metadata type. Register a distinct function in `bsl-xml`
and fail actual use until the metadata-to-XML-type relation is available.
The XML function-code density test grows mechanically from eleven entries
to twelve.

The next live UNF compilation reaches `Новый ОписаниеПередаваемогоФайла`
in a document-recognition module. Syntax Assistant 8.3.27.2342 documents
zero through two optional string arguments and writable `Name` and
`Location` properties. A direct 1C probe confirms empty-string defaults,
both aliases, mutation, and conversion of numeric constructor and property
values to strings. This is a small value object with no database service;
place it in the base runtime's existing constructor and member tables.
Use the call context's user-facing formatter for conversion, as other
platform value objects do. Compare the whole probe output with the CLI.

The next UNF compilation reaches
`СтрНайтиИВыделитьОформлением` / `StrFindAndHighlightByAppearance`.
Syntax Assistant 8.3.27.2342 documents exactly two arguments: a plain or
formatted source string and a string or array of sought words. Matching
starts at word boundaries, has special digit rules, and can search a
hierarchy; the result is a highlighted formatted string or `Undefined`.
The current formatted-string value stores plain text and does not yet
model highlighted spans. Declare the name in the existing core function
table so dormant configuration methods compile, and reject actual calls
instead of returning an unhighlighted string or false `Undefined`.

The next UNF compilation reaches `Новый ПреобразованиеXSL` / `New
XSLTransform`. Syntax Assistant 8.3.27.2342 documents only its empty
constructor, with the actual object implementing XSLT 1.0 transforms. The
existing XML component has no XSLT engine. Register the constructor there
for dormant configuration methods, but reject actual construction rather
than return a pretend transformer that cannot apply a stylesheet. The XML
component's constructor-code density test grows from 17 to 18.

The next UNF compilation reaches `Новый МоментВремени` / `New PointInTime`.
Syntax Assistant 8.3.27.2342 documents one required date and an optional
infobase object reference. Its value orders otherwise equal timestamps by
reference; no measured reference-ordering contract or query binding exists
in the current runtime. Register only the documented constructor arity in
the base runtime for dormant configuration methods and reject actual
construction. A date-only stand-in would silently lose the reference part
of a query boundary.

A temporary diagnostic pass beyond `PointInTime` reaches one
`HTTPСервисОтвет` / `HTTPServiceResponse` constructor before stopping at
the global `ДополнительныеНастройкиАутентификации` /
`AdditionalAuthenticationSettings`. Syntax Assistant 8.3.27.2342 gives
the response a mandatory status and optional reason and headers. Its
body and headers belong to a server HTTP response, distinct from the
existing HTTP client response; register it in `bsl-http` with a catchable
construction error until the response model exists. The authentication
global exposes a manager of infobase policy, including write methods;
declare it in Session, not the base runtime, and fail a read when no
authorized settings adapter exists. Neither an empty manager nor a
client-response substitute would preserve platform behavior.

A platform-only control probe places `New NonexistentType()` in a sibling
export of a common module. Leaving the module unused does not stop the
entry, but calling even its otherwise valid export stops the form after
the initial output line despite a surrounding `Try`, with no platform log
message. This does not justify treating a platform-valid but currently
unimplemented constructor as a runtime value or extending the measured
per-method arity deferral to every unresolved constructor. The probe's
invalid method is overlaid only for that measurement.

The next UNF compilation reaches `Новый ФлагиИнтернетПочтовогоСообщения` /
`New InternetMailMessageFlags`. Syntax Assistant 8.3.27.2342 lists seven
Boolean IMAP flags but does not state their initial values. A direct 1C
probe shows all seven start as `Undefined`, six writable properties accept
Boolean or `Undefined`, and the `New` property is read-only. Numeric and
string assignments are rejected. The type prints `InternetMailMessageFlags`.
Represent this small value in the base runtime without a mail transport;
the constructor and property table can reproduce the complete observed
surface without inventing message retrieval or delivery.

The next UNF compilation reaches `РазделитьФайл` / `SplitFile`. Syntax
Assistant 8.3.27.2342 defines two required arguments (source path and byte
count) and an optional destination directory; the platform XML export
contains eleven calls with two or three arguments. This is a global file
operation, so register the documented surface in the base runtime. Until
splitting is implemented through the State's `FileSystem`, actual calls
fail explicitly and catchably before touching any file. Direct process
filesystem access would escape the State's file boundary, while a returned
empty array would falsely report completed output.

After `SplitFile`, the next UNF compiler failure is `Новый ИзвлечениеТекста`
/ `New TextExtraction`. Three calls in the platform XML source pass a file
name. Syntax Assistant 8.3.27.2342 documents zero or one argument and states
that arbitrary-format extraction is available only on Windows. Declare the
constructor in the base runtime for dormant methods, but reject actual
construction until a host text-extraction capability exists; a text-document
or empty-string substitute would claim to support formats it cannot read.

The next strict UNF run reaches `Новый ОтборКомпоновкиДанных` / `New
DataCompositionFilter`. The platform XML export contains ten empty
constructions, and Syntax Assistant 8.3.27.2342 documents only that empty
constructor. The object has mutable filter items and a settings identity;
register its construction surface for dormant methods, but reject actual
construction until the data-composition filter model is implemented.

The next UNF compiler failure is the zero-argument global `ПолноеИмяПользователя()`
/ `UserFullName()`. Syntax Assistant 8.3.27.2342 describes the full name
of the user selected at program launch. A direct empty-user 1C probe returns
an empty String and confirms the English alias. `open-sdbl` already decodes
the separate `v8users` full-name field. Copy it into the immutable
`bsl-orm::Principal` when constructing candidates; the existing
`Principal::new` retains an empty full name for manually created and
empty-platform principals, while an additive builder supplies the database
field. Expose the global in `bsl-orm` alongside `UserName`, using the
current Session's frozen principal so a later database edit cannot change
an active session. The named-user mapping is marked unmeasured until a
platform run with a nonempty full name confirms it.

The next UNF graph failure is `НачалоМинуты` / `BegOfMinute`.
Syntax Assistant 8.3.27.2342 also documents the paired `КонецМинуты` /
`EndOfMinute`. A direct 1C probe confirms that they clear the seconds or
set them to 59, respectively, for an ordinary Date; on the empty Date,
the beginning remains empty and the end is `0001-01-01 00:00:59`.
Add both as selectors beside the existing hour boundaries in the base
runtime, without involving Session or duplicating date arithmetic.

The next UNF graph failure is the zero-argument
`ПолучитьСоставСтандартногоИнтерфейсаOData` /
`GetStandardODataInterfaceContent`. Syntax Assistant 8.3.27.2342 says it
returns the information base's configured array of metadata objects. The
platform XML source has three dormant calls. This content is database
administrative state, not every eligible object in the immutable
configuration catalog. Register the name in `bsl-orm`, where metadata
objects and the session data adapter already meet, but fail actual calls
explicitly until that adapter supplies the configured OData selection.
An empty or all-objects array would assert an unobserved database setting.

The OData name is passed and the next UNF failure is again an unsupported
`New` type, but the current generic semantic error omits that type's name.
Add a dedicated named error for registry-backed resolution only. It keeps
the same rejection behavior and avoids temporary logging or substitute
values during live database runs. The legacy resolver without a registry
retains its existing `Unsupported` error and test expectation.

The named error identifies `Новый ИзбранноеРаботыПользователя` /
`New UserWorkFavorites` as the next UNF surface. The platform XML export
contains three empty constructions. Syntax Assistant 8.3.27.2342 defines
only the empty constructor and a mutable collection of favorite items
stored in per-user system settings. Register that constructor in the base
runtime for dormant methods; until the collection and item value exist,
actual construction fails explicitly instead of yielding an empty array
that would lack the platform's methods and item type.

The next strict run reaches the paired `Новый
ЭлементИзбранногоРаботыПользователя` / `New UserWorkFavoritesItem`.
The XML export has three empty constructions, and Syntax Assistant
8.3.27.2342 documents only that form. Register it beside the collection
constructor for dormant methods; until its Important, URL and Presentation
properties are implemented, construction fails explicitly rather than
returning an untyped record.

The next UNF failure is the setter paired with the OData content getter:
`УстановитьСоставСтандартногоИнтерфейсаOData` /
`SetStandardODataInterfaceContent`. Syntax Assistant 8.3.27.2342 requires
one array argument containing metadata objects or their names; the XML
export has four such calls. Register the procedure in `bsl-orm` because
it writes the same information-base selection as the getter. Until the
current Session's data adapter can apply the setting with its rights,
execution fails explicitly. Reporting success without writing would make
subsequent reads inconsistent and conceal a missing administrative update.

After the OData setter, the next UNF error is `Новый БлокировкаСеансов`
/ `New SessionsLock`. The same XML modules also call the paired
`ПолучитьБлокировкуСеансов` / `GetSessionsLock` and
`УстановитьБлокировкуСеансов` / `SetSessionsLock`. Syntax Assistant
8.3.27.2342 documents an empty constructor, a zero-argument getter
returning the lock description, and a one-argument setter. The value type
belongs to the base runtime; reading or changing the infobase's current
session lock belongs to `bsl-session`. Register all three reached names
for dormant methods, but reject actual calls until the lock value and a
rights-aware administrative adapter exist. An inactive default lock or a
no-op setter would misreport access control state.

The next UNF graph failure is the bare global
`ВариантОтображенияСообщенияОбОшибке` / `ErrorMessageDisplayVariant`.
Syntax Assistant 8.3.27.2342 lists four members. A direct 1C probe confirms
all four aliases, the `ErrorMessageDisplayVariant` type name, and the
exact English String presentations, including trailing spaces on three
members. Add the closed set through the base runtime's existing enum
tables; no Session state or separate name resolver is needed.

The next UNF graph failure is `Новый ТекстыСообщенияОбОшибке` /
`New ErrorMessageTexts`. The exported `ЦентрМониторингаСлужебный` module
uses exactly two string arguments. A direct platform 8.3.27.2342 probe
confirms that two arguments succeed, while zero, one and three fail; it
also observes the resulting type and both English property aliases.
The constructor belongs to the base runtime. Register its measured name
and arity so dormant monitoring methods compile. Until its value and
property behavior are implemented, actual construction must fail
catchably: an ordinary structure would misrepresent its platform type.
The Unica documentation provider was unavailable in this workspace, so
the arity decision rests on the platform probe and exported call site,
not an inferred translation of the help page.

The next UNF graph failure reads `ЭлектронныйДокумент`, a common
application identifier. The current facade error preserves the semantic
phase but discards which of thousands of configuration modules failed.
Wrap parse and semantic phase errors in a configuration-module context
carrying the recipe's module name and the original phase error as the
Rust error source. Standalone `Engine::compile` remains unchanged. This
is a diagnostic improvement, not a new resolution rule; it avoids
guessing whether the identifier is a platform global, object attribute,
parameter or ordinary local variable.

The named failure is in `ИнтеграцияИСМПВызовСервера`:
`Для Каждого ЭлектронныйДокумент Из ЭлектронныйДокумент Цикл`.
The XML source itself uses the loop variable as its own collection without
a prior assignment. This is not a global name. Platform 8.3.27.2342
loads a module containing such a dormant function and catches the error
when that function is called (`measure-foreach-dormant` and
`measure-foreach-call-caught`). A broader attempt to defer every
`UndefinedVariable` in configuration method bodies failed the existing
`ordinary_unknown_name_remains_a_build_error_with_named_modules` test.
Keep that diagnostic unchanged. Only when the for-each collection is the
same bare identifier as its loop variable, and no prior local or module
binding exists, make the loop slot visible to its own collection in a
configuration method. It evaluates as uninitialized and raises
catchably when reached. Standalone and top-level resolution remain
strict. This uses the existing local-slot and iteration machinery rather
than replacing an entire invalid method with a generic error.

After deferring the invalid dormant method, UNF reaches
`ПолучитьСклоненияСтрокиПоЧислу` / `GetStringDeclensionsByNumber` in
`СкидкиНаценкиСервер`. Platform 8.3.27.2342 recognizes two through six
arguments; one and seven are compile-time arity errors. In the throwaway
file infobase, calls without an explicit Russian language option reach
the method but fail with unsupported `L`; with `Л=ru_RU`, it returns an
array. Register the reached global and arity in the existing base builtin
table. Do not synthesize declined forms: until the platform morphology
is implemented, actual calls return an explicit catchable error, as the
neighboring `ПолучитьСклоненияСтроки` currently does.

UNF next reaches `ПолучитьДопустимыеЧасовыеПояса` /
`GetAvailableTimeZones` in `ЭлектроннаяПодписьСлужебныйРФ`. Platform
8.3.27.2342 accepts zero arguments, rejects one, and returns a nonempty
array containing `Europe/Moscow` in the measured throwaway infobase.
The installed Syntax Assistant indexes the same English name. Register
the global through the base builtin table, but fail explicitly on actual
calls until the host can supply the platform's time-zone catalog. A fixed
`["Europe/Moscow"]` or empty array would turn a capability check into a
false answer on other hosts.

UNF next reaches `Новый Диапазон(, Реквизиты.Период)` in
`ПрослеживаемостьБРУ`. Platform 8.3.27.2342 accepts `Диапазон` / `Range`
with one or two positional arguments, including an omitted first or
second bound. Zero and three arguments fail. Reading the omitted first
bound returns `Неопределено`; the measured type and property aliases are
`Range`, `Начало` / `Begin`, and `Конец` / `End`
(`measure-range-constructor.platform.txt`). Register the constructor in
the base runtime with measured arity. The data-locking API that consumes
this value is not yet implemented; actual construction must fail
catchably until a genuine range value can be carried into that API.
Do not rewrite the source or replace its omitted bound with a value.

The next UNF failure is `РежимИспользованияПараметраКоманды` in
`ИнтерфейсДокументовЭДО`, used as a system enumeration with `Одиночный`
and `Множественный`. Platform 8.3.27.2342 reports the type as
`CommandParameterUseMode`, presentations `Single` and `Multiple`, and
equal English aliases. The installed Syntax Assistant lists exactly
these two members. Add the enumeration to the single runtime enum table
and the type table; do not turn the name into a local variable.

UNF next reaches `Новый МакетКоллекцииЗначенийОбластиКомпоновкиДанных`
in `КоллекцииБЗК`. Platform 8.3.27.2342 accepts the zero-argument
Russian constructor and `New DataCompositionAreaTemplateValueCollection`,
rejects one argument, and reports type presentation
`Data composition template of value collection area`
(`measure-data-composition-area-template-value-collection.platform.txt`).
The constructor has no useful standalone behavior until the data
composition template model exists. Register its measured surface and
raise an explicit catchable error on actual construction rather than
returning an empty structure that could appear to work.

The next live UNF failure is the adjacent
`МакетЗаголовкаКоллекцииЗначенийОбластиКомпоновкиДанных` constructor.
Static review of the same `КоллекцииБЗК` module finds three more
data-composition constructors. One platform 8.3.27.2342
probe measures all four: zero arguments succeed, one fails, and their
English names from the installed Syntax Assistant succeed. The types are
`DataCompositionAreaTemplateValueCollectionHeader`,
`DataCompositionTemplate`, `DataCompositionProcessor`, and
`DataCompositionResultValueCollectionOutputProcessor`. The probe uses
`ПроверяемыйМакетЗаголовка`, because `Заголовок` alone is a managed-form
property and would report the form title's `String` type instead of the
constructed value. The two processors are already registered by
`bsl-orm` as typed objects, with tests for their methods and properties;
registering them again in the base runtime makes `EngineBuilder::build`
reject the duplicate names. Add only the two missing templates to the
base runtime and retain explicit runtime errors for those templates
until their typed values exist. The existing processor behavior remains
owned by `bsl-orm`.

UNF next reaches the system enumeration
`АвтоматическоеСохранениеДанныхФормыВНастройках` in
`ОбщегоНазначенияБЗК`. Platform 8.3.27.2342 reports type
`AutoSaveFormDataInSettings`, two members `Использовать` / `Use`
and `НеИспользовать` / `DontUse`, with presentations `Use` and
`Do not use`. The installed Syntax Assistant lists the same two
members. Add the enumeration and value type through the existing
single-source tables; the form property itself is separate.

UNF next reaches `ТипФайлаДокументаPDF` in
`ЭДОБЗКВызовСервера`. The installed Syntax Assistant lists exactly
`PDF`, `PDF_A_1`, `PDF_A_2`, and `PDF_A_3` under
`PDFDocumentFileType`; platform 8.3.27.2342 confirms both language
names, the member type, and presentations `PDF`, `PDF/A-1`,
`PDF/A-2`, `PDF/A-3` (`measure-pdf-document-file-type.platform.txt`).
Add this complete measured enumeration through the base runtime table.

UNF next reaches `Новый ИдентификаторПриложенияСистемыВзаимодействия`
in `ИнтеграцияКабинетСотрудникаОбсуждения`. The same module also names
`ИдентификаторСообщенияСистемыВзаимодействия`,
`КоллекцияИдентификаторовПриложенийСистемыВзаимодействия`, and
`СовместноеИспользованиеПриложенийСистемыВзаимодействия`.
Platform 8.3.27.2342 accepts one String argument for each ID and zero
for the collection and links; extra arguments fail. English names and
type presentations are measured in
`measure-collaboration-application-constructors.platform.txt`.
The installed Syntax Assistant describes the application-ID string
parameter as optional, but the platform's expression compiler reports
`Constructor not found` for both zero-argument spellings (with and
without parentheses) in `measure-collaboration-application-id-zero`.
Use the measured exact-one arity. These values belong to the existing
`bsl-session` collaboration surface, not the base runtime. Register
the four constructors there and raise catchable explicit errors until
the session has a real collaboration service.

The next UNF compilation barrier is the global `ПолучитьДанныеВыбора` /
`GetChoiceData` call in `ЦенообразованиеВызовСервера`. The installed
8.3.27.2342 Syntax Assistant assigns it a `Type` and `Structure` argument
and a `ValueList` result. The direct platform probe
`measure-get-choice-data.bsl` accepts exactly two arguments and its English
alias; one and three arguments fail during expression compilation. The
measured two-argument probe with the generic String type fails because no
matching metadata object exists. The function belongs to `bsl-orm`, where
metadata and principal-bound data access already live. Register the measured
name and arity there, but raise a catchable explicit error on execution until
the choice-data service can apply metadata and current-user rights. Returning
an empty value list would falsely claim a valid choice set.

UNF next reaches `ПолучитьBase64СтрокуИзБуфераДвоичныхДанных` /
`GetBase64StringFromBinaryDataBuffer` while hashing a VETIS selection. The
installed 8.3.27.2342 Syntax Assistant page
`objects/Global context/methods/catalog5161/GetBase64StringFromBinaryDataBuffer5184.html`
defines one required `BinaryDataBuffer` argument and a String result wrapped
at 64 characters with CRLF. The direct platform probe
`measure-base64-buffer-string.bsl` covers zero, short, and 64-character
boundaries. It exposes an additional trailing-CRLF rule: one complete
64-character line has no terminator, but every later complete line does,
including the last. Reuse the base runtime's existing byte-buffer snapshot
and Base64 encoder, then add only this measured wrapping in the new builtin.
Do not change the existing `ДвоичныеДанные` variant, which has separate
behavior and callers.

The next UNF graph barrier is the closed system enumeration
`СпособПоискаСтрокиПриВводеПоСтроке` /
`SearchStringModeOnInputByString` in `ИнтеграцияВЕТИСВызовСервера`.
The installed Syntax Assistant page
`objects/catalog2/catalog370/SearchStringModeOnInputByString.html` lists
only `Begin` and `AnyPart`. The direct 8.3.27.2342 probe
`measure-search-string-mode-on-input.bsl` confirms both presentations,
the value type, and Russian/English alias equality. Add the complete
closed set to the base-runtime enum and type tables; no search algorithm
is implied by the enumeration value.

UNF next calls the legacy global
`ТекущийВариантИнтерфейсаКлиентскогоПриложения` /
`ClientApplicationInterfaceCurrentVariant`. The precise installed
8.3.27.2342 Syntax Assistant page is
`objects/Global context/methods/catalog33/ClientApplicationInterfaceCurrentVariant6704.html`;
the `ClientApplication.CurrentInterfaceVariant` page describes its modern
replacement, not the global name. The direct platform probe
`measure-current-client-interface-variant.bsl` confirms the legacy function's
zero arity, the documented English alias, `Taxi` in the thin-client test
session, and `Taxi` inside a background job started from that session.
Two plausible but undocumented global English names are rejected by the
platform. The two-value enumeration already exists in the base runtime.

The current variant is a property of the actual host session. Add
`SessionResources::with_current_client_interface_variant(EnumValue)` and
validate its optional value at Session opening against
the existing two members. The global reads only that value; absence fails
explicitly. A background Session receives its own resource value, so its host
can supply the measured `Taxi` without assuming that every background job
always inherits a parent's interface variant. The CLI's existing thin-client
test profile supplies `Taxi` explicitly, matching this measured platform
session; standalone execution without a configuration Session retains its
existing unknown-function behavior. Do not derive the current value from the
detached `НастройкиКлиентскогоПриложения` object, whose measured constructor
default is `Version8_2` and need not equal the active client UI.

UNF then reaches `ТипФайлаПакетаОтображаемыхДокументов` /
`RepresentableDocumentBatchFileType` in its regulated-report export helpers.
The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog2501/RepresentableDocumentBatchFileType.html`
lists exactly eight members: DOCX, HTML4, HTML5, ODS, PDF, TXT, XLS, and
XLSX. The direct platform probe
`measure-representable-document-batch-file-type.bsl` confirms each member's
presentation, type, and English alias. The closed enumeration belongs in the
base-runtime enum and type tables. It declares values only; the enum does not
imply support for exporting all eight document formats.

The next name in the same UNF report module is `ВидШрифта` / `FontType`.
The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog2088/FontType.html` lists exactly WindowsFont,
Absolute, AutoFont, and StyleItem. The direct platform probe
`measure-font-type.bsl` confirms their presentations and value type. Add the
closed enumeration to the base-runtime tables; this does not add font
rendering or modify an existing `Шрифт` value.

After `FontType`, UNF reaches `ВидКартинки` / `PictureType` in
`ВизуализацияЭДОСлужебный`. The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog2088/PictureType.html` lists exactly Absolute,
FromLib, and Empty. The direct platform probe `measure-picture-type.bsl`
confirms their presentations, type, and English alias. Add this closed
enumeration to the base-runtime tables without implying support for loading
or rendering a `Картинка` value.

The next UNF graph barrier is `ИспользованиеМетаданныхПолнотекстовогоПоиска`
in `ОбменСВнешнимиСистемамиНоменклатура`. The installed 8.3.27.2342 Syntax
Assistant page `objects/catalog2/catalog2088/FullTextSearchMetadataUse.html`
lists exactly Use and DontUse. The direct platform probe
`measure-full-text-search-metadata-use.bsl` confirms their presentations,
type, and English alias. Add the closed enumeration to the base-runtime
tables; this does not implement full-text-search metadata processing.

The next UNF graph barrier is `РежимИзмененияСвязанногоЗначения` /
`LinkedValueChangeMode` in `ПодписиДокументовФормы`. The installed
8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog277/LinkedValueChangeMode.html` lists exactly
DontChange and Clear. The direct platform probe
`measure-linked-value-change-mode.bsl` confirms their presentations, type,
and English alias. Add the closed enumeration to the base-runtime tables;
this does not implement managed-form choice-parameter links.

The next UNF graph barrier is
`СпособКодированияИнтернетПочтовогоВложения` /
`InternetMailAttachmentEncodingMode` in `ОбменСГИСЭПД`. The installed
8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog2086/InternetMailAttachmentEncodingMode.html`
lists exactly MIME and UUEncode. The direct platform probe
`measure-internet-mail-attachment-encoding-mode.bsl` confirms their
presentations, type, and English alias. Add the closed enumeration to the
base-runtime tables; this does not implement an InternetMailAttachment
value or MIME encoding.

UNF then reaches `Новый ОбработкаРасшифровкиКомпоновкиДанных` /
`New DataCompositionDetailsProcess` in `ЗарплатаКадрыОтчетыБазовый`.
The installed 8.3.27.2342 Syntax Assistant documents two overloads, both
with exactly two required arguments: details data or its storage address,
and an available-settings source. The direct platform probe
`measure-data-composition-details-process-constructor.bsl` confirms that
zero, one, and three arguments cannot select a constructor, while two
arguments reach the constructor's type check. Register that exact arity in
the existing `bsl-orm` constructor table. Until the details value model
exists, actual construction raises the same explicit unsupported-model
error pattern used by its neighboring data-composition constructors;
it must not manufacture a placeholder value.

After that constructor, UNF reaches `КодЛокализацииИнформационнойБазы()` /
`InfoBaseLocaleCode()` in `ОбменСБанкамиСлужебный`. The installed
8.3.27.2342 Syntax Assistant page
`objects/Global context/methods/catalog8/InfoBaseLocaleCode1710.html`
defines a zero-argument string result for the infobase's localization code.
The direct platform probe `measure-infobase-locale-code.bsl` confirms the
type and English alias. The separate `CurrentLocaleCode` help page defines
the current Session code, which can come from `/VL`; equality in the
throwaway probe does not make the two sources interchangeable.

`open-sdbl` does not currently expose the infobase localization code.
Carry this value explicitly in `SessionResources`, separately from its
existing current-session locale, and expose it through the Session library.
The CLI accepts an optional `--infobase-locale-code` with `--config-db` and
passes it to the current Session. Without a supplied value, an actual call
fails explicitly; compilation does not substitute the configuration's
primary language or the current-session locale. A background Session gets
its own host-supplied value and inherits no unprovided Session resource.

UNF next reaches `СтатусТранзакцииЗаписиЖурналаРегистрации` /
`EventLogEntryTransactionStatus` in the same bank-exchange module. This is
distinct from the existing event-log entry transaction *mode* enumeration.
The installed 8.3.27.2342 Syntax Assistant page
`objects/catalog2/catalog1449/EventLogEntryTransactionStatus.html` lists
Committed, Unfinished, NotApplicable, and RolledBack. The direct platform
probe `measure-event-log-entry-transaction-status.bsl` confirms all four
presentations, the value type, and the English alias. Add the closed
enumeration to the base-runtime tables without changing event-log writes.

UNF then reaches `Новый ОписаниеОбновленияКонфигурации` /
`New ConfigurationUpdateDescription` in `ОбновлениеКонфигурации`. The
installed 8.3.27.2342 Syntax Assistant page
`objects/catalog63/catalog272/ConfigurationUpdateDescription/ctors/ctor283.html`
defines one required `ДвоичныеДанные` argument containing a CFU file.
The direct platform probe
`measure-configuration-update-description-constructor.bsl` confirms that
zero and two arguments cannot select a constructor, while one argument
reaches its type check. Register that exact arity in the existing `bsl-orm`
constructor table. Until a CFU parser and value model exist, executing a
valid-arity call returns an explicit unsupported-model error and no
placeholder update description.

After this constructor, the UNF graph reaches the bytecode compiler's
`TooManyLocals` limit. The source-to-XML comparison excludes missing or
corrupted module text, but the current compiler error loses the module and
method names. Attach names to the existing error as the compiler traverses
configuration modules and methods. Preserve the underlying limit error and
the standalone diagnostic; do not increase the bytecode field width or
silently omit a method before identifying the failing source. Keep each
context variant behind one `Box`: an inline `String` plus `Box` enlarged
`CompileError` enough to overflow the stack in the existing deep-expression
compiler test, while the boxed pair preserves that test's previous frame
size.

The identified `УчетНДФЛ.РассчитатьНалогПоОсновнойСтавке` has more source
locals than the bytecode's `u8` frame count. Its XML body has no `Try` or
global dynamic `Execute` (the `Execute()` calls are query-object methods).
For such sparse methods, invoke the existing SSA slot allocator only when
the source-local count exceeds the frame limit. The allocator already pins
parameters and module variables and maps source slots to compact register
numbers; no new bytecode form or VM dependency is needed. Keep the current
path for ordinary methods, methods with exception handlers, dynamic source
evaluation, or debug-local materialization. Reject an allocation that still
needs more than 255 frame slots, including a mapping to register 255 which
cannot be represented by `Chunk.n_locals`. Verify the large-method path
against executable tests with sequential locals, simultaneously live
locals, and by-reference mutation before relying on the UNF result.

The first live attempt still failed: 351 resolved locals and nine parameters
entered the sparse path, but source-order greedy coloring found no register
for slot 278 after about 230 seconds. This is a coloring failure, not an
early dynamic/debug guard; it does not prove 256 values coexist. Represent
slot interference as bit sets and color the high-degree slots first only on
the sparse path. Ordinary opt-in `ssa-regalloc` retains source order. The
bit-set graph has the same edges as the former pairwise `HashSet` graph,
while avoiding repeated hash insertions at every live point. The bytecode
frame bound and explicit error remain unchanged.

Degree-first coloring alone still exhausted the frame on UNF. Investigation
found a separate bug in the existing liveness transfer: a value defined and
read within one CFG block was removed from live-in and then reintroduced by
the block's undifferentiated use list. Filter uses of values defined in the
same block before the fixed-point pass. A small predecessor/branch test
reproduces this before the correction and passes after it. This is an
internal correction to the existing liveness invariant, not a new BSL
semantic rule; the whole optimization corpus must still agree with the
plain run.

The branch-heavy synthetic large method exposed a second overestimate:
`live_out` added operands of every `φ`, including `φ` results never used.
Trace `φ` dependencies backwards from actual SSA uses and add edge operands
only for live `φ` nodes. Tests cover both sides: a dead merge does not keep
branch values live, and a used merge still keeps each reachable operand
live. The 300-branch executable case now uses two local registers and
returns its expected sum. This is another correction to existing SSA
liveness, with no change to language semantics or bytecode width.

The next UNF attempt still failed at source slot 344. A focused test then
exposed a remaining inconsistency: `live_out` skipped unused phi operands,
but interference construction still treated every dead phi at a join as a
simultaneous write. Three hundred dead merges at one join therefore exhausted
the frame even though their results were never read. Use the same
actual-use-and-phi-dependency closure when visiting the join point, while
retaining conflicts for live phi writes. The focused test fails before this
correction and passes after it; the real UNF method still needs a live check.

The full optimization corpus exposed a required implicit read: numeric `For`
compiles to `NumericForNext`, which reads its counter on the loop back edge.
The CFG represents the loop header as a statement but SSA had recorded only
the bound expressions, so the counter phi looked dead and its slot could
alias another live value. Record the counter as a header read before its
write. A focused SSA test fails before the correction, and the `n-body-smoke`
optimized output again matches the plain run. This keeps the dead-phi rule
restricted to results without actual bytecode consumers.

The next UNF run reached program assembly and exposed a pre-existing
case-sensitive lookup in `compile_links`: the shared `NameInterner` keeps the
first spelling of a method across the whole catalog, while a later module's
object-method link retains its own spelling. Resolve both type and method
names with BSL's folded comparison, as the interner already does. A focused
test covers differently cased spellings. This changes no resolver priority
or language behavior; it restores the intended link to the same `NameId`.

The live UNF start after the host began invoking the private session handler
reached component linking and stopped at `QuerySchema`: a dormant method calls
one of its methods, but the type deliberately has no object-member table
because its constructor fails explicitly until the model exists. Linking all
chunks used to require a table even for a dormant call. Keep the type and
method name link, leave its descriptor absent, and use the VM's existing
runtime fallback if execution ever reaches the call. No query-schema object
or method is invented; the unsupported constructor still fails first. The
platform XML export confirms these calls occur in UNF source modules, while
the existing constructor contract covers their actual invocation behavior.
The reached UNF startup path reads `Метаданные.Подсистемы`. Syntax Assistant
8.3.27.2342 documents both the root collection and the nested `Подсистемы`
collection on subsystem metadata objects. `bsl-config-db` will project the
subsystem descriptors and their parent-child relationship from acquired Config
resources into the existing immutable `bsl-orm` metadata catalog. The ORM
reuses its metadata-collection lookup behavior. XML exports are verification
artifacts, not a runtime source or fallback.
The UNF Config root identifies subsystem class
`37f2fa9a-b276-11d4-9435-004095e12fc7` and 67 top-level descriptors.
The measured CRM descriptor ends in `{class,14,<child GUIDs>}`; the first
child's descriptor ends in `{class,0}` and names the same child as the native
XML export. Selected acquisition currently retains only root-listed bare
GUIDs, so it must also retain bare-GUID description resources for nested
subsystems. On this UNF database all 27,229 bare-GUID resources total about
16.5 MB compressed; retaining those descriptions is a simple bounded choice
compared with introducing another dynamic selection round trip. It does not
retain unrelated `.0`/form/template resource bodies.
Syntax Assistant 8.3.27.2342 specifies
`Подсистема.ВключатьВКомандныйИнтерфейс` as a read-only boolean. Comparison of
UNF Config descriptors with the native XML export found the flag at index 4
of the descriptor's 22-field list: a true and a false top-level subsystem
have the same documented help-in-contents setting, while this slot matches
their command-interface inclusion setting. The adapter reads that slot as a
boolean and rejects any other serialized form.
The existing ORM metadata collection already computes its length for indexed
access. `Количество()` calls that same length calculation; Syntax Assistant
8.3.27.2342 documents its zero-argument numeric contract for
`КоллекцияОбъектовМетаданных`. No second collection representation is needed.

## Information-register record-key signature
The demo compile-only run after PlannerItem reaches `СоздатьКлючЗаписи` in `РегистрСведений.УчастникиМероприятия.МодульМенеджера`. Native installed syntax help 8.3.27.2342, document `platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/CreateRecordKey847.html`, confirms one required Structure argument and a register-specific key result. Add the signature to the existing manager descriptor table and public methods table, which already drives the manager's own receiver context. Follow the existing explicit unsupported record-manager/set behavior until native key semantics and acquired dimension metadata are available. This is a host implementation boundary, not a claimed platform result or measured successful key implementation.

## Accumulation-register record-set signature
After route-point projection, demo compilation reaches `СоздатьНаборЗаписей`
in `РегистрНакопления.КоличествоЗадачПоИсполнителям.МодульМенеджера`.
Read-only source acquisition confirms that its update procedure subsequently
filters by recorder, reads, adds/deletes records and writes the set. None of
these operations was executed. Installed Russian vendor syntax help
8.3.27.2342, document
`platform-syntax-help:syntax-context:objects/catalog125/catalog204/object210/methods/CreateRecordSet542.html`,
confirms a zero-argument method returning an initially empty register-specific
set. Declare the signature using the existing manager table/context mechanism.
Follow the explicit unsupported-operation boundary already used by information
registers. Do not claim that a placeholder implements the native empty set.
Actual runtime creation and reached read/write operations remain separate
unfinished implementation work, subject to native measurements and the existing
principal-bound adapter contract.

## Accumulation-register balance result

The catalog uses the existing `DocumentAttribute` field description for
dimensions and resources, as catalog and characteristic attributes already
do. This keeps identifiers, names and declared types in one representation;
it does not give these fields document behavior. The register kind and both
collections remain independently optional. Catalog attachment validates the
owner kind and uses the existing immutable object-array ownership mechanism.

`Value::new_table_with_typed_columns` constructs the host result schema
before any name lookup or row creation. It reuses normal table initialization
and allows repeated column names without changing BSL column addition.
The existing facade value and column-type exports suffice; no private
table storage type is added to the consumer API and no dependency is added.

Demo's acquired manager module calls `Остатки(, Отбор, Измерения)`.
Native 8.3.27.2342 controls are recorded in
`docs/research/fixtures/accumulation-register-balance/`: empty result,
query comparison, twelve argument cases and repeated-column names.
The manager returns `Количество`, while the virtual query table returns
`КоличествоОстаток`; only the query descriptions include `Null`.
The numeric result precision is 38 with the resource's scale, measured
for a resource declared 15,2. Repeated dimension names produce repeated
unchanged column names, which ordinary `QueryResultData` disallows.

Reuse the existing SDK virtual-table query compiler and principal-bound
adapter. Give query projections unique internal aliases where necessary,
then construct the method's result columns with their measured names and
descriptions. Do not weaken ordinary query validation. Values of filters
remain bound parameters. Acquire the register kind and complete ordered
field collections through SDK; physical SQL ownership does not prove
metadata collection completeness. The concrete SDK request is recorded
in `docs/plans/open-sdbl-accumulation-register-metadata-request.md`.
Nonempty movement results and temporal boundaries still require native
controls before their implementation is considered verified.

The first connected ORM path supports omitted/date moments and the measured
15,2 unrestricted-sign resource description. Other resource descriptions and
point-in-time/boundary values remain explicit unsupported errors pending
measurement and SDK parameter support. This is unfinished scope, not a
replacement for the full balance-method requirement. Adapter tests prove
request construction, principal forwarding and result conversion; they do
not establish live database query correctness or RLS behavior.

## Reached second-factor template global, 2026-10-06
Read-only demo acquisition after HTML document construction reaches
`ШаблоныНастроекВторогоФактораАутентификации` in the catalog manager
`ШаблоныДвухфакторнойАутентификации`, lines 267/270 of the acquired source.
No matching common module exists in the selected catalog. The full installed
Syntax Assistant property and manager pages identify a readonly global,
`SecondAuthenticationFactorSettingsTemplates`, available since 8.3.14.
Declare it through the existing ORM GlobalDescriptor/FunctionDescriptor
pair, following the already-declared external data processors and data
history services. This does not change global-name resolution or variable
priority. Access without the real principal-bound service reports explicit
unsupported execution; template modification is not implemented through a
successful dummy manager. If original startup executes this service, its
real adapter and native behavior remain required before startup completion.

The reached second-factor setting is a separate in-memory value with two
properties, not the missing template manager service. Native controls
`setting-map` and `setting-rejection` show independent empty maps, retained
assigned-map identity and a non-atomic parameter setter: assigning a
structure or number throws but clears the property to undefined, while the
old map remains intact. This side effect must be retained rather than
replaced with a reasoned failure-atomic setter. Numeric template names use
the ordinary string conversion, confirmed for 42 and 12345.6. Other argument
forms remain explicit unsupported cases until measured; no user record is
modified by constructing this value.

## Reached second-factor settings processing enumeration
Demo manager compilation reaches this enumeration after the setting constructor. Installed platform help lists exactly two members; the native 8.3.27.2342 processing-type control measures both aliases, distinctness, string presentations and type name. Extend the existing runtime enumeration tables; do not introduce an ORM enumeration implementation or user write capability.

## Business-process object creation reached in demo
The original acquaintance manager uses creation followed by fill, own tabular sections, write and check-filling. Native controls in `docs/research/fixtures/business-process-object` establish only the local new-object contract. Reuse the existing configuration-instance lifecycle and manager receiver machinery; do not copy document settings or implement another interpreter. SDK 0.9.8 lacks acquired complete business-process attributes, tabular sections and number settings; their projection remains required before claiming original object execution. Persistence/start and filling handlers remain separate unfinished operations within this change.

The module-state native control establishes a phase distinction specifically for Date: readable but not assignable during the module body, assignable after creation. Use the existing `ConfigurationInstance` initialization path with explicit lifecycle state; do not generalize these observations to unmeasured properties. Two instances retain independent counters.

The independent standard-properties control establishes the same initialization boundary for Number, DeletionMark, Started and Completed. Contrary to treating lifecycle flags as intrinsically read-only, native assignments to Started and Completed succeed on a completed, unsaved new object. Keep local property mutation separate from actual start and persistence operations. Stored-object mutation and coercions are not established by this fixture.

ORM accepts business-process metadata through `MetadataCatalog::with_business_process_metadata` and exposes it on `MetadataObject`. `BusinessProcessNumberSettings` is separate from document settings while reusing the existing string/number enum; member descriptions reuse `DocumentAttribute` and `DocumentTabularSection`. The catalog is consumed by its builder; existing metadata-object clones remain unchanged through copy-on-write. This step does not project SDK data or implement object creation. No new BSL members or external dependencies are introduced.

Business-process object type descriptions are now shared by the catalog and registered through the existing SessionFactory value-type registry, separately from references. Cross-configuration identity and case-insensitive aliases are checked through the facade and an actual session factory. Localized type presentation remains open under `TYPE.BUSINESS_PROCESS_OBJECT_PRESENTATION`; the currently chosen canonical name is not claimed as a native presentation. Mutable object construction and object-module receiver contexts remain unfinished.

Unsaved object creation now uses the existing ConfigurationInstance lifecycle. It supports only the measured string/variable/length-11 number configuration with known empty own collections; unsupported settings and members fail explicitly. Native alias and module controls replay completely; fill, member initialization, persistence and start remain unfinished, so the original creation procedure is not yet supported. Existing manager exports with the creation name retain priority. Empty-date explicit-pattern formatting was independently measured and corrected after full native control validation; formatting without a pattern is unchanged.

Empty-structure filling without a handler now matches the nine-line native receipt. Other input forms, filling during module initialization and filling handlers fail explicitly until supported. Installed help describes a three-parameter business-process Filling event; its invocation will be measured independently rather than copied from document handler variants.

The three-parameter handler is now invoked through the existing configuration-instance handler API. Native receipts distinguish empty structure normalization from shared nonempty input, and establish undefined filling text and initially true standard processing. The measured standard-processing-disabled path preserves handler effects. Standard processing after the handler and other signatures remain explicit unsupported paths. The first unguarded native probe failed on undefined filling data; its negative receipt is retained separately from the two complete twelve-line controls.

Three complete standard-filling controls separate replaced handler data, unchanged direct data and no handler. They establish that the measured standard properties are not populated from structure entries, including read-only names and incompatible standard values. Since construction still admits only known empty own collections, standard filling has no own members to process and can complete without direct standard-property assignments. Changed filling text and unmeasured returned data forms remain explicit unsupported paths. Full own-member filling remains unfinished.

Own-field native controls now establish empty string/false defaults, string-12 truncation and numeric conversion, Boolean numeric conversion, readable attributes and rejected assignments during the module body. Both fields explicitly disable FillFromFillingValue in the native source. Reuse existing document attribute descriptions and type adjustment; do not duplicate decoding or infer enabled filling, tabular sections or other unmeasured types from this control. Extend configuration object receiver property names with acquired attributes for bare names in the original module.

Business-process section controls establish the measured creation/add/default/conversion/index/iteration/clear path and all four type aliases, with distinct row versus section types. Extend the existing section implementation to accept explicit owner kinds and type descriptions rather than creating another storage engine. Preserve document behavior and tests. Business-process types use their own configuration/owner/section identities and are exposed through a separate catalog iterator; unmeasured section methods and lifecycle phases are not inferred from document support.

The native module-phase section control differs from ordinary own attributes: reading/counting/adding/clearing succeed during the body, while replacing the section property fails. Do not propagate the ordinary attribute assignment guard to section mutation. Preserve the same section object through module initialization.

Empty structure filling without a handler preserves existing business-process rows and their values in the complete native control. Admit that measured path instead of rejecting every object with sections. Nonempty section filling and filling handlers on objects with sections remain explicit unsupported paths until independently measured.

A separate native control establishes that a one-element structure naming a section with a value table does not populate the measured business-process section: fresh sections stay empty and existing values remain. Support that exact no-handler data form, preserve the source table, and keep other data forms explicit until measured. No new copying algorithm is introduced.

The native section filling-handler control establishes row mutation through the existing object receiver and standard-processing-disabled completion. Do not reject measured section objects before invoking that handler. Preserve per-instance state on repeated calls. Defer unsupported standard filling checks until after the handler's flag is inspected; do not erase handler effects.

Standard processing after the measured empty-input section handler now preserves its added rows. The acquired own-field policy is still checked when standard processing remains enabled; unsupported nonempty returned handler data for section filling stays explicit. The complete eight-line receipt joins replay without changing native expectations.

CheckFilling requires explicit standard-attribute checking policy in addition to the existing own-field checking mode. Preserve ordered policy with unknown/empty distinction, reusing the existing named standard-attribute setting record; do not add another checking-mode enum or guess policy from object construction defaults. Native controls establish zero arity and the required string ShowError behavior independently from document signatures. The checking handler and other required types remain to be measured before implementation.

The required-field message control confirms the existing standard filling-message template and field name also apply to the measured business-process attribute. Extend the existing catalog message emitter to find acquired business-process attributes rather than creating another message formatter. The initial supported CheckFilling path requires explicit complete checking metadata, no sections and no checking handler. Only the measured required string form and disabled checking modes are supported; unknown policy remains an error.

The native two-parameter business-process checking handler receives false refusal and the acquired required-field list before value validation, even when the field is already assigned. Clearing the shared array suppresses checking. Invoke the existing configuration-instance handler and reuse the mutated list; other refusal changes, list replacement and unmeasured paths remain explicit unsupported cases.

The native check-filling-handler-refusal receipt confirms that a Boolean true refusal survives clearing the shared field list. CheckFilling combines this explicit refusal with subsequent required-field checks. Array replacement and non-Boolean refusal values remain outside the measured subset.

Native check-filling-handler-replace shows that assigning a new empty array to the handler parameter does not replace the list used by standard validation. The runtime retains its original shared array while allowing the measured array reassignment. This differs from mutating or clearing that original array, which does affect validation. Other replacement types remain unsupported.

The 2026-10-07 fill-enabled control supplies paired native/XML metadata differing only in two enabled filling flags and 21 native outcomes. Reuse the document attribute-filling algorithm for enabled business-process attributes, retaining original slot indices. Keep disabled business-process attributes inert as previously measured. Require authoritative default graphs when a default is needed; unknown defaults are not Undefined. Accept Undefined input only in the measured no-handler, empty-tabular-section path. Enabled filling after a handler remains explicitly unsupported until measured.

The fill-enabled-handler native control now distinguishes shared mutation from parameter replacement. The returned filling-data parameter is the source of standard filling, just as measured for documents; replacement does not replace the caller’s variable. Preserve all existing handler instance, arity, text and section guards. Remove only the enabled-attribute handler rejection covered by the new 16-line control.

### Empty data returned by the enabled business-process filling handler

The isolated `fill-enabled-handler-empty` control on 8.3.27.2342 returns 16 complete native lines. Replacing the three-parameter handler data with an empty Structure or Undefined leaves fresh string-12 and Boolean attributes at their empty defaults and preserves assigned values; it does not replace the caller's structure. This is additional evidence for the existing returned-data requirement, not a new normalization rule inferred from the interpreter. Replay must retain the acquired undefined filling graphs and compare every native line.

### SDK 0.9.9 runtime metadata projection

SDK commits 1e8af74/ab0e44a provide the measured business-process and accumulation-register descriptor fields. Extend the existing document-attribute projection rather than implement another type/filling parser. Business processes reuse the existing section representation and standard checking-policy representation; register dimensions/resources reuse DocumentAttribute. Preserve Option collection completeness. Number and register-kind mappings are exhaustive enum conversions, with no platform defaults. No core runtime or VM dependencies change. Native paired fixtures from the SDK and existing owned open-bsl controls establish the acquisition boundary; read-only original demo compile-only remains a separate gate.

The SDK-to-runtime replay exposed a representation guard: earlier hand-built ORM controls used string qualifiers [12], while the paired native XML and Config acquire [12, 1] for the same variable-length string. Accept both representations only for the measured string-12 shape, including its checking path. Preserve the acquired qualifiers rather than strip them in the adapter; fixed/unknown combinations remain outside the measured execution scope. The full native filling body provides the failing integration check before this guard correction.

### Reached business-process Write declaration

After SDK 0.9.9 projection, original demo compile-only reaches bare Записать in BusinessProcess.Исполнение.ObjectModule. Installed syntax help 8.3.27.2342 document platform-syntax-help:syntax-context:objects/catalog125/catalog702/object707/methods/Write1879.html declares zero arguments. The own isolated write control confirms both aliases reject one argument; with an explicit date, native persistence changes new/ref/number state and reads back stored attributes, while repeated writes retain reference/number. A separate receipt shows empty-date rejection even with DontCheck fill checking. These are native evidence, not yet an ORM persistence implementation.

Add the declaration to the existing component method table and own-receiver context. Match the current document/catalog fail-closed persistence boundary: until a real adapter is connected, return an explicit error before events or object mutation. Preserve an already callable matching object-module export, following the existing catalog compatibility exception; this is host API additivity, not a claim about native source-name collisions. Do not turn the native successful receipt into a fabricated ORM replay. Real writing still needs acquired numbering/uniqueness rules, principal-bound transaction handling and measured event/refusal behavior.

### Measured Read of unsaved business-process objects

Native 8.3.27.2342 accepts Read on a new object, contradicting the installed help restriction in this context. It resets stored scalar fields, preserves private module state and the additional-properties container, and replaces tabular-section containers. The read-new-section control distinguishes replacement from Clear: old aliases retain their rows, whereas explicit Clear invalidates old rows. Therefore make only the private section vector replaceable, reuse the existing section constructor, and leave old containers untouched. Share scalar/attribute initialization with creation rather than introduce another default-value algorithm. Compute replacement values and sections before mutating the object. Preserve existing same-name exports using the existing compatibility path. No VM, name-priority or dependency-boundary changes are needed. The supported object representation is currently unsaved; stored reload remains a separate real-adapter task. Initialization-phase reads remain explicitly unsupported until measured/implemented.


## Business-process HeadTask after the Read compile-only pass

The next original demo compile failure is `ВедущаяЗадача`. Installed platform help identifies a writable task reference. Three native controls measure initial type/default and conversion (8 lines), non-empty reference assignment and Read reset (5), and module initialization (2). Get is permitted during initialization; set is rejected.

The existing SDK already exposes SchemaStorage columns and resolved metadata objects. Read-only demo acquisition shows ten business-process HeadTask columns, each a single R target Task708. Project that explicit relation through the existing SDK models; do not add a second Config decoder or require another SDK release. The ORM catalog records the configured task identity, validates its kind, and uses the existing typed-reference factory and identity checks. Missing/ambiguous relations remain unknown. This does not promise support for storing tasks or advancing business-process routes.

Existing synthetic controls without the task binding remain constructible; accessing unavailable HeadTask metadata fails explicitly. The object retains the typed reference per instance; Read computes its empty replacement before mutation. Existing resolver variable priority remains unchanged.


The first runtime replay also exposed missing task manager reference factories and the existing reference-type presentation using the metadata name instead of its synonym. Add only the measured reference factories using the shared document/configuration implementations and preserve manager exports. Additional native identity controls compare type objects directly; retain the earlier presentation receipts and record their mismatch separately instead of changing oracle output. Installed GetRef help confirms the optional UUID argument (`GetRef1919.html`).


## Business-process Modified after HeadTask

Native controls establish a sticky operation flag rather than a comparison with initial values: assigning the same number, date, Boolean or own string sets it. Read clears it. Clearing an empty current section sets it; mutating old section/row aliases after Read does not. A handler cancelling standard Fill without field changes leaves it clear; successful standard Fill with empty data sets it. Initialization calls are rejected.

Store the flag in a shared Rc<Cell<bool>> for the current object/section generation, replaced together with sections on Read. Previous aliases retain their previous flag, so they cannot dirty the new generation. Extend the existing section factory with an internal tracked variant; document sections continue using the untracked path. Set the flag on measured successful assignments and section operations, and in the standard filling path after handler cancellation has been considered. No VM, resolver or core dependencies change. Unknown persisted-object state remains outside the existing unsaved-only representation.


## Business-process DataExchange after Modified

The original demo next requires ОбменДанными. Existing catalog/document exchange-parameter values already model Load and Recipients.AutoFill. Native business-process controls prove the same defaults, per-object isolation, persistence of values and aliases across Read, and no modification flag changes. Separate initialization reads are rejected for both parameters, so guard the business-process property getter until selection rather than adding a new parameter implementation or changing catalog/document behavior. Retain one existing exchange value per business-process object and do not replace it on Read. The read-only property and aliases use the existing component descriptor mechanism.


## Pending business-process reference after DataExchange

The next original demo failure occurs in ComplexProcess.ObjectModule at GetNewObjectRef. Three native controls measure 13 qualified, 6 own-module and 9 invalid/factory lines. A pending reference is separate from Ref and modification state, survives Read and starts typed-empty. Initialization calls are rejected. Wrong measured input preserves the previous value.

Keep a private pending reference per instance, reuse configuration_reference_value and reference_matches_owner, and preserve existing exports. Extend business-process manager methods using the same existing empty/generated reference factories already used by task/document managers. No database operation, persistence claim, resolver or VM change is involved. Read continues replacing only the measured data/section/modification generation, leaving pending reference, additional properties, exchange parameters and module state intact.


## DeleteObjects declaration in the read-only host

The next compile-only demo failure is the global DeleteObjects call in the deletion processing manager. Installed platform help 8.3.27.2342 documents one required and five optional arguments. Native empty-list evidence is retained under docs/research/fixtures/delete-objects: unchecked empty calls succeed, while checked calls without exclusive mode fail and preserve output. Neither observation grants a write adapter.

Use the existing EraseInfoBaseData pattern: declare the procedure in the ORM component and reject invocation explicitly in the read-only host. This is a deliberate host restriction, not an inferred platform semantic, and no successful deletion result is manufactured. Do not change the resolver or dependencies. Real deletion, reference-integrity checks, separation scopes and output replacement remain unimplemented and are not closed by this declaration.


## Empty graphical schema after DeleteObjects

Compile-only demo next requires GraphicalSchema in СхемыПроцессовКлиентСервер. Read-only source inspection shows construction for a process-data structure and separate routines that find/enumerate schema items. Add the empty native-measured value in the existing bsl-spreadsheet component, which already owns geographical-schema declaration and document graphics. No new crate or dependency is needed.

Retain both English and explicit /Lru native receipts in docs/research/fixtures/graphical-schema. Match the Russian presentation contract used by the current runtime. Each items getter creates a fresh wrapper: native equality is false even for repeated access on one schema. Do not infer populated map semantics from this empty observation. Reading native graphical-schema files, graphics editing and route-map acquisition remain unfinished. The existing constructor-density assertion needs an explicitly approved expectation update from 1..=2 to 1..=3; approval is pending and the assertion has not been edited.

Native Find follow-up resolves the documentation discrepancy: missing input raises insufficient actual parameters, while Undefined and a number raise argument-1 type mismatch. Retain exact(1) based on measurement, not the optional annotation. Replay six status/value lines; retain the three original native diagnostic lines without asserting identical error wording/context.


## Graphical-schema picture location

The next original-demo compile failure is the picture-location enumeration in the process-schema catalog manager. Installed syntax help and fourteen native /Lru lines confirm the complete five-member set, presentations, aliases and metatype. Extend the existing EnumValue/EnumKind/name/member tables in bsl-rt; do not create an ORM enumeration object or change resolution priority. Bare-name metatype is measured directly instead of inferred from the prefix. No graphics editing or populated schema support is implied by these constants.


## Graphical-schema shapes

The next process-schema manager compile failure requires GraphicalSchemaShapes. The installed help lists fourteen members; a complete native /Lru control records all presentations, aliases and type/metatype values in 32 lines. Extend the same existing enum and type tables as picture location, append Rust variants to preserve existing ordinal order, and do not infer presentations by splitting identifiers. No graphical mutation or populated diagram is implied.


## Related graphical-schema line enumerations

Read-only inspection of the complete process-schema manager also finds ConnectorLineType, ConnectorTextLocation and ArrowStyle. Measure these together rather than wait for three separate whole-configuration compile failures. The 34-line native control covers all documented members and all type/metatype presentations. Extend existing enum/type tables together with shapes; preserve existing variant ordinals. The unexpected DashDottedDotted text is copied from platform output, never generated from its identifier.


## New characteristic-item predefined flag

After the graphics constants, original-demo compilation reaches Предопределенный in ПланВидовХарактеристик.ВидыДоступа.МодульОбъекта. Installed syntax help declares a read-only Boolean. The nine-line native control confirms false during module initialization and after selection, both aliases, own-module access, rejected writes and another new item.

Add a getter to the existing characteristic-object property table; current creation only represents a new unsaved item, so no extra mutable flag is needed. Do not require selection for this getter because native initialization reads succeed. Keep setter absent. Both component exposure and own-module receiver declarations already consume this same table. Existing stored/predefined item loading remains outside this measured implementation and must not use a fabricated false result.

## Archive path restoration enumeration

The next original demo compile-only barrier is ArchiveFileRestoreFilePathsMode,
introduced in 8.3.26. Installed Syntax Assistant and fourteen native expressions
in `docs/research/fixtures/archive-path-enum/` prove that this is a separate
closed type, not another spelling for ZIPRestoreFilePathsMode. Add separate
members, kind and type in the existing runtime tables; preserve legacy ZIP
identity and consumers. The native type presentations are English even in the
Russian UI. This slice supplies values only: it does not claim implementation
of newer archive reader operations or extend ZIP reader argument conversion
without separate evidence.

## Own data processor creation

After archive enumeration support, the actual original demo manager calls bare
Create and then exported Initialize/UnloadData/Close methods on the result.
Native controls in `docs/research/fixtures/data-processor-object/` prove the
zero-argument factory, object identity, initialized and independent module
state, exported variable assignment and own-manager/own-object receivers.
Reuse ConfigurationInstance and the current configuration module machinery;
do not add another interpreter or return a Structure as the processor.

SDK 0.9.9 does not yet publish complete DataProcessor attribute/section lists
or its acquired object type. The change request is
`docs/plans/open-sdbl-data-processor-metadata-request.md`. A runtime value may
be tested against explicitly complete fixture metadata, but production
projection must preserve unknown versus empty and cannot infer these lists
from module exports. Native empty-object controls do not prove Config decoding.


The additional processor-fields control proves that body-created rows survive
creation, unlike the deliberately measured document reset. Reuse typed
DocumentAttribute/DocumentTabularSection descriptions already shared by other
ORM classes; do not duplicate table storage. Add optional processor attribute
and section collections to MetadataObject, with RU-documented builders and
accessors, and register their object/section/row types through MetadataCatalog
and SessionFactory. Unknown collections reject creation explicitly. The native
control covers String/Boolean and variable string length 12; other conversion
forms require evidence rather than silent analogy. Preserve the existing
business-process checking markers and document restrictions when extending
the shared section storage.

The first processor replay exposes a genuine missing shared service: qualified
exported variable assignment reaches ObjectProtocol but no existing CallContext
operation accesses that instance's module slots. Add read/write methods on
CallContext and one FunctionCall target, dispatching in the existing three VM
callback paths to one cold helper. It reuses module lookup, exported-module-var
flags, owner/program validation and active/parked instance storage. Do not
synthesize BSL functions or maintain a parallel counter map. The primitive
instruction dispatcher and its opcode classification remain unchanged.


### Explicit acquired reference type presentation

Reuse ValueTypeDescriptor's separate display and lookup names. Add an optional
presentation to MetadataObject, populated by an explicit host builder. The
catalog consumes it once when constructing immutable reference descriptors;
there is no per-session calculation or second type registry. The native HeadTask
receipts measure the Russian task synonym distinct from its metadata name.
This API does not infer a language or guess a synonym; production SDK selection
of the acquired presentation remains a separate integration step. Objects with
no explicit presentation keep the existing name, preserving prior host behavior.


The independent task-reference-presentation controls now measure ru/en primary
language selection and fallback to metadata name when only another language's
synonym exists. During metadata_catalog construction retain task synonyms already
parsed by SDK and language codes already parsed by the existing language reader;
apply the selected display before immutable type construction. Do not decode
Config again or select another language as fallback. These controls do not cover
user-specific interface-language overrides.

### SDK 0.9.11 processor field projection

Project complete DataProcessor attribute and section collections from the SDK descriptors through the existing shared attribute projection. Add MetadataCatalog::with_data_processor_metadata(owner, attributes, sections), preserving None versus Some(empty) and rebuilding only derived processor section types. Reject an absent or wrong-kind owner with a dedicated CatalogError. Do not reparse Config in the adapter. Validate projection against the paired native data-processor-0911 fixture, including order, empty sections and unknown collections. The acquired object type GUID is handled separately from member collection completeness; this projection must not claim all declared processor-valued attributes are already supported.

### Reached processor object GetTemplate declaration

Read-only demo compilation after SDK 0.9.11 field projection reaches ПолучитьМакет in Обработка.ПротоколОбменаПоДокументуЭДО.МодульОбъекта. The full installed Russian Syntax Assistant 8.3.27.2342 page `platform-syntax-help:syntax-context:objects/catalog125/catalog139/object142/methods/GetTemplate291.html` declares one required argument (String or template metadata). Add the existing manager template descriptor to the processor object's own method table. Reuse its explicit unsupported acquisition result; do not create a template object without resources, change the resolver or add a VM path. This is an intentional unsupported capability declaration, not a claim of measured template contents or successful original session startup.

### Reached PDFReader declaration

Read-only demo compilation after the processor object GetTemplate declaration reaches Новый ЧтениеPDF in Обработка.ФорматДоговорныйДокумент101.МодульМенеджера. Full Russian Syntax Assistant 8.3.27.2342 pages `platform-syntax-help:syntax-context:objects/catalog63/catalog2657/PDFReader/ctors/ctor313.html`, `ctor316.html` and `ctor312.html` confirm file-name, uninitialized and stream constructors (zero or one argument). Follow the existing PDFWriter unsupported declaration inside bsl-pdf, with a new stable constructor code and no new dependency or PDF parser. The reader's runtime lifecycle and document contents remain unsupported; this declaration must not be described as successful PDF reading. Updating the existing constructor-density expectation from four to five requires the user's explicit test-change approval.

### Reached information register SliceFirst declaration

After PDFReader, read-only demo compilation reaches СрезПервых in РегистрСведений.ЗначенияПоказателейПроцессов.МодульМенеджера. Full Russian Syntax Assistant 8.3.27.2342 `platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/SliceFirst353.html` confirms two optional arguments and a periodic-register value-table result. Add only the reached method to the existing information-register manager table, with an explicit unsupported result. Do not synthesize SQL, assume unknown periodicity or return a successful empty table. Preserve existing same-named exports through the same compatibility dispatch branch as Get. Actual slice execution remains a separate unfinished part of this change and requires native data controls and complete SDK field/periodicity projection.

The new processor object GetTemplate descriptor must preserve existing same-named exports just as the current catalog/document object dispatch does. Check configuration_has_export only for the newly declared template aliases before selecting the platform table. This preserves previously accepted object-module behavior without changing general resolver priority.

Processor objects must use the same dynamic ObjectProtocol dispatch as existing catalog/document objects: publishing a static method_table to the VM would bypass call_method and reject a valid same-named export at the inline-cache arity check. The public members and own-module receiver table remain METHODS; only runtime object dispatch uses the existing dynamic fallback. No VM change is required.

Read-only demo compilation after SliceFirst reaches SliceLast in the same manager. Full installed help8.3.27.2342 page `platform-syntax-help:syntax-context:objects/catalog125/catalog161/object163/methods/SliceLast369.html` independently confirms two optional arguments and latest-record semantics. Extend only the existing manager declaration, explicit unsupported callback and same-named export compatibility tests. Real latest-record data selection is not implemented by this declaration.


## Reached constant value-manager declaration

Read-only demo compilation reaches CreateValueManager in the constant manager ИспользоватьКонтрольОтраженияВУчетеЭДО. Full installed help 8.3.27.2342 (objects/catalog125/catalog136/object211/methods/CreateValueManager1758.html) specifies no arguments and a separate ConstantValueManager result. Reuse the existing manager receiver table and export precedence; do not equate this object with a scalar constant read or fake a successful instance. Declaration-only execution fails explicitly until measured object semantics are implemented. This does not close original startup execution.


## Reached catalog object lock declaration

Read-only demo compilation reaches Заблокировать in Справочник.СообщенияОбсуждений.МодульОбъекта. Full installed help 8.3.27.2342 (objects/catalog125/catalog126/object130/methods/Lock277.html) declares a zero-argument procedure. Reuse the existing object receiver table and preserve existing exports, following Write. Explicit unsupported execution must not pretend that an in-memory flag protects database objects or interoperates with platform sessions. Actual lifetime and two-session conflict behavior require native measurements and a host-side implementation; this declaration does not close them.


## Reached information register selection

Read-only demo reaches Выбрать in ЗапросыРазрешенийНаИспользованиеВнешнихРесурсов. Full installed 8.3.27.2342 help objects/catalog125/catalog161/object163/methods/Select327.html specifies optional four-argument periodic and two-argument nonperiodic forms. Declare their union 0..4 only for compilation; without acquired periodicity, execution explicitly fails before selecting a syntax variant or interpreting arguments. Do not reuse catalog selection semantics, invent rows, introduce SQL parsing, or claim nonperiodic four-argument support. Existing manager exports retain precedence. Native selection behavior, ordering, filters, rights and periodicity remain separate unfinished work.


## Reached report manager template

Read-only demo compilation reaches ПолучитьМакет in Отчет.ДосьеКонтрагента.МодульМенеджера. Full installed 8.3.27.2342 help objects/catalog125/catalog279/object358/methods/GetTemplate829.html confirms one required template name or metadata argument. Add a report-specific receiver table and explicit unsupported callback, following existing catalog/document/processor template declarations. Preserve report exports before this new platform method, keep variables and global resolution unchanged. Actual resource decoding and resulting template types remain unfinished; declaration does not imply a real template.


## Reached data composition period type

Read-only demo reaches ТипПериодаКомпоновкиДанных in УниверсальныйОтчет. Installed 8.3.27.2342 help objects/catalog2/catalog1073/DataCompositionPeriodType.html lists Additional and Main. Measure both values and aliases, bare and member type presentations and type lookup in an isolated file infobase before adding EnumKind, EnumValue, TypeId and their existing exhaustive table mappings. Do not introduce a component enumeration or infer presentations from the neighboring period-addition kind.

Native receipt: tests/conformance/measure/1c/measure-data-composition-period-type.platform.txt (13 lines). Main presents as Default, Additional as Additional; member type is DataCompositionPeriodType and bare enumeration type is EnumDataCompositionPeriodType. Both language aliases resolve. Equality checks use explicit 1/0 strings to avoid the isolated file infobase's English Boolean presentation affecting comparison with the facade's default locale. The initial probe exposed that locale difference; it is not evidence to change unrelated Boolean formatting.


## Reached data composition balance type

Read-only demo reaches ТипОстаткаКомпоновкиДанных in УниверсальныйОтчет after period type. Full installed 8.3.27.2342 help objects/catalog2/catalog1073/DataCompositionBalanceType.html lists OpeningBalance, ClosingBalance and None. Measure their presentations, aliases, member and bare types, type lookup and equality in an isolated file infobase; use explicit 1/0 for Boolean results to avoid unrelated locale differences. Extend existing enum/type mappings without a component-specific enumeration.


## Reached data composition accounting balance type

Read-only demo reaches ТипБухгалтерскогоОстаткаКомпоновкиДанных in УниверсальныйОтчет. Full installed 8.3.27.2342 help objects/catalog2/catalog1073/DataCompositionAccountingBalanceType.html lists Debit, Credit and None. Measure values, aliases, member and bare type presentations, type lookup and distinction from ordinary balance in a separate file infobase before extending existing enum/type tables. Boolean equality controls use explicit 1/0 strings to avoid locale-dependent Boolean formatting.


## Reached data composition field placement

Read-only demo reaches РасположениеПоляКомпоновкиДанных in УниверсальныйОтчет. Full installed 8.3.27.2342 help objects/catalog2/catalog1073/DataCompositionFieldPlacement.html lists five members. Measure all value presentations, aliases, member and bare types, type lookup, equality and distinction from resource placement in a separate file infobase before extending the existing enum/type mappings. Use explicit 1/0 for Boolean checks; do not reuse resource placement identities.


## Reached conditional appearance use

Read-only demo and the captured УниверсальныйОтчет manager reach ИспользованиеУсловногоОформленияКомпоновкиДанных. Full installed 8.3.27.2342 help objects/catalog2/catalog1073/DataCompositionConditionalAppearanceUse.html lists Use and DontUse. Measure member presentations, aliases, member and bare types, type lookup, equality and distinction from field placement in a separate file infobase before extending the existing enum/type mappings. Boolean controls use explicit 1/0 strings.


## Acquired processor object GUID mapping
Use SDK data_processor_object_type directly to associate declared type GUIDs with existing catalog processor object descriptors. Do not decode Config again or register transferable object state. The filling-value path accepts reference kinds only and must explicitly reject processor objects before looking for a materializer. Test the adapter with the native acquired GUID and a constructed declared-type input; this is an adapter test, not evidence that a native configuration attribute accepts an object value.


## Correct the common-module value measurement
The retained module-argument-context native 8.3.27.2342 control accepts own and other module names in arguments and assignments and as JSON restoration receivers. Clean ServerCall=false targets also succeed. The earlier common-module-value job failure was caused by the intentionally invalid target ЗамерыФоновыйСосед, which declares a variable and module body; its recorded error explicitly identifies that target. It did not prove that static names are invalid values. Reuse RExpr::ImportedModule and GetImportedModule in static resolution at the existing module-name position, preserving variables and library globals. Remove only the obsolete static/dynamic gate and its false deferred-error branch. Keep real unknown-name deferral and invalid target initialization behavior. Two existing expectations require explicit user approval; their prepared patch stays unapplied until approval.


## Reached metadata root common attributes
Original isolated demo initialization now reaches the missing ОбщиеРеквизиты property after static module value correction (395.85 seconds; factory266.65 seconds). The loader already validates and retains root common_attribute_ids from native XML/Config evidence but skips their metadata objects because the class is absent from metadata_kind. Add CommonAttribute to the ORM kind and existing collection table and recognize the already measured class GUID in the loader. Reuse MetadataCollection and preserve root IDs as the ordering source, rather than filtering applicable catalog attributes or sorting names. The root getter validates known membership and resolves every member before publishing the collection. No SQL, Config decoder, new dependencies or fake empty collections. Full installed native help CommonAttributes9013 confirms read-only collection membership; measure basic collection operations in an isolated file infobase. FullName and additional common-attribute properties are outside this reached root-property implementation and are not claimed by the replay.
# Common attribute property enumeration identities

The retained native common-attribute property-enumeration control distinguishes
three value types and a shared metadata enumerated-property container. Bare
names are rejected. Reuse the runtime enumeration representation without
registering new global enumeration names, and generalize the private existing
CompatibilityMode metadata-property wrapper to carry its enumeration kind.
This preserves variable resolution and the compatibility-mode container.
The seven measured values can be implemented independently of SDK acquisition;
this does not complete acquired DataSeparation, AutoUse or Content properties.
# Acquired automatic use before the remaining SDK metadata

SDK 0.9.11 already carries explicit CommonAttributeUsage.auto_use with two
known variants. Project that setting into an optional boolean on MetadataObject
and return the existing measured CommonAttributeAutoUse value. An absent SDK
setting remains unknown. Keep the new property out of runtime property tables
for other metadata kinds so custom string properties retain their behavior.
This step does not infer DataSeparation or construct the platform Content
collection from the explicit usage list; those remain in the SDK request.
# Explicit separator mode and ambiguous SDK settings

SDK DataSeparationSettings is documented as settings in Separate mode, but
retained demo artifacts contradict that restriction: the decoder provides
Some(settings) for all thirteen common attributes, including native
НаименованиеЯзык1 whose XML specifies DontUse. Neither presence nor absence
establishes the mode. Expose an optional explicit host catalog flag, matching
automatic-use storage, but do not populate it from existing SDK settings.
The SDK request must acquire the actual mode and correct this contract.
No Config decoding is added.
# Reuse acquired DBNames classification

Read-only acquisition of demo on 2026-10-09 found a unique MetadataField for
each of the thirteen common attributes, with two data_separator=true and
eleven false. The retained native DontUse attribute matches false even though
its descriptor.separation is Some. Reuse that explicit SDK classification by
GUID in load_catalog_base_with_metadata, which already receives the snapshot.
No field or multiple fields leaves the mode unknown. The raw DBNames resource
is retained for offline projection tests and parsed only by the SDK.
This supplies stored common attributes without another API or Config parser;
the complete Content collection and unstored mode classification remain open.


### Metadata full names measured in both script variants

The two native `measure-metadata-full-name` controls establish the prefix for
catalogs and common attributes in English and Russian configurations. Reuse
MetadataValue, its catalog configuration and the existing component method table.
Do not introduce a second naming registry or infer other metadata kinds from
these two measurements. Unknown configuration properties and unimplemented kinds
remain explicit errors. This is an incremental implementation of the required
metadata model, not completion of all metadata FullName behavior.


### Extend measured full names to conditional-separation constants

Original common-attribute initialization accesses FullName on conditional
separation metadata; the platform help permits a constant there. Measure both
script variants for constants and functional options using only metadata reads
in fresh private infobases, then extend the existing MetadataValue method match.
No value evaluation or new registry is needed. Extend the new uncommitted
full-name tests; the unsupported-kind control uses an unimplemented report
instead of a constant once constants gain measured support. Baseline regression
expectations remain untouched.


### Functional-option evaluation after acquisition support

Four native CF/XML corpora now independently establish source bindings for a
constant, a catalog attribute and an independent nonperiodical register resource,
plus the constant-source privilege boundary. The runtime evaluation contract is
recorded before implementation; SDK 0.9.11 has no typed source/binding metadata
and remains a concrete integration blocker. The prepared SDK request provides
the exact corpora without prescribing speculative public DTO names.

Keep Config decoding in SDK. Project only complete acquired option and parameter
bindings into the existing immutable configuration catalog, preserve unknown
states, and implement evaluation behind the existing Session-owned DataAccess
method. Reuse the existing SDK query compiler/connection path; do not introduce
another SQL generator or an option-name-to-constant-name convention. Privileged
constant evaluation must be confined to that acquired option and leave normal
reads and the principal unchanged. RLS, separators and shared cache behavior are
not established by these controls and must not be inferred from them.


### Explicit Undefined functional-option parameters

The 8.3.27.2342 native control measure-functional-option-arguments records
11 lines in a fresh isolated file infobase. Omission, explicit Undefined
and a mutable empty Structure return Boolean false for the test constant
option. Number, String, Array and FixedStructure parameters fail; Number
and Undefined names also fail. Normalize only the explicit Undefined
parameter to the existing empty request. Keep all other type checks and
the current Session adapter path. No acquired-source SDK API is required
for this correction.


### ORM projection of functional-option descriptors

Store FunctionalOptionSettings on the existing MetadataObject. Its optional
location, privileged-get flag and parameter list preserve acquisition
unknowns independently. FunctionalOptionLocation distinguishes Constant,
CatalogAttribute, InformationRegisterResource and Absent using existing
configuration identities; FunctionalOptionParameter stores the canonical
name and the parameter target identity. MetadataObject exposes the same
consuming builder and borrowed accessor pattern as its existing settings.
The stored description is immutable behind Arc, contains no Session state,
and does not introduce an open-sdbl dependency into ORM or the core.
Projection reads the SDK acquired fields, never serialized Config bytes.


The catalog consuming builder validates that the settings owner is a
functional option. Loader projection validates each known source against
the existing catalog kind and acquired field ownership. Complete root
parameter membership is projected only when every referenced parameter
and Use target resolves; otherwise the binding list stays unknown as a
whole. Constant-backed sources use no parameter bindings, consistent with
the measured ignored-parameter behavior. No serialized Config parsing is
added by this projection.


### Measured privilege boundary for catalog/register functional options

The owned file-infobase source-rights control on native 8.3.27.2342 extends the prior constant control. A denied authenticated user has neither catalog nor register Read; both ordinary options fail before and after the privileged option, while each privileged Boolean option returns true. Source Read remains false. The receipt is `tests/conformance/measure/1c/measure-functional-option-source-rights.platform.txt`; setup writes only the owned temporary infobase.

The adapter passes permission only as a private value of the current query invocation. Its prepared restriction request must contain exactly the acquired source object and no tabular section or additional target. Ordinary reads continue through the same AccessStore. No global principal change, thread-local flag or separate database connection is introduced. These measurements do not establish RLS, separator or shared-cache behavior.


## Inverse primitive XML type lookup reached by original demo
With an eligible existing principal assigned FullAccess, the isolated original demo startup passes the previous constant Read denial and reaches missing global ИзXMLТипа. Syntax Assistant 8.3.27.2342 documents both overloads for the global and serializer method; document IDs are `platform-syntax-help:syntax-context:objects/Global context/methods/catalog1566/FromXMLType1630.html` and `platform-syntax-help:syntax-context:objects/catalog63/catalog1015/XDTOSerializer/methods/FromXMLType2458.html`. The fresh native control establishes an inverse mapping only for the five canonical primitive names: derived lexical types accepted by XDTO do not map back through this API. Lift the existing canonical TypeId-to-XML-name helper used by explicit serializer reads and share it with primitive XMLTypeOf and inverse lookup; do not create a second XSD lexical mapping. The serializer receiver must be validated through its existing model path. The control constructs an explicit schema factory so it does not require the still-unavailable global configuration factory. No core/VM change or external dependency is needed.

The separate native `measure-common-attribute-content-surface.bsl` control also confirms that CommonAttributeContent has Count/iteration but no numeric indexing or Get. Its retained result is evidence for the pending acquired Content surface; it requires separating iteration access from public indexing rather than pretending the native collection supports brackets.


## Dynamic common-module names missing from the rooted closure
After inverse primitive XML lookup, original demo startup fails on dynamic lookup of СервисМобильнойПодписиСлужебный. A read-only snapshot diagnostic confirms that its source exists (185007 bytes), it is server-visible, and its name appears in string arguments to ОбщегоНазначения.ОбщийМодуль. The immutable snapshot is complete for this target; changing name-resolution priority would not fix the cause. Rooted compilation currently retains semantic static dependencies, then constructs the dynamic compiler's import list from that reduced catalog. The target is absent solely because the runtime string does not add a static edge.

The Session host declares every server-visible common module as a root through the already available EngineBuilder::configuration_roots API. The generic explicit-root contract must remain unchanged: automatically widening it could make previously valid restricted standalone graphs fail on unused modules. Only the Session factory owns the full platform common-module namespace. Private managers/object modules still follow semantic reachability. This reuses the same resolver, compiler, lazy initialization and ModuleReference implementation without new core API, source preprocessing, runtime dispatch or mutable engine catalog. Compilation is one factory operation; its increased preparation cost must be recorded rather than described as free.


## Unsupported constructors in retained common modules
The full common-module namespace exposes demo's unused РаботаСРечьюБМОСерверТолько8323 module and its unsupported ПараметрыВнешнегоПодключенияРаботыСРечью constructor. Do not add unrelated speech implementation or remove the module namespace. Extend the existing opt-in whole-module name-error path for UnsupportedConstructor in method bodies only; do not alter constructor lookup, strict defaults, private/explicit modules or the method-only arity policy. Native controls are measure-module-unknown-constructor-unused (completed startup) and measure-module-unknown-constructor-lazy (partial output, module compilation failure; not a successful completed oracle). A repeat in an owned file database records the native diagnostic before implementation.


## Reached Gantt interval enumeration
After constructor error deferral, original demo compilation reaches Отчет.АнализЖурналаРегистрации.МодульМенеджера and lacks ОтображениеИнтервалаДиаграммыГанта. Syntax Assistant 8.3.27.2342 confirms four properties under platform-syntax-help:syntax-context:objects/catalog2/catalog277/catalog649/GanttChartIntervalRepresentation/properties/ (Flat2660.html, ThreeDimensional2661.html, Rhomb2662.html, Gradient2663.html). Record native presentations before adding rows to the existing EnumKind/EnumValue registry and its name tables. No new resolver branch, runtime component or rendering implementation is needed.


## Reached chart legend placement
The next original demo compilation after Gantt interval registration stops on РасположениеЛегендыДиаграммы in the same report manager (144.0 seconds). Full Syntax Assistant pages on 8.3.27.2342 confirm seven properties under platform-syntax-help:syntax-context:objects/catalog2/catalog277/catalog278/ChartLegendPlacement/properties/: Auto11427.html, Bottom11428.html, UseCoordinates11429.html, Left11430.html, None11431.html, Right11432.html and Top11433.html. An owned native file-infobase control returns eleven lines, including root/type, every member, alias identity and folded identity. Add rows to the same enumeration and type tables as the preceding Gantt declaration; do not add rendering or change resolver priority.


## Last missing chart enumeration in the reached report manager
The original demo startup after ChartLegendPlacement stops on РастягиваниеПоВертикалиДиаграммыГанта. A read-only source diagnostic of АнализЖурналаРегистрации completes in 30.3 seconds and finds only that missing chart enumeration, with the member РастягиватьСтрокиИДанные; the other chart-related roots in that manager are already registered. Full Syntax Assistant 8.3.27.2342 pages confirm the three members and server availability: platform-syntax-help:syntax-context:objects/catalog2/catalog277/catalog649/GanttChartVerticalStretch.html and its properties/None2664.html, StretchRows2665.html, StretchRowsAndData2666.html. Measure presentations before adding rows to the same enum/type registry. Do not infer XDTO serialization or rendering behavior from this declaration-only slice.


## Reached time-scale units
After Gantt vertical stretching, original demo compilation stops on ТипЕдиницыШкалыВремени in АнализЖурналаРегистрации. An expanded read-only source diagnostic completes in 30.26 seconds and finds Day, Minute and Hour references; the other unknown-looking prefixed roots are common modules. Full Syntax Assistant 8.3.27.2342 pages confirm eight members under platform-syntax-help:syntax-context:objects/catalog2/catalog277/catalog649/TimeScaleUnitType/properties/ (Second2642.html through Year2649.html). Measure all presentations before adding rows to the existing enum/type tables. Keep renderer and XDTO behavior outside this declaration-only change.


## Reached time-scale day format
The original demo after unit registration fails on ФорматДняШкалыВремени in the same report manager. Expanded analysis of the retained read-only source finds ДеньМесяца; no additional database read is required. Full Syntax Assistant 8.3.27.2342 root/member pages under platform-syntax-help:syntax-context:objects/catalog2/catalog277/catalog649/TimeScaleDayFormat confirm four members (properties/MonthDay2656.html, WeekDay2657.html, MonthDayWeekDay2658.html, WeekDayMonthDay2659.html). Measure their presentations before adding rows to the same enum/type tables. Renderer, XDTO and actual chart date-format behavior remain outside this declaration slice.

## Native separation-bound parameter startup

The final String/Boolean control uses a separately declared ordinary Boolean parameter and a selective SessionModule handler that throws `separator-request:<name>`. The native receipt proves that both bound separator parameters are already readable (false/empty string), while the ordinary parameter reaches the handler and fails. This is a narrow platform initialization rule, not a relaxation of ordinary parameter initialization. SDK 0.9.13 has separately measured disabled Independent refusal and disabled Shared unfiltered access; its two commits are reviewed without merging main.

Project acquired bindings and declared value types into the existing ORM metadata model. SessionFactory consumes only this dependency-free model, seeds supported bindings when creating a fresh parameter store, and calls the unchanged original startup handler afterward. An inherited background parameter store is not seeded again. Do not add arbitrary initial-parameter overrides to CLI, preprocess source, or depend on SDK names in Session or the interpreter facade. Unsupported/unknown types remain explicit failures; the retained native Numeric control has confirmed zero before its implementation.

## Measured global conditional separation

The traced original demo fails while reading `ОграничиватьДоступЧерезВебСерверы`. Its table really contains the Independent separator; removing its applicability would be incorrect. The acquired common-attribute resource also names a global Boolean condition constant after the value/use bindings, but SDK 0.9.13 does not retain this link. The complete Syntax Assistant ConditionalSeparation page identifies a Boolean Constant outside that common attribute's content.

The retained String and Numeric controls create distinguishable empty/A/B records. With the condition enabled, B is selected; with the condition disabled and value C supplied, only `empty` is selected. False use is then permitted, while reenabling the condition restores the Independent refusal in the same Session. This is an empty-area filter, not unrestricted reading. A permanent initial-condition cache would also contradict the measured same-Session update.

Describe the SDK change separately in `docs/plans/open-sdbl-conditional-separation-request.md`; await its real public interface before implementing the adapter. Project identities through ORM metadata and use the existing Session data connection to supply specifically requested Boolean conditions. Preserve the structured session-parameter retry path for actual missing use/value inputs; conditions must have a distinct diagnostic so a constant name never becomes a fictitious session parameter. SQL generation remains entirely in the SDK. Shared conditions and per-content conditions are separate measurement scopes; do not generalize from the Independent controls.

## Complete common-attribute Content

SDK 0.9.12 already exposes `common_attribute_content` separately from explicit usage settings. Project this known full list into a neutral ORM model rather than rebuilding candidate classes or hiding Auto/DontUse entries. If the SDK list or represented member set is incomplete, keep the whole collection unknown; public host builders validate known member identities and Constant condition targets. Per-item unknown conditions remain unknown instead of becoming Undefined. This model does not introduce SDK names into runtime, VM, facade, or Session.

Use one component object for Content and another for its items. Reuse the existing CommonAttributeUse enum values, generic metadata value identity, descriptor tables, and the collection_item extension already specified and implemented for non-indexable collections. No new VM operation, numeric get_index override, Get method, builtin-method mirror, or mutable metadata state is needed. Register the two measured type aliases and presentations through the existing type tables and regenerate the API reference.

The retained surface and additional lookup receipts establish folded short-name Find, absent results for qualified names, false String Contains, rejected numeric/Boolean Contains, and stable item equality between Find and iteration. Both missing-argument calls report `Not enough actual parameters`; despite an optional-object wording in the Find help page, use the measured minimum arity of one. Count has zero arguments, Find and Contains have one. Native type comparisons confirm both aliases for collection and item.

The retained Shared String conditional control proves empty-area selection for a false global condition and all-area selection for true condition/false use in the same Session. Integration must preserve the distinct Independent refusal; it cannot reuse that terminal decision for Shared. The SDK remains the only SQL generator. Numeric Shared has not been measured by this control.

A subsequent separate Numeric Shared control proves the same decisions for areas zero/one/two/three. It complements the String receipt; no decision is inferred from another type.

Extend the existing guarded shared-body recognizer only for the retained five/four ordinary frames. The common command slice accesses slot zero alone; emit each method against its own variable table so different parameter names remain correct. Keep exact guard/body checks and reject unknown flags, slots, defaults and additional entries. This is source recovery through the existing compiler, not a native VM or partial module.

The native shared-label-frame control proves per-frame label resolution: instruction 24/index 4 reaches modifying body 32 from the public frame and End 42 from the private frame. Preserve that distinction before global jump normalization. Do not treat an overlapping private declaration as a boundary of the active public frame, and do not generalize the retained ordinary descriptors to the unknown original frames.

For the retained pure prologue, replace its proven side-effect-free commands with source-position no-ops while keeping the public entry at the beginning of the image, before resolving any shared label globally. Preserve the numeric modifying body for the public method and an independently owned empty body for the private method. Validate every removed prologue command, ordinary frame, exact label targets and absence of another entry or outside incoming direct jump; conservatively reject any outside label instruction because its frame has not been measured by this control; preserve both declarations and let the existing recovery emit them. This excludes initializers and arbitrary overlapping code.


## Acquired direct-read information-register declarations

The current loader projects accumulation-register declarations but still does not
attach SDK information-register dimensions, resources and periodicity to the
existing ORM direct-read settings. Reuse the same acquired field materializer,
including typed defaults and reference type resolution; do not decode Config or
reconstruct field lists. Project only a positively acquired Independent write
mode. Map the two measured direct-read periods, Nonperiodical and Day; preserve
all other or missing periods as unknown to this reader. Preserve missing lists
as unknown and verified empty lists as empty. If SDK resolution invalidates completeness after losing a field descriptor,
preserve the unknown collection and reject direct reading. If an acquired list
still names a field without its own descriptor, fail loading explicitly. This fills the existing
nonperiodic/direct-date contract, without adding another SQL path, new public
API, successful unsupported reads, or subordinate-register behavior.

Verify projection with retained native SDK descriptors, changed write modes and
missing collection/period declarations. Verify execution through the existing
Session adapter and preserve principal/RLS errors. Broader field types remain
explicitly unsupported until their separate native controls are implemented.

## Measured pure private constant-backed returns

The separate numeric controls establish 1338/0 → Const[0], 1338/1 → Const[1],
and all four original descriptor flags at external slot zero → Const[0] in
pure private functions. Restrict recovery to their complete NOP/return envelope,
one argument, measured defaults and full one/four-slot frame shapes. Emit a
private BSL function returning the selected numeric literal; its argument is
unused and remains unchanged. Preserve the ordinary return baseline through
linear recovery. Validate all descriptors, names, targets and the complete body;
reject stores, dynamic code, unknown flags, slots, constants and extra commands.
Do not generalize bit masks or rewrite shared commands by address. The original
unknown procedure bodies and their overlaps remain separate work.

## Bounded numeric private-slot write recovery
The retained alias-write and direct-observation controls establish local numeric initialization from Const[0], addition of that same numeric constant, a local store and return. Direct observation distinguishes the private result 14 from caller Const[0]=7 and argument 5 on two calls. Recover only this complete instruction envelope, descriptor 1338/0 and one empty default. Use a fresh BSL local for the private slot so the ordinary argument stays unchanged; do not modify constant tables, generalize flag bits, or accept absent defaults. Existing constant literal rendering and the ordinary linear caller recovery remain the only mechanisms. Negative mutations and all retained receipts verify the boundary.

## Absent defaults in the retained four-slot read frame
The successful one-argument control omits private DefPrm entirely while preserving flags 1, declared parameter count one, four descriptors 1338/0,1459/0,3091/0,3027/0 and the pure twelve-command body. Recover only this measured form. Two-argument native controls are retained as evidence but rejected by source recovery: accepting them would require a separate measured argument-binding contract rather than silently dropping actual evaluation. The zero-argument forged image crashed Enterprise and provides no BSL-language oracle. Check static calls use exactly one argument and exclude dynamic commands for this new profile. Ordinary defaults and other existing profiles keep their current boundaries; missing DefPrm on the single-slot frame remains unsupported.

## Three-path measurement of zero label targets
The paired native controls distinguish address0 (return11), address32 (return7) and fallthrough (return23). Their private function has flags1 and the measured four-slot numeric frame without DefPrm. Both native receipts complete and preserve caller argument5. Initial controls accidentally returned an uninitialized wrapper local; provenance explicitly disqualifies those receipts as label-routing evidence. The corrected but non-discriminating pair returning7 is retained with narrower interpretation. Do not extend these observations to original flags2 procedures: a separate terminating effect observation is still required. Public method ranges cannot simply end at every physical private entry, but whole-module recovery must preserve all private declarations and frame-specific behavior.

## Source control-flow blocks for the measured overlapping functions
Use frame-local BSL labels and Goto, which the existing parser/compiler already support, rather than another native executor. The initial bounded domain is the retained three-function numeric control: ordinary one-parameter public/wrapper frames and the measured read-only four-slot private function without DefPrm. Read-only private slots become fresh local numeric values; their incoming argument is kept separate so initialization never writes through a caller reference. Reuse existing expression recovery inside blocks and reject nonempty expression stacks at their boundaries. Only measured empty try guards may be skipped. Every return must have a pure cleanup path to End. Frame-local labels resolve zero as an actual address. Other frame layouts, private writes and unsupported control bodies remain explicit errors. This introduces a reusable source-block path, not a byte-for-byte fixture template or a VM. Procedure frames and the original protected module remain separate unfinished work.

## Measured procedure routes in the source-block path
Extend the existing block path to the paired four-declaration controls with the read-only flags2 private procedure and the external Message declaration. Preserve actual Message effects through the facade's existing output service; the native driver drains pending messages into the receipt, while recovered BSL emits through the established immediate output path. Do not change runtime message queues. Allow only the measured literal Message callback in private blocks; reject other callbacks and stores. Read-only private slots all denote the same numeric Const[0], so equality of their read with Const[0] is a known true predicate: pruning this pure comparison keeps the unreachable public value-return tail out of the procedure body. This is source control-flow recovery using measured slot reads and existing numeric equality, not new runtime semantics. Unknown flags2 frames and arbitrary value returns remain errors.

## Explicit frame ownership for recovered foreach loops

The original protected processor diagnosis exposes a foreach region after a private method entry. The existing loop recovery helper identifies the hidden iterator from the nearest preceding method address, which selects the wrong frame for overlapping bodies. The retained native scoped-foreach-frame image distinguishes the ordinary public iterator from the private alias slot and executes both declarations; all seven lines are complete.

Reuse the existing loop recovery and emit ordinary BSL foreach statements. Pass the current method's variable descriptors explicitly in scoped recovery; retain the existing default path for previously supported non-overlapping profiles. Do not change original entries, parameter counts or flags to make the helper accept a frame. Keep the whole-module validation gate, reject unsupported loop shapes and preserve the private declaration. No native execution engine or VM opcode is introduced. A diagnostic reachable-leader experiment is separate evidence and must not silently authorize arbitrary dead-code removal.

## Scoped leaders come from reachable commands

The original-image audit shows that nearest physical entries truncate public
frames1 and25 before their reachable return/End paths. Independently, the
scoped-foreach supplemental control places an extra jump after all callable
terminals. A target inside the public setup expression reproduces a false
nonempty-stack boundary in the current block builder. Native receipts must
complete before this control authorizes a production change.

Reuse the existing per-method reachability result and include a command's
branch destination as a block leader only when that command is reachable in
the current frame. Keep frame-local label interpretation. This changes block
partitioning, not the instruction array or entries; no unknown command is
reclassified as harmless, and no procedure is discarded. Preserve explicit
errors when an unsupported branch is reachable. This bounded correction does
not expand the original module's unmeasured frame layouts or declare its
whole recovery complete.

## Reuse conditional-expression recovery for a shared return

Original frame7 fails because a True/Undefined return expression is split
into separate source blocks before its shared Ret. The existing expression
recoverer already handles opcode40/41 diamonds, lazy alternatives and
statement preludes. Reuse it for a forward conditional diamond whose join
is Ret and whose subsequent cleanup reaches End through the existing pure
return-tail guard. Do not add a second expression stack implementation or a
native executor. Preserve private-frame restrictions and reject unsupported
cleanup and merge states. Supplemental native controls retain the foreach
body and all three declarations, execute both alternatives and observe the
private result and unchanged caller argument.

## Leaders depend on feasible edges as well as reachable commands

Original frame20 has a literal True/JmpFalse whose dead destination is a
CallLoc reached later through ordinary fallthrough. Filtering only on command
reachability still adds that destination as a leader and separates ArgNum
from its call. Reuse the existing predicate result: a known conditional
branch contributes a target leader only when its taken edge is feasible.
Unknown predicates retain both paths. This is a block-partition correction,
not a new constant evaluator, instruction reclassification or runtime.
Paired native literal True/40 and False/41 controls execute the same public
foreach and private wrapper; feasible and unknown mid-call mutations must
continue to fail.

## Outside foreach back-edge trampoline

The original ordinary frame22 has Jump3490 -> Jump3368 -> iterator
check3430. Conditional false edge3487 also targets3368; true path3488
exits to cleanup3491. Supplemental whole-image controls retain three
method declarations and isolate an outside single direct jump, followed
by conditional continuation and early break. Extend the existing loop
recognizer only after complete native receipts and a failing source
replay test. Reuse its existing loop-exit translation; do not add a native
executor or rewrite the original image. Reject changed destinations,
operations and cycles. Full original recovery remains a separate proof.

## Scoped backward boolean assignment

Original frame23 is rejected before the existing backward_boolean_join
helper receives the complete reference target and predicate stack. Reuse
that measured helper and expression reconstruction, scheduling continuation
after the already represented False/Assign pair. The supplemental image
retains a public ordinary flags16 procedure and two existing function
frames; full caller observations must be measured before implementation.
Extending scoped declarations to this measured ordinary procedure header
must preserve export and procedure source syntax. Do not relax protected
entry checks or infer original private-frame semantics.

## Private table cardinality27

Original Proc36 has27 zero labels instead of the previously measured5.
Change only that cardinality in the complete flags2 queue-observation control
before expanding recognition. Keep the original numeric-alias frame guards,
all declarations and source routes. This does not establish the original
private body, string constant frame, other cardinalities or malformed entry
semantics. Reuse frame-local label lookup and existing BSL reconstruction.

## String alias in the measured27-label procedure

The original four-slot private frame has a string Const0 and declared Var
size18. Supplemental controls isolate the string type first, then declared
size18 with four actual descriptors. Both must produce complete native
queue receipts before source support. Limit the new string recognition to
flags2 and27 zero labels; previous five-label numeric profiles stay unchanged.
Reuse constant_literals and equality recovery, including the same-slot
predicate fact, without introducing another value evaluator. None of these
controls establishes the original body or its other global context.

## Integrate ordinary scoped bodies with module declaration assembly

The original ordinary-body audit is diagnostic only because the production
scoped dispatcher is limited to the measured3/4-method private frame shape.
Reuse linear-module header/default/local/global/external validation and source
assembly for a separate ordinary frame-scoped body strategy. Retain the raw
image and its per-method labels: global label normalization loses the owning
frame in shared bodies. Preserve existing successful decoding paths before
trying the additional strategy. Reject unsupported private declarations before
returning any source; this extension does not waive the complete-image rule.
Measure a complete ordinary shared-body module including external/global
bindings before implementation, then use full receipt replay and negative
header/label/private-declaration mutations as the acceptance check.

The complete private27-context native receipt retains the measured flags2
private frame (one argument, absent defaults, four constant-backed slots,
declared size18 and27 zero labels) in a module with a global Metadata name
and an additional external NStr33/2 declaration. All nine observations equal
the retained isolated frame receipt. Complete module assembly will reuse
the same measured private frame formatter and scoped body recovery, while
keeping existing global/external/header validation and refusing every
unsupported declaration. This does not establish the unrelated original
flags3 out-of-range entry or flags2 twelve-slot frame.

The ordinary-backward-shared native control changes only the second
function's local label target11 to3 before its entry8. Both functions
return42 on the platform. A frame entry is therefore not a lower bound
for its reachable commands. Scoped recovery should retain the selected
frame's labels and slots and existing command/label bounds, without the
recent physical lower-bound guard. The existing negative test includes
this exact now-measured valid mutation; its removal and name adjustment
require the user's test-change approval. Protected EPF diagnostics show
the same physical-bound restriction blocks most ordinary methods; those
diagnostics alone do not establish their full source equivalence.

The scoped-initializer control retains all ordinary-shared-context declarations
and appends the21 reachable original initialization commands with rebased
handler, branch and short-circuit addresses. Its four native observations
match the baseline. Original initialization also recovers diagnostically
as861 BSL bytes and runs through the facade with Undefined and no output.
The common scoped generator will accept an explicit frame descriptor,
using a synthetic initialization descriptor only as translation context;
no additional BSL declaration or VM execution mechanism is emitted.
Existing successful initializer recovery remains first. Missing global
label tables are not invented, and unsupported callable declarations still
reject the full image.

## Conservative cleanup of generated source

Apply cleanup only after complete native recovery succeeds, including the
scoped fallback. Use the existing bsl-syntax lexer for spans, identifiers,
strings and comments; add its in-tree dependency to bsl-config-db rather
than implement another lexer or extend the embedding facade with a new
public API. Runtime and VM normal dependencies remain unchanged.

The pass recognizes only generated fallthrough labels, the four agreed
boolean forms, unread pure constant assignments and unused explicit local
declarations. Parameters, module storage, calls, property access and other
possibly failing expressions are preserved. Dynamic Execute/Eval disables
variable cleanup in its scope. Keep original source spelling and edit
bounded spans, without regenerating the complete AST. Tests must cover
shared labels, case-insensitive identifier reads, literals and comments,
reference arguments, dynamic code and whole native receipt replay.

## Reached extension purpose
The loader-owned extension description retains an optional neutral typed purpose.
Existing host constructors remain valid, but omission means unknown acquired
metadata rather than a platform default. Purpose is immutable and getter access
fails explicitly when unknown. The runtime reuses its existing enum member and
metadata property namespace tables; it does not expose a new bare global name
or ordinary named type. Native 8.3.27.2342 controls distinguish the scoped member
type, its two-line presentation and the generic enumerated metadata namespace.
SDK projection awaits a measured typed field and must not derive it from EF
names, activity, version or scope. A native transient description's default
Customization is not evidence for the purpose of a stored acquired extension.

## Reached XML canonicalization constants
Original UNF references XMLCanonicalizationType in its existing data-processor module. Add its six constants to the existing runtime enum/type tables after a native isolated-base control; preserve the current resolver and defer actual canonicalization methods to their independently measured implementations. This is a declaration extension, not an XML algorithm implementation.

## Reached DOM canonicalization
Use the existing DOM model, XPath evaluator, host filesystem and neutral ByteStreamProtocol. Canonical rendering has different rules from ordinary XmlWriter; reuse the tree/parser/selection infrastructure, but keep its measured rendering rules separate. Native receipts live in docs/research/fixtures/xml-canonicalization. W3C Canonical XML1.1 (https://www.w3.org/TR/xml-c14n11/) and Exclusive XML Canonicalization (https://www.w3.org/TR/xml-exc-c14n/) explain the algorithm structure; they do not replace the native platform oracle. Register constructor code21 additively. The prepared one-line old density-test change remains pending user approval; no existing expectations may be edited without it.

### Internal namespace selection for canonicalization

The canonicalizer shares the existing XPath parser and evaluator. Its
internal parser entry enables the namespace axis; ordinary public XPath
entry points retain the existing rejection. Namespace bindings are transient
selection records associated with the owning element, represented internally
as detached attribute-shaped DOM nodes. They are never inserted into the DOM
or exposed as BSL namespace objects. This avoids inventing public properties
from the limited native namespace-object probe. Native all-node canonical
controls validate selection and emission for all six algorithms. Public
namespace-object support remains a separate completeness question.

### Reached legacy canonicalization workflow

Original UNF now reaches XMLCanonicalizingWriter and DOMCanonicalization
in the same data processor. The native control establishes four supported
algorithms and two rejected version 1.1 algorithms. Reuse the canonical
renderer; retain separate writer lifecycle and measured legacy selection
semantics. The vendor Canonicalize page documents no parameters despite
original code passing two or three, so native arity controls are mandatory.
No implementation is inferred from the modern converter's API.

### Reached ZIP OS encoding with UTF8

The existing enum currently has two members; the native platform has a third,
reached by original UNF. Preserve prior unsupported spellings such as the bare
OSEncoding member. Extend the single enum table after native verification.
The writer currently validates enum kind and always writes UTF8, so declaration
alone would silently admit an unimplemented mode. Measure container headers
and extra fields first; reuse the existing zip dependency for implementation.

The native Linux ZIP receipt requires Unicode Path 0x7075 in local and central
headers and cleared UTF8 flags. zip 2.4.2 validates custom Unicode Path CRC
against an empty filename while constructing FileOptions. Use the existing
writer to emit the payload under a temporary field ID, then use offsets from
its ZipArchive reader to replace that ID and clear flags in the completed
owned buffer. No second container writer or compression implementation is
introduced. The intermediate ID never leaves the builder. CRC is supplied by
the already used flate2 Crc implementation. Ordinary archives remain unchanged.

The OS-codepage bytes have been measured on Linux with UTF8 only. Until another
OS encoding is measured, the writer explicitly rejects this mode outside Linux;
it does not substitute UTF8 for an unknown OS codepage. The enum remains available
on every OS. Store the mode in the writer instance and replace it only after a
successful Open, so failed reconfiguration cannot change an active writer.

### Reached formatted document file type

Original UNF compilation reaches the missing FormattedDocumentFileType enum.
The installed vendor Syntax Assistant lists ANSITXT, HTML, PDF and TXT. A
14-line native control verifies presentations, bilingual aliases, bare enum
and value type identity, and bilingual type lookup on 8.3.27.2342. Extend the
existing enum tables only; this requirement does not imply implementing a
formatted-document renderer or changing resolver precedence.

### Document object filling check

The original UNF object module calls CheckFilling without a receiver in its
exported CheckDocumentData procedure. DocumentObject's method table currently
lacks it. Native document control confirms both names, own-module resolution,
Date checked path, handler invocation counts and refusal. Capture handler state
rather than relying on direct message emission from the handler.

Acquire document standard-attribute checking policies before applying runtime
checking; open-sdbl currently exposes standard checking for catalogs and business
processes only. Do not hardcode the native baseline's Date path for other document
configurations. Reuse the existing configuration-instance handler mechanism and
attribute fill-checking descriptions. Preserve matching module exports as for
existing object methods. This method does not perform persistence.

Store document standard policies in MetadataObject as an optional Arc slice
using the existing CatalogStandardAttributeFillChecking record shared by the
business-process model. Add with_document_standard_fill_checking and
 document_standard_fill_checking accessors. Store the section's own policy as
Option<FillChecking> on DocumentTabularSection, with with_fill_checking and
fill_checking. This is separate from its attributes' policies. New objects
retain None until the host supplies acquired information. No decoder defaults
or database dependencies enter ORM.

### Document object copy
Native controls in docs/research/fixtures/document-copy establish empty Date,
number and reference, fresh module initialization, independent tabular rows and
OnCopy receiving the source. Reuse the existing document construction and
configuration-instance event path; no persistence or new global builtin is
introduced. Additional properties do not transfer. Protected module behavior
continues to follow the selected existing source policy. Safe-mode transitions
mentioned by platform help require separate security work; this operation does
not invent a privilege bypass. Mutable attribute payloads require their own
native controls before claiming a deep-copy contract.

### Imported procedure expression timing
The original UNF helper is genuinely a procedure, not a decoded signature
error. Native controls under imported-procedure-expression prove catchable
runtime refusal after argument effects and before the procedure body. Lower
only qualified foreign procedure expressions through the existing module
reference/open method call path, which already validates result_required.
Do not replace a whole method with an unconditional deferred failure: that
would incorrectly fail branches not taken. No bytecode format change is needed.

### Global XML reader type lookup

`GetXMLType` reuses the XML reader cursor, its namespace stack and the existing
`XMLDataType` value. The retained twelve native controls establish initial and
whitespace advancement, element namespace inference, explicit `xsi:type` and
undefined results for missing prefixes, nil, text and end elements. The existing
XDTO built-in type table supplies XML Schema names; no second parser or type
registry is introduced. Russian and English global names share one descriptor.

### Custom settings storage Load

The original UNF manager's unqualified Load call requires a receiver method.
Vendor help confirms that custom storage Load delegates to LoadProcessing; this
is distinct from the standard client-settings adapter. The retained native
fixture confirms four calls, including own manager context and a numeric object
key, with omitted key and description passed as Undefined and an empty current
user name in the userless probe. Reuse configuration handler execution and its
returned by-reference parameter vector. No database settings SQL is added: the
configuration handler determines how its settings are stored.

The handler-error probe is retained separately and has no accepted oracle:
Enterprise exited with code 127 after the four baseline controls, before the
catch marker. Returning the handler error to the host remains explicitly
unmeasured under `SETTINGSSTORAGE.LOAD_HANDLER_ERROR` with its registry and
measurement-script entry. No interactive error handling is inferred from this
failed attempt.

### Report object creation

Report objects reuse the existing data-processor object memory and lifecycle
implementation, with report-specific type descriptors and acquired field
collections. They are not standard DataProcessor objects and cannot acquire
field absence by inference. Retained native report controls establish zero-arg
creation, per-instance module initialization, ThisObject identity, string
truncation, boolean conversion and independent tabular rows. The SDK currently
exposes complete processor field/type metadata but has no equivalent report
fields; prepare a dedicated SDK request rather than decode a second copy of
configuration headers in open-bsl. Database report execution requires that
projection as well as the object runtime.

### Report settings composer

The original UNF report object module reads its own `SettingsComposer.FixedSettings.DataParameters`. The native report control proves that SettingsComposer is read-only, has the DataCompositionSettingsComposer type, supports its English alias, and is distinct between report objects. Reuse the existing ORM type declaration rather than introduce another runtime type. The existing unsupported constructor contract remains until the settings model is implemented; merely resolving the report property does not satisfy runtime support. Measure the nested settings objects before implementing their defaults or mutation behavior.

The report property declaration uses the existing unsupported-settings constructor handler while its model remains unfinished. This bridge lets original dormant handlers compile without source rewriting and fails explicitly when reached. Its descriptor is attached only to report objects, not processors, and both names enter the report own-module property context. Keep the complete settings-model task open; native composer measurements remain the basis for that later implementation.

Native report schema controls prove read/write availability, Undefined without a main schema, assignment of a new schema and resetting to Undefined. The catalog currently lacks main-template acquisition, so this implementation cannot distinguish true absence from unknown schema and must preserve an explicit unsupported-model error for both access directions. Reuse the existing schema constructor error while declaring both receiver-context names. The native supported behavior remains an unfinished implementation task, not a claim that every report has an empty schema.

Characteristic predefined values already exist in the SDK snapshot, including the exact original UNF value. The consumer discarded them through Catalog-only filters. Extend existing snapshot projection and manager-context registration to ChartOfCharacteristicTypes, retaining the existing predefined-name storage API rather than adding a parallel collection. Manager lookup uses the same query adapter with the characteristic path prefix and existing reference_matches_owner validation; no second SQL decoder or fabricated record UUID. Native controls prove qualified/bare/global equality, explicit local priority and read-only access.

TextPositionRelativeToPicture is an existing platform enum reached by original UNF processor code. Add it through EnumKind, EnumValue/MEMBERS and TypeId/name/display tables, reusing existing resolver and VM behavior. The vendor full page and nine native lines prove all six members and the `On top` presentation. No component or UI implementation is needed to represent these values.


Document FindByAttribute uses the existing request, session adapter and restricted query execution shared with catalog lookup. The document descriptor has exact arity two; it reads complete DocumentAttribute descriptions rather than fabricating catalog attributes. No reference constructor or alternative resolver is added. A native transaction in the own file infobase established all thirteen retained controls, including own-manager lookup, aliases, trailing-space handling and empty results. Wrong adapter references are an integration contract, not a platform inference.


GetDatabaseDataSize is a global context function, not an information-register receiver method. The vendor page specifies three optional arguments and visible readable data, with metadata inclusion/exclusion and separator-set selection. Its whole-table physical relation size is therefore not an implementation. The reached original UNF call belongs to file-cleanup progress, not a proven session-initializer execution path. Declare its finite global component signature with explicit unsupported execution while retaining native controls and preparing a separate SDK operation request; leave actual data-size execution unchecked. No alternate resolver, invented zero, SQL in ORM or new VM opcode is introduced.


CatalogObject.Level follows the measured selection guard and reads the object's current parent, including edits not yet written. Ancestors are read through existing QueryRequest/DataAccess execution with a bound typed reference; no new SDK method, reference property protocol or separate connection is needed. The parent chain is walked iteratively; a set rejects cycles from malformed adapters. Native missing stored references stop the walk after counting that reference. A no-row query can also mean RLS hid the ancestor; this equivalence is not measured and receives CATALOG.LEVEL_HIDDEN_PARENT in the source, question registry and central measurement script. No global Level function or reference Level method is added by this work.

The new Level declaration preserves an existing object-module export of the same name, including a custom arity, through the already established Write/CheckFilling/Lock export guard. This is the host's explicit additive compatibility policy, not a new claim about native redefinition rules. Other existing receiver-method priorities are unchanged.

### Catalog new-code event

The seven retained native controls under catalog-new-code prove the selected-object guard, initial standard-processing flag, omitted empty prefix, cancellation, changed prefix and base-event safe-mode restoration. Reuse the existing configuration-instance handler bridge, returned parameter vector and catalog code setter. A scoped handler-call option must alter only the nested execution's inherited security snapshot; toggling the caller's safe-mode counters is incorrect because their suppression latch can outlive a balanced toggle. Existing handler calls retain their current behavior. The caller must identify a base configuration handler before requesting this exception; extension origin must not be guessed from a module name.

The implemented facade call `call_configuration_instance_handler_without_safe_mode` sets an explicit flag on the existing instance-call target. Object-instance execution clones its inherited security snapshot and suppresses safe mode only in that clone. The caller's frame counters and latch remain untouched; existing targets explicitly leave the flag false. The new facade-only integration test reproduced the ordinary bridge's inherited safe mode before implementation and verifies success, error, final by-reference parameters, ordinary-handler behavior and fresh States afterward. Existing native frame-suppression return/exception controls remain the underlying scope evidence. No bytecode instruction, database dependency or global thread state is added.

The loader already replaces base object modules with actual extension source under the base logical name and owner. ConfigurationModule now retains a source-origin flag, defaulting to base for a newly supplied configuration module; the loader sets it only for source replacement. When common-module execution settings are changed without replacement source, the retained source keeps its previous origin. ORM binding projects the flag through the separate extension-object-module builder; ordinary object-module binding resets it to base. The two new loader tests cover own/adopted object replacement and unchanged base source, without modifying previous test expectations. This flag records host source provenance and does not itself change BSL dispatch or security modes.

Allocation belongs to the current Session adapter, as with document numbering. The available SDK does not expose catalog code-series settings or an allocator. Retain explicit unsupported allocation until that boundary is supplied and measured; do not derive a per-object counter, issue unmeasured max-code SQL, or imply that binding the original dormant handler proves initialization succeeded. Numeric-code events, existing-record progression, concurrency and extension-event execution need additional native controls.

The catalog implementation replays all seven retained native lines with injected session allocators. It supports the measured two-parameter string-code Russian and English events, reads its final cancellation and prefix parameters, checks the allocator result type and reuses existing code adjustment. A separate numeric-code control establishes the one-parameter event, initial true flag and cancellation retaining code 42 with or without the supplied string prefix. All four numeric lines replay without an allocator. Other event forms remain explicitly unsupported. The separately retained English event controls reproduce all seven native Russian lines and share the same event bridge and allocator; the integration test first reproduced the previous explicit English-name refusal. The public method's English alias is measured and supported. Existing object exports with the new method's names retain their own arity and result through the established additive export guard.

Extension-origin events use the ordinary handler bridge, following the vendor exception; CATALOG.NEW_CODE_EXTENSION_SAFE_MODE remains in the source, registry and central measure script pending an own native extension control. The integration test verifies this selected host behavior and caller restoration but does not replace the native extension oracle. Live SDK allocation remains unsupported, and CN4/CN5 stay open.

## Measured predefined full names (2026-10-17)

The paired native controls in `docs/research/fixtures/predefined-full-name-1017/` explicitly set Russian and English ScriptVariant with the same `/Lru` interface. Full names of system enumerations follow ScriptVariant, and empty applied enumeration references have a full name ending in `ПустаяСсылка` or `EmptyRef`. The earlier always-English system contract was based on an English-configuration observation and was too broad. Reuse the immutable catalog of an applied reference and the current Session catalog for system enums; do not read SQL or construct another reference. Preserve existing synthetic-catalog fallback behavior until its separate host contract is resolved; this measurement does not establish a platform state without ScriptVariant.

## Measured data-separation frame counters and conversion (2026-10-17)

The own-file-infobase controls in `docs/research/fixtures/data-separation-frames-1017/` run on 8.3.27.2342 with CompatibilityMode=Version8_3_27. Fifteen frame rows confirm inherited visibility, a frame-owned counter, rejection of disabling the caller counter, normal-return and exception cleanup. The 27-row argument matrix shows ordinary Boolean conversion rather than strict Boolean variants: 0 is false, nonzero Numbers are true; Истина/Да/True and Ложь/Нет/False are recognized after trimming. Invalid words, Undefined, Null, arrays and dates fail before changing the counter. Reuse Value::as_condition and preserve the operation-specific error context; do not duplicate conversion. The original String common-attribute boundary remains supported; arbitrary-value name conversion and exact native error wording are not added by this correction.

The existing getter deliberately refuses an unavailable zone baseline; this correction does not replace it with the VM counter query. A later host replay must observe counters through the existing CallContext capability and explicitly identify any substitution of the native getter. The old validation-order test used numeric 1 as an invalid mode; native 1C accepts it. A proposed one-line change substitutes an actually unconvertible word while preserving its Boolean-error assertion. That existing-test edit requires the user's approval and has not been applied.

## Critical custom-settings load failures (2026-10-17)

The retained own-base 8.3.27.2342 control explicitly raises settings-load-probe inside LoadProcessing. The platform shows a restart-required critical dialog and never executes the surrounding BSL catch or continuation. Evidence: approved-sdk-production-integration-1016/settings-storage-handler-error-1017/investigation.md. A headless host returns a typed error instead of opening a dialog.

Add a boxed original cause to RtError::CriticalConfiguration, classify that error as noncatchable through the existing exhaustive table, and retain it at the owning facade State boundary. Gate State start/instantiate/configuration-handler entry and ModuleInstance calls; pollable errors pass the same terminal-result recording. Existing VM unwinding handles the noncatchable error; no dispatcher, database dependency, thread-local storage or serialized opcode is added. A new State starts without a terminal error, even when its Engine is reused. Resource cleanup remains available and normal Drop releases the failed State's resources; no rollback is implied.

Only an explicit Raised error from the custom load-handler call is promoted in this correction. Cancellation and compilation errors retain their existing classification. Other load-handler runtime error classes remain under SETTINGSSTORAGE.LOAD_HANDLER_ERROR until independently measured. This is not permission to silently classify every callback or host-capability failure as fatal. Existing ordinary exceptions retain their catchability and state-reuse behavior.

This changes the public Rust error surface and facade lifecycle. It is covered by this active change's custom-settings requirement; implementation follows strict validation and failing regression tests. No existing test expectation needs to change.

The callback may initialize its target module before entering the handler. Its Raised result currently has no body/handler origin tag. The measured proof covers an explicit exception inside LoadProcessing; the existing open question also retains the unmeasured initialization-error case. No platform-equivalence claim is made for that path.


## Distinguish preparation from handler failure (2026-10-17)

The owned-base division-by-zero control shows a restart-required dialog. The initial manager-body probe showed a different dialog, but that dialog alone did not establish a runtime initialization exception. Reading its local detailed error report established ScriptCompileError: logical end of module source text after the procedure definition. A settings-manager module does not admit those body operators. The proposed ConfigurationHandlerInitialization type was based on an incorrect interpretation and is removed before accepting this correction. The native control remains as negative compilation evidence; neither followup callback nor retry receipts establish runtime initialization or its retry behavior. No report was sent externally.

The old post-callback Raised mapping cannot distinguish target preparation from handler execution. Move critical promotion to the existing VM callback bridge, after successful target preparation and around handler execution only. Add a Critical(operation) entry policy to the existing neutral host API. Escaping catchable BSL failures from that handler become CriticalConfiguration; cancellation, invalid bytecode and compilation failures retain their classifications. Target preparation errors are returned unchanged, just as in the existing bridge. No additional error type, language instruction or dependency is needed.

A synthetic host-defined module with a preparation Raised tests this neutral API boundary. It is valid input to the existing host recipe API, not a claim that 1C permits body operators in settings-manager or common modules. General JOB.MODULE.INIT outside this callback remains untouched. SETTINGSSTORAGE.LOAD_HANDLER_ERROR remains for other runtime classes; the native measurements establish explicit Raised and division by zero inside the handler. Existing test expectations stay unchanged.


## Custom settings handler aliases (2026-10-17)

The existing requirement names both ОбработкаЗагрузки and LoadProcessing, but runtime currently requests only the Russian name. The owned-base English-handler control executes the acquired English procedure successfully. Dual-handler native profiles prefer the Russian procedure for both ScriptVariant values and both declaration orders. Evidence is retained in settings-storage-runtime-error-classes-1017/{english-handler,dual-handlers-*}; only completed zero-exit receipts are accepted as the native alias oracle.

Reuse CallContext::configuration_function_parameter_count, already used to find the two catalog-event spellings. Query Russian first, then English; if neither exists, call the original Russian target to preserve its existing diagnostic. Lookup only inspects metadata and does not initialize the manager. Keep the critical entry policy and its canonical Russian operation label independent of the selected source spelling. No new BSL member, external dependency or VM mechanism is needed. Existing tests remain untouched; separate alias regressions cover English-only spelling, case, both declaration orders, normal results and critical handler errors.


## Principal-bound standard-settings reads (2026-10-17)

Owned-base 8.3.27.2342 controls in client-settings-rights-1017 separate no rights, SaveUserData only, DataAdministration only, Administration and a full control role. Both common and system stores allow own-namespace reads without SaveUserData. Other exact namespaces require DataAdministration, except in effective privileged mode. SaveUserData is separately required for saves; the database adapter remains read-only and does not implement writes. Native import normalizes Administration to include DataAdministration: use compiled role resources, not the input XML alone.

Storage namespaces are literal strings. Lowercasing a current name selects another namespace: nonadministrators are denied, while authorized readers observe absence. Explicit empty names read the seeded empty namespace, and a seeded nonexistent user name is valid. Do not resolve names through the user directory or fold them like BSL identifiers.

Replace the blanket unsupported cross-user-read branch in DatabaseDataAccess with a permission check using its existing AccessStore, acquired configuration object identity and effective CallContext privilege. Reuse the current role-ID and grants helpers; no second permission evaluator is introduced. Keep identity checks and separator scoping, run authorization before SQL, and preserve unsupported storage/write failures. Unknown root identity or missing/unreadable role resources are explicit errors. No new public Rust/BSL API or dependency is needed.

The measurement's first attempt incorrectly tried to disable privileged mode before enabling it in that procedure. Its explicit native error is retained separately and not used as a rights oracle. Accepted profiles verify the authenticated identity and an initially false getter, then separately measure privileged entry and restore it after a matching enable.


The owned PostgreSQL Pro 18 replay completed 95 native comparisons through the actual CLI and adapter: 89 match. Six differently cased namespace reads by authorized users still differ. Native `_UserID` columns are `mvarchar`, whose equality matches differently cased names; the SDK currently emits plain equality. Casting to `text COLLATE "C"` distinguishes the measured names, but SQL generation belongs to open-sdbl. Retain the failing replay and request the SDK fix in `docs/plans/open-sdbl-settings-user-case-request.md`; do not patch generated SQL in the host or mark the requirement complete. The replay uses a deliberately empty initializer in an owned configuration and does not certify original production session startup.


## Own-extension code event: measured entry policy (2026-10-17)

The retained own catalog control now covers both Boolean extension settings,
caller safe mode off/on, and caller suppression off/on on 8.3.27.2342. The own
event observes its extension setting in all eight calls, including true under
caller suppression. Effective caller modes and the parent frame are restored.
Evidence: `docs/research/fixtures/catalog-new-code-borrowed-1017/own-latch-safe-*.platform.txt`.

Do not reuse a Boolean source-origin mark as the entry policy. Pass the known
setting for an own object along the existing final-source binding path and
apply it only to this platform event. Keep the existing base event exception.
Unknown settings are not false. Preserve source replacement semantics and
avoid a second BSL runtime or a general new interception implementation for
this correction. Public binding and cold callback API details must be
specified and strictly validated before their code changes.

Separate adopted-object controls show different behavior. With only BorrowProbe
active and safe-mode=no, before/after hooks retain the caller's effective mode
(including suppression), while the base event sees safe mode off. With the
setting yes the direct extension method still runs, but the retained hooks
have no effects: unchanged prefix, unchanged description and manual code.
These results are not equivalent to the own-event policy. A mixed two-extension
attempt failed its direct-method canary; it is excluded from conclusions about
hooks or extension ordering. No general ordering rule is inferred.

The current marker CATALOG.NEW_CODE_EXTENSION_SAFE_MODE remains open until the
binding, error restoration and supported execution boundary are implemented and
verified. Native measurements alone do not close the runtime requirement.


### Concrete own-event API and cold entry

Add `ConfigurationModule::with_own_extension_source(bool)` and
`own_extension_safe_mode() -> Option<bool>`. Keep the existing origin builder;
it clears any own-event policy when replacing that origin. Add the corresponding
`MetadataObject::with_own_extension_object_module(name, bool)` and
`own_extension_object_module_safe_mode() -> Option<bool>`; base/generic extension
binders clear the own policy. No dependency is added to bsl-config.

Database structural loading stamps the known setting only on final own object
module sources, after the existing source application. Adopted replacements
clear the policy through the generic builder; an absent replacement preserves
the existing final source and policy. The complete factory path already rejects
unknown extension settings while projecting extension metadata. A code-only
snapshot with unknown settings does not manufacture a false policy.

Replace the raw FunctionCall::ConfigurationInstance Boolean with
`entry_mode: ConfigurationInstanceEntryMode`. Its three variants are Inherit,
WithoutSafeMode (the unchanged existing base-event policy), and
OwnExtensionSafeMode(bool). Re-export it through the facade. Add
`CallContext::call_configuration_instance_handler_with_safe_mode` with the normal
handler arguments and a final Boolean setting. Existing helper signatures and
BSL behavior remain unchanged; the raw Rust callback field migrates explicitly.

Sixteen native getter/setter controls distinguish event policy from an ordinary
inherited flag. With setting false, the event sees disabled=YES and can release
that local counter once; it cannot release an inherited safe-mode request.
With setting true, disabled=NO and neither release is permitted. Therefore own
false entry seeds one local suppression count/latch in the new root frame in
addition to the copied suppression snapshot; true entry overrides safe=true and
safe_disabled=false without seeding a local count. The inherited privilege
request remains, suppressed by safe mode when true. The caller snapshot is never
modified. Keep the separate old base variant to avoid changing its counter
observation as a side effect. No dispatch-loop change or new dependency is needed.
Retain the legacy generic-extension branch and its unresolved marker until its
remaining boundary is closed; do not apply an own-object policy to a borrowed
source solely because it originated in an extension.


Named security profiles are not Boolean policies: the database loader stamps
an own policy only when the acquired profile name is known empty. Unknown
extension properties still fail the complete factory's metadata projection;
code-only acquisition does not invent a setting. Named-profile execution stays
in the retained generic branch and open question.

Native own-error controls confirm Raised is caught at SetNewCode in all sixteen
mode/privilege combinations, retains the assigned code and restores caller and
parent modes. The Session test replays these exact controls independently for
two principals. CATALOG.NEW_CODE_EXTENSION_SAFE_MODE is narrowed, not removed:
own Boolean profiles, getters/setters and Raised restoration are measured;
named profiles and mixed-extension admission are not certified.


## Actual own-catalog acquisition projection defect (2026-10-17)

The current release was replayed against two owned native DT snapshots restored
to PostgreSQL Pro 18 with server-enforced default_transaction_read_only. Both
stop before the own event: catalog standard attributes are unknown. The same
native CFE resource through the existing public SDK decoder returns known
String code length 9, description length 25, hierarchy, own attribute membership,
empty sections and empty owners. This is a host projection defect, not an SDK
API gap. Evidence: catalog-own-extension-db-1017.

The base loader already projects these SDK fields; the extension loader appends
bare MetadataObject headers and skips that projection. Lift the existing catalog
field projection into one internal helper and apply it to acquired own-catalog
descriptors as well. Reuse parse_config_descriptors and Resources; add no decoder,
public API, guessed default, new dependency or permissions bypass. Preserve
module binding/policy, base behavior, known empty collections and unknown data.
Retain native resource regressions and replay both actual CLI cases. This is a
bug fix of the existing own-event/metadata contract, not a new language feature.


## Entry-source dependency closure before graph freezing (2026-10-17)

After catalog projection, both actual CLI controls reach a second failure:
the object's module is absent. SessionFactory roots currently include the
session module and visible common modules; compile_entry runs after the graph
is frozen and cannot add a private component family. Do not add dummy calls
to the session module or make every private module a root.

Add EngineBuilder::configuration_entry_sources with owned source texts. In
rooted_recipe resolve each source using the existing resolve_program_with_imports,
visible common-module surfaces and existing component-access imports; add only
its resolved links and deferred module dependencies to the pending closure.
Reuse the same dependency classifier used by configuration module traversal.
Do not add a synthetic module, execute the source or store it in the VM. Empty
sources add no import-table cloning overhead. A precompiled image or absent
recipe cannot be combined with these source roots. Without root filtering the
whole catalog is already compiled; this input is a closure hint, not an executor.

Add SessionFactory::new_with_entry_sources(code, catalog, session_module, kind,
platform_version, entry_sources). The existing new delegates with an empty
source iterator and retains its behavior. The CLI already reads the script:
pass that exact text before building the factory, then execute it through the
existing Session path. Syntax/semantic failures in declared source texts are
reported while building the selected graph, before opening the Session.

This is an additive host/compiler API and CLI fix. It does not add runtime
catalog mutation, another BSL resolver, source rewriting or new RT/VM dependencies.
Unknown future dynamic fragments are not claimed to gain new private families
without declaration; the API supplies known entry texts before graph freezing.


## Constructing entries for database denial logging

Native root SaveUserData controls in `docs/research/fixtures/verify-access-rights-native-1017/`
prove catchable denial and the platform-generated AccessDenied event. Existing
Session-owned `EventLogWriter` is the delivery boundary; no second journal or
thread-local sink is introduced. Its public `write` input currently has private
fields and getters only. Add one six-argument `EventLogEntry::new(event_name,
level, metadata, data, comment, transaction_mode)` constructor. This is a Rust
host API and makes no additional platform-semantics decision. The existing BSL
WriteLogEvent path uses the same constructor, preserving argument checks and
writer selection. Construction does not deliver anything. Keep fields private
and retain the same Value ownership model and Arc string storage.

Database verification wiring and native coverage of further metadata/right
cases remain AVDB2–AVDB4 work. This constructor alone does not certify that
requirement. Do not infer transaction rollback behavior from the native
no-transaction journal profile.


## Preserving metadata at the access-verification boundary

The native-valid root call currently fails with TypeError before any adapter
invocation (`docs/research/fixtures/verify-root-metadata-gap-1017/`). Reuse the
existing AccessRight metadata validation for VerifyAccessRights: validate the
current catalog by identity, accept its acquired root as Configuration, and
retain operation-specific diagnostics. Do not maintain a second root decoder.

Retain the original validated Value in the private AccessRightRequest state
and expose a read-only `metadata_value()` getter. Both call paths supply it.
The database adapter needs the same object for a Session journal entry; it must
not reconstruct an ORM metadata wrapper from names or identifiers. This is an
additive Rust API with no new dependency or resolution rule. Missing acquired
root identity and foreign-catalog metadata remain explicit errors before
adapter dispatch. Native standard-attribute results remain separate evidence
and do not imply that unknown paths can be guessed by the adapter.


## Database verification and Session journal delivery

Override DatabaseDataAccess::verify_access_rights through a small helper beside
configuration_rights. Reuse its existing AccessStore/SDK grant check after the
adapter's principal-identity check; do not decode rights again. Obtain and
clone the writer from the invoking SessionRuntime in the current CallContext,
not from the database adapter or thread state. Require that capability even
for an allowed result; the adapter cannot promise verification-with-journaling
without it. Unknown/inapplicable/unsupported checks keep their existing errors.

For a denied supported check construct the native event with EventLogEntry::new,
Information level, empty comment and a dictionary Structure with preserved
spelling Right and the SDK's canonical English right name. Configuration-root
entries use Undefined metadata; object entries retain the validated original
Value. The native explicit-transaction/rollback profile proves independent
journal delivery, and the native privileged profile proves grant bypass for
the current root principal. Write once and return the permission failure only
after successful delivery. Propagate writer failures without pretending that
a denial was recorded. No persistent journal backend or native log-file writer
is added: the already accepted Session-owned EventLogWriter remains the host
boundary. Actual acquired-database replay and broader requirement verification
remain separate AVDB work; a test wrapper around a helper is not that proof.


## Standard-right applicability before privileged bypass

The 384 native observations in `verify-access-rights-native-1017/standard-right-results.json`
separate invalid targets, inapplicable rights, refusals and grants. A valid
standard Read is an applicability error for AccessRight but successful no-op
verification. The current ordinary non-root privileged shortcut returns true
for an unknown right/path and inapplicable Read without adapter dispatch; the
current-host reproduction is `metadata-right-privileged-gap-1017/`. This is a
remaining implementation defect, not evidence of platform behavior.

Request structural resolution/applicability and cached role evaluation from
SDK (`docs/plans/open-sdbl-metadata-right-evaluation-request.md`). Do not copy
its member/default/parent rules into open-bsl or change SQL/RLS field checking
from metadata-right observations. Integrate the new SDK decision through the
existing adapter with operator-specific inapplicability policy, validate before
privileged grant bypass, and preserve current metadata identity and journal
boundaries. SDK assignment and version remain pending user direction; no SDK
or runtime change is made in this measurement phase. The previously verified
ordinary access-right requirement is reopened on this direct native/host mismatch.

## Native two-role standard-right evaluation (2026-10-17)

The 108 retained outcomes in `docs/research/fixtures/verify-access-rights-native-1017/multiple-roles/` distinguish role-local parent gating from a union of all parents and all children. With false attribute defaults and independent-child mode disabled, owner View granted by role A and Code View granted by role B whose owner View is false remain denied. Granting owner View in B makes the same member allowed; both child refusals remain denied. Evaluate the measured parent/member combination within a role before combining role grants. The SDK request now includes these native CF resources; no second host evaluator or SDK implementation is added here. Other multiple-role flags and metadata kinds remain unmeasured.

## SDK0.9.22 independent-child correction before host integration (2026-10-18)

The full 492-row SDK replay found two retained native outcomes previously summarized incorrectly: with independent=true, attribute defaults=false, owner View=false and Code View=true, ordinary AccessRight refuses while VerifyAccessRights succeeds. The host must use AccessStore::metadata_right and metadata_right_for_verification respectively, after the same structural resolver. The prior blanket statement about independent flags is corrected by this evidence; parent gating still applies to ordinary access and to the measured multi-role profiles with independent=false. Inapplicable standard Read remains an ordinary applicability error and a successful verification. SDK resolution precedes current-principal privileged grant bypass. Preserve the reached root/other-owner paths through their existing adapter; unsupported targets remain explicit, never silently granted. No SDK/DB knowledge enters ORM or RT/VM.

Host regression correction before code: structural String validation is delegated first to preserve the measured unknown-right precedence, but successful custom adapters must not make an empty standard path valid. Both ORM operations retain a final empty-path guard after successful adapter return. The production SDK rejects invalid paths before grant evaluation or journal delivery; this final host guard preserves the existing custom-adapter contract. The unchanged old invalid-argument test reproduces the missing guard.

## Native dynamic compilation inside settings LoadProcessing (2026-10-18)

`docs/research/fixtures/settings-storage-dynamic-compile-error-1018/` retains the native restart-required dialog and local error details for Evaluate of an unknown function from the already executing handler body. Native details classify the failure as ScriptRuntimeError/ScriptUseError (error compiling or calculating an expression), not manager-module compilation. Preserve this execution-phase distinction: the existing critical callback entry is expected to wrap the original dynamic-expression runtime cause, while preparation errors remain unchanged. This adds a measured body-error class; it does not close the question about every remaining class. No platform report was sent.


### Measured explicit subjects of catalog rights (2026-10-18)

The120 native controls in
`docs/research/fixtures/metadata-explicit-subjects-native-1018/` cover Read/View,
Code/Description, two current users, explicit roles/self descriptions and both
privilege modes. Role names are not grant summaries; effective CF/XML is
retained. Native structural/applicability errors precede the explicit-user
administrative prerequisite. Privilege bypasses grants only for an implicit
current-principal check. For an explicit user it bypasses only Administration;
explicit role/user grants still use the SDK evaluator.

Reuse the existing configuration-root subject-role selection for the catalog
path. Resolve the root object ID from the current Session catalog only when an
explicit-user Administration check needs it. Keep the existing immutable rights
snapshot: requested roles must already be loaded, and missing roles fail through
the existing completeness boundary. This does not introduce a second rights
store, implicit role fetching, a host-side role parser or SQL. The evaluator and
its ordinary/verification policy distinction remain in open-sdbl. The new
controls do not establish other metadata kinds or malformed-subject priority.

## SDK 0.9.24 user language integration

The CLI resolves the selected acquired user after principal selection. Unknown SDK language data fails startup; known nil and the platform empty user use the acquired primary language. Validate the resulting identity as Language metadata, then store it in the immutable Principal before opening Session. No user/configuration rereads, new cache, SQL or dependency. Native evidence is current-language-session-audit/native-selected-language-1017 and native-user-records-1017; SDK decodes only their measured record form. The direct host API retains its existing lazy error behavior.
