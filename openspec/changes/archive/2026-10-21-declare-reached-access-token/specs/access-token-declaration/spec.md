## ADDED Requirements

### Requirement: Declaration-only AccessToken constructor
The crypto component SHALL declare ТокенДоступа / AccessToken with exactly zero or two arguments using existing case-insensitive constructor resolution. Dormant calls SHALL compile. Actual construction SHALL fail explicitly as unsupported without returning a fabricated token.

#### Scenario: Dormant constructor calls
- **WHEN** an unexecuted method contains either constructor alias with zero or two arguments
- **THEN** compilation succeeds without creating a token

#### Scenario: Invalid arity and actual execution
- **WHEN** compilation supplies one or three arguments
- **THEN** ordinary constructor arity checking rejects the call
- **WHEN** execution reaches a valid declared constructor
- **THEN** an explicit unsupported AccessToken error is raised

### Requirement: Measured HS256 enumeration declaration
The runtime SHALL expose HS256 through АлгоритмПодписиТокенаДоступа / AccessTokenSignAlgorithm using existing enumeration resolution. Alias equality, type and presentation SHALL match retained native measurements; primitive ordering SHALL fail.

#### Scenario: Aliases and presentation
- **WHEN** code reads HS256 through either family alias
- **THEN** values and types compare equal and the value presentation is HS256

#### Scenario: Ordering
- **WHEN** code applies a primitive ordering operator to HS256
- **THEN** the existing unsupported comparison error is raised
