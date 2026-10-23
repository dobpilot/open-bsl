# configuration-code-loading Specification

## Purpose
Define a source-neutral configuration code snapshot and the database loader that produces it, including module visibility, extension handling, and placeholders for future XML and EDT providers.

## Requirements

### Requirement: Source-neutral configuration code snapshot
The system SHALL represent configuration code as an immutable snapshot containing the configuration identity and one logical graph of modules. Every module SHALL have one effective source body, stable identity, kind, execution attributes, imports, and an optional owning metadata object. The snapshot SHALL expose no database, XML, or EDT storage details.

#### Scenario: One effective module
- **WHEN** a loader constructs a snapshot for a logical module
- **THEN** the snapshot contains exactly one source body for that module
- **AND** a consumer does not choose between base and extension variants

#### Scenario: Whole-snapshot source selection
- **WHEN** a configuration snapshot is loaded
- **THEN** every module comes from the same selected database, XML, or EDT loader

#### Scenario: Code source differs from metadata source
- **WHEN** a future XML or EDT loader supplies the complete code snapshot and matching ORM metadata is acquired from an information base
- **THEN** the two inputs remain independently sourced
- **AND** no module from the database code store is mixed into that code snapshot

### Requirement: Database configuration loader
`bsl-config-db` SHALL build the neutral snapshot from the configuration resources acquired through `open-sdbl-db`. It SHALL reuse the database adapter's provider handling, limits, resource-part assembly, metadata acquisition, and extension resource acquisition. It SHALL NOT implement SQL provider access independently.

#### Scenario: Code and metadata from one acquisition
- **WHEN** code and metadata are selected from the same information base
- **THEN** the database configuration resources are acquired once
- **AND** the acquisition also returns the resolved metadata snapshot and report

#### Scenario: Unsupported active extension
- **WHEN** an active extension cannot be reduced to an effective module graph
- **THEN** loading fails with an error identifying the extension
- **AND** base sources are not presented as the effective configuration

### Requirement: Configuration common-module visibility
The configuration module catalog SHALL expose a non-global common module under its configuration name and SHALL expose exported procedures and functions of a global common module as bare BSL names. It SHALL use the existing open-bsl resolver, compiler, module instances, and VM without rewriting the acquired BSL source. A declaration in the current module SHALL retain its existing priority over the configuration-wide global surface. Duplicate bare exports from different global common modules SHALL fail engine construction explicitly.

The database loader SHALL preserve the managed-client, ordinary-client, server, external-connection, global, privileged, and server-call attributes encoded by the common-module descriptor.

#### Scenario: Serialized execution attributes
- **WHEN** a database common-module descriptor carries independently distinguishable execution flags
- **THEN** the neutral snapshot reports each flag under its matching attribute
- **AND** client availability is true when either client flag is enabled

#### Scenario: Named common module
- **WHEN** one configuration module calls an export as `ModuleName.Method()`
- **THEN** the call is linked to the non-global common module named `ModuleName`

#### Scenario: Global common module export
- **WHEN** a configuration module calls an exported method of a global common module by its bare name
- **THEN** the call is linked to that common module in the same State

#### Scenario: Mutually referring modules
- **WHEN** two configuration modules refer to each other's exported declarations
- **THEN** both bodies are resolved against the complete declaration catalog
- **AND** source order does not make either name undefined

#### Scenario: Duplicate global export
- **WHEN** two global common modules export the same folded bare name
- **THEN** engine construction fails with an error naming the conflicting export

### Requirement: Future source crate placeholders
The workspace SHALL contain `bsl-config-xml` and `bsl-config-edt` as documented placeholders. Until their respective implementations are specified, they SHALL expose no loading operation and add no CLI option or dependency.

#### Scenario: Placeholder build
- **WHEN** the workspace is built
- **THEN** both placeholder crates compile
- **AND** neither crate claims to load a configuration

### Requirement: Configured foreground CLI session kind
Database script mode SHALL accept optional `--session-kind` values
`thin-client` and `web-client`, passing the selected kind to its existing
session factory. Omission SHALL preserve thin-client behavior. The option
SHALL NOT simulate or launch a background job.

#### Scenario: Foreground kind selection
- **WHEN** database script mode specifies either accepted kind
- **THEN** the existing factory opens that foreground session kind

#### Scenario: Existing configured invocation
- **WHEN** database script mode omits the option
- **THEN** it retains the existing thin-client session

