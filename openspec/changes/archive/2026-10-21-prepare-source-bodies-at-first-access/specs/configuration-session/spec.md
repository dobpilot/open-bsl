## MODIFIED Requirements

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
