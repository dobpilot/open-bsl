## ADDED Requirements

### Requirement: Reached server administration constructor declaration
The session library SHALL declare АдминистрированиеСервера and ServerAdministration as aliases of the same constructor accepting zero through two arguments. Compilation SHALL use the existing component registry and SHALL NOT execute a constructor merely because an unused function mentions it. While ras transport is unavailable, invoking this constructor SHALL fail explicitly and SHALL NOT return a fabricated administration object or claim a network connection.

#### Scenario: Unused constructor in a common module
- **GIVEN** a registered session library and a common module containing an unused function constructing АдминистрированиеСервера
- **WHEN** a separate valid export runs
- **THEN** compilation and the valid export SHALL succeed without constructing a ras object

#### Scenario: Aliases and optional arguments
- **WHEN** a program contains either constructor name with zero, one or two arguments
- **THEN** semantic compilation SHALL accept the declaration
- **AND** actual invocation without implemented ras transport SHALL fail explicitly

#### Scenario: Excess arguments
- **WHEN** either constructor is supplied three arguments
- **THEN** semantic compilation SHALL reject the arity through the existing constructor diagnostic
