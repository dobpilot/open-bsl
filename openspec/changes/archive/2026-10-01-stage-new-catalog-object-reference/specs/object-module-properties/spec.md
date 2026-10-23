## ADDED Requirements

### Requirement: New catalog object staged reference
A catalog object SHALL expose ПолучитьСсылкуНового / GetNewObjectRef with zero arguments and УстановитьСсылкуНового / SetNewObjectRef with one argument. These methods SHALL use the existing owning object receiver and preserve the measured native default, validation and preselection behavior. Staging SHALL NOT write a database record or manufacture another object's reference type.

#### Scenario: Original object module stages a reference
- **WHEN** an original catalog object module stages an own reference and reads it again
- **THEN** getter results retain its owning reference type and UUID according to the preserved native oracle
- **AND** the source runs through existing finite receiver contexts without preprocessing

#### Scenario: Native argument and selection rules
- **WHEN** a caller passes an invalid value or accesses the methods during object-module initialization
- **THEN** behavior matches the preserved native measurement
- **AND** invalid arity is rejected through the existing method-table contract

### Requirement: Staged catalog references belong to their object
Staged-reference state SHALL belong to the explicit catalog-object instance. Staging SHALL remain separate from record persistence and SHALL NOT be copied into unrelated new objects or sessions by shared mutable state.

#### Scenario: Two new objects
- **WHEN** one new object stages an own reference
- **THEN** another new object retains its measured initial staged-reference state
- **AND** no database connection or session rights are transferred with the reference
