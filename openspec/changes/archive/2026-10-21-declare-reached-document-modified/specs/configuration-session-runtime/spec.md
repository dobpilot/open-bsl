## ADDED Requirements

### Requirement: Declare the reached document modification-state method
Document object receivers SHALL expose Модифицированность/Modified with zero arguments, including bare calls in the owning object module. Names SHALL use existing case-insensitive component matching. Runtime calls SHALL fail explicitly while document modification-state tracking is unavailable. Existing local-function precedence and standalone unresolved-name behavior SHALL be preserved.

#### Scenario: Dormant original method
- **WHEN** a compiled document object module contains an uncalled bare Modified invocation
- **THEN** receiver binding succeeds through its existing method context

#### Scenario: Actual call without tracking support
- **WHEN** configuration code calls the declared method on a document object
- **THEN** it receives an explicit unsupported tracking error, never a fabricated Boolean

#### Scenario: Local function retains precedence
- **WHEN** an object module defines its own local function with that name
- **THEN** its bare calls retain existing local function resolution

#### Scenario: Invalid arity or unrelated scope
- **WHEN** a call supplies one argument or uses a standalone global name
- **THEN** the existing arity or unresolved-name error remains effective

#### Scenario: Existing same-named export
- **WHEN** the owning module already exposes an exported method with the same name and its own arity
- **THEN** direct object calls preserve that export's result and arity instead of selecting the newly declared platform method
