# value-list-presentation-sort Specification

## Purpose
Define platform-compatible sorting of value list items by their presentations while preserving item identity and properties.

## Requirements

### Requirement: Sort existing value list items by presentation
ValueList SHALL expose СортироватьПоПредставлению and SortByPresentation through the existing component dispatch. The method SHALL accept zero or one direction argument. Omitted direction and Undefined SHALL select ascending order; SortDirection SHALL select ascending or descending order. An invalid direction SHALL fail without changing the list.

#### Scenario: Default and aliases
- **WHEN** a value list is sorted using either method name with omitted, Undefined or ascending direction
- **THEN** its order matches retained native 1C controls

#### Scenario: Descending direction
- **WHEN** descending direction is supplied
- **THEN** the presentation order matches retained native 1C controls

#### Scenario: Invalid direction
- **WHEN** a number is supplied as direction
- **THEN** the call fails and the list retains its previous order

### Requirement: Sorting preserves the items and uses their presentations
Sorting SHALL reorder existing items without replacing them or changing their values and properties. Missing presentations SHALL NOT fall back to item values. Ordering for retained native controls SHALL match the platform, including the measured distinctions between ASCII case, Cyrillic case and Е/Ё.

#### Scenario: Missing presentations
- **GIVEN** values b, A and a are added without presentations
- **WHEN** the list is sorted in ascending order
- **THEN** the items remain in their insertion order

#### Scenario: Measured presentation order
- **WHEN** the retained native presentation controls are executed
- **THEN** every reported item order matches the platform oracle
- **AND** references obtained before sorting still identify the same items afterward
