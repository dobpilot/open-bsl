## ADDED Requirements

### Requirement: Reached spreadsheet document pattern value
The runtime SHALL resolve ТипУзораТабличногоДокумента / SpreadsheetDocumentPatternType and its БезУзора / WithoutPattern member without regard to case. The aliases SHALL identify the same value. Value presentation SHALL be No pattern and its type SHALL resolve through both type aliases with presentation SpreadsheetDocumentPatternType. The enum namespace and its metatype SHALL have presentation EnumSpreadsheetDocumentPatternType.

#### Scenario: Original report pattern reference
- **WHEN** original report source references ТипУзораТабличногоДокумента.БезУзора
- **THEN** compilation accepts the member through the existing enum resolver

#### Scenario: Aliases and presentation
- **WHEN** code reads either language alias with mixed case
- **THEN** it obtains the same enum value with presentation No pattern
- **AND** type lookup through either language identifies its native type
