## ADDED Requirements

### Requirement: Current session exposes its immutable start date
SessionResources SHALL accept an optional host-provided start Date before session initialization. The existing current-session object SHALL expose it as НачалоСеанса and SessionStarted, without consulting the current clock on later reads. The property SHALL be read-only. Missing host data SHALL fail explicitly rather than returning an invented Date.

#### Scenario: Startup reads the supplied value
- **WHEN** the session handler reads the property during initialization
- **THEN** it obtains the Date supplied for that Session

#### Scenario: Native property controls
- **WHEN** the retained native control is replayed with a supplied nonempty start Date
- **THEN** type, alias, stability and assignment outcomes match the platform

#### Scenario: Missing host value
- **WHEN** a Session without a supplied start Date reads the property
- **THEN** it receives an explicit missing-host-data error

#### Scenario: Different session resources
- **WHEN** foreground or background Sessions are opened with different start Dates
- **THEN** each observes its own supplied value
- **AND** the background value is not inherited from the parent

### Requirement: Configured CLI supplies its host session start
The configured CLI SHALL capture the start Date once immediately before opening its Session, using its existing default clock and time zone. It SHALL reuse the existing calendar conversion through the open-bsl facade and SHALL NOT read or fabricate native server session telemetry. Standalone CLI behavior SHALL remain unchanged.

#### Scenario: Configured startup
- **WHEN** original code reads НачалоСеанса during configured initialization
- **THEN** the host start Date is already available

#### Scenario: Facade consumer
- **WHEN** a consumer converts a host timestamp to a Date
- **THEN** it can use the existing conversion through open-bsl without an internal crate import
