# structure-index-assignment Specification

## Purpose
Specify native-compatible writes to existing mutable Structure fields through string indices and property output arguments.

## Requirements

### Requirement: Mutable structure field assignment by string index
A mutable Structure SHALL allow assignment to an existing field by a case-insensitive string index, preserving its field count and identity. Writes to absent fields, numeric indices and immutable FixedStructure values SHALL fail. The existing Structure.Property output argument SHALL update the same existing field when passed by string index.

#### Scenario: Dynamic field update
- **WHEN** a structure with field Поле is assigned through index пОЛЕ
- **THEN** Поле contains the assigned value and the field count is unchanged

#### Scenario: Invalid mutation
- **WHEN** code assigns an absent field, a numeric index or an immutable structure field
- **THEN** execution raises an error without inserting or modifying a field

#### Scenario: Output argument
- **WHEN** an existing structure field is passed by string index as the output argument of Structure.Property
- **THEN** the caller sees the changed field value
