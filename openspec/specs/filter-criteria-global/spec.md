# filter-criteria-global Specification

## Purpose
Bind the filter-criteria global through the existing library resolver while explicitly refusing unavailable criterion execution.

## Requirements

### Requirement: Reached filter criteria global declaration
The ORM library SHALL declare КритерииОтбора and FilterCriteria as aliases of the same library-provided global through the existing resolver. Local and module variables SHALL keep priority. Dormant original calls through named criteria SHALL compile. Until acquired criterion composition and manager operations are supported, evaluating this global SHALL raise an explicit catchable unsupported error rather than return an empty or synthetic manager. Registering this global SHALL NOT implement criterion filtering.

#### Scenario: Original document-journal manager declaration
- **WHEN** a module declares a dormant call to `КритерииОтбора.ДокументыПоКонтрагенту.СоставДокументов()` with the ORM library registered
- **THEN** it compiles without source rewriting
- **AND** the English global alias resolves through the same library declaration

#### Scenario: Actual lookup without implementation
- **WHEN** code evaluates either global alias
- **THEN** execution raises the explicit unsupported error
- **AND** BSL exception handling can catch it

#### Scenario: Variable precedence and absent library
- **WHEN** a local or module variable has the same name
- **THEN** it retains its existing priority
- **AND** an unregistered global remains an undefined variable
