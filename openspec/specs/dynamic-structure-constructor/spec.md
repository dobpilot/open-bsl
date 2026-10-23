# dynamic-structure-constructor Specification

## Purpose
Define how Structure constructors handle computed and literal field lists, including repeated field names, while preserving field order and evaluation behavior.

## Requirements

### Requirement: Computed Structure field list
The language SHALL accept `Новый Структура(<FieldListExpression>[, <Values>...])` and `New Structure(<FieldListExpression>[, <Values>...])` when the first argument is not a string literal. It SHALL evaluate all constructor arguments in source order. A first value other than a fixed structure SHALL be converted with the standard BSL string representation and interpreted as a comma-separated field list whose names are trimmed and whose empty entries are omitted.

#### Scenario: Computed string field list
- **WHEN** the first argument evaluates to `"А, Б"`
- **THEN** the result is a Structure with fields `А` and `Б` in that order

#### Scenario: English syntax alias
- **WHEN** the source uses `New Structure` with a computed field list
- **THEN** it has the same behavior as the Russian syntax

### Requirement: Runtime Structure values
Values after the computed field list SHALL be assigned to fields in order. A field without a corresponding value SHALL receive `Неопределено`. Values beyond the field count SHALL be evaluated and ignored.

#### Scenario: Missing value
- **WHEN** two fields and one value are supplied
- **THEN** the first field receives that value and the second receives `Неопределено`

#### Scenario: Excess value
- **WHEN** two fields and three values are supplied
- **THEN** all three value expressions are evaluated and the Structure contains only the first two values

### Requirement: Fixed Structure copy
When the sole computed first argument evaluates to a `ФиксированнаяСтруктура` / `FixedStructure`, the constructor SHALL create a mutable Structure containing the same fields and values in the same order.

#### Scenario: Copy a fixed structure
- **WHEN** a fixed structure with two fields is passed as the sole argument
- **THEN** the result is an independent mutable Structure with those two fields and values

### Requirement: Literal Structure compatibility
A string literal in the first argument with distinct folded field names SHALL use the existing compile-time shaped representation when it has no following values or exactly one value per field. A literal with repeated folded field names SHALL use the runtime constructor while preserving source argument evaluation and the platform's duplicate-field behavior. When the number of following values differs from the field count, the constructor SHALL preserve the same measured missing-value and excess-value behavior as a computed field list, including left-to-right evaluation and ignoring already evaluated excess values.

#### Scenario: Existing literal constructor
- **WHEN** source uses `Новый Структура("А,Б", 1, 2)`
- **THEN** its semantic and execution path remains the compile-time shaped representation

#### Scenario: Duplicate literal constructor
- **WHEN** source uses `Новый Структура("А,а", 1, 2)`
- **THEN** compilation succeeds without a duplicate-name bytecode shape
- **AND** the resulting Structure has one field with value `2`

#### Scenario: Literal constructor with missing values
- **WHEN** a literal names more fields than the constructor supplies values
- **THEN** supplied values are assigned in order and remaining fields receive `Неопределено`

#### Scenario: Literal constructor with excess values
- **WHEN** a literal names fewer fields than the constructor supplies values
- **THEN** every value expression is evaluated and values beyond the field count are ignored

### Requirement: Bounded runtime storage
Structures created from computed field lists SHALL use dictionary storage directly and SHALL NOT intern their field lists in the execution-wide shape table.

#### Scenario: Many distinct computed lists
- **WHEN** a loop constructs Structures from many distinct runtime strings
- **THEN** the shape table does not retain one shape per constructed list

### Requirement: Versioned bytecode representation
The textual bytecode format SHALL represent the runtime-key Structure constructor without changing the meaning of the existing `NewStructure` instruction. Readers SHALL reject images from incompatible format versions.

#### Scenario: Runtime-key constructor round trip
- **WHEN** a program containing a computed Structure field list is printed and parsed as textual bytecode
- **THEN** its destination, argument register range, and argument count are preserved

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
