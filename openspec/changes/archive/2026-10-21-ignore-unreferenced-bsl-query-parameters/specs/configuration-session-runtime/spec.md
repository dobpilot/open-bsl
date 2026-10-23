## ADDED Requirements

### Requirement: Ignore unreferenced parameters of a BSL query
The configured database adapter SHALL pass to the SDK only BSL query parameters referenced by the current query text. It SHALL preserve the query object's complete parameter collection and SHALL NOT convert unused values. Parameter names SHALL remain case-insensitive; text in comments and string literals SHALL NOT count as a reference. Missing referenced parameters SHALL retain their error behavior.

#### Scenario: Retained parameter
- **WHEN** a query retains a supplied UUID or value table that its current text does not reference
- **THEN** execution is not rejected because of that unused value

#### Scenario: Missing used parameter
- **WHEN** the current text references a parameter that has not been supplied
- **THEN** execution still reports the missing parameter
