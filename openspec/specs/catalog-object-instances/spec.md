# catalog-object-instances Specification

## Purpose
Specify catalog object creation and concrete object-module execution through the
existing runtime component protocol and VM, without an implicit database write.

## Requirements

### Requirement: Reusable value constraint adjustment
The facade SHALL expose the runtime's existing primitive type constraint
adjustment for component reuse. Existing value-table adjustment behavior SHALL
remain unchanged.

#### Scenario: Shared primitive qualifiers
- **WHEN** a component adjusts a value to a declared primitive type constraint
- **THEN** the existing conversion, rounding and length qualifiers apply
- **AND** the same constraint on a value-table column retains its previous result

### Requirement: Acquired catalog object settings
The immutable ORM catalog SHALL retain available standard-attribute settings and
object-module bindings from the configuration snapshot. Unknown settings SHALL
remain distinguishable from measured settings.

#### Scenario: Numeric catalog code
- **WHEN** the acquired catalog specifies numeric code and zero description length
- **THEN** those settings reach the ORM without being replaced by string defaults

#### Scenario: Unknown descriptor shape
- **WHEN** the source cannot provide the required settings
- **THEN** object creation fails explicitly rather than manufacturing defaults

### Requirement: New catalog object
CreateItem SHALL take no arguments and construct a new object of the exact
configuration catalog object type without writing a database row. Its reference
SHALL be empty, its metadata SHALL identify the owning catalog, and standard
attributes SHALL follow the acquired settings and measured native behavior.

#### Scenario: New string-code object
- **WHEN** CreateItem is called for a supported string-code catalog
- **THEN** the object is new and has an empty reference, code and description
- **AND** its type is the owning CatalogObject type

#### Scenario: Numeric-code object without writable description
- **WHEN** CreateItem is called for the measured numeric-code catalog with zero description length
- **THEN** its code is numeric zero and its description reads as an empty string
- **AND** assigning its description fails

### Requirement: Concrete object module instance
Every catalog object SHALL retain its own object-module variables and receiver
identity. Object-module methods SHALL execute through the existing BSL VM, with
common modules shared only within the current State.

#### Scenario: Body initialization during creation
- **WHEN** CreateItem returns a supported object whose object module has a body
- **THEN** that body has already executed for the new object
- **AND** its subsequent export calls do not execute the body again

#### Scenario: Two objects of one catalog
- **WHEN** the same counter method is called on first, second and first objects
- **THEN** it returns 1, 1 and 2 respectively
- **AND** ThisObject identifies the corresponding receiver

#### Scenario: Separate States
- **WHEN** separate States create objects from the same compiled configuration
- **THEN** neither object's module variables expose the other State's values

#### Scenario: Mutable object capture
- **WHEN** an immutable value graph receives a mutable catalog object
- **THEN** capture fails rather than transferring its live module state or connection

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
