# functional-new Specification

## Purpose
Define the functional `Новый(<Тип>[, <ПараметрыКонструктора>])` form so BSL code can select an existing native or component constructor at runtime while preserving static constructor behavior and bytecode compatibility.

## Requirements

### Requirement: Functional constructor syntax
The language SHALL accept `Новый(<Тип>[, <ПараметрыКонструктора>])` and `New(<Type>[, <ConstructorParameters>])` as a functional constructor form distinct from the static `Новый ИмяТипа(...)` form. The type expression SHALL be evaluated before the optional constructor-parameter expression.

#### Scenario: Runtime type value
- **WHEN** the first argument evaluates to a `Тип` value for a constructible type
- **THEN** the runtime constructs a value of that type

#### Scenario: Runtime type name
- **WHEN** the first argument evaluates to a type-name string
- **THEN** the runtime resolves the name case-insensitively through the current runtime type catalog
- **AND** constructs a value of the resolved type

#### Scenario: Existing static form
- **WHEN** code uses `Новый ИмяТипа(...)`
- **THEN** its existing compile-time resolution, arity checks, and execution remain unchanged

### Requirement: Functional constructor parameters
When the optional second argument is an array, the runtime SHALL pass its elements to the selected constructor in array order. When the second argument is absent or is not an array, the runtime SHALL invoke the selected constructor without arguments.

#### Scenario: Array supplies constructor arguments
- **WHEN** the parameter expression evaluates to an array containing two values
- **THEN** the selected constructor receives those two values in the same order

#### Scenario: Non-array parameter value
- **WHEN** the parameter expression evaluates to a value that is not an array
- **THEN** the value is ignored
- **AND** the selected constructor receives no arguments

### Requirement: Existing constructor registry is authoritative
The functional form SHALL use the constructor already associated with a resolved native type or registered component `TypeDescriptor`. It SHALL preserve the constructor's arity validation, host capabilities, and returned object behavior.

#### Scenario: Registered component type
- **GIVEN** a component registers a constructor and associates it with a public type descriptor
- **WHEN** the functional form receives that type value
- **THEN** it invokes that registered constructor with the current call context

#### Scenario: Type has no constructor
- **WHEN** the first argument resolves to a type for which the current runtime has no constructor
- **THEN** execution fails with a catchable runtime error

### Requirement: Versioned bytecode representation
The textual bytecode format SHALL represent the functional constructor without changing the meaning of existing constructor instructions. Readers SHALL reject images from incompatible format versions.

#### Scenario: Functional constructor round trip
- **WHEN** a program containing the functional constructor is printed and parsed as textual bytecode
- **THEN** the type and optional parameter registers are preserved
