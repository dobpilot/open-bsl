## ADDED Requirements

### Requirement: Acquired catalog predefined manager properties
An acquired catalog manager SHALL expose read-only properties named by its declared predefined items. The owning manager module SHALL resolve the same properties as bare context names through the existing engine, preserving local and module variable precedence. Unknown names SHALL retain existing diagnostics.

#### Scenario: Original manager reads its predefined item
- **WHEN** an original catalog manager reads an acquired predefined item by its symbolic name
- **THEN** bare and qualified access resolve the same owning catalog reference
- **AND** reading the property does not execute a rewritten version of the original source
- **AND** PredefinedValue of the same catalog path resolves the same session reference

#### Scenario: Ordinary variable takes precedence
- **WHEN** a local or module variable has the same name as a predefined manager property
- **THEN** the existing variable resolution priority is preserved

### Requirement: Predefined references use session data
Predefined property resolution SHALL use the actual record reference of the current information base through its session data adapter. A configuration predefined identity SHALL NOT be fabricated into a record UUID. The implementation SHALL preserve applicable existing rights and data-separation checks and SHALL NOT share live session connections through global or thread-local state.

#### Scenario: Resolve an existing predefined record
- **WHEN** the current session reads a predefined manager property backed by an existing record
- **THEN** its UUID and owning reference type match native 1C and the actual database record

#### Scenario: Session isolation
- **WHEN** different sessions resolve the same predefined symbolic name
- **THEN** each resolution uses that session's data adapter and information base

#### Scenario: Predefined data has not been created
- **WHEN** the declared predefined item has no record in the current information base
- **THEN** reading the manager property fails explicitly, matching the native probe
- **AND** no empty reference, metadata UUID or Undefined is substituted
