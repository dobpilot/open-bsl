## ADDED Requirements

### Requirement: Owned finite receiver property names
The facade SHALL accept a finite receiver property-name context owned by the engine for a configuration module. Names SHALL use existing case-insensitive comparison and variable priority. Dynamic code and reconstructed background workers SHALL retain this context. Existing descriptor-table registration SHALL remain supported.

#### Scenario: Configuration-owned name
- **WHEN** the host declares an acquired catalog attribute name for its object module
- **THEN** an unqualified read or assignment resolves through the current object receiver
- **AND** a local, parameter or module variable with the same name retains priority

#### Scenario: Unknown name outside the context
- **WHEN** an identifier is absent from the module's finite property context and existing name scopes
- **THEN** the existing undefined-variable diagnostic remains unchanged

### Requirement: Acquired attribute declarations do not fabricate values
The ORM SHALL retain catalog attribute names from authoritative acquired metadata without including unrelated nested fields. An attribute declared for compilation but lacking its typed runtime implementation SHALL fail explicitly on reads and writes, naming that attribute.

#### Scenario: Declared but unavailable attribute
- **WHEN** original object code reads or writes a declared attribute without a real value implementation
- **THEN** execution raises an explicit unavailable-attribute error
- **AND** no Undefined default or successful untyped assignment is fabricated
