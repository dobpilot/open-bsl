## MODIFIED Requirements

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
