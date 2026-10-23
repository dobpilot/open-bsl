## ADDED Requirements

### Requirement: Finite receiver property context
The facade SHALL allow the host to declare a finite property context for a
configuration module. An unqualified name in that context SHALL read or assign
the current receiver property through existing property operations. Modules
without this context SHALL retain their previous resolution behavior.

#### Scenario: Unqualified property
- **WHEN** an object module with a declared Code property reads or assigns Code
- **THEN** the operation uses the current object instance
- **AND** assignment does not introduce an implicit local variable

#### Scenario: Explicit variable priority
- **WHEN** a parameter or explicit local or module variable has the property name
- **THEN** it retains priority over the receiver property

#### Scenario: Unknown identifier
- **WHEN** an undeclared name is absent from the receiver property table
- **THEN** existing undefined-name and implicit-local rules apply

### Requirement: Context continuity
Dynamic execution and worker reconstruction SHALL retain declared receiver
property contexts consistently with their compiled modules.

#### Scenario: Dynamic property access
- **WHEN** dynamic code reads or assigns a supported receiver property
- **THEN** it affects the same receiver as an ordinary method of that module

### Requirement: Object exchange and additional properties
New catalog objects SHALL provide the measured additional-property collection
and data exchange settings. These mutable values SHALL belong to the concrete
object and SHALL NOT perform database writes by themselves.

#### Scenario: Independent settings
- **WHEN** a caller changes supported settings or additional properties of one object
- **THEN** another newly created object retains its own defaults
- **AND** the operation does not write a database row

### Requirement: Object selection after body initialization
A newly created catalog object SHALL become selected after its object module
body finishes. Supported Code, Description and Ref reads SHALL be available in
the body, but Code/Description assignment and IsNew SHALL fail before selection.

#### Scenario: Initialization before selection
- **WHEN** the object module body reads standard attributes and attempts supported writes or IsNew
- **THEN** reads succeed and the writes and IsNew fail
- **AND** the same operations succeed after creation completes, subject to acquired qualifiers
