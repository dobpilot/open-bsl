## ADDED Requirements

### Requirement: Acquired current-user UUID
CurrentUser SHALL expose read-only УникальныйИдентификатор / UUID returning the existing UUID value type with the current user object's acquired immutable Principal identifier. Invalid host identifiers SHALL fail explicitly rather than being generated or replaced with nil.

#### Scenario: Acquired identity
- **WHEN** the current principal has a valid acquired UUID
- **THEN** both property names return that exact UUID and the UUID runtime type

#### Scenario: Read-only identity
- **WHEN** BSL assigns the property
- **THEN** assignment fails as read-only

#### Scenario: Invalid or different principals
- **WHEN** the host supplies an invalid identifier
- **THEN** property access fails explicitly
- **WHEN** distinct Sessions have different principal identifiers
- **THEN** each returns its own identifier
