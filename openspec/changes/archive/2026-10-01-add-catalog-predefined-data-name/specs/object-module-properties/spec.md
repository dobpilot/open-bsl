## ADDED Requirements

### Requirement: Staged catalog predefined data name
A concrete catalog object SHALL expose ИмяПредопределенныхДанных / PredefinedDataName through the existing receiver context. Initial state, object-body access, valid and invalid assignments SHALL match the preserved native oracle and use acquired predefined names. Staging this property SHALL NOT imply database persistence.

#### Scenario: Original module reads the standard property
- **WHEN** an original catalog object module reads the predefined data name
- **THEN** the existing receiver returns the native-measured staged value
- **AND** ordinary local and module variable priority remains unchanged

#### Scenario: Invalid assignment preserves state
- **WHEN** an invalid predefined data name is assigned
- **THEN** the receiver follows the native-measured failure and retained-value behavior

#### Scenario: Concrete object isolation
- **WHEN** one object's predefined data name changes
- **THEN** another object's value remains independent
