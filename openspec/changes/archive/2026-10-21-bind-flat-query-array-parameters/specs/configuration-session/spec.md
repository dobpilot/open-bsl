## ADDED Requirements

### Requirement: Flat BSL array query parameters
The database adapter SHALL accept flat Array parameters whose elements have supported scalar encodings and SHALL preserve their element order, reference owner identities and scalar encodings through the existing SDK list carrier. Empty arrays SHALL be supported for IN predicates. Binding SHALL capture the current array values at execution without replacing the BSL parameter object.

#### Scenario: Original reference IN list
- **WHEN** original code passes an array of catalog references to an IN parameter
- **THEN** binding uses those references instead of raising an unsupported Array error

#### Scenario: Empty IN list
- **WHEN** an empty array is bound to an IN predicate
- **THEN** execution returns no matching rows

#### Scenario: Live parameter and execution snapshot
- **WHEN** the application appends to its array after SetParameter and executes the query
- **THEN** the execution snapshot includes the append
- **AND** a previously captured host parameter snapshot is unchanged
