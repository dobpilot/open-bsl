## ADDED Requirements

### Requirement: Metadata lookup for supported configuration types
Metadata SHALL expose НайтиПоТипу/FindByType for supported acquired configuration reference, object and selection types and return their existing metadata owner identity.

#### Scenario: Acquired reference owner
- **WHEN** FindByType receives a supported acquired configuration reference, object or selection type
- **THEN** the returned metadata value equals that owner obtained from its metadata collection
- **AND** Russian and English method names agree

#### Scenario: Primitive type
- **WHEN** FindByType receives Number
- **THEN** it returns Undefined

#### Scenario: Invalid argument
- **WHEN** FindByType receives a number instead of a Type value
- **THEN** it raises a type error

#### Scenario: Unsupported owned type family
- **WHEN** a configuration-owned type has no supported owner association
- **THEN** lookup raises an explicit unsupported error instead of reporting absent metadata
