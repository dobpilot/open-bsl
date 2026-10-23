## ADDED Requirements

### Requirement: Data composition settings item state enum
The runtime SHALL expose СостояниеЭлементаНастройкиКомпоновкиДанных and DataCompositionSettingsItemState as the same system enumeration with Включен/Enabled, Отключен/Disabled and УдаленПользователем/DeletedByUser. Member type presentation SHALL be DataCompositionSettingsItemState; the bare enum and its meta-type SHALL print EnumDataCompositionSettingsItemState.

#### Scenario: Native aliases and presentations
- **WHEN** a program obtains any listed value through either language alias
- **THEN** both forms compare equal and have the declared member type
- **AND** string conversion returns Enabled, Disabled or Deleted by user respectively

#### Scenario: Existing variable priority
- **WHEN** a local or module variable uses the enum name
- **THEN** it retains the existing precedence over the system enum
