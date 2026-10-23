## ADDED Requirements

### Requirement: Document manager template method declaration
Document managers SHALL declare ПолучитьМакет / GetTemplate with one required argument through their existing finite method context. Bare original manager calls SHALL link, and the declaration SHALL retain case-insensitive aliases and arity validation. Actual retrieval without a real resource implementation SHALL fail explicitly rather than fabricate a template.

#### Scenario: Original document manager source
- **WHEN** unchanged document manager source calls ПолучитьМакет with a template name
- **THEN** the call resolves through its own declared receiver
- **AND** an actual unsupported retrieval produces an explicit unavailable-template error

#### Scenario: Existing processor behavior
- **WHEN** a data processor manager uses the previously declared method
- **THEN** its aliases, arity and existing diagnostics remain unchanged
