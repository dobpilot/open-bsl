# Support runtime Structure keys

## Why
Real configuration startup modules construct `Структура` with a field list computed at runtime. open-bsl currently rejects that source during semantic resolution even though the same constructor with a string literal is supported.

## What Changes
- Accept a computed first argument in the static `Новый Структура(...)` / `New Structure(...)` form.
- Preserve the existing compile-time shape path and diagnostics for literal field lists.
- Build runtime-key structures without interning an unbounded set of shapes.
- Reproduce the measured value assignment, missing-value, excess-value, alias, and fixed-structure-copy behavior.
- Add a versioned bytecode instruction for the runtime-key form.

## Impact
The semantic representation, compiler, bytecode format, base runtime collections, and VM are affected. Existing literal constructors retain their current resolution and execution. No external dependency or normal dependency edge is added.
