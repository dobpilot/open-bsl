# internal-value-resolution Specification

## Purpose
Specify measured configuration data access, typed result materialization and explicit unsupported/error boundaries.

## Requirements

### Requirement: Opt-in host scalar object resolution
The internal-value decoder SHALL allow an explicitly supplied host resolver to materialize scalar object representations through the existing parser. Existing entry points without a resolver SHALL preserve their prior opaque transit behavior. Runtime and VM SHALL remain independent of database and configuration providers.

#### Scenario: Standalone transit
- **WHEN** a standalone consumer decodes an unknown scalar object without a resolver
- **THEN** the previous opaque representation and round-trip behavior remain unchanged

#### Scenario: Resolved nested object
- **WHEN** an explicitly supplied resolver recognizes a scalar object inside a collection
- **THEN** the collection contains the resolved value and resolver errors propagate

### Requirement: Reached configured storage reference identity
The configured storage acquisition path SHALL restore the reached catalog reference using the acquired platform type identity and record UUID, and SHALL restore measured UUID values. Existing storage snapshot isolation SHALL remain intact. Unknown or malformed representations SHALL fail explicitly without successful empty replacements.

#### Scenario: Original extension version storage
- **WHEN** storage contains the acquired ВерсииРасширений reference type and a supported UUID representation
- **THEN** Get returns real typed reference and UUID values with their original identities

#### Scenario: Unknown configured type
- **WHEN** no acquired owner resolves the serialized reference type
- **THEN** acquisition fails explicitly without fabricating a reference
