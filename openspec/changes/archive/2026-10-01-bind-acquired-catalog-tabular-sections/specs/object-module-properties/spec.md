## ADDED Requirements

### Requirement: Acquired catalog tabular section receiver names
The configuration host SHALL retain authoritative owned catalog tabular section names and include them in the object's finite receiver property context. Nested row attributes SHALL NOT become direct object receiver properties. Unknown acquisition SHALL remain distinct from a known empty list.

#### Scenario: Native section declaration
- **WHEN** a catalog contains an acquired section Состав with a nested Пользователь attribute
- **THEN** bare Состав resolves through the catalog object receiver
- **AND** Пользователь does not become a direct receiver property
- **AND** existing parameter, local and module variable priority remains unchanged

### Requirement: Unavailable section values fail explicitly
A declared catalog section lacking its typed runtime implementation SHALL raise an explicit named error on access rather than fabricate an empty collection or accept an untyped assignment.

#### Scenario: Declared section accessed before implementation
- **WHEN** code reads or writes a declared section whose runtime value is unavailable
- **THEN** execution fails explicitly naming that section
- **AND** no substitute section value or database write is produced
