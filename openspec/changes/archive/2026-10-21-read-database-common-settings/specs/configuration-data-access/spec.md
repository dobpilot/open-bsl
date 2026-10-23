## ADDED Requirements

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
