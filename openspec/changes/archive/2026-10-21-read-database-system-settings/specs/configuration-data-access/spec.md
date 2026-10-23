## ADDED Requirements

### Requirement: Read current-session system settings from the database
The database adapter SHALL support reached SystemSettingsStorage.Load through the current principal's read-only connection and active call context. It SHALL select System settings independently of Common settings, using complete supported separation scope from the acquired System table. Existing Common settings behavior SHALL be preserved. Other-user requests, unsupported stores, providers, scope profiles and write operations SHALL fail explicitly.

#### Scenario: Identical keys in distinct stores
- **WHEN** Common and System contain different values under the same current-user object and settings keys
- **THEN** each Load returns the value from its requested store

#### Scenario: Complete measured scope
- **WHEN** a supported string or numeric separator is active, inactive or globally disabled
- **THEN** System Load uses that exact SDK scope
- **AND** inactive scope does not read the value parameter and globally disabled scope reads neither use nor value parameters

#### Scenario: Payload and absence
- **WHEN** scoped System Load yields one valid ordinary value or ValueStorage payload
- **THEN** the existing bounded codec materializes it in the active context
- **WHEN** it yields no row
- **THEN** Load returns Undefined
- **WHEN** it yields ambiguous rows, malformed data or incomplete scope
- **THEN** Load fails explicitly
