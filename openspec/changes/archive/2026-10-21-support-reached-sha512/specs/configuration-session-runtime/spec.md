## ADDED Requirements

### Requirement: Reached SHA512 data hashing
DataHashing SHALL accept HashFunction.SHA512 and compute the native SHA512 binary digest using the existing input conversion. Reading HashSum SHALL NOT consume the accumulated input; later Add calls SHALL extend it. Existing algorithms SHALL retain their behavior.

#### Scenario: Native digest controls
- **WHEN** empty input, abc and incremental a then bc are hashed with SHA512
- **THEN** results match retained native 1C controls
- **AND** repeated snapshots preserve the accumulated input
