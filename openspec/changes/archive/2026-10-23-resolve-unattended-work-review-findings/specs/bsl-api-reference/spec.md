## ADDED Requirements

### Requirement: Declared-only names are marked
A global function, constructor or component type that is registered only so that configuration code resolves and passes arity checks, with no implementation behind it, SHALL be flagged as declared-only in its descriptor. Calling or constructing it SHALL raise one uniform runtime error that names it. The generated API reference SHALL mark every declared-only entry so that it cannot be read as supported.

#### Scenario: Calling a declared-only function
- **WHEN** a script calls a declared-only global function
- **THEN** compilation succeeds
- **AND** the call raises the uniform declared-only error naming the function

#### Scenario: Reference generation
- **WHEN** the API reference is generated
- **THEN** every declared-only function, constructor and type carries the declared-only mark
- **AND** implemented entries carry no such mark
