## ADDED Requirements

### Requirement: Configured CLI delivers independent session events to stderr
With --config-db, the CLI SHALL attach a Session-owned event writer that delivers independent WriteLogEvent calls to stderr, retaining event name, level, comment, metadata and data. BSL values SHALL use the existing user-facing formatter. Output or formatting failure SHALL fail the call explicitly. Execution without --config-db SHALL retain its existing behavior.

#### Scenario: Original session writes an independent event
- **WHEN** original configuration initialization writes an independent event
- **THEN** the configured CLI delivers its fields to stderr and preserves script stdout

#### Scenario: Unsupported transactional event
- **WHEN** configuration code requests transactional delivery through the CLI writer
- **THEN** the call fails explicitly without pretending to persist a database journal entry

#### Scenario: Destination failure
- **WHEN** the stderr destination rejects an event
- **THEN** the call reports the output error instead of silently discarding the event
