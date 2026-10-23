## ADDED Requirements

### Requirement: Standalone planner item values
The session component library SHALL expose `ЭлементПланировщика` and `PlannerItem` as aliases of a zero-argument constructor. Each call SHALL create independent state with the native measured defaults. Constructor argument errors SHALL use the existing arity diagnostics.

#### Scenario: New independent items
- **WHEN** two items are constructed
- **THEN** each has undefined Value, Deleted and Parent, empty Begin and End dates, empty Text and ToolTip and an empty fixed map of DimensionValues
- **AND** changes to one item do not change the other

#### Scenario: Constructor arguments
- **WHEN** the constructor receives one, two or three arguments
- **THEN** it rejects the call through the existing arity validation

### Requirement: Measured planner item assignments
The item SHALL support the native measured writes to Value, Text, ToolTip, Begin, End and Deleted. Parent and DimensionValues SHALL reject property replacement. Russian and English property aliases SHALL identify the same fields.

#### Scenario: Text conversion
- **WHEN** 42 is assigned to Text or ToolTip
- **THEN** the property returns the string `42`

#### Scenario: Invalid date replacement
- **GIVEN** Begin contains a nonempty date
- **WHEN** 42 is assigned to Begin
- **THEN** Begin becomes the empty date

#### Scenario: Standalone deletion flag
- **WHEN** True is assigned to Deleted of a standalone item
- **THEN** Deleted remains Undefined

#### Scenario: Read-only replacement
- **WHEN** code assigns Parent or DimensionValues
- **THEN** the existing property write error is returned

### Requirement: Facade fixed-map snapshot factory
The facade SHALL expose a Rust factory that creates the existing fixed-map value from an existing mutable map value. It SHALL preserve the map entries as an independent immutable snapshot and SHALL reject an input of a different type through the existing runtime error mechanism.

#### Scenario: Snapshot an empty map
- **WHEN** a component supplies an empty mutable map
- **THEN** the factory returns the existing fixed-map type with zero entries

#### Scenario: Snapshot preserves entries
- **WHEN** a component snapshots a mutable map and later changes its entries
- **THEN** the fixed map retains its original entries