#### Scenario: Invalid modifier
- **WHEN** the option is repeated, has an unsupported value, or appears without database mode before the script path
- **THEN** argument validation fails before connecting

#### Scenario: Script arguments
- **WHEN** the same option text appears after the script path
- **THEN** it remains a script argument and does not select the session kind

### Requirement: Structural configuration extension application
`bsl-config-db` SHALL build the neutral snapshot from the configuration
resources acquired through `open-sdbl-db`. It SHALL reuse the database
adapter's provider handling, limits, resource-part assembly, metadata
acquisition, and extension resource acquisition. It SHALL NOT implement SQL
provider access independently.

Active extensions SHALL be applied in platform order through explicit
extension-to-base identity links. For an adopted StyleItem, CommonPicture, Language, DefinedType, CommonCommand,
Catalog, or Enum whose measured
header contains no target identifier, the adapter SHALL use the measured
class-and-name binding rule. It SHALL NOT apply this fallback to an explicit
target identifier or to an unmeasured object class.
An extension MAY add objects and modules or
replace the effective module of a borrowed object. The result SHALL contain one
effective source body per logical module and SHALL expose neither base/patch
variants nor extension storage identities to consumers. Inactive extensions
SHALL NOT modify the code graph. Unknown activity, an incomplete catalog, or an
operation that cannot be reduced to an effective module graph SHALL fail with
an error identifying the extension.

#### Scenario: Structural active extension
- **WHEN** an active extension adds a module and changes a module of an explicitly borrowed base object
- **THEN** both changes appear in the one effective module graph
- **AND** the borrowed object is linked by identity rather than by name

#### Scenario: Ordered active extensions
- **WHEN** two active extensions change the same logical module
- **THEN** they are applied in platform order and the final snapshot contains only the resulting source body

#### Scenario: Adopted object of a measured class without an explicit target
- **WHEN** an adopted object of one of the measured classes has no target identifier in its measured header
- **AND** the base configuration contains the matching class and exact name
- **THEN** the operation binds to that base object using the measured name rule
- **AND** a renamed base object no longer matches even if it retains its identifier

#### Scenario: Explicit target remains authoritative
- **WHEN** an adopted object carries an explicit target identifier that is absent from the base
- **THEN** loading fails even if a base object has the same name

#### Scenario: Case differs for a checked name
- **WHEN** the base object name differs only in case
- **AND** acquisition does not establish that its Name value check is disabled
- **THEN** loading fails instead of treating folded lookup as successful applicability

#### Scenario: Unmeasured name binding
- **WHEN** an adopted object of an unmeasured class lacks an explicit target identifier
- **THEN** loading fails rather than generalizing the measured class set

#### Scenario: Inactive structural extension
- **WHEN** a structural extension is present but inactive
- **THEN** its modules do not enter the effective code graph

#### Scenario: Unsupported structural operation
- **WHEN** an active extension operation cannot be represented as one effective module graph
- **THEN** loading fails explicitly and base sources are not presented as effective

### Requirement: Explicit protected module body recovery
The database loader SHALL default to an exported-interface-only source for
image-only protected modules. It SHALL retain exported procedure/function
names and supported parameter passing modes and default values. For an opaque
export header declaring a positive number of valid named parameters with external slots
zero and no default table, interface-only loading SHALL permit an explicitly
simplified signature with that number of mandatory parameters and no defaults. This exception
SHALL NOT claim native parameter modes or call arity. Procedures SHALL have
empty bodies and functions SHALL return True. Placeholders SHALL be synchronous,
including measured asynchronous exported declarations; they SHALL NOT claim
the native promise or reference-update semantics. It SHALL omit private methods,
external declarations, module variables and module initialization.
This mode SHALL NOT claim equivalence to the original implementation.

The loader SHALL expose a per-load option to request existing complete body
recovery for both base and extension modules. Complete recovery SHALL reject
unsupported declarations or bodies without falling back to placeholders.
Malformed image tables and unsupported exported signatures SHALL remain errors.
Modules with available source text SHALL remain unchanged in both modes.

#### Scenario: Default exported interface
- **WHEN** an image-only module is loaded without options
- **THEN** only its exported declarations are emitted
- **AND** a procedure performs no work and a function returns True
- **AND** private bodies and initialization do not execute.

#### Scenario: Explicit complete recovery
- **WHEN** complete decompilation is enabled
- **THEN** supported complete modules retain their native observations
- **AND** unsupported modules fail rather than receive placeholder bodies.

