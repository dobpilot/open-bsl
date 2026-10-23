## ADDED Requirements

### Requirement: Query record type enum
The runtime SHALL expose ТипЗаписиЗапроса and QueryRecordType as the same system enum with ДетальнаяЗапись/DetailRecord, ИтогПоГруппировке/GroupTotal, ИтогПоИерархии/TotalByHierarchy and ОбщийИтог/Overall. Member type presentation SHALL be QueryRecordType; the enum and its meta-type SHALL print EnumQueryRecordType. Type lookup SHALL accept both native language names.

#### Scenario: Native values and type lookup
- **WHEN** a program reads any listed member using either language alias
- **THEN** both forms compare equal and TypeOf matches Type with either native language name
- **AND** string conversion returns Detailed record, Total by group, Total by hierarchy or Grand total respectively

#### Scenario: Existing variable priority
- **WHEN** a module or local variable has the same name as the enum
- **THEN** it retains its existing resolution priority
