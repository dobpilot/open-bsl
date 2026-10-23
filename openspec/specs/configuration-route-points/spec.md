# configuration-route-points Specification

## Purpose
Expose acquired owner-scoped business-process route points through the existing session facade.

## Requirements

### Requirement: Acquired owner-scoped route-point composition
The ORM SHALL preserve the complete acquired route-point composition for each
business process, including names, point identifiers and owner identifiers.
Unknown composition SHALL remain distinguishable from a known empty list.
The projection SHALL use the SDK decoder and SHALL NOT index points solely by
their point identifier.

#### Scenario: Shared identifier belongs to different owners
- **WHEN** two business processes contain points with the same point identifier
- **THEN** both owner-scoped descriptions are retained with their own names

#### Scenario: Unavailable composition
- **WHEN** the SDK cannot establish a complete composition
- **THEN** access SHALL fail explicitly rather than report an empty collection

### Requirement: Manager route-point property
Business-process managers SHALL expose the read-only property `ТочкиМаршрута`
and its alias `RoutePoints`. The same property SHALL be available in their own
manager modules through the existing component-module context. Existing local
and module-variable precedence SHALL remain unchanged.

#### Scenario: Original manager access
- **WHEN** an original manager module reads its own `ТочкиМаршрута`
- **THEN** it receives the same owner's collection as qualified manager access

#### Scenario: Variable shadowing
- **WHEN** a local or module variable has the property name
- **THEN** the existing resolver selects that variable

### Requirement: Route-reference identity
Named members SHALL produce immutable references whose identity includes the
configuration, business-process owner and point identifier. Runtime behavior
exposed by this change SHALL be verified against native control receipts.

#### Scenario: Same point identifier in different processes
- **WHEN** references are obtained from different business-process owners
- **THEN** their shared point identifier SHALL NOT make them equal

### Requirement: Measured route-point collection behavior
Known compositions SHALL support case-insensitive named reads, count and
iteration in the acquired order. Named-reference presentation SHALL use
acquired localized synonyms where available. Reading an unknown named member
SHALL fail. Assignment to an existing named member SHALL preserve its original
reference, as measured on the native platform; assignment to the manager
property itself SHALL fail.

#### Scenario: Native nonempty collection
- **WHEN** the independent ten-point native control is projected
- **THEN** count and iteration return ten points and named references match
  the recorded native receipt

#### Scenario: Assignment to a known point
- **WHEN** another route reference or a number is assigned to an existing member
- **THEN** the assignment succeeds and the original reference remains unchanged
