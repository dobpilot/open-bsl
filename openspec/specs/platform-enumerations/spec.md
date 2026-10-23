# platform-enumerations Specification

## Purpose
Define measured platform enumeration identities and presentations used by chart-related configuration source without implying rendering support.

## Requirements

### Requirement: Chart legend visibility enumeration
The runtime SHALL expose ОтображениеВЛегендеДиаграммы and ShowInChartLegend with equivalent case-insensitive member aliases Авто/Auto, Отображать/Show and НеОтображать/DontShow.

#### Scenario: Native values and presentations
- **WHEN** code evaluates the three members
- **THEN** their presentations are `Auto`, `Show` and `Do not show` respectively
- **AND** they are distinct values of type ShowInChartLegend, available under either language's type name

#### Scenario: Primitive ordering
- **WHEN** code applies primitive less-than to Auto and Show
- **THEN** it reports the existing unsupported comparison error
