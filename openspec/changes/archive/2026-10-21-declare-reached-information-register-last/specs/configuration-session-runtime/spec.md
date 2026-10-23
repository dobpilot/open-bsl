## ADDED Requirements

### Requirement: Information register last-record method declaration
Information register managers SHALL declare ПолучитьПоследнее/GetLast with zero through two arguments in qualified and own-manager contexts. Until the periodic-register last-record execution profile is implemented, invocation SHALL fail explicitly instead of returning fabricated successful data. Existing same-named manager-module exports SHALL preserve their previous dispatch and arity. Other metadata manager kinds SHALL NOT acquire this method.

#### Scenario: Original dormant own-manager call
- **WHEN** a BUH information-register manager module contains ПолучитьПоследнее(Период,Отбор) in an unexecuted procedure
- **THEN** the established receiver mechanism resolves it without a global builtin

#### Scenario: Unsupported execution
- **WHEN** either alias is invoked with zero, one or two arguments
- **THEN** invocation raises the explicit unsupported GetLast error

#### Scenario: Existing manager export
- **GIVEN** a manager module exports a same-named three-argument function
- **WHEN** configuration code calls that export
- **THEN** the previous export dispatch and arity remain usable

#### Scenario: Excess arguments without an export
- **WHEN** the own-manager platform call supplies three arguments
- **THEN** the existing deferred arity error is preserved
