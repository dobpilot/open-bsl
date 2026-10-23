## ADDED Requirements

### Requirement: Explicitly acquired empty configuration default interface
The ORM metadata root SHALL declare read-only ОсновнойИнтерфейс / DefaultInterface through existing case-insensitive property dispatch. An explicitly acquired nil default-interface UUID SHALL return Undefined. Unacquired data and non-nil interface metadata that is not implemented SHALL fail explicitly, never masquerading as an absent interface.

#### Scenario: Original user-description expression
- **GIVEN** an acquired configuration with a nil default-interface UUID
- **WHEN** the original user-description expression tests Metadata.DefaultInterface against Undefined
- **THEN** it selects the empty-name branch without reading an interface name
- **AND** Russian, English and mixed-case reads return the same Undefined value

#### Scenario: Read-only property
- **WHEN** BSL assigns either property alias
- **THEN** the existing read-only property error is raised

#### Scenario: Unknown and unsupported presence
- **WHEN** the host has not acquired the default-interface UUID or has acquired a non-nil UUID without implemented interface metadata
- **THEN** reading either alias fails explicitly
- **AND** a different catalog with proven absence remains independently readable

### Requirement: Default-interface acquisition preserves unknown data
The configuration adapter SHALL project the default-interface UUID only from a natively established root profile using the existing SDK representation. Unknown layouts, missing fields or malformed UUID values SHALL remain unacquired. The immutable ORM catalog SHALL accept an explicitly supplied acquired UUID without process-wide state or new runtime dependencies.

#### Scenario: Native empty and configured profiles
- **WHEN** retained native empty and configured-interface root resources are projected
- **THEN** their acquired UUIDs equal the corresponding native XML absence or Interface UUID

#### Scenario: Malformed or unknown profile
- **WHEN** the root profile or default-interface field is missing, malformed or unsupported
- **THEN** no absence value is fabricated
