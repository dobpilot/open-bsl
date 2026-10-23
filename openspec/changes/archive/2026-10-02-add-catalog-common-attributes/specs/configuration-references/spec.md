## ADDED Requirements

### Requirement: Complete applied catalog common attributes
The configuration catalog SHALL retain acquired applicable common attributes separately from own attributes. Explicit acquired inclusion or exclusion SHALL override the acquired automatic default for the measured catalog modes. Unknown collection membership or usage SHALL NOT be silently discarded or represented by a known empty or complete partial collection.

#### Scenario: Explicit inclusion
- **WHEN** automatic use is disabled and Content explicitly includes a catalog
- **THEN** that catalog's object receiver SHALL expose the included common attribute name
- **AND** the own Attributes collection SHALL remain unchanged

#### Scenario: Automatic inclusion and explicit exclusion
- **WHEN** automatic use is enabled and Content explicitly excludes one catalog
- **THEN** a catalog without an override SHALL expose the common attribute
- **AND** the excluded catalog SHALL reject access to that attribute

#### Scenario: Unknown acquisition
- **WHEN** authoritative collection membership or applicable usage is unknown
- **THEN** the host SHALL receive an explicit acquisition or access error
- **AND** the ORM SHALL NOT claim it acquired a complete partial collection

### Requirement: Measured common string attribute object state
The included string common attribute SHALL be case-insensitive, have the acquired string length, and keep its value in the individual catalog object. Its initial value SHALL be an empty string readable during object-module initialization; assignment SHALL require a selected object. Unsupported common attribute types SHALL fail explicitly until implemented from platform evidence.

#### Scenario: Body and subsequent assignment
- **WHEN** the included НаименованиеЯзык1 has variable string length 150
- **THEN** the module body SHALL read an empty string and its assignment SHALL fail
- **AND** after creation a number assignment SHALL produce its string representation and a 151-character ASCII string SHALL truncate to 150 characters

#### Scenario: Instance isolation
- **WHEN** one object assigns the common attribute
- **THEN** another object or Session SHALL keep its own initial value
- **AND** assignment SHALL NOT write a database row
