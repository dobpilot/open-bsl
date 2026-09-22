# Support unary plus

## Why
Real configuration modules use unary `+` before a numeric expression. The 1C:Enterprise language defines this operator, while open-bsl currently rejects it during parsing, so unchanged configuration source cannot be compiled.

## What Changes
- Parse unary `+` with the same precedence and right-associative chaining as unary `-`.
- Evaluate its operand once, require a number, and return that number unchanged.
- Preserve a runtime type error for non-numeric operands.

## Impact
The syntax AST and compiler gain the unary-plus operation. The existing bytecode format, VM instruction set, and crate dependencies do not change.
