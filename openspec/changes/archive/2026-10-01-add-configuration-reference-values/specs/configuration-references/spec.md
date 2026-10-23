## ADDED Requirements

### Requirement: Host-declared configuration value types
The facade SHALL accept immutable value type descriptions owned by the engine.
TypeOf and type-name lookup SHALL return the same declared type. Declarations
SHALL provide explicit lookup names separately from the printed representation.
Existing native and static component type resolution SHALL retain its behavior.
Conflicting declarations SHALL fail engine construction.

#### Scenario: Qualified catalog reference type
- **WHEN** a catalog reference belongs to catalog C
- **THEN** TypeOf prints C's acquired name
- **AND** its declared Russian and English qualified names resolve to that type
- **AND** the printed name alone does not become an implicit lookup alias

#### Scenario: Type lifetime
- **WHEN** a value type is retained after its engine is dropped
- **THEN** its immutable description remains valid without leaked static storage

#### Scenario: Background engine declarations
- **WHEN** a background worker reconstructs its engine from the parent's configuration recipe
- **THEN** it retains the parent's immutable value type declarations
- **AND** a transferred type value resolves in that catalog

#### Scenario: Conflicting declarations
- **WHEN** two host value types claim the same lookup name
- **THEN** engine construction fails instead of choosing one by registration order

### Requirement: Explicit manager receiver context
A configuration manager module SHALL be able to call the platform methods
declared for its own receiver without qualifying that receiver. Only names
explicitly supplied by the host SHALL become receiver methods. The invocation
SHALL use the current State's ThisObject and existing method dispatch.
The receiver's platform method SHALL take priority over a global
common-module export with the same name.

#### Scenario: Own reference method
- **WHEN** a catalog manager module calls its declared EmptyRef method
- **THEN** the call reaches the current catalog manager in the same State

#### Scenario: No context in a different module
- **WHEN** another module calls that name without its own declaration or an ordinary visible function
- **THEN** it retains the existing undefined-function diagnostic

#### Scenario: Global export with the same name
- **WHEN** a global common module exports the name of a receiver method
- **THEN** the manager module calls its receiver method
- **AND** a module without that receiver context calls the global export

### Requirement: Catalog reference identity
Catalog EmptyRef SHALL return an unfilled reference with a nil UUID.
GetRef SHALL preserve an explicit UUID and create a new unique reference when
the UUID is omitted. References of the same catalog with the same UUID SHALL
compare equal. UUID retrieval SHALL return the UUID value, not a string.
Creation of a reference SHALL NOT read or write its catalog row.

#### Scenario: Nil UUID
- **WHEN** GetRef receives the nil UUID
- **THEN** its result equals EmptyRef and is not filled

#### Scenario: Repeated explicit UUID
- **WHEN** GetRef is called twice with the same non-nil UUID
- **THEN** the references compare equal and expose that UUID

#### Scenario: Omitted UUID
- **WHEN** GetRef is called twice without its optional argument
- **THEN** the resulting references are filled and distinct
- **AND** their UUIDs have version 1 as measured on native 1C 8.3.27.2342

#### Scenario: Typed query parameter
- **WHEN** a catalog reference is supplied as a query parameter
- **THEN** the database adapter preserves its reference type and UUID
- **AND** the query retains the current principal's rights checks

#### Scenario: Undefined optional UUID
- **WHEN** GetRef receives explicit Undefined
- **THEN** it creates a new filled reference as when the argument is omitted

#### Scenario: Invalid UUID argument
- **WHEN** GetRef receives a number, UUID string or Null
- **THEN** it rejects the argument instead of coercing it

#### Scenario: Typed query result reference
- **WHEN** a supported catalog reference is returned by a query
- **THEN** the adapter materializes a reference of that acquired catalog
- **AND** its UUID preserves the native physical reference identity

### Requirement: Component presentation failures
The object protocol SHALL provide a fallible presentation hook separately
from diagnostic Display. Existing component objects SHALL retain their current
presentation by default. A catalog's empty reference SHALL have an empty
presentation. A reference requiring unavailable data access SHALL report an
explicit error instead of exposing diagnostic text as a platform presentation.

#### Scenario: Empty reference presentation
- **WHEN** String receives a catalog EmptyRef
- **THEN** it returns the empty string

#### Scenario: Presentation without the required adapter
- **WHEN** a filled catalog reference has no supported presentation data access
- **THEN** formatting reports an error instead of inventing its presentation

### Requirement: Component access to the host clock
The VM SHALL attach the current State's host clock to component contexts
without replacing a host-injected Clock. Reduced contexts without
this capability SHALL report CapabilityMissing with Capability Clock.

#### Scenario: Reference generation with a host clock
- **WHEN** the host supplies a fixed clock and requests several new catalog references
- **THEN** version 1 UUID timestamps advance without duplicate identifiers
- **AND** the ordinary UUID constructor retains version 4 generation

#### Scenario: Clock unavailable
- **WHEN** a reduced context requests its clock without a supplied capability
- **THEN** the request fails with CapabilityMissing Clock