#### Scenario: Malformed exported signature
- **WHEN** an exported declaration has an unsupported parameter/default encoding
- **THEN** interface-only loading fails explicitly.

#### Scenario: Text and extension sources
- **WHEN** either a base or extension container has text source
- **THEN** its source is preserved regardless of the decompilation option
- **AND** image-only extension modules use the same option as base modules.

#### Scenario: Asynchronous exported interface
- **WHEN** a supported measured async export flag is present
- **THEN** interface-only loading retains its name and parameters as a synchronous placeholder
- **AND** complete decompilation retains its existing strict recovery rules.

#### Scenario: Opaque parameter placeholder
- **WHEN** an exported header has the approved opaque shape with no default table
- **THEN** interface-only loading emits all declared parameter names as mandatory parameters
- **AND** calling the placeholder leaves argument values unchanged
- **AND** complete recovery still rejects this unsupported signature.

#### Scenario: Invalid opaque parameter names
- **WHEN** the opaque parameters have invalid or duplicate names or fewer records than declared
- **THEN** interface-only loading rejects the header.

### Requirement: CLI protected body switch
Database script mode SHALL accept `--decompile-protected-modules` to enable
complete body recovery. Omission SHALL select exported-interface-only loading.
The flag SHALL be documented and SHALL require database mode.

#### Scenario: Flag selects complete recovery
- **WHEN** the flag precedes a database script path
- **THEN** factory input loading enables complete decompilation.

#### Scenario: Invalid flag use
- **WHEN** the flag is repeated, has an attached value or appears without database mode before the script path
- **THEN** validation fails before connecting.

#### Scenario: Script argument remains unchanged
- **WHEN** the flag follows the script path
- **THEN** it remains a script argument and does not enable decompilation.

### Requirement: Configuration compilation errors identify their source
When bytecode generation rejects a configuration method or module initializer,
the diagnostic SHALL identify the configuration module and, for a method,
its function or procedure name. Standalone compilation SHALL keep its
existing diagnostic.

#### Scenario: A configuration method exceeds the frame limit
- **WHEN** a method in a named configuration module uses more local slots than the bytecode frame can represent
- **THEN** compilation fails with the existing limit error and names both the module and method

#### Scenario: A standalone script exceeds the frame limit
- **WHEN** a standalone script uses more local slots than the bytecode frame can represent
- **THEN** compilation fails with the existing standalone limit error

### Requirement: Sparse local names fit the bytecode frame
A configuration method without exception handlers, dynamic source evaluation,
or debug-local materialization, whose source declares more than 255 distinct
local names SHALL compile when its simultaneously needed local values and
temporary registers fit the existing bytecode frame. The compiler SHALL preserve parameter
positions, module variable slots, by-reference updates, and observable
behavior. A method that cannot fit SHALL fail with the existing frame-limit
error; bytecode register fields SHALL remain unchanged.

#### Scenario: Configuration method has many sequential locals
- **WHEN** a configuration method has more than 255 distinct locals that are used in non-overlapping phases
- **THEN** it compiles and yields the same result as its source semantics

#### Scenario: Values do not fit simultaneously
- **WHEN** more local values must coexist than the frame can represent
- **THEN** compilation fails explicitly without truncating register indices

### Requirement: Configuration method arity failures are lazy
When a configuration module contains a method whose body calls a local or imported function with too many arguments or omits a required trailing argument, engine construction SHALL retain the valid methods of that module. Calling the invalid method SHALL raise a catchable arity error. Strict arity validation for standalone scripts and compiled entries SHALL remain unchanged.

#### Scenario: Another export in the same module is valid
- **WHEN** a configuration module has one valid export and another export with a too-many-arguments call
- **THEN** engine construction succeeds and the valid export executes normally

#### Scenario: Invalid local or imported call is executed
- **WHEN** the invalid export calls a local or imported function with too many arguments or omits a required trailing argument
- **THEN** that export raises a catchable arity error when invoked

#### Scenario: Standalone entry remains strict
- **WHEN** a compiled entry calls an imported function with too many arguments
- **THEN** compilation returns the existing arity diagnostic

### Requirement: Common-module names resolve to module values
A configuration module SHALL resolve an unshadowed bare imported common-module name to the same Session module-reference path used by dynamic evaluation. Local and module variables SHALL retain priority. The reference SHALL preserve the existing initialization behavior of its target; an invalid target SHALL fail on access without being mistaken for an invalid variable name in the reader. No new interpreter or rewritten configuration source SHALL be introduced.

