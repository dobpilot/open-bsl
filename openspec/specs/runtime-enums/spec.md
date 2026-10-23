# runtime-enums Specification

## Purpose
Specify measured runtime enumeration values and type lookup needed by configured BSL modules.

## Requirements

### Requirement: Reached chart plot area placement value
The runtime SHALL resolve РасположениеОбластиПостроенияДиаграммы / ChartPlotAreaPlacement and its УказываетсяРасположение / UseCoordinates member without regard to case. Both aliases SHALL identify the same value with presentation Use coordinates and type ChartPlotAreaPlacement. The namespace and its metatype SHALL have presentation EnumChartPlotAreaPlacement.

#### Scenario: Original manager reference
- **WHEN** original report manager source references РасположениеОбластиПостроенияДиаграммы.УказываетсяРасположение
- **THEN** compilation accepts it through the existing enum resolver

#### Scenario: Aliases and type
- **WHEN** code reads either language alias with mixed case
- **THEN** it obtains the same value with the measured presentation
- **AND** type lookup through either alias identifies the same native type
- **AND** the namespace and its metatype retain the measured presentation

### Requirement: Reached spreadsheet document pattern value
The runtime SHALL resolve ТипУзораТабличногоДокумента / SpreadsheetDocumentPatternType and its БезУзора / WithoutPattern member without regard to case. The aliases SHALL identify the same value. Value presentation SHALL be No pattern and its type SHALL resolve through both type aliases with presentation SpreadsheetDocumentPatternType. The enum namespace and its metatype SHALL have presentation EnumSpreadsheetDocumentPatternType.

#### Scenario: Original report pattern reference
- **WHEN** original report source references ТипУзораТабличногоДокумента.БезУзора
- **THEN** compilation accepts the member through the existing enum resolver

#### Scenario: Aliases and presentation
- **WHEN** code reads either language alias with mixed case
- **THEN** it obtains the same enum value with presentation No pattern
- **AND** type lookup through either language identifies its native type
