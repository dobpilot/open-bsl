## ADDED Requirements

### Requirement: Logical compiled query result materialization
The adapter SHALL expose one BSL result column for each logical selected value described by the compiler's complete projection. It SHALL use the compiler's logical names and exact physical indices, preserving independent occurrences of repeated selected fields. Composite values SHALL decode only their active payload through existing value converters. Rights and mandatory data separation SHALL remain prerequisites to SQL execution.

#### Scenario: Reached Boolean or String constant
- **WHEN** a complete projection describes one Boolean|String selected value
- **THEN** the result contains one logical column with the active Boolean or String value rather than physical filler or TYPE columns

#### Scenario: Two aliases of one field
- **WHEN** the same composite field is selected twice with different aliases
- **THEN** the result retains two independent named logical positions

#### Scenario: Existing scalar conversion
- **WHEN** a complete projection describes scalar values
- **THEN** their existing type and identity conversion behavior is preserved

### Requirement: Explicit invalid projection failure
The adapter SHALL reject unavailable, incomplete or invalid logical projections and malformed composite rows explicitly. It SHALL NOT infer recipes from physical labels or replace errors with empty successful values.

#### Scenario: Unknown discriminator
- **WHEN** a row has an unsupported discriminator or no unique active alternative
- **THEN** materialization fails explicitly

#### Scenario: Unavailable compiler recipe
- **WHEN** the compiler cannot provide a complete unambiguous logical projection
- **THEN** the adapter rejects the result instead of exposing guessed or partial BSL columns
