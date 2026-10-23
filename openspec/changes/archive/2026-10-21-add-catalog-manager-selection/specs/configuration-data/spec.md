## ADDED Requirements

### Requirement: Principal-bound catalog selection
A catalog manager SHALL expose Выбрать / Select through the existing configuration receiver. A selection SHALL obtain actual rows through the current session's principal-bound data adapter, preserve native-measured ordering, iteration and field access, and expose the configuration-owned selection type. No successful empty fallback SHALL hide adapter errors or unavailable capabilities.

#### Scenario: Original manager selects catalog rows
- **WHEN** the original catalog manager calls Выбрать().Следующий()
- **THEN** the existing interpreter obtains actual accessible catalog rows for the current session
- **AND** the iteration result matches the preserved native contract

#### Scenario: Data access fails
- **WHEN** the adapter cannot read the catalog
- **THEN** the selection operation propagates an explicit failure instead of returning a fabricated empty result

#### Scenario: Variable priority remains unchanged
- **WHEN** a local or module declaration has the same name as a manager member
- **THEN** the existing declaration retains its resolver priority

### Requirement: Configuration-owned selection type registration
MetadataCatalog SHALL expose catalog_selection_types() yielding immutable descriptors for its concrete catalog selection types. SessionFactory SHALL register these through the existing engine value-type API without adding serialization of mutable cursor state.

#### Scenario: Concrete selection type names
- **WHEN** a configuration session resolves СправочникВыборка.CatalogName or CatalogSelection.CatalogName
- **THEN** both names identify the same configuration-owned selection type
