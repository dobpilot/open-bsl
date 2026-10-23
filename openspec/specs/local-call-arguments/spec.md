# local-call-arguments Specification

## Purpose
Define explicit argument omission for local calls while distinguishing empty
argument positions, declared defaults, and absent required trailing positions.

## Requirements

### Requirement: Explicitly omitted local argument retains its position

A local procedure or function call SHALL pass `Undefined` in an explicitly omitted required argument position. An omitted position whose parameter has a declared default SHALL use that default. A required trailing parameter for which no position is supplied SHALL remain a compile error. Existing argument-count and procedure-as-function diagnostics SHALL remain unchanged.

#### Scenario: Empty required slot before supplied arguments

- **WHEN** a local function with a required first parameter is called with an empty first position and later supplied positions
- **THEN** the function receives `Undefined` for its first parameter
- **AND** later supplied values retain their positions

#### Scenario: Explicitly empty final slot

- **WHEN** a local function with two required parameters is called with a value followed by an explicit empty second position
- **THEN** the function receives `Undefined` for its second parameter

#### Scenario: Absent required trailing slot

- **WHEN** the same function is called with only the first position and no second position
- **THEN** resolution reports the existing missing-required-argument diagnostic

#### Scenario: Defaulted position

- **WHEN** a local function is called with an explicitly empty position whose parameter has a declared default
- **THEN** the declared default is used
