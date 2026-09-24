## ADDED Requirements

### Requirement: Invalid bare module names fail on module initialization
When a configuration module reads the unshadowed bare name of an imported named common module, engine construction SHALL retain the platform's lazy module failure instead of treating the name as a module-reference value or rejecting the whole configuration graph eagerly. The affected module SHALL fail when its instance is first initialized. A local or module variable with the same name SHALL retain priority, and an otherwise unknown identifier SHALL retain the existing semantic diagnostic.

#### Scenario: Invalid module remains untouched
- **WHEN** a module containing a bare imported-module name belongs to the conservative static closure but no execution path initializes that module
- **THEN** engine construction and execution of the selected root succeed

#### Scenario: Invalid module is initialized
- **WHEN** execution first accesses an export of the affected module
- **THEN** that module's initialization fails explicitly before its body or export runs

#### Scenario: Ordinary unknown variable
- **WHEN** a module reads a name that is neither a variable nor an imported module alias
- **THEN** engine construction returns the existing undefined-variable semantic error