#### Scenario: Static arguments and assignment
- **WHEN** a configuration method passes its own or another common-module name as an argument, or assigns that name to a variable
- **THEN** it obtains the corresponding module reference, as measured by the native module-argument-context control

#### Scenario: Dynamic common-module reference
- **WHEN** a configuration method evaluates a string containing an imported common-module name and calls an export on the resulting value
- **THEN** the call reaches that Session's module instance, including its initialized module variables

#### Scenario: Configuration module reference type
- **WHEN** a configuration method reads an imported common-module name
- **THEN** the resulting reference has type `ОбщийМодуль` / `CommonModule`, and standalone module references retain their existing `МодульBSL` type

#### Scenario: Invalid target stays invalid
- **WHEN** reading a common-module reference initializes a target with a genuine module compilation error
- **THEN** its original module initialization failure remains explicit

#### Scenario: Ordinary unknown variable
- **WHEN** a module reads a name that is neither a variable nor an imported module alias
- **THEN** the existing semantic diagnostic and configured deferral policy remain unchanged

### Requirement: Named modules resolve self-qualified calls
A named or global configuration module SHALL resolve its own module name
through the same module-reference path by which another module resolves it.
The self-reference SHALL NOT add an initialization-graph cycle. Reading the
bare self name SHALL return that same module reference, including when
passed as an argument or assigned to a variable.

#### Scenario: Self-qualified exported call
- **WHEN** a named module calls one of its exports through its own module name
- **THEN** the call executes on that module's existing instance

#### Scenario: Bare self name
- **WHEN** a named module reads its own unshadowed bare module name as a value
- **THEN** it obtains its own module reference without an initialization cycle

### Requirement: Unreachable branch targets preserve scoped source blocks
For the retained overlapping-method controls, source recovery SHALL preserve
all declarations and callable behavior when a command unreachable from every
entry targets the middle of an otherwise valid expression. Such a target
SHALL NOT split the current method's expression block. Recovery SHALL use the
active method's labels and SHALL retain explicit rejection of unsupported
reachable instructions or invalid reachable targets.

#### Scenario: Unreachable jump targets a public initialization expression
- **WHEN** the scoped-foreach expression-target control is recovered into BSL
- **THEN** both nonempty and empty public loops and the private wrapper replay all seven native receipt lines
- **AND** all three method declarations remain present

#### Scenario: The same jump becomes reachable
- **WHEN** a mutation makes the unsupported mid-expression jump reachable from a method entry
- **THEN** recovery rejects the image instead of silently omitting that path

### Requirement: Scoped conditional returns preserve the shared result
For the measured overlapping-method return-join profiles, recovery SHALL
preserve both alternatives of a conditional expression feeding one shared
return command. The resulting source SHALL compile and execute through the
existing BSL front end and VM, retain all declarations, and preserve private
caller arguments. Unsupported merge states or cleanup paths SHALL remain
explicit recovery errors.

#### Scenario: False-branch return join
- **WHEN** the scoped-return-join-40 control is recovered
- **THEN** both public alternatives and the private wrapper replay every native receipt line

#### Scenario: True-branch return join
- **WHEN** the scoped-return-join-41 control is recovered
- **THEN** branch polarity and both returned types remain identical to the native receipt

#### Scenario: Unsupported return cleanup
- **WHEN** a mutation adds an unsupported operation to the shared return's cleanup path
- **THEN** recovery rejects the image instead of omitting that operation

### Requirement: Infeasible conditional edges preserve call preparation
For measured scoped literal-condition profiles, a conditional edge proven
infeasible by the existing reachability analysis SHALL NOT split argument
preparation from its call. Recovery SHALL preserve the call, result and
caller arguments. A feasible or unknown unsupported edge into an incomplete
call preparation SHALL remain an explicit error.

#### Scenario: Literal True before a false jump
- **WHEN** scoped-infeasible-call-edge-40 is recovered
- **THEN** its complete native public and private observations are reproduced

#### Scenario: Literal False before a true jump
- **WHEN** scoped-infeasible-call-edge-41 is recovered
- **THEN** its complete native observations are reproduced with unchanged branch polarity

#### Scenario: The incomplete call edge can execute
- **WHEN** a mutation makes the mid-call conditional edge feasible or unknown
- **THEN** recovery rejects that unsupported path rather than dropping the call

