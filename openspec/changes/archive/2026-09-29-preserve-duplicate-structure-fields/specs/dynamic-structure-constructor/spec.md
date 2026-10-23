## MODIFIED Requirements

### Requirement: Literal Structure compatibility
A string literal in the first argument with distinct folded field names SHALL
retain the existing compile-time shaped representation, argument-count
validation, field behavior, and bytecode instruction. A literal with repeated
folded field names SHALL use the runtime constructor while preserving source
argument evaluation and the platform's duplicate-field behavior.

#### Scenario: Existing literal constructor
- **WHEN** source uses `Новый Структура("А,Б", 1, 2)`
- **THEN** its semantic and execution path remains unchanged

#### Scenario: Duplicate literal constructor
- **WHEN** source uses `Новый Структура("А,а", 1, 2)`
- **THEN** compilation succeeds without a duplicate-name bytecode shape
- **AND** the resulting Structure has one field with value `2`

## ADDED Requirements

### Requirement: Duplicate Structure field spelling
Structure field names SHALL compare without case. When a constructor or
`Вставить` / `Insert` repeats a field, the Structure SHALL retain its first
position, replace its value and expose the spelling of the last occurrence
when the field name is returned as a string. This spelling SHALL belong to
the individual Structure and SHALL NOT change another Structure or the
shared name table.

#### Scenario: Repeated constructor field
- **WHEN** source constructs `Новый Структура("А,Б,а", 1, 2, 3)`
- **THEN** it contains two fields with values `А = 3` and `Б = 2`
- **AND** iteration returns field names `а`, then `Б`

#### Scenario: Repeated insertion
- **WHEN** a Structure with field `А` receives `Вставить("а", 2)`
- **THEN** it has one field with value `2` and iteration returns the key `а`

#### Scenario: Computed field list
- **WHEN** a computed field list evaluates to `"Поле,поле"` with values `1, 2`
- **THEN** the Structure has one field with value `2` and its key spelling is `поле`

#### Scenario: Source evaluation order
- **WHEN** both value expressions of a duplicate literal record a side effect
- **THEN** both execute in source order and the second value remains in the field

#### Scenario: Separate Structures
- **WHEN** one Structure changes the spelling of `А` to `а` through `Insert`
- **THEN** another Structure with field `А` retains its own key spelling

#### Scenario: Copy and serialization
- **WHEN** a Structure with a replaced field spelling is copied through a FixedStructure or serialized and restored
- **THEN** the restored mutable Structure exposes the last spelling and value
