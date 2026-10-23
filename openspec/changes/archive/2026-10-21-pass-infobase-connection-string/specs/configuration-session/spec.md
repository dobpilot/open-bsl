## ADDED Requirements

### Requirement: CLI registration string for database Sessions
A --config-db invocation SHALL accept optional --infobase-connection-string in separated and equals forms and pass the nonempty string unchanged to its SessionFactory. CLI SHALL NOT derive it from SQL connection parameters. Omission SHALL preserve current behavior. Empty or duplicate values and use without --config-db SHALL fail before a database connection is opened.

#### Scenario: Preserve supplied registration
- **WHEN** a configured invocation supplies a registration string
- **THEN** its SessionFactory receives the exact value without parsing or normalization

#### Scenario: Keep standalone behavior
- **WHEN** a standalone invocation omits the option
- **THEN** its existing execution behavior is unchanged

#### Scenario: Reject invalid option use
- **WHEN** the option is empty, duplicate, or supplied without --config-db
- **THEN** parsing fails without connecting or consuming the invocation
