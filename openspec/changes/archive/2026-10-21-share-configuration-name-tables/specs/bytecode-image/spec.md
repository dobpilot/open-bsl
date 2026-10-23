## ADDED Requirements

### Requirement: Shared immutable configuration name storage
The public Rust Program.names field SHALL expose immutable shared string-array storage. A single compiled configuration and its optional entry SHALL use one allocation for their common final name table, preserving every string, table order and NameId. Standalone and dynamic compilation SHALL retain their existing name-prefix and execution behavior. No new interpreter dependency SHALL be introduced.

#### Scenario: Compile a multi-module catalog
- **WHEN** a catalog containing distinct field names in multiple modules is compiled
- **THEN** all module tables have the same contents and share the same string-array allocation
- **AND** their name identifiers retain the pre-change meanings

#### Scenario: Append names for a dynamic entry
- **GIVEN** a catalog with a frozen shared name table
- **WHEN** an entry or dynamic snippet introduces a new name
- **THEN** it preserves the catalog prefix without mutating any existing module table

### Requirement: Preserve textual names while sharing decoded storage
The textual image SHALL retain its current format version and name-table contents. Configuration image reading SHALL share equal module tables without accepting, overwriting or hiding table mismatches that existing verification rejects.

#### Scenario: Decode a valid shared-name configuration
- **WHEN** a printed configuration with equal module name tables is read back
- **THEN** equal tables share storage and reprinting preserves the same serialized names

#### Scenario: Decode inconsistent name tables
- **WHEN** an image has incompatible module name tables
- **THEN** the existing verification failure remains observable rather than being hidden by replacing a table
