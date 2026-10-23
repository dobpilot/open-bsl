# chart-values-by-series-connection-type Specification

## Purpose
Define native chart-series connection members and language aliases required by original report source without promising chart rendering.

## Requirements

### Requirement: Chart values by series connection enum
The runtime SHALL expose ТипСоединенияЗначенийПоСериямДиаграммы/ChartValuesBySeriesConnectionType with native values Нет/None and СоединениеКраев/EdgesConnection. Presentations SHALL be No and Edge joint, respectively, in that member order. Both language spellings SHALL identify the same value type and present as ChartValuesBySeriesConnectionType; the bare enum SHALL present as EnumChartValuesBySeriesConnectionType. Existing variable precedence and rejection of primitive ordering SHALL remain unchanged. This declaration SHALL NOT implement chart mutation or rendering.

#### Scenario: Reached original report references
- **WHEN** original BUH code reads either measured Russian member
- **THEN** both member aliases, types and presentations match the native control

#### Scenario: Type lookup and namespace
- **WHEN** code uses either type spelling or reads the bare enum
- **THEN** the resulting type identities and presentations match the native control

#### Scenario: Existing language rules
- **WHEN** a variable shadows the enum or code attempts primitive ordering
- **THEN** variable precedence is preserved and primitive ordering remains a catchable error
