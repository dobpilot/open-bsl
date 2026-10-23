## ADDED Requirements

### Requirement: Principal-bound catalog attribute lookup
A catalog manager SHALL expose НайтиПоРеквизиту / FindByAttribute with the native signature and measured lookup behavior. It SHALL use acquired metadata and the current session's principal-bound data adapter, returning a concrete catalog reference for an accessible match or the own empty reference for a genuine no-match result. Unsupported metadata, missing capabilities and adapter errors SHALL fail explicitly.

#### Scenario: Original manager finds an own attribute
- **WHEN** an original manager module searches an acquired attribute
- **THEN** the existing interpreter queries accessible data in the current session
- **AND** returns the native-measured own reference result

#### Scenario: No accessible match
- **WHEN** the adapter successfully returns no matching row
- **THEN** the manager returns the own empty reference

#### Scenario: Adapter failure
- **WHEN** the adapter reports a read or permission failure
- **THEN** the manager propagates the failure rather than treating it as no match

### Requirement: Typed attribute lookup adapter request
The public ORM data adapter contract SHALL provide FindByAttributeRequest carrying the acquired attribute name, value, optional parent and owner, and a find_by_attribute operation receiving the owner metadata, immutable principal and CallContext. The provider SHALL validate attribute eligibility using its acquired field types and SHALL preserve native type-sensitive comparison. Adapters without this operation SHALL fail explicitly by default.

#### Scenario: Existing adapter has no lookup implementation
- **WHEN** lookup is invoked on an adapter that only implements the previous data operations
- **THEN** the default implementation reports lookup as unsupported instead of claiming no match

#### Scenario: Mixed value kinds
- **WHEN** a String attribute containing 17 is searched with Number 17
- **THEN** lookup follows the preserved native no-match behavior without converting the number to String
