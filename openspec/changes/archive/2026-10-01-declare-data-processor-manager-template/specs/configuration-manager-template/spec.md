## ADDED Requirements

### Requirement: Data processor manager template method declaration
Data processor managers SHALL declare ПолучитьМакет / GetTemplate with exactly one required argument through their existing method context. The unchanged manager module SHALL resolve the bare call without accepting arbitrary unresolved functions or affecting other manager kinds.

#### Scenario: Original manager module call
- **WHEN** a data processor manager module contains ПолучитьМакет("МакетОтчета")
- **THEN** the original source links through its declared receiver method
- **AND** the same method recognizes its English alias and rejects incorrect argument counts

### Requirement: Missing template implementation is explicit
Until real template retrieval is available, executing the declared method SHALL raise an explicit unavailable-template error rather than return a fabricated template.

#### Scenario: Actual template retrieval
- **WHEN** code executes the declared method without a real resource implementation
- **THEN** it fails explicitly
- **AND** no empty document or successful substitute result is returned
