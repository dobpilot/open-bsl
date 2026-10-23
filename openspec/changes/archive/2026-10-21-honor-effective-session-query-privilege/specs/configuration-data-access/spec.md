## ADDED Requirements

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
