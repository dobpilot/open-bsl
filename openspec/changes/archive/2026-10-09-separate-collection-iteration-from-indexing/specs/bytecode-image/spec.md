## ADDED Requirements

### Requirement: Iteration access has a distinct serialized operation
The textual bytecode format SHALL represent iteration-element access separately from ordinary indexed access. The current format version SHALL be 44. The printer and parser SHALL share the same operation name and operands, and the existing exhaustive instruction-effects table SHALL describe the new operation.

#### Scenario: Round-trip iteration-element access
- **WHEN** a program containing iteration-element access is printed and parsed
- **THEN** its operation and register operands are preserved

#### Scenario: Reject an older format header
- **WHEN** the current parser receives the preceding format version
- **THEN** it rejects the image before execution with its existing version diagnostic

#### Scenario: Reject invalid iteration registers
- **WHEN** an image names an out-of-range register for iteration-element access
- **THEN** the existing image verifier rejects it before execution
