## ADDED Requirements

### Requirement: Reached adjusted point constructor declaration
When the ORM library is registered, compilation SHALL recognize МоментВремениСУточнениемПериода and PointInTimeWithPeriodAdjustment without regard to case, with one through three arguments. Both type aliases SHALL identify the declared type with presentation `Point in time with period adjustment`. Existing constructor codes and ordinary PointInTime behavior SHALL remain unchanged.

#### Scenario: Dormant original manager constructor
- **WHEN** a compiled method contains the constructor with a supported arity without executing it
- **THEN** compilation succeeds and no object is constructed

#### Scenario: Explicit unsupported execution
- **WHEN** execution calls the declared constructor before its object model is available
- **THEN** it fails explicitly with an error identifying the unsupported adjusted point object model
- **AND** it does not return an undefined or fabricated value

#### Scenario: Arity and library boundaries
- **WHEN** the registered constructor is called with zero or four arguments
- **THEN** normal semantic arity diagnostics apply
- **WHEN** the ORM library is absent
- **THEN** the existing unsupported-constructor diagnostic remains
