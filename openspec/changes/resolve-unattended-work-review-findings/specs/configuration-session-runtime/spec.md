## REMOVED Requirements

### Requirement: Load custom settings through their configuration handler
**Reason**: Replaced by "Load custom settings through the acquired handler" and "Load handler errors leave Load as catchable exceptions". The project follows the platform documentation for handler errors instead of the restart that 1C 8.3.27.2342 demands.
**Migration**: Hosts that relied on `RtError::CriticalConfiguration` from `Загрузить` now receive the original runtime error, and BSL `Попытка` around `Загрузить` catches it.

## ADDED Requirements

### Requirement: Load custom settings through the acquired handler
Загрузить/Load on an acquired custom settings storage manager, also called from
its own manager module, SHALL run the original ОбработкаЗагрузки/LoadProcessing
handler and return its final settings parameter. The handler SHALL be found
case-insensitively, preferring ОбработкаЗагрузки regardless of ScriptVariant
and declaration order; a missing handler SHALL keep the Russian-name diagnostic.

#### Scenario: Native defaults and explicit user
- **WHEN** the retained native probe loads an object with omitted optional
  arguments or with an explicit settings key and user
- **THEN** the handler inputs and returned settings match the native output

#### Scenario: Own manager context
- **WHEN** an exported function of a settings storage manager calls Load
  without qualifying the receiver
- **THEN** its own storage handler supplies the result

#### Scenario: English custom load handler
- **WHEN** a manager defines only LoadProcessing, including another letter case
- **THEN** Load invokes that handler and returns its final settings parameter

#### Scenario: Both handler names are defined
- **WHEN** a manager defines both handler names in either order
- **THEN** Load invokes ОбработкаЗагрузки for Russian and English ScriptVariant

### Requirement: Load handler errors leave Load as catchable exceptions
Following the platform documentation, which states only that Загрузить invokes
ОбработкаЗагрузки, an uncaught BSL exception inside that handler SHALL leave
Загрузить as an ordinary catchable exception carrying the original cause, and
the session SHALL remain usable. This deliberately deviates from 1C
8.3.27.2342, which demands a restart in these cases.

#### Scenario: Raised load-handler error is catchable
- **WHEN** the acquired load handler explicitly raises an exception inside an enclosing BSL try/catch
- **THEN** the enclosing catch runs and `ОписаниеОшибки()` contains the raised text
- **AND** later calls in the same session, including a normal load, succeed

#### Scenario: Runtime failure inside the handler keeps its cause
- **WHEN** the acquired load handler divides by zero, reads a missing field, converts an invalid string, indexes out of bounds or fails to compile a dynamic expression
- **THEN** a direct Load call fails with that original runtime error
- **AND** the same Load inside BSL try/catch is caught

#### Scenario: Startup catches a handler error
- **WHEN** УстановкаПараметровСеанса calls Load inside try/catch and the handler raises
- **THEN** the session opens
