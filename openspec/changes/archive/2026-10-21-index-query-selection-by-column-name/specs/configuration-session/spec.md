## ADDED Requirements

### Requirement: Query selection indexing by column name
Query selections SHALL accept string column names as indices using case-insensitive lookup while preserving existing numeric indexing.

#### Scenario: Named column
- **WHEN** a positioned selection is indexed by its column name in a different case
- **THEN** the value equals the value obtained with the corresponding numeric index

#### Scenario: Missing column
- **WHEN** a positioned selection is indexed by an unknown column name
- **THEN** the operation raises an error
