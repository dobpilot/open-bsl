# account-type-enumeration Specification

## Purpose
Define the measured account-type enumeration used by original configuration
code while preserving existing name resolution and diagnostics.

## Requirements

### Requirement: Complete AccountType enumeration
The runtime SHALL expose ВидСчета / AccountType with Активный / Active, Пассивный / Passive and АктивноПассивный / ActivePassive through existing case-insensitive system enumeration resolution. Values and their type presentation SHALL match preserved native platform measurements.

#### Scenario: Original chart manager source
- **WHEN** unchanged source reads each documented member using Russian or English aliases
- **THEN** both aliases refer to the same corresponding enum value
- **AND** its type and presentation match the native oracle

### Requirement: Existing resolution and diagnostics remain unchanged
Adding AccountType SHALL preserve existing local and module variable priority and unknown-member failure behavior.

#### Scenario: Variable priority and unknown member
- **WHEN** a variable shadows the enumeration name or code requests an unknown member
- **THEN** variable resolution keeps its existing priority
- **AND** the unknown member follows the existing diagnostic path
