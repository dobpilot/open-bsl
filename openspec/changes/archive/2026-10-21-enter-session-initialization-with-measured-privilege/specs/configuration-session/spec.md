## ADDED Requirements

### Requirement: Automatic session parameter initialization context
Automatic initial and selective session parameter initialization SHALL enter the acquired SessionModule handler with privileged mode requested for that invocation. The caller's existing mode state and authenticated identity SHALL remain unchanged after normal return or an error. Existing effective safe-mode suppression SHALL remain authoritative. The ordinary generic host handler default SHALL remain unchanged.

#### Scenario: Initial automatic entry
- **WHEN** a session invokes its original initialization handler with Undefined
- **THEN** that invocation observes privileged mode in the measured ordinary managed-application context
- **AND** subsequent ordinary session execution observes ordinary mode

#### Scenario: Selective automatic entry
- **GIVEN** an unset declared session parameter and an ordinary caller
- **WHEN** reading the parameter invokes the original selective handler
- **THEN** that handler observes privileged mode in the measured ordinary context
- **AND** the caller remains in ordinary mode after receiving the initialized value

#### Scenario: Exceptional return
- **WHEN** automatic initialization raises an error
- **THEN** the error propagates through existing session initialization handling
- **AND** the entry privilege does not remain in the caller or another session

#### Scenario: Generic handler default
- **WHEN** a host invokes an unrelated configuration handler without selecting privileged entry
- **THEN** the existing inherited security context applies
- **AND** its module or procedure spelling does not implicitly enable privilege
