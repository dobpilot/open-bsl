## ADDED Requirements

### Requirement: Value list lookup by value
ValueList SHALL expose НайтиПоЗначению / FindByValue with exactly one required argument. It SHALL return the first existing item in current order whose value equals the argument, or Undefined if no item matches. Lookup SHALL preserve the selected item identity and SHALL NOT mutate the list.

#### Scenario: First equal item wins
- **WHEN** a list contains repeated numeric values
- **THEN** lookup returns the first matching existing item

#### Scenario: Lookup uses measured equality
- **WHEN** the retained native lookup control executes
- **THEN** its sixteen output lines match the native oracle

#### Scenario: Missing value
- **WHEN** no stored value equals the argument
- **THEN** lookup returns Undefined

#### Scenario: Arity and aliases
- **WHEN** a differently cased language alias is called with one argument
- **THEN** it performs the same lookup
- **AND** calls with zero or two arguments are rejected
