## ADDED Requirements

### Requirement: New catalog object predefined flag
The supported new catalog object SHALL expose read-only Предопределенный / Predefined through its receiver context. Both aliases SHALL return Boolean false initially and during the object body, true when the predefined data name is nonempty, and false after clearing it. Assignment to the flag SHALL fail without changing the name or flag.

#### Scenario: Name transitions
- **WHEN** a new object's acquired valid predefined name is assigned and then cleared
- **THEN** both aliases SHALL report true and then false

#### Scenario: Body access
- **WHEN** the new object's body reads bare Предопределенный
- **THEN** it SHALL read the current object's false initial flag

#### Scenario: Read-only flag
- **WHEN** code assigns the flag
- **THEN** assignment SHALL fail and retain the previous name and flag
