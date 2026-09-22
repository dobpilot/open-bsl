## ADDED Requirements

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
