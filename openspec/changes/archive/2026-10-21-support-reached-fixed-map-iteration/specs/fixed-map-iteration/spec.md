## ADDED Requirements

### Requirement: Traverse an immutable map snapshot
For Each SHALL traverse FixedMap values using existing KeyAndValue pairs. Traversal SHALL visit every entry of the snapshot and SHALL preserve the fixed map's immutability and existing public indexing behavior.

#### Scenario: Empty snapshot
- **WHEN** For Each traverses an empty fixed map
- **THEN** its body SHALL execute zero times

#### Scenario: Independent snapshot entries
- **GIVEN** a fixed map was captured before source entries were replaced or added
- **WHEN** For Each traverses it
- **THEN** it SHALL yield the original snapshot keys and values
- **AND** assigning the yielded pair value SHALL fail without changing the snapshot
