## ADDED Requirements

### Requirement: Portable fatal module compilation failures
Bytecode images SHALL preserve a deferred module compilation failure as a distinct operation with a diagnostic string. The image verifier SHALL reject invalid operands. Execution of that operation SHALL produce a module compilation error which a BSL exception handler cannot recover from. The text format version SHALL advance for the new instruction; previous format versions SHALL be rejected by the existing version check.

#### Scenario: Deferred failure survives serialization
- **WHEN** a configuration containing a deferred common-module compilation failure is written and parsed as a bytecode image
- **THEN** an unrelated parent export remains callable and first access to the faulty module produces the same fatal diagnostic

#### Scenario: Invalid diagnostic operand
- **WHEN** the operation refers to a register outside its chunk frame
- **THEN** image validation rejects it before execution

#### Scenario: Previous image version
- **WHEN** the reader receives the previous bytecode text version
- **THEN** it returns the existing unsupported-version diagnostic
