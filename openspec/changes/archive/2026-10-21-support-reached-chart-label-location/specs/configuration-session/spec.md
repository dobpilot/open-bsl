## ADDED Requirements

### Requirement: Reached chart label location enum
The runtime SHALL expose ПоложениеПодписейКДиаграмме.КрайАвто and ChartLabelLocation.EdgeAuto as the same system enumeration value. It SHALL also expose ПоложениеПодписейКДиаграмме.КрайВнутри and ChartLabelLocation.EdgeInside as a second distinct value with presentation `Edge inside`. The existing EdgeAuto presentation SHALL remain `Edge auto`.

#### Scenario: Original BUH manager declaration
- **WHEN** original manager code uses ПоложениеПодписейКДиаграмме.КрайАвто
- **THEN** the expression resolves without undefined-variable errors
- **AND** its value equals ChartLabelLocation.EdgeAuto

#### Scenario: Type aliases
- **WHEN** either declared type name is passed to Type
- **THEN** it equals TypeOf of the reached member

#### Scenario: Subsequently reached inside-edge label
- **WHEN** original BUH manager code uses ПоложениеПодписейКДиаграмме.КрайВнутри or ChartLabelLocation.EdgeInside with any letter case
- **THEN** both resolve to the same value and differ from EdgeAuto
- **AND** its type is ChartLabelLocation and its presentation is `Edge inside`

#### Scenario: Unsupported guessed alias
- **WHEN** code uses ChartLabelLocation.InsideEdge
- **THEN** normal unresolved-member diagnostics are retained

#### Scenario: Primitive label ordering
- **WHEN** code compares EdgeAuto and EdgeInside with the primitive less-than operator
- **THEN** execution reports the existing unsupported comparison error

### Requirement: Reached chart scale label location enum
The runtime SHALL expose ПоложениеПодписейШкалыДиаграммы.Нет and ChartScaleLabelLocation.None as the same system enumeration value.

#### Scenario: Following original BUH declaration
- **WHEN** the same original report manager uses ПоложениеПодписейШкалыДиаграммы.Нет
- **THEN** the expression resolves and equals ChartScaleLabelLocation.None
- **AND** both declared type names resolve to its type
