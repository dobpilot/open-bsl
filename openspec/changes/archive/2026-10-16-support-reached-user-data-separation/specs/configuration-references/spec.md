## ADDED Requirements

### Requirement: Acquired user data separation snapshot
The host SHALL be able to attach an optional serialized user data separation snapshot to Principal. The runtime SHALL expose it as CurrentUser().РазделениеДанных / DataSeparation using a Structure value. Unknown snapshots SHALL fail explicitly rather than return an empty structure. Malformed snapshots with a root count other than one or a non-Structure root SHALL fail explicitly.

#### Scenario: Known empty snapshot
- **WHEN** the host supplies a graph containing one empty Structure
- **THEN** DataSeparation.Count() SHALL be zero

#### Scenario: Unknown snapshot
- **WHEN** the host has not supplied the snapshot
- **THEN** reading DataSeparation SHALL fail explicitly

### Requirement: User object structure lifetime
Repeated reads of DataSeparation on the same user object SHALL return the same mutable structure. Each CurrentUser call SHALL materialize an independent structure from the frozen Principal snapshot. Separate states SHALL never share mutable structures through that snapshot.

#### Scenario: Modify one returned structure
- **WHEN** code modifies a structure obtained from one current-user object
- **THEN** the same user object SHALL expose the change
- **AND** a subsequent CurrentUser call SHALL expose the original acquired snapshot

#### Scenario: Separate session states
- **WHEN** two states use the same immutable acquired Principal snapshot
- **THEN** mutation in one state SHALL NOT affect the other state
