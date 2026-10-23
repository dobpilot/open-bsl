## ADDED Requirements

### Requirement: Reached StrReplace argument conversion
StrReplace SHALL convert its measured simple text, search and replacement arguments as native 1C does. Numbers SHALL use no grouping and a decimal point. Existing UUID receivers and unsupported-type handling SHALL remain supported through the existing runtime method.

#### Scenario: Native conversion in each position
- **WHEN** Undefined, number and Boolean arguments occur in the retained replacement controls
- **THEN** the results match the native oracle, including an Undefined replacement value becoming an empty string

#### Scenario: Existing receiver support
- **WHEN** a supported UUID receiver reaches replacement
- **THEN** its previous receiver behavior is preserved
