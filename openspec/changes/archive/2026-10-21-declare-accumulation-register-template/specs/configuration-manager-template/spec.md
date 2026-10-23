## ADDED Requirements

### Requirement: Accumulation register manager template method declaration
Accumulation register managers SHALL declare ПолучитьМакет / GetTemplate with exactly one required argument through their existing finite receiver context. Aliases SHALL be case insensitive. The declaration SHALL preserve existing same-named exports and variable values and SHALL NOT introduce a global function. Actual retrieval without real resource support SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Dormant original call
- **WHEN** an unchanged accumulation register manager contains an unexecuted method calling either template alias with one argument
- **THEN** compilation links the call through the manager receiver context
- **AND** no template is retrieved

#### Scenario: Explicit unsupported retrieval
- **WHEN** execution invokes either alias without a same-named export or resource implementation
- **THEN** it raises an explicit unsupported-template error
- **AND** returns no fabricated value

#### Scenario: Existing export and variables
- **WHEN** a manager defines a same-named export or variable
- **THEN** the export retains its own arity and result
- **AND** reading the variable preserves its value

#### Scenario: Finite scope and arity
- **WHEN** a bare manager call supplies zero or two arguments without a same-named export
- **THEN** normal semantic arity validation rejects it
- **WHEN** unrelated code calls the name as a global function
- **THEN** the existing undefined-function diagnostic remains
