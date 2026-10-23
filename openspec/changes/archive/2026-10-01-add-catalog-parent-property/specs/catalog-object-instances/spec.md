## ADDED Requirements

### Requirement: Measured catalog object Parent state
A new catalog object SHALL expose `Родитель` / `Parent` as its own catalog reference, initially empty. Reads SHALL be available during object module initialization. Writes SHALL fail before selection. After selection, a nonhierarchical catalog SHALL reject writes as readonly; an unknown acquired hierarchy setting SHALL cause an explicit error rather than an assumed policy. A hierarchical catalog SHALL preserve accepted references of the owning catalog and convert measured incompatible input values to its own empty reference.

#### Scenario: Initialization and hierarchy distinction
- **WHEN** otherwise equivalent hierarchical and nonhierarchical fresh objects initialize
- **THEN** both expose an own empty Parent reference
- **AND** both reject a Parent write before selection
- **AND** after selection only the hierarchical object accepts a Parent write

#### Scenario: Unknown hierarchy setting
- **WHEN** a selected object lacks an acquired hierarchy setting
- **AND** code writes its Parent property
- **THEN** execution fails explicitly without changing the previous value

#### Scenario: Own typed reference
- **WHEN** code assigns an accepted filled reference of the same catalog to Parent
- **THEN** reading Parent returns that typed reference and UUID
- **AND** the other concrete object's Parent value does not change

### Requirement: Catalog reference emptiness method
Catalog references SHALL expose zero-argument `Пустая` / `IsEmpty` and return a boolean matching the native empty-reference contract.

#### Scenario: Empty and filled references
- **WHEN** the method is called on an empty and on a filled catalog reference
- **THEN** it returns true for the empty reference and false for the filled reference
- **AND** the reference UUID and type remain unchanged
