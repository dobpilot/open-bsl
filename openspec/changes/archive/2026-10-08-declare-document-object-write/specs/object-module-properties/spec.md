## ADDED Requirements

### Requirement: Document object Write declaration
The document object receiver SHALL declare Записать / Write with zero through two arguments through the existing receiver context. Previously callable exported methods with either name SHALL retain their behavior. Until persistence is connected, invoking the platform method SHALL fail explicitly without database writes, handler invocation, allocation, or object-state changes.

#### Scenario: Uncalled write branch compiles
- **WHEN** an original document object module contains bare Записать() in an uncalled procedure
- **THEN** compiling and creating the object SHALL not fail because that name is missing

#### Scenario: Finite platform signature
- **WHEN** a document receiver calls either alias with zero, one or two arguments
- **THEN** the method SHALL be resolved and execution SHALL report the unsupported persistence limit
- **AND** three arguments SHALL be rejected by the finite method arity

#### Scenario: Existing exported Write
- **WHEN** a document object module exports a callable Write or Записать method
- **THEN** the new platform declaration SHALL not intercept that export

#### Scenario: Failure preserves object state
- **WHEN** the platform Write method fails because persistence is not connected
- **THEN** the object's reference, new-object status, attributes and tabular sections SHALL remain unchanged
