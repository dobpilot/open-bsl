# configuration-data-access Specification

## Purpose
Define principal-bound database settings reads through the current Session connection, preserving explicit absence, decoding errors and separation conditions.

## Requirements

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

### Requirement: Read current-session common settings from the database
The database adapter SHALL load reached Common settings for the current principal through its existing read-only connection and the active call context. It SHALL resolve complete supported string/numeric separation scope in Independent and IndependentAndShared modes through current Session parameters and SDK metadata, without reading a value parameter when its use flag is false. Unsupported profiles, other-user requests, stores and write operations SHALL fail explicitly. SDK SQL compilation SHALL verify physical scope coverage before execution.

#### Scenario: Current-user setting
- **WHEN** supported Common Load returns exactly one binary payload
- **THEN** the adapter materializes its ordinary value or ValueStorage contents using existing codecs in the active context

#### Scenario: Missing or ambiguous setting
- **WHEN** the scoped read returns no row
- **THEN** Load returns Undefined
- **WHEN** the read returns more than one row or an invalid payload shape
- **THEN** Load fails explicitly

#### Scenario: Inactive separator
- **WHEN** the bound separator use flag is false
- **THEN** the read uses the inactive SDK scope without requesting the value parameter

#### Scenario: Unsupported request
- **WHEN** a store, scope or operation is unsupported
- **THEN** the adapter fails before any write or unrestricted read

### Requirement: Respect the global condition of common settings
The adapter SHALL resolve the global separation condition before requesting the use/value parameters. A false condition SHALL use the SDK canonical globally disabled scope and SHALL NOT request either parameter. A true or absent condition SHALL resolve use before value and SHALL NOT request value when use is false.

#### Scenario: False global condition
- **WHEN** the acquired Boolean condition is false
- **THEN** the adapter passes ConditionDisabled to the SDK without reading use/value parameters

#### Scenario: True global condition
- **WHEN** the acquired Boolean condition is true and use is false
- **THEN** the adapter passes Inactive and does not read the value parameter

### Requirement: Logical compiled query result materialization
The adapter SHALL expose one BSL result column for each logical selected value described by the compiler's complete projection. It SHALL use the compiler's logical names and exact physical indices, preserving independent occurrences of repeated selected fields. Composite values SHALL decode only their active payload through existing value converters. Rights and mandatory data separation SHALL remain prerequisites to SQL execution.

#### Scenario: Reached Boolean or String constant
- **WHEN** a complete projection describes one Boolean|String selected value
- **THEN** the result contains one logical column with the active Boolean or String value rather than physical filler or TYPE columns

#### Scenario: Two aliases of one field
- **WHEN** the same composite field is selected twice with different aliases
- **THEN** the result retains two independent named logical positions

#### Scenario: Existing scalar conversion
- **WHEN** a complete projection describes scalar values
- **THEN** their existing type and identity conversion behavior is preserved

### Requirement: Explicit invalid projection failure
The adapter SHALL reject unavailable, incomplete or invalid logical projections and malformed composite rows explicitly. It SHALL NOT infer recipes from physical labels or replace errors with empty successful values.

#### Scenario: Unknown discriminator
- **WHEN** a row has an unsupported discriminator or no unique active alternative
- **THEN** materialization fails explicitly

#### Scenario: Unavailable compiler recipe
- **WHEN** the compiler cannot provide a complete unambiguous logical projection
- **THEN** the adapter rejects the result instead of exposing guessed or partial BSL columns

### Requirement: Acquired inline ValueStorage results
The adapter SHALL materialize a supported inline ValueStorage cell only when the acquired field declaration identifies ValueStorage. The result SHALL be the existing BSL ValueStorage object whose Get returns the decoded value with existing detached snapshot semantics. Reading SHALL retain applicable rights and mandatory data-separation checks.

#### Scenario: Original extension settings storage
- **WHEN** the original session reads a declared ValueStorage field with supported inline content
- **THEN** the result is a ValueStorage object and Get returns its decoded content

#### Scenario: Unclassified binary column
- **WHEN** an unrelated Binary column happens to share a storage-like prefix
- **THEN** it is not silently treated as ValueStorage

#### Scenario: Unsupported or malformed representation
- **WHEN** content is malformed, oversized, externally referenced, or contains an unsupported serialized value
- **THEN** an explicit error is returned without an empty successful replacement

### Requirement: Closed facade internal-value decoder
The facade SHALL expose a pure Rust entry point to the existing internal-value decoder using facade-owned Value and RuntimeShapes names. It SHALL retain the existing BSL decoder's accepted forms and error behavior without a separate parser.

#### Scenario: Facade-only decoding
- **WHEN** a consumer decodes a measured internal serialization using only facade names
- **THEN** it receives the same value or error as the existing runtime decoder

### Requirement: Effective frame privilege in configured queries
The database query adapter SHALL consult the existing effective privileged mode of the current call frame. In effective privileged mode, reads SHALL not be refused by ordinary role field rights or row restrictions. When that mode is disabled or suppressed, the existing user checks SHALL apply. This SHALL not change authenticated user identity, persistently grant roles, permit SQL writes, or leak mode between sessions.

#### Scenario: Original privileged settings query
- **WHEN** original configured code enables effective privileged mode before its query
- **THEN** the adapter uses privileged read policy for that query

#### Scenario: Reset to ordinary access
- **WHEN** code disables privileged mode before the next query
- **THEN** that query uses the ordinary authenticated user's field and row checks

#### Scenario: Suppressed privilege
- **WHEN** the existing core reports privilege suppressed by safe mode
- **THEN** the adapter applies ordinary access policy despite a requested privileged flag

#### Scenario: New session isolation
- **WHEN** another session starts using the same immutable configuration
- **THEN** it does not inherit the earlier call frame's privileged mode
