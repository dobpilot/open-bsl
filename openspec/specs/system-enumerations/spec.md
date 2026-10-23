# system-enumerations Specification

## Purpose
Specify measured system enumeration values used by original configuration code, including their aliases and type identities.

## Requirements

### Requirement: Reached spreadsheet detail use values
The runtime SHALL resolve ИспользованиеРасшифровкиТабличногоДокумента / SpreadsheetDocumentDetailUse with БезОбработки / WithoutProcessing and Ячейка / Cell. The aliases SHALL denote the same corresponding values. Type lookup and presentations SHALL match the retained native1C8.3.27.2342 oracle.

#### Scenario: Without processing in original BUH
- **WHEN** the PaymentCalendar manager module references БезОбработки
- **THEN** compilation accepts the member and its presentation is No processing

#### Scenario: Cell detail
- **WHEN** either language alias references Cell
- **THEN** it denotes the cell value, distinct from WithoutProcessing, with presentation Cell

### Requirement: Reached query schema order directions
The runtime SHALL resolve НаправлениеПорядкаСхемыЗапроса / QuerySchemaOrderDirection with the reached members ПоВозрастанию / Ascending and ПоУбыванию / Descending. Each language alias SHALL name the same corresponding value. Type lookup, presentation and member inequality SHALL match the retained native1C8.3.27.2342 oracle.

#### Scenario: Ascending direction in original BUH
- **WHEN** the ContactPersons manager module names ПоВозрастанию
- **THEN** name resolution accepts the member and its English alias is Ascending

#### Scenario: Descending direction
- **WHEN** code reads ПоУбыванию or Descending
- **THEN** it receives the same descending value, distinct from Ascending
