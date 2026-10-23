## ADDED Requirements

### Requirement: Reached query schema order directions
The runtime SHALL resolve НаправлениеПорядкаСхемыЗапроса / QuerySchemaOrderDirection with the reached members ПоВозрастанию / Ascending and ПоУбыванию / Descending. Each language alias SHALL name the same corresponding value. Type lookup, presentation and member inequality SHALL match the retained native1C8.3.27.2342 oracle.

#### Scenario: Ascending direction in original BUH
- **WHEN** the ContactPersons manager module names ПоВозрастанию
- **THEN** name resolution accepts the member and its English alias is Ascending

#### Scenario: Descending direction
- **WHEN** code reads ПоУбыванию or Descending
- **THEN** it receives the same descending value, distinct from Ascending
