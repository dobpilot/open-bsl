## ADDED Requirements

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
A string literal in the first argument SHALL retain the existing compile-time shaped representation, argument-count validation, field behavior, and bytecode instruction.

#### Scenario: Existing literal constructor
- **WHEN** source uses `Новый Структура("А,Б", 1, 2)`
- **THEN** its semantic and execution path remains unchanged

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
