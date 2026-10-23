# open-method-arguments Specification

## Purpose
Preserve omitted arguments in open component calls while retaining existing closed builtin checks and runtime diagnostics.

## Requirements

### Requirement: Open method omissions survive core receiver inference
A method name absent from the closed builtin method table SHALL retain explicitly omitted positions through the existing open object-call argument representation, including when its receiver is statically known as a core object. Receiver inference SHALL NOT turn this open call into a closed builtin argument check. Existing known closed method restrictions SHALL remain unchanged. This requirement SHALL NOT add implementations for unknown methods.

#### Scenario: Reached dormant writer method
- **WHEN** an original module declares a procedure containing `Writer = New TextWriter; Writer.Open(Path,,,True)`
- **THEN** the module compiles and retains both omitted argument positions
- **AND** the dormant procedure is not executed during compilation

#### Scenario: Missing method remains an execution error
- **WHEN** an open call with omitted positions targets a nonexistent method on a structure
- **THEN** the call compiles
- **AND** executing it raises a catchable missing-method error

#### Scenario: Existing closed method restriction
- **WHEN** a statically known array calls the known closed Find method with an omitted argument
- **THEN** its previous semantic rejection remains unchanged