### Requirement: Scoped collection loops preserve outside back-edge trampolines
For the measured scoped foreach profiles with an outside direct-jump
trampoline, recovery SHALL preserve iteration, conditional continuation,
break and iterator cleanup through the existing BSL compiler and VM. All
method declarations and caller arguments SHALL be retained. Unsupported
trampoline operations, cycles or destinations SHALL remain explicit errors.

#### Scenario: Back-edge outside the loop body
- **WHEN** scoped-foreach-outside-trampoline is recovered
- **THEN** its entire native receipt is reproduced for nonempty and empty collections and private caller observations

#### Scenario: Conditional continuation and early break
- **WHEN** scoped-foreach-outside-conditional is recovered
- **THEN** false comparisons continue through the outside trampoline and a true comparison breaks with the same native result

#### Scenario: Unsupported trampoline
- **WHEN** the outside trampoline contains an operation other than the measured direct jump or does not lead to the iterator check
- **THEN** recovery rejects the image without omitting the operation

### Requirement: Scoped backward boolean assignments preserve reference targets
For the measured scoped ordinary procedure profile, recovery SHALL preserve
both alternatives of a backward boolean assignment and its reference target,
retain every method declaration, and execute the resulting source through
the existing BSL compiler and VM. Unsupported shared-entry mutations SHALL
remain explicit recovery errors.

#### Scenario: Both caller-visible boolean alternatives
- **WHEN** scoped-backward-bool is recovered
- **THEN** the exported procedure reproduces the true and false reference-parameter observations and every private caller receipt line

#### Scenario: An extra entry reaches the assignment join
- **WHEN** a mutation introduces another entry into the protected boolean join
- **THEN** recovery rejects the unsupported shared stack state

### Requirement: Measured private zero-label tables preserve callable routes
For the measured four-slot private procedure profile with 27 zero-address
labels, recovery SHALL retain every declaration, preserve private and public
routes, message ordering, return values and caller arguments, and execute
through the existing BSL compiler and VM. Unsupported label targets or frame
mutations SHALL remain explicit recovery errors.

#### Scenario: Private table has 27 zero labels
- **WHEN** zero-label-procedure-27 is recovered
- **THEN** every native public/private message and value observation is reproduced

#### Scenario: Private table contains an unmeasured address
- **WHEN** a mutation changes a private zero label to an unsupported address
- **THEN** recovery rejects the unsupported route instead of choosing another frame's labels

### Requirement: Measured private string frames preserve route observations
For the measured four-slot flags2 procedure with27 zero labels and a string
constant in slot0, recovery SHALL preserve all declarations, route messages,
return values and unchanged caller arguments through the existing BSL
compiler and VM. This includes the measured declared Var size18 with four
actual descriptors. Unmeasured frame or constant combinations SHALL remain
explicit recovery errors.

#### Scenario: String alias replaces the numeric constant
- **WHEN** zero-label-procedure-27-string is recovered
- **THEN** every native public/private route and caller observation is reproduced

#### Scenario: Declared frame size exceeds descriptor count
- **WHEN** zero-label-procedure-27-string-frame18 is recovered
- **THEN** all complete native receipt lines remain identical to the measured string-frame control

### Requirement: Ordinary scoped bodies reuse complete module declarations
For the measured ordinary shared-body module profile, recovery SHALL preserve
ordinary method signatures, reference parameters, defaults, visibility and
external/global bindings while recovering each body in its own frame and
label context. The source SHALL execute through the existing BSL compiler
and VM. The complete image SHALL remain rejected when another declaration
has an unsupported header or body; no successful partial source SHALL be
returned.

#### Scenario: Ordinary methods share a body with contextual declarations
- **WHEN** ordinary-shared-context is recovered
- **THEN** both ordinary declarations remain, the global and external declarations retain existing resolver bindings, and every native receipt line is reproduced

#### Scenario: An unsupported private declaration remains
- **WHEN** a mutation appends an unsupported private declaration to the same module
- **THEN** recovery rejects the entire image instead of returning only the ordinary methods

### Requirement: Configuration temporary-register pressure uses available frame capacity
A configuration method whose source locals fit the bytecode frame but whose ordinary emission exhausts temporary registers SHALL compile when existing local-slot allocation makes its simultaneously needed values and temporaries fit the unchanged frame. Observable evaluation order, parameter positions, ByRef updates and source-name identities SHALL be preserved. Successful ordinary compilation and standalone compilation SHALL retain their existing paths. Exception-handler, dynamic-source and debug-local materialization restrictions SHALL remain unchanged. Genuine simultaneous frame overflow SHALL fail explicitly with the existing contextual diagnostic.

