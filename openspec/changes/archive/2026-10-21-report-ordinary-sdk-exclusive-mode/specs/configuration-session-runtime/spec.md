## MODIFIED Requirements

### Requirement: Connection-bound exclusive mode
The ORM component SHALL delegate ExclusiveMode and SetExclusiveMode to the current Session data adapter. A host-created ordinary SDK session without exclusive ownership SHALL report false after validating its bound principal and open connection. This result SHALL NOT assert that exclusive access can be acquired. That adapter SHALL reject both setter values. Other adapters SHALL fail explicitly when exclusive-mode capability is absent; capable adapters SHALL report actual connection-owned state without a process-local substitute.

#### Scenario: Ordinary SDK session
- **WHEN** the bound principal reads exclusive mode on an open ordinary SDK session
- **THEN** the getter returns false
- **AND** either setter value fails explicitly

#### Scenario: Invalid session context
- **WHEN** the connection is closed or the principal does not match
- **THEN** the getter fails instead of reporting false

#### Scenario: Read and change exclusive mode
- **WHEN** a capable adapter enters and then leaves exclusive mode
- **THEN** the getter reports the state of that same Session connection after each operation

#### Scenario: Exclusive-mode capability is absent
- **WHEN** startup reads or changes exclusive mode without a capable adapter
- **THEN** execution fails explicitly instead of reporting or storing a fabricated state
