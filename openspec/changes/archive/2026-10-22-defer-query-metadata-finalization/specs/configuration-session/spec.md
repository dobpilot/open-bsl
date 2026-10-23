## ADDED Requirements

### Requirement: ORM metadata acquisition defers query finalization
Materializing reached ORM metadata SHALL acquire and validate the normalized
runtime facts it needs without constructing a complete query snapshot merely
for ORM projection. Query metadata finalization SHALL occur when a complete
snapshot is explicitly requested or query preparation requires it. Runtime
facts SHALL NOT masquerade as a complete MetadataSnapshot.

#### Scenario: Runtime object access before a query
- **WHEN** startup reaches a new ORM metadata object without preparing a query
- **THEN** its normalized facts and required dependencies are acquired
- **AND** no query fingerprint, query index or query resolution report is built solely for that object access

#### Scenario: Query after runtime metadata growth
- **WHEN** a query or explicit snapshot request follows acquisition of new runtime facts
- **THEN** a complete immutable snapshot is finalized for the current acquired composition
- **AND** its facts, diagnostics and fingerprint match the existing strict full-snapshot resolver
- **AND** a previously prepared query retains its original snapshot identity

#### Scenario: Invalid runtime dependency
- **WHEN** an ORM object's required named type, reference, separator or extension fact cannot be resolved
- **THEN** acquisition obtains the dependency or reports the existing applicable failure
- **AND** it does not publish an incomplete runtime fact set as successful

#### Scenario: Reuse without new acquisition
- **WHEN** repeated ORM access or snapshot requests add no resources
- **THEN** the corresponding immutable runtime facts or full snapshot are reused
- **AND** mutable state remains isolated between Sessions
