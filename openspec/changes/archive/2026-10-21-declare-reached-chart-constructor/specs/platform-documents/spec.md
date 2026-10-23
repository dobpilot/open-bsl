## ADDED Requirements

### Requirement: Reached chart constructor declaration
The document component SHALL resolve zero-argument `Диаграмма` and `Chart` constructors to the same declared type with presentation `Chart`. Names SHALL use the existing case-insensitive constructor lookup. The component SHALL retain existing constructor identities. Until the chart object model is implemented, actual construction SHALL explicitly report the unsupported object model and SHALL NOT return fabricated chart data.

#### Scenario: Dormant original chart creation
- **WHEN** an unexecuted original module branch constructs a chart with either alias and zero arguments
- **THEN** the module compiles through the ordinary component constructor path
- **AND** execution of unrelated code succeeds

#### Scenario: Chart type aliases
- **WHEN** code looks up `Тип("Диаграмма")` or `Type("Chart")`
- **THEN** both identify the same declared type with presentation `Chart`

#### Scenario: Unsupported actual construction
- **WHEN** either declared constructor actually executes
- **THEN** a clear runtime error reports that the chart object model is not implemented
- **AND** no substitute value is returned

#### Scenario: Excess constructor arguments
- **WHEN** a static chart constructor call supplies any arguments
- **THEN** ordinary component arity validation rejects it
