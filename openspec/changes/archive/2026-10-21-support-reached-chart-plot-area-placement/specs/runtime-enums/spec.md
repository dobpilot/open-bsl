## ADDED Requirements

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
