# system-enums Specification

## Purpose
Specify native system-enumeration identities, members and lookup behavior required by configuration execution.

## Requirements

### Requirement: Query record type enum
The runtime SHALL expose ТипЗаписиЗапроса and QueryRecordType as the same system enum with ДетальнаяЗапись/DetailRecord, ИтогПоГруппировке/GroupTotal, ИтогПоИерархии/TotalByHierarchy and ОбщийИтог/Overall. Member type presentation SHALL be QueryRecordType; the enum and its meta-type SHALL print EnumQueryRecordType. Type lookup SHALL accept both native language names.

#### Scenario: Native values and type lookup
- **WHEN** a program reads any listed member using either language alias
- **THEN** both forms compare equal and TypeOf matches Type with either native language name
- **AND** string conversion returns Detailed record, Total by group, Total by hierarchy or Grand total respectively

#### Scenario: Existing variable priority
- **WHEN** a module or local variable has the same name as the enum
- **THEN** it retains its existing resolution priority

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

### Requirement: Data composition settings item state enum
The runtime SHALL expose СостояниеЭлементаНастройкиКомпоновкиДанных and DataCompositionSettingsItemState as the same system enumeration with Включен/Enabled, Отключен/Disabled and УдаленПользователем/DeletedByUser. Member type presentation SHALL be DataCompositionSettingsItemState; the bare enum and its meta-type SHALL print EnumDataCompositionSettingsItemState.

#### Scenario: Native aliases and presentations
- **WHEN** a program obtains any listed value through either language alias
- **THEN** both forms compare equal and have the declared member type
- **AND** string conversion returns Enabled, Disabled or Deleted by user respectively

#### Scenario: Existing variable priority
- **WHEN** a local or module variable uses the enum name
- **THEN** it retains the existing precedence over the system enum
