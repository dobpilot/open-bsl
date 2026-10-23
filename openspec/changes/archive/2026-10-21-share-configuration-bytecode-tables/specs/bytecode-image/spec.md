## ADDED Requirements

### Requirement: Shared immutable configuration tables in text images
The version45 configuration text image SHALL represent name and shape tables equal to those of the first module once and reference them from eligible later modules and the optional entry. Parsing SHALL preserve their indices and immutable sharing. Programs with different tables and standalone programs SHALL retain complete table definitions. The existing image verifier SHALL still validate every decoded program.

#### Scenario: Repeated configuration tables
- **GIVEN** multiple configuration programs with the same large name and shape tables
- **WHEN** the image is written and parsed
- **THEN** the shared tables occur once and each eligible later program references them
- **AND** all program instructions and table indices round-trip unchanged
- **AND** adding such modules adds their bodies rather than another copy of those tables

#### Scenario: Independent tables
- **WHEN** a configuration module has different tables from the first module
- **THEN** its full table definitions remain in the image and are restored independently

#### Scenario: Reference without a base
- **WHEN** a standalone program or first configuration module requests shared tables
- **THEN** parsing fails explicitly rather than constructing empty or guessed tables

#### Scenario: Unsupported reference directive
- **WHEN** a shared-table directive has an unknown target or extra fields
- **THEN** parsing rejects it explicitly

#### Scenario: Format compatibility boundary
- **WHEN** a version44 or other unsupported header is supplied
- **THEN** the existing unsupported-version diagnostic rejects it

### Requirement: Shared ordered configuration import suffixes
Version45 configuration text SHALL allow a program's import table to comprise its explicit prefix followed by an equal ordered suffix from the first module. The decoded import aliases, target module IDs, global flags and numeric indices SHALL remain identical to the original complete table. Programs without a shared suffix SHALL retain complete import records.

#### Scenario: Different receiver prefixes and the same visible modules
- **GIVEN** programs with different local import prefixes and the same ordered visible-module suffix
- **WHEN** the configuration image is written and parsed
- **THEN** repeated suffix records reference the first module instead of being printed repeatedly
- **AND** each local prefix and the complete decoded table remain unchanged

#### Scenario: Invalid suffix reference
- **WHEN** a suffix reference has no first module, exceeds its imports, has an invalid or zero count, or contains extra fields
- **THEN** parsing fails explicitly without allocating an unbounded import table

#### Scenario: Mixed independent import sets
- **WHEN** an intermediate module has independent imports followed by a later module sharing the first module's suffix
- **THEN** the later reference still targets the first module
- **AND** independent import entries remain complete
