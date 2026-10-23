## ADDED Requirements

### Requirement: Database Session transaction delegation
The database adapter SHALL implement the existing transaction operations through the same concrete SDK connection used by Session reads. The outer begin SHALL start a real read-only transaction. Nested begin and completion SHALL preserve the logical nesting depth without committing the connection early. Completion without begin SHALL fail. Connection errors SHALL never be reported as successful transaction completion.

#### Scenario: Complete a nested transaction
- **WHEN** a Session begins twice and commits once
- **THEN** the transaction remains active on its original connection
- **WHEN** it commits the outer level
- **THEN** the real connection transaction is committed and becomes inactive

#### Scenario: Fail empty completion
- **WHEN** a Session commits or rolls back without an active transaction
- **THEN** the operation fails explicitly

### Requirement: Cancelled nested transactions prevent reads
After an inner rollback, the adapter SHALL report an active transaction until outer completion and SHALL reject data reads. Outer commit of that cancelled transaction SHALL complete successfully by rolling back the real connection transaction, preserving no changes. Cancellation state SHALL belong only to that Session. A nested begin after inner rollback SHALL remain allowed without clearing cancellation or allowing reads.

#### Scenario: Complete after inner rollback
- **WHEN** a Session begins twice and rolls back the inner level
- **THEN** it remains active and its data reads fail
- **WHEN** it commits the outer level
- **THEN** its real transaction is rolled back and subsequent reads are allowed

#### Scenario: Keep another Session independent
- **WHEN** one Session cancels an inner transaction
- **THEN** another Session does not inherit its nesting or cancellation state
