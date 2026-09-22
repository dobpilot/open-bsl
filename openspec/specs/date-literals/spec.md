# date-literals Specification

## Purpose
Define the accepted source forms, normalization, and validation of BSL date literals.

## Requirements

### Requirement: Date literal may omit seconds
The language SHALL accept date literals containing year, month, day, hour, and minute without a seconds field. It SHALL assign zero to the omitted seconds while preserving the supplied hour and minute.

#### Scenario: Minute-precision literal
- **WHEN** source contains the literal `'202401151230'`
- **THEN** it evaluates to 15 January 2024 at 12:30:00

### Requirement: Date literal separators are ignored
The language SHALL ignore every non-digit character between the quotes of a date literal. After removing them, it SHALL accept exactly 8, 12, or 14 ASCII digits and SHALL reject every other digit count during lexing.

#### Scenario: Hyphen-separated date
- **WHEN** source contains the literal `'2024-01-15'`
- **THEN** it evaluates to 15 January 2024 at 00:00:00

#### Scenario: Arbitrary non-digit separators
- **WHEN** source contains non-digit characters between the digit fields of an otherwise valid date literal
- **THEN** those characters do not affect the resulting date

#### Scenario: Unsupported digit count
- **WHEN** source contains a date literal with 10 digits after non-digit characters are removed
- **THEN** lexing fails with the existing invalid-date-literal diagnostic

### Requirement: Date validity remains checked
The language SHALL reject a syntactically admitted date literal whose date or time components are outside the supported calendar ranges.

#### Scenario: Invalid minute
- **WHEN** source contains the literal `'202401151260'`
- **THEN** semantic date construction fails with the existing invalid-date-literal diagnostic
