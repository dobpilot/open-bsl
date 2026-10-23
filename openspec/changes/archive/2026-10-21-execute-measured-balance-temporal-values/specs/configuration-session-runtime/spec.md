## MODIFIED Requirements

### Requirement: Query boundary construction resolves with documented arity

The ORM library SHALL resolve `Новый Граница` / `New Boundary` with one required value and one optional boundary-kind argument. It SHALL construct the measured immutable Boundary wrapper for Date, PointInTime, Number, Boolean, String, Null and Undefined values. An omitted kind SHALL mean Including; an explicitly supplied invalid kind SHALL fail.

#### Scenario: UNF names a query boundary
- **WHEN** configuration code constructs `Новый Граница(Дата(2024, 1, 1))`
- **THEN** construction succeeds and the boundary retains the Date and Including kind

#### Scenario: Boundary call has no value
- **WHEN** code calls `Новый Boundary()`
- **THEN** semantic resolution reports an argument-count error

#### Scenario: Wrapper values and readonly properties
- **WHEN** code reads Value/Значение or BoundaryType/ВидГраницы
- **THEN** it obtains the original value or selected kind
- **AND** writes to either property fail
- **AND** PointInTime/МоментВремени is not exposed as a property

#### Scenario: Explicit invalid kind
- **WHEN** code supplies Undefined or Number as the second constructor argument
- **THEN** construction reports an invalid second argument

#### Scenario: Measured equality
- **WHEN** two boundaries contain equal measured Date, PointInTime, Number or Undefined values and equal kinds
- **THEN** they compare equal
- **AND** different boundary kinds compare unequal

## ADDED Requirements

### Requirement: Temporal balance values use the existing data-access path

Manager and query Balance calls SHALL forward measured Date, PointInTime and Boundary values through the same principal-bound SDK compiler and data-access adapter. They SHALL reproduce the retained cutoff observations without bypassing rights, RLS or conditional separation. Other Boundary payloads and unsupported temporal positions SHALL fail explicitly.

#### Scenario: Same-date document cutoffs
- **GIVEN** the retained three same-date recorders across two document kinds
- **WHEN** manager and query Balance execute with each measured point and inclusive/exclusive boundary
- **THEN** both return the corresponding retained native result

#### Scenario: Empty recorder reference
- **WHEN** a Balance argument contains an absent or typed empty recorder reference
- **THEN** its cutoff matches the retained date-based native observation
- **AND** generic PointInTime comparison behavior is unchanged

### Requirement: Temporal logical results retain PointInTime identity

An SDK logical result recipe for PointInTime SHALL produce one typed runtime PointInTime value from its declared date and reference columns. Malformed recipes SHALL fail explicitly rather than guess from result labels.

#### Scenario: Declared temporal projection
- **WHEN** an acquired query result provides a valid PointInTime logical recipe
- **THEN** the host reconstructs the date and recorder reference as one PointInTime value

#### Scenario: Invalid recipe
- **WHEN** the recipe references missing or incompatible columns
- **THEN** conversion reports an explicit data-access error

### Requirement: Temporal adapters can inspect typed configuration references

The ORM public Rust API SHALL expose configuration-reference metadata and its canonical UUID without stringifying the runtime value or reading a database row. The existing catalog-only accessor SHALL retain its catalog-only behavior. Temporal parameter conversion SHALL preserve the recorder metadata identity and convert UUID bytes with the existing 1C byte-order conversion.

#### Scenario: Document recorder parameter
- **WHEN** an adapter captures a PointInTime with a typed Document reference
- **THEN** it obtains that Document identity and its UUID even for an empty reference
- **AND** absence of a reference remains distinct from a typed empty reference
