## ADDED Requirements

### Requirement: New catalog item deletion mark
A new catalog item SHALL expose ПометкаУдаления / DeletionMark through the existing receiver property context. Its default, object-body availability, assignment conversion and failure behavior SHALL match the preserved native oracle. The property SHALL belong to the concrete object and SHALL NOT implicitly write a database record.

#### Scenario: Original module reads and assigns the property
- **WHEN** an original object module reads or assigns the deletion mark
- **THEN** the existing engine uses that object's measured state and conversion rules
- **AND** Russian and English aliases address the same property

#### Scenario: Independent new objects
- **WHEN** one new object's deletion mark changes
- **THEN** another new object retains its measured initial value
- **AND** ordinary local and module variable priority remains unchanged

### Requirement: Reusable Boolean conversion through the facade
The public open-bsl facade SHALL expose convert_to_boolean taking a borrowed Value and returning RtResult<bool>. It SHALL preserve the existing Boolean builtin conversion and errors. The deletion-mark setter SHALL default conversion errors to false without changing the builtin or table-column behavior.

#### Scenario: Existing builtin behavior remains unchanged
- **WHEN** a host calls the helper with a recognized Boolean value, number or string
- **THEN** it receives the same Boolean result as the existing Boolean builtin
- **AND** unsupported inputs return the existing conversion error

#### Scenario: Consumer uses only the facade
- **WHEN** an external consumer invokes the conversion helper
- **THEN** it needs no internal runtime crate name
