# configuration-enumeration-values Specification

## Purpose
Expose acquired enumeration identities, localized presentation and immutable transfer through the existing facade.

## Requirements

### Requirement: Concrete enumeration reference types
Each acquired enumeration SHALL expose its own reference type through the existing facade type registry. Both qualified Russian and English names SHALL resolve to that type. Nonempty values SHALL be filled; empty references SHALL retain their enumeration type and be unfilled. Enumeration references SHALL expose metadata and empty-state inspection without exposing catalog UUID operations.

#### Scenario: Configured member
- **WHEN** a member is obtained from its enumeration manager or PredefinedValue
- **THEN** both paths return equal values of that enumeration's concrete reference type
- **AND** another member of that enumeration compares unequal

#### Scenario: Empty reference
- **WHEN** EmptyRef is called on an enumeration manager
- **THEN** its result has the same reference type as the enumeration members
- **AND** its presentation is empty and IsEmpty returns true
- **AND** its metadata identifies the owning enumeration

### Requirement: Acquired enumeration presentation and full names
Enumeration member presentation SHALL use the acquired synonym selected for the configuration's default language and fall back to the member name when that synonym is absent. The full predefined name SHALL use the configuration's script variant and retain metadata names for the owner and member.

#### Scenario: Russian synonym
- **WHEN** the configuration default language is Russian and a member has a Russian synonym
- **THEN** String returns that synonym
- **AND** the full predefined name retains the member's metadata name

#### Scenario: Independent script variant
- **WHEN** a configuration uses Russian ScriptVariant with an English default language and no English member synonym
- **THEN** String returns the member name
- **AND** the full predefined name begins with Перечисление

### Requirement: Enumeration manager member access
An enumeration manager SHALL provide its configured members by name and preserve acquired member order for Count, Get and IndexOf. These operations SHALL use existing component dispatch and preserve ordinary variable resolution priorities.

#### Scenario: Second member
- **WHEN** a three-member enumeration manager receives Get(1)
- **THEN** it returns its second acquired member
- **AND** IndexOf for that member returns 1
- **AND** Count returns 3

### Requirement: Immutable enumeration value transfer
Enumeration reference graph transfer SHALL preserve concrete type and member identity using the receiving configuration's trusted materializer. It SHALL carry no mutable session state or connection.

#### Scenario: Enumeration parameter restoration
- **WHEN** a supported enumeration value is restored from the existing value graph in a matching configuration
- **THEN** it has the same concrete type and compares equal to that configured member
- **AND** restoration does not share a parent Session or its host-data container

#### Scenario: Receiving configuration mismatch
- **WHEN** the receiving configuration does not declare the transferred type or member
- **THEN** restoration fails rather than binding by a coincident name

### Requirement: Facade number parser access
The facade SHALL expose the existing numeric parser and its format description so library components can reuse existing BSL numeric conversion without an implementation-crate dependency.

#### Scenario: Manager index conversion
- **WHEN** a library parses a numeric string through the facade
- **THEN** it uses the existing format parser and its conversion errors
- **AND** enumeration Get truncates a successfully converted fractional index toward zero

#### Scenario: Invalid enumeration index lookup
- **WHEN** IndexOf receives an empty reference, a foreign value or a value of a different type
- **THEN** it fails instead of inventing a member index
