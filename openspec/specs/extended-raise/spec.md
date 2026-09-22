# extended-raise Specification

## Purpose
Define the parenthesized structured `ВызватьИсключение` form and the resulting immutable error information so BSL code can preserve category, code, additional data, and cause chains through existing exception handling.

## Requirements

### Requirement: Parenthesized structured exception
The language SHALL accept `ВызватьИсключение` and `Raise` with up to five parenthesized positional arguments for description, category, code, additional information, and cause. Every position SHALL be optional, and omitted middle positions SHALL retain their documented defaults. Supplied expressions SHALL be evaluated from left to right before the exception is raised.

#### Scenario: Omitted middle arguments
- **WHEN** code executes `ВызватьИсключение(Описание, Категория,,, Причина)`
- **THEN** the raised error retains the supplied description, category, and cause
- **AND** code and additional information have their default values

#### Scenario: Existing forms
- **WHEN** code uses `ВызватьИсключение Выражение` or a bare `ВызватьИсключение`
- **THEN** the existing expression-raise or rethrow behavior remains unchanged

### Requirement: Structured error information
An exception raised by the parenthesized form SHALL be observed in its handler as immutable `ИнформацияОбОшибке`. Its description, code, additional information, cause, and error category SHALL reflect the supplied arguments and documented defaults. Category checks SHALL recognize the stored category and `ВсеОшибки`, and SHALL inspect the cause when requested.

#### Scenario: Handler observes supplied fields
- **WHEN** a handler catches an exception raised with description, category, code, and additional information
- **THEN** `ИнформацияОбОшибке()` exposes those values
- **AND** `ЯвляетсяОшибкойКатегории` recognizes the supplied category

#### Scenario: Cause chain
- **WHEN** a structured exception is raised with another `ИнформацияОбОшибке` as its cause
- **THEN** the `Причина` property returns that immutable error snapshot
- **AND** a category check that includes causes can match a category stored in the cause chain

### Requirement: Versioned bytecode representation
The textual bytecode format SHALL represent the structured raise form without changing the meaning of existing `Raise` instructions. Readers SHALL reject images from incompatible format versions rather than guessing the missing structured arguments.

#### Scenario: Structured raise round trip
- **WHEN** a program containing the parenthesized raise form is printed and parsed as textual bytecode
- **THEN** all five positional values and their order are preserved
