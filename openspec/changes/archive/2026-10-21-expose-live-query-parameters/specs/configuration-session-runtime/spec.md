## ADDED Requirements

### Requirement: Live parameters of a query
Query SHALL expose Параметры/Parameters as a read-only property containing the query's live mutable Structure. Each new query SHALL start with an independent empty structure. Saved property values, SetParameter and execution SHALL share the same entries; changing query text SHALL NOT replace or repopulate them. Repeated SetParameter SHALL expose the latest supplied key spelling while retaining the existing insertion position.

#### Scenario: Shared mutations
- **WHEN** a saved Parameters structure is changed by Insert or Delete, or SetParameter changes the query
- **THEN** both the query and the saved structure observe the change and execution captures the current ordered entries

#### Scenario: Identity and isolation
- **WHEN** the property is read again or another query is created
- **THEN** the first query retains its structure identity and the other starts with a separate empty structure

#### Scenario: Property assignment
- **WHEN** code assigns a new structure to Parameters itself
- **THEN** it receives the existing read-only-property error
