## MODIFIED Requirements

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
- **WHEN** an XML or EDT loader supplies the complete code snapshot and matching ORM metadata is acquired from an information base
- **THEN** the two inputs remain independently sourced
- **AND** no module from the database code store is mixed into that code snapshot

## REMOVED Requirements

### Requirement: Future source crate placeholders
**Reason**: XML and EDT loading is now specified as implemented file providers
under configuration-file-sources rather than indefinite placeholder crates.
**Migration**: Existing standalone and database-only users keep their current
paths; the new file readers are selected explicitly and remain optional.
