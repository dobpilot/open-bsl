# number-conversion Specification

## Purpose
Define native-measured numeric conversion inputs and catchable failures while preserving existing numeric string parsing.

## Requirements

### Requirement: Native numeric conversion inputs
The `Число` builtin SHALL accept measured numbers, Boolean values and
numeric strings and SHALL return a numeric value.

#### Scenario: Numeric and Boolean inputs
- **WHEN** a number or Boolean value is converted
- **THEN** a number retains its value and a Boolean becomes one or zero
- **AND** the result has the numeric type

#### Scenario: Unsupported inputs
- **WHEN** an invalid numeric string, Undefined or Null is converted
- **THEN** execution raises a catchable error

#### Scenario: Existing numeric strings
- **WHEN** a valid numeric string is converted
- **THEN** existing string parsing behavior remains unchanged
