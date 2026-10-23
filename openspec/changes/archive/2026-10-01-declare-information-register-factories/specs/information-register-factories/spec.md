## ADDED Requirements

### Requirement: Finite information register factory declarations
Information register managers SHALL declare `СоздатьМенеджерЗаписи` / `CreateRecordManager` and `СоздатьНаборЗаписей` / `CreateRecordSet` with zero arguments. Their registered manager module receiver context SHALL expose the same methods. These declarations SHALL NOT add either method to other metadata manager kinds.

#### Scenario: Dormant original procedure
- **WHEN** an information register manager module contains an unqualified call to either factory
- **THEN** that call resolves through its finite receiver context
- **AND** argument counts other than zero are rejected

#### Scenario: Explicit unavailable implementation
- **WHEN** either declared factory executes before a real record implementation is available
- **THEN** it fails with an error naming the unavailable factory
- **AND** it does not return a fabricated record or change database data

#### Scenario: Other manager kinds
- **WHEN** a catalog manager is inspected for these static methods
- **THEN** neither factory belongs to its platform method context
