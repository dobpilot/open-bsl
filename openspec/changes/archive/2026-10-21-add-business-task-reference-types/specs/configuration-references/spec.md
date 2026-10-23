## ADDED Requirements

### Requirement: Business process and task reference declarations
The ORM catalog SHALL declare acquired business process and task reference types through the existing immutable configuration value-type mechanism. Russian and English qualified names SHALL resolve to the same declaration for a configured owner. The database adapter SHALL use acquired reference TypeIds to preserve these types in compound characteristic metadata.

#### Scenario: Native control reference identities
- **GIVEN** the independently compiled native business process and task controls
- **WHEN** their acquired metadata is projected into the ORM catalog
- **THEN** BusinessProcessRef and TaskRef identities resolve to their corresponding declarations
- **AND** the Russian qualified names resolve to those same declarations
- **AND** the two kinds remain distinct

#### Scenario: Missing reference declaration
- **GIVEN** a characteristic configured with a reference TypeId that has no supported acquired owner
- **WHEN** the database adapter projects its value types
- **THEN** projection fails explicitly without omitting that type
