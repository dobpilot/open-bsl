## ADDED Requirements

### Requirement: Reached hashing string input
DataHashing.Add SHALL accept strings as UTF-8 bytes and preserve its existing binary input and accumulation behavior. Other unsupported inputs SHALL remain errors rather than being implicitly formatted.

#### Scenario: Native string hashing
- **WHEN** ASCII abc and Cyrillic Привет are added to SHA512 hashing
- **THEN** their sums match the retained native 1C controls and explicitly encoded UTF-8 binary input
