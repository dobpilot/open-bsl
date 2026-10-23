## ADDED Requirements

### Requirement: Configuration temporary-register pressure uses available frame capacity
A configuration method whose source locals fit the bytecode frame but whose ordinary emission exhausts temporary registers SHALL compile when existing local-slot allocation makes its simultaneously needed values and temporaries fit the unchanged frame. Observable evaluation order, parameter positions, ByRef updates and source-name identities SHALL be preserved. Successful ordinary compilation and standalone compilation SHALL retain their existing paths. Exception-handler, dynamic-source and debug-local materialization restrictions SHALL remain unchanged. Genuine simultaneous frame overflow SHALL fail explicitly with the existing contextual diagnostic.

#### Scenario: Near-limit locals with a short expression
- **WHEN** a configuration method has243source locals and an expression with thirteen binary nodes but only a small live subset
- **THEN** it compiles within the existing255-register frame and executes with the native result

#### Scenario: ByRef updates and call order
- **WHEN** an eligible method computes a pressured expression and writes its result through a ByRef parameter
- **THEN** the caller observes the native result
- **WHEN** two calls within a pressured expression append distinct markers to a shared log
- **THEN** their observable order matches the native platform

#### Scenario: Interned names survive the retry
- **WHEN** structure literals and property names occur before and after the pressured expression and in another module
- **THEN** their names, spelling, shape identities and executable values remain valid after compilation

#### Scenario: Restricted or truly overflowing frames
- **WHEN** a method requires dynamic/debug locals, contains exception handlers or cannot fit simultaneous values and temporaries
- **THEN** compilation retains the existing explicit frame-limit failure
- **WHEN** the same pressured source is compiled standalone
- **THEN** its existing standalone diagnostic is unchanged
