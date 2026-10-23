# cli-session-dynamic-status Specification

## Purpose
Specify the initial dynamic configuration state of a newly opened configured CLI session and explicit host resource responsibility.

## Requirements

### Requirement: Initial dynamic status of a configured CLI session
When starting its new configured logical session, the CLI SHALL supply false as its initial session-specific dynamic configuration state. This state SHALL be available to the session initialization handler through both existing language aliases. The CLI SHALL NOT claim to infer another existing platform session's state from current database metadata.

#### Scenario: New configured session initializes
- **WHEN** the initialization handler calls КонфигурацияБазыДанныхИзмененаДинамически or DataBaseConfigurationChangedDynamically in a newly opened configured CLI session
- **THEN** it receives Boolean false

### Requirement: Other hosts retain explicit resource responsibility
The library SHALL retain its explicit unavailable-resource error when a host does not supply the dynamic configuration state. Standalone CLI behavior SHALL remain unchanged.

#### Scenario: Host supplies no session state
- **WHEN** an arbitrary host calls the session function without providing the existing resource
- **THEN** the existing explicit missing-state error is returned

#### Scenario: Standalone execution
- **WHEN** CLI runs without configuration database options
- **THEN** its existing behavior is unchanged
