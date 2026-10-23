# data-composition-data-sets-link-type Specification

## Purpose
Define measured data-composition data-set link kinds, type identities and aliases required by original configuration code.

## Requirements

### Requirement: Data composition data-set link enum
The runtime SHALL expose ТипСвязиНаборовДанныхКомпоновкиДанных/DataCompositionDataSetsLinkType through its existing system-enum mechanism. Values Внешняя/Outer and Внутренняя/Inner SHALL identify the same respective values and present as External and Internal. Member order SHALL follow the native Outer, Inner order. Both language spellings SHALL identify the same value type, presenting as DataCompositionDataSetsLinkType; the bare enum SHALL present as EnumDataCompositionDataSetsLinkType. Existing variable precedence and rejection of primitive ordering SHALL remain unchanged. This declaration SHALL NOT implement link execution or data composition.

#### Scenario: Original outer-link reference
- **WHEN** original BUH code reads ТипСвязиНаборовДанныхКомпоновкиДанных.Внешняя
- **THEN** the resolver accepts the existing enum-member expression path
- **AND** its alias, type and presentation match the native control

#### Scenario: Inner value and type aliases
- **WHEN** code reads either spelling of the inner value or looks up the type in either language
- **THEN** identity and presentation match the native control

#### Scenario: Existing language rules
- **WHEN** a variable shadows the enum name or code attempts primitive ordering
- **THEN** variable precedence is preserved and primitive ordering remains a catchable runtime error
