## ADDED Requirements

### Requirement: Report lock state of supported new document objects
Supported new unsaved document objects SHALL expose Заблокирован/IsLocked with zero arguments and return Boolean False after selection. The same method SHALL bind bare calls in the owning object module. Calls before object selection SHALL retain the selected-object error. Existing local function precedence and standalone undefined-name behavior SHALL remain unchanged. This requirement SHALL NOT imply persistence or object-lock acquisition support.

#### Scenario: New selected object
- **WHEN** code calls either alias on a newly created document or through an exported owning-module helper
- **THEN** the result is False

#### Scenario: Object-module body
- **WHEN** the object module calls IsLocked before object selection
- **THEN** it raises the existing selected-object error

#### Scenario: Wrong arity or scope
- **WHEN** code supplies one argument or uses a standalone global IsLocked
- **THEN** the existing arity or unresolved-name error is retained

#### Scenario: Existing local function
- **WHEN** an owning module declares its own same-named local function
- **THEN** bare calls retain existing local function precedence

#### Scenario: Existing same-named export
- **WHEN** the owning module already exposes an exported method with the same name and its own arity
- **THEN** direct object calls preserve that export's result and arity instead of selecting the newly declared platform method
