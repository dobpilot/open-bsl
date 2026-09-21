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

### Requirement: Future source crate placeholders
The workspace SHALL contain `bsl-config-xml` and `bsl-config-edt` as documented placeholders. Until their respective implementations are specified, they SHALL expose no loading operation and add no CLI option or dependency.

#### Scenario: Placeholder build
- **WHEN** the workspace is built
- **THEN** both placeholder crates compile
- **AND** neither crate claims to load a configuration
