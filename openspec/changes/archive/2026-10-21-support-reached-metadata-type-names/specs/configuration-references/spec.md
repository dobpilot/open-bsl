## ADDED Requirements

### Requirement: Native generic metadata type lookup names
The existing generic metadata value type SHALL be available through the Russian name ОбъектМетаданных and the English name MetadataObject. Existing lookup names and the distinct configuration-root type SHALL remain available.

#### Scenario: Original metadata type comparison
- **WHEN** BSL compares the type of a nested metadata object with Type("ОбъектМетаданных") or Type("MetadataObject")
- **THEN** both comparisons SHALL be true
- **AND** both names SHALL resolve to the same type as the existing generic metadata lookup name

#### Scenario: Separate configuration root
- **WHEN** BSL compares the configuration root metadata type with Type("ОбъектМетаданных")
- **THEN** the types SHALL differ
