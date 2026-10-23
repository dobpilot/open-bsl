## ADDED Requirements

### Requirement: Configured foreground CLI session kind
Database script mode SHALL accept optional `--session-kind` values
`thin-client` and `web-client`, passing the selected kind to its existing
session factory. Omission SHALL preserve thin-client behavior. The option
SHALL NOT simulate or launch a background job.

#### Scenario: Foreground kind selection
- **WHEN** database script mode specifies either accepted kind
- **THEN** the existing factory opens that foreground session kind

#### Scenario: Existing configured invocation
- **WHEN** database script mode omits the option
- **THEN** it retains the existing thin-client session

#### Scenario: Invalid modifier
- **WHEN** the option is repeated, has an unsupported value, or appears without database mode before the script path
- **THEN** argument validation fails before connecting

#### Scenario: Script arguments
- **WHEN** the same option text appears after the script path
- **THEN** it remains a script argument and does not select the session kind
