## ADDED Requirements

### Requirement: Data composition group placement enum
The runtime SHALL expose РасположениеГруппировкиКомпоновкиДанных and DataCompositionGroupPlacement as the same system enumeration with Конец/End, Начало/Begin, НачалоИКонец/BeginAndEnd and Нет/None. Its type presentation SHALL be DataCompositionGroupPlacement. Value presentations SHALL match the native platform. The bare enumeration and its meta-type SHALL print EnumDataCompositionGroupPlacement.

#### Scenario: Native aliases and presentations
- **WHEN** a program obtains any of the four values through either language alias
- **THEN** both forms compare equal and TypeOf resolves to the same enum type
- **AND** string conversion returns End, Begin, Begin and end, or None respectively

#### Scenario: Existing resolution precedence
- **WHEN** a local or module variable has the same name as this enum
- **THEN** the variable retains its existing name-resolution precedence


### Requirement: Data composition text placement enum
The runtime SHALL expose ТипРазмещенияТекстаКомпоновкиДанных and DataCompositionTextPlacementType as the same system enumeration with Выступать/Overflow, Забивать/Block, Обрезать/Cut and Переносить/Wrap. Value and type presentations SHALL match the measured native platform. Its identity SHALL remain distinct from spreadsheet text placement.

#### Scenario: Native text placement members
- **WHEN** a program obtains any listed member through either language alias
- **THEN** both forms compare equal and have the DataCompositionTextPlacementType member type
- **AND** the string presentation matches the native control

#### Scenario: No spreadsheet Auto member
- **WHEN** a program requests Auto from DataCompositionTextPlacementType
- **THEN** member lookup fails instead of substituting spreadsheet placement

#### Scenario: Distinct enum identity
- **WHEN** Wrap values from data-composition and spreadsheet text placement are compared
- **THEN** they are unequal and have distinct types


### Requirement: Native localized enum type lookup
Type lookup for the enums declared by this change SHALL accept their measured Russian and English member-type names while preserving the measured English type presentation.

#### Scenario: Both native type names
- **WHEN** Type is called with either native language name of a declared enum member type
- **THEN** it returns the same type as TypeOf for a member of that enum
