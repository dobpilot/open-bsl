## ADDED Requirements

### Requirement: Unary plus validates and preserves a number
The language SHALL accept unary `+` before an expression with the same precedence as unary `-`. It SHALL evaluate the operand exactly once, require a numeric value at runtime, and return that value unchanged.

#### Scenario: Numeric operand
- **WHEN** unary `+` is applied to a number
- **THEN** the result is the same number

#### Scenario: Operand evaluation
- **WHEN** the operand of unary `+` has a visible side effect
- **THEN** the side effect occurs exactly once

#### Scenario: Non-numeric operand
- **WHEN** unary `+` is applied to a non-numeric value
- **THEN** execution raises a catchable runtime type error

### Requirement: Unary signs associate from right to left
A chain of unary `+` and `-` operators SHALL bind more tightly than multiplicative operators and SHALL associate from right to left.

#### Scenario: Mixed sign chain
- **WHEN** source evaluates `+-5`
- **THEN** it produces negative five
