## ADDED Requirements

### Requirement: Missing enumeration reference identity
A valid enumeration reference SHALL retain its type and UUID even when the current configuration value list does not contain that UUID. Such a nonempty reference SHALL remain filled, compare by type and identity, and survive value-graph capture and restoration. Named enumeration value lookup SHALL continue to use the declared list; accepting a reference SHALL NOT manufacture a named metadata value.

#### Scenario: Acquired absent filling value
- **WHEN** acquired document metadata contains a valid enumeration filling reference absent from the declared enumeration value list
- **THEN** acquisition SHALL preserve that reference instead of rejecting the catalog
- **AND** document filling SHALL transfer it as a nonempty reference of the same type

#### Scenario: Graph round trip
- **WHEN** a missing enumeration reference is captured and restored against the receiving catalog
- **THEN** its type, UUID, equality and filled state SHALL remain unchanged

#### Scenario: Strict malformed identity
- **WHEN** a payload has a malformed UUID or its owner is not an enumeration
- **THEN** materialization SHALL fail

#### Scenario: No fabricated named value
- **WHEN** code looks up an undeclared named enumeration member
- **THEN** it SHALL retain the existing missing-member failure

### Requirement: Missing enumeration reference presentation
A missing enumeration reference SHALL be presented with its acquired DBNames number and physical UUID bytes. The missing-object label SHALL follow the current session UI language. Rendering SHALL NOT mutate a shared configuration catalog or cache another session's language. Unknown presentation identity SHALL NOT be replaced with a fabricated database number.

#### Scenario: Native storage identity
- **WHEN** the native probe references an absent value of the enumeration stored in _Enum54
- **THEN** its presentation SHALL contain the measured `(54:84093272f97870604114bca67664d5d3)` identity

#### Scenario: Session language isolation
- **WHEN** the same acquired reference is presented by Russian and English UI sessions
- **THEN** their labels SHALL be Объект не найден and Object not found respectively
- **AND** formatting one session SHALL NOT change the other session's result

### Requirement: Contextual component presentation
The runtime SHALL provide an additive component presentation hook receiving the current call context, with a default preserving existing component presentation. String, Format, Message and component call-context formatting of extension values SHALL use the current presentation context. Diagnostic display SHALL remain separate. The facade SHALL expose the hook's required types without requiring implementation-crate imports.

#### Scenario: Existing component
- **WHEN** a component does not override contextual presentation
- **THEN** user-visible formatting SHALL retain its previous presentation result

#### Scenario: Missing presentation context
- **WHEN** a missing enumeration reference lacks an acquired storage number or current UI language
- **THEN** user-visible formatting SHALL fail explicitly
- **AND** capture, restoration and equality SHALL remain available without that formatting context

#### Scenario: Current state context
- **WHEN** two States format the same immutable acquired reference with different UI languages
- **THEN** each call SHALL use the invoking State's language
- **AND** the shared materializer SHALL NOT retain either State

#### Scenario: Facade UUID validation
- **WHEN** a configuration component validates a reference UUID
- **THEN** the facade SHALL expose the existing canonical UUID parser and formatter without an implementation-crate dependency
