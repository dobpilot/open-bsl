## ADDED Requirements

### Requirement: Enumeration constant identity
The database adapter SHALL materialize supported enumeration constants using the declared enumeration reference type and existing enumeration member identity.

#### Scenario: Known member
- **WHEN** a constant contains a known enumeration member
- **THEN** its value equals that member obtained from the enumeration manager
- **AND** its type is the declared enumeration reference type

#### Scenario: Empty enumeration constant
- **WHEN** a constant contains the nil enumeration reference
- **THEN** its value equals the enumeration manager's empty reference