#### Scenario: Near-limit locals with a short expression
- **WHEN** a configuration method has243source locals and an expression with thirteen binary nodes but only a small live subset
- **THEN** it compiles within the existing255-register frame and executes with the native result

#### Scenario: ByRef updates and call order
- **WHEN** an eligible method computes a pressured expression and writes its result through a ByRef parameter
- **THEN** the caller observes the native result
- **WHEN** two calls within a pressured expression append distinct markers to a shared log
- **THEN** their observable order matches the native platform

#### Scenario: Interned names survive the retry
- **WHEN** structure literals and property names occur before and after the pressured expression and in another module
- **THEN** their names, spelling, shape identities and executable values remain valid after compilation

#### Scenario: Restricted or truly overflowing frames
- **WHEN** a method requires dynamic/debug locals, contains exception handlers or cannot fit simultaneous values and temporaries
- **THEN** compilation retains the existing explicit frame-limit failure
- **WHEN** the same pressured source is compiled standalone
- **THEN** its existing standalone diagnostic is unchanged

### Requirement: Distinct fatal runtime compilation error
The runtime SHALL expose `RtError::ModuleCompilation(String)` as a distinct error carrying the compilation diagnostic. It SHALL bypass BSL exception handlers through the existing VM error classification boundary. This classification SHALL NOT change existing runtime error classes or State isolation.

#### Scenario: Host returns a compilation error during module initialization
- **WHEN** a component call returns that error from a module initializer inside nested BSL exception handlers
- **THEN** neither handler nor subsequent caller code runs, and the host receives the typed diagnostic
- **AND** an unrelated State of the same engine remains usable

### Requirement: Host-controlled deferred common-module name failures
The facade SHALL provide an explicit opt-in to defer undefined-variable semantic failures and undefined global-function failures in method bodies of named and global configuration common modules until the affected module instance is initialized. Existing facade builder defaults and resolver entry points SHALL retain strict semantic rejection. The option SHALL NOT defer semantic validation of private object/manager modules or explicit session modules. The implementation SHALL use the existing resolver, compiler, and module initialization machinery.

#### Scenario: Unused erroneous child common module
- **GIVEN** the host enables the policy and a valid parent export has a neighboring export referencing a child with an undefined variable
- **WHEN** execution calls only the valid parent export
- **THEN** engine construction and the parent call succeed without initializing the child

#### Scenario: Erroneous common module is accessed
- **GIVEN** the host enables the policy and an affected common module contains an undefined variable
- **WHEN** execution first accesses that module
- **THEN** initialization fails before any export body of that module runs
- **AND** the compilation failure is not converted to an ordinary exception recoverable by the caller's BSL `Попытка/Исключение`

#### Scenario: Default compilation remains strict
- **WHEN** an existing consumer builds a configuration without enabling the policy
- **THEN** an undefined variable retains its existing compile-time diagnostic

#### Scenario: Caller and argument side effects precede child compilation failure
- **GIVEN** the host enables the policy and the child contains an undefined variable
- **WHEN** a parent method performs a side effect and computes a side-effecting argument before calling the child
- **THEN** both caller and argument side effects occur before the fatal child compilation failure
- **AND** the child export body does not run

#### Scenario: Private module diagnostics remain strict
- **WHEN** the policy is enabled and a selected private object or manager module contains an undefined variable
- **THEN** engine construction rejects that module with its existing semantic diagnostic

### Requirement: Session factories use deferred common-module name failures
Session factories SHALL enable the host policy for server common modules in their configuration graph. They SHALL preserve module instance isolation between Sessions and SHALL NOT rewrite source text or fabricate a missing module.

#### Scenario: Faulty common module is not reached during startup
- **GIVEN** an erroneous common module belongs to the conservative static closure but startup never accesses it
- **WHEN** a Session initializes its parameters
- **THEN** that module does not prevent initialization of unrelated modules

#### Scenario: Unused common module with an unknown global function
- **GIVEN** the host enables the policy and a child common module has a method body calling an unknown global function
- **WHEN** execution calls only a valid parent export which does not access that child
- **THEN** construction and the parent call succeed
- **AND** first access to the child fails with a fatal module compilation diagnostic before any child export body runs

#### Scenario: Missing imported method retains its existing policy
- **GIVEN** a common-module method calls an absent method of an existing imported common module
- **WHEN** another valid export of that module is called
- **THEN** the existing method-scoped failure policy is preserved
