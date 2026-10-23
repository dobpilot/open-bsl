## ADDED Requirements

### Requirement: Reached spreadsheet detail use values
The runtime SHALL resolve ИспользованиеРасшифровкиТабличногоДокумента / SpreadsheetDocumentDetailUse with БезОбработки / WithoutProcessing and Ячейка / Cell. The aliases SHALL denote the same corresponding values. Type lookup and presentations SHALL match the retained native1C8.3.27.2342 oracle.

#### Scenario: Without processing in original BUH
- **WHEN** the PaymentCalendar manager module references БезОбработки
- **THEN** compilation accepts the member and its presentation is No processing

#### Scenario: Cell detail
- **WHEN** either language alias references Cell
- **THEN** it denotes the cell value, distinct from WithoutProcessing, with presentation Cell
