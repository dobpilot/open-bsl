## ADDED Requirements

### Requirement: Ordinary catalog standard-field lookup
Catalog managers SHALL expose `НайтиПоКоду`/`FindByCode` and `НайтиПоНаименованию`/`FindByDescription` with one required and up to four total arguments. Supported ordinary string searches SHALL use the current Session's principal-bound data adapter and SHALL retain rights and RLS enforcement. Adapter failures SHALL propagate as errors, not missing results. Unsupported modes or unknown metadata SHALL fail explicitly.

#### Scenario: String code match
- **GIVEN** the catalog contains code `AbC`
- **WHEN** ordinary search supplies `AbC` or `AbC `
- **THEN** the matching catalog reference is returned
- **AND** `abc` and ` AbC` do not match

#### Scenario: Exact and prefix descriptions
- **GIVEN** the catalog contains description `AbC name`
- **WHEN** exact search supplies `AbC name ` or prefix search supplies `AbC`
- **THEN** a matching reference is returned
- **AND** exact search for `AbC` and prefix search for `abc` do not match

#### Scenario: No matching row
- **WHEN** the adapter successfully returns no matching candidates
- **THEN** the manager returns the empty reference of that catalog

#### Scenario: Rights failure
- **WHEN** the current principal is denied the query
- **THEN** that failure reaches the caller without producing an empty reference

### Requirement: Search linking preserves existing exports
The same descriptor tables SHALL support qualified calls and own manager receiver calls through the existing resolver. Existing same-named exports SHALL preserve their signatures and behavior.

#### Scenario: Original manager body
- **WHEN** its body calls `НайтиПоКоду` with one argument
- **THEN** it binds to its own manager receiver method

#### Scenario: Existing export
- **GIVEN** the manager previously exported a same-named function
- **WHEN** a consumer invokes that export
- **THEN** its previous arity and result are preserved
