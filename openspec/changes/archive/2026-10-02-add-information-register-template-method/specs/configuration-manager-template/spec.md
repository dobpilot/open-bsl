## ADDED Requirements

### Requirement: Information register manager template method declaration
Information register managers SHALL declare ПолучитьМакет / GetTemplate with
one required argument through their finite receiver context. Original bare
manager calls SHALL link, aliases SHALL be case insensitive, and incorrect
arity SHALL be rejected. Existing exported manager methods with the same name
SHALL retain their arity and result. Retrieval without real resource support
SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Dormant original manager call
- **WHEN** an information register manager contains an unexecuted procedure calling ПолучитьМакет with one argument
- **THEN** the original module links through its own receiver context
- **AND** the declaration does not introduce a global function outside that context

#### Scenario: Actual retrieval
- **WHEN** code executes either alias without real template resource support
- **THEN** it raises an explicit unsupported-template error
- **AND** it returns no empty document or fabricated value

#### Scenario: Existing export and variable precedence
- **WHEN** a manager exports a same-named function or declares a same-named variable
- **THEN** the existing export retains its arity and result
- **AND** reading the variable preserves its existing value

#### Scenario: Invalid argument count
- **WHEN** a receiver method call supplies zero or two arguments without a same-named export
- **THEN** normal semantic arity validation rejects it
