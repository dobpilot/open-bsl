# type-description-qualifiers Specification

## Purpose
Preserve and expose native-measured primitive type qualifiers through existing runtime objects and column type descriptions.

## Requirements

### Requirement: Qualified column type properties
A qualified type description returned by an accumulation balance result column
SHALL expose its measured string length or number precision and fraction through
the existing runtime qualifier objects. Reading the properties SHALL preserve
the description and its conversion behavior.

#### Scenario: String balance dimension
- **WHEN** the control balance dimension has string length 12
- **THEN** reading `ТипЗначения.КвалификаторыСтроки.Длина` SHALL return 12

#### Scenario: Number balance resource
- **WHEN** the control balance resource has resulting precision 38 and fraction 2
- **THEN** its number qualifier properties SHALL return 38 and 2 respectively

### Requirement: Single primitive description presentation
Formatting a type description containing only String or only Number SHALL
return the corresponding primitive type presentation in the existing runtime
presentation language, without including qualifiers.

#### Scenario: Russian primitive presentation
- **WHEN** a single string or number type description is formatted by the current Russian runtime
- **THEN** the result SHALL be `Строка` or `Число` respectively

### Requirement: Primitive constructor qualifier retention
The existing type-description constructor SHALL retain measured string and
number qualifiers supplied in their native argument positions, using the
existing qualified type-description representation.

#### Scenario: Explicit string and number qualifiers
- **WHEN** a string description is constructed with string length 12 or a number description with precision 38 and fraction 2
- **THEN** reading its qualifier properties SHALL return those values
- **AND** assigning the qualifier property SHALL fail without changing the description

#### Scenario: Fixed string length
- **WHEN** a string description has fixed length 5
- **THEN** its qualifier property SHALL retain the fixed policy
- **AND** adjusting `ab` SHALL return `ab` followed by three spaces
- **AND** adjusting `abcdef` SHALL return `abcde`
- **AND** its default string SHALL contain five spaces
