## ADDED Requirements

### Requirement: New catalog item folder flag
The already supported new catalog item object SHALL expose read-only ЭтоГруппа / IsFolder as a boolean standard property through the existing receiver context. Its value and object-body availability SHALL match the preserved native oracle. Declaring the property SHALL NOT fabricate persisted records or implement folder creation implicitly.

#### Scenario: Original object module reads the flag
- **WHEN** an original catalog object module reads the flag bare or through the object
- **THEN** it obtains the measured flag of that specific new item
- **AND** Russian and English aliases agree without changing ordinary variable priority

#### Scenario: Assignment is rejected
- **WHEN** code attempts to assign the folder flag
- **THEN** it receives a read-only-property error
- **AND** the object flag remains unchanged
