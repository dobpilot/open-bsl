## ADDED Requirements

### Requirement: Data composition chart legend enum
The runtime SHALL expose РасположениеЛегендыДиаграммыКомпоновкиДанных/DataCompositionChartLegendPlacement with five native values in order: Нет/None, Лево/Left, Право/Right, Верх/Top, Низ/Bottom. Their presentations SHALL match their English aliases. Both language spellings SHALL identify the same value type presenting as DataCompositionChartLegendPlacement; the bare enum SHALL present as EnumDataCompositionChartLegendPlacement. This type SHALL remain distinct from ChartLegendPlacement, SHALL NOT acquire its Auto member, and SHALL retain existing variable precedence and rejection of primitive ordering. This declaration SHALL NOT implement chart or data-composition operations.

#### Scenario: Reached original report references
- **WHEN** original BUH code refers to Право, Нет or Низ
- **THEN** the resolver accepts the measured members
- **AND** all five member aliases and presentations match the native control

#### Scenario: Native type distinction
- **WHEN** code compares the value types of this enum and ChartLegendPlacement
- **THEN** the types are different
- **AND** Auto remains undefined for this enum

#### Scenario: Existing language rules
- **WHEN** a variable shadows the enum name or code attempts primitive ordering
- **THEN** variable precedence is preserved and primitive ordering remains a catchable error
