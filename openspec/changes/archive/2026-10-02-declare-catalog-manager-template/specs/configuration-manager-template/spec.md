## ADDED Requirements

### Requirement: Catalog manager template method declaration
Catalog managers SHALL declare ПолучитьМакет / GetTemplate with one required argument through their finite receiver context. Original bare manager calls SHALL link, aliases SHALL be case insensitive, and incorrect arity SHALL be rejected. Existing exported manager methods with the same name SHALL retain their behavior. Retrieval without real resource support SHALL fail explicitly instead of returning a fabricated template.

#### Scenario: Original catalog manager source
- **WHEN** an original catalog manager procedure calls ПолучитьМакет with a template name
- **THEN** the module links through its receiver context
- **AND** executing unsupported retrieval raises an explicit error

#### Scenario: Existing manager export
- **WHEN** a catalog manager already exports a procedure with the same name
- **THEN** the existing procedure remains callable with its existing behavior
