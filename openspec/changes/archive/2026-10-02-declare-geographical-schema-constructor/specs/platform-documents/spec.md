## ADDED Requirements

### Requirement: Reached geographical document constructor
The document component SHALL resolve zero-argument `ГеографическаяСхема`
and `GeographicalSchema` constructor names to the same distinct declared type.
Until its object model is implemented, executing this constructor SHALL fail
explicitly instead of fabricating document data.

#### Scenario: Dormant geographic file branch
- **WHEN** an unexecuted configuration branch constructs a geographical schema
- **THEN** the module compiles through the ordinary component constructor path

#### Scenario: Missing geographical object model
- **WHEN** the declared constructor executes
- **THEN** it reports that the geographical object model is not implemented

#### Scenario: Constructor argument
- **WHEN** a static constructor call supplies an argument
- **THEN** compilation reports an arity error
