## ADDED Requirements

### Requirement: Source-neutral incremental execution catalog
The bytecode boundary SHALL expose a source-neutral catalog interface that discovers stable module identities without materializing module code and retrieves executable programs on demand. The VM SHALL allocate execution caches and module state for an unprepared module only when it is reached. Existing complete configuration images SHALL remain usable through the same execution entry points without a database or frontend dependency in the VM.

#### Scenario: Attach an indexed catalog
- **WHEN** execution attaches a catalog with prepared and unprepared modules
- **THEN** attachment does not request executable code for unprepared modules
- **AND** the first actual access prepares the requested program before executing its instructions

### Requirement: Dependency-directed configured session preparation
Configured session startup SHALL prepare the session module, supplied entry sources and their transitive dependencies without resolving, compiling or verifying every server common-module body. Name discovery SHALL NOT imply preparation of that named module. Existing name precedence and global declaration conflict detection SHALL remain unchanged.

#### Scenario: Many unused server modules
- **WHEN** a configuration contains a session root, one transitive dependency chain and unrelated server modules
- **THEN** only the root, entries and reached dependencies undergo body resolution, code generation and verification
- **AND** unrelated modules are not selected merely because they are server common modules

#### Scenario: Keep declaration conflicts explicit
- **WHEN** the declaration index contains conflicting global exports
- **THEN** factory preparation reports the conflict without resolving unrelated bodies

### Requirement: Prepare runtime-discovered dependencies at first access
A configured session SHALL support dependencies whose names become known only during execution. First access SHALL prepare the requested module and its dependencies through the existing BSL pipeline before execution. Preparation SHALL preserve published module identities and existing live module state. Immutable prepared code SHALL be reusable by Sessions of the same factory; mutable module state SHALL remain isolated.

#### Scenario: Runtime common-module name
- **WHEN** a valid module name is passed through a parameter into `Вычислить`
- **THEN** the target and its dependencies are prepared on first access and its export executes normally
- **AND** unrelated module bodies remain unprepared

#### Scenario: Reuse in an isolated Session
- **WHEN** a second Session of the same factory calls a previously prepared module
- **THEN** its immutable prepared code is reused
- **AND** its module variables, initialization state and session parameters belong to the second Session

#### Scenario: Late code after runtime field-name growth
- **WHEN** a Session has more than 65,536 runtime field names and reaches a valid module or dynamic fragment whose own member operands fit the bytecode format
- **THEN** execution keeps that code usable without an artificial limit from the Session's accumulated names
- **AND** structures retain their fields and literal key spelling across static and dynamic calls

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
