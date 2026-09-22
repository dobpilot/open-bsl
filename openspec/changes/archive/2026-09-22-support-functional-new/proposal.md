# Support the functional New operator

## Why
Real configuration modules use the platform form `Новый(<Тип>[, <ПараметрыКонструктора>])`, where the type is computed at runtime. open-bsl currently accepts only the static `Новый ИмяТипа(...)` form, so unchanged configuration source cannot be compiled.

## What Changes
- Parse the parenthesized functional form separately from the existing static constructor syntax.
- Resolve a runtime type from a type value or type-name string.
- Expand an array in the optional second position into constructor arguments and ignore a non-array value as documented by the 1C Syntax Assistant.
- Invoke the existing constructor for the resolved native or registered component type.
- Extend the versioned bytecode representation for the functional form.

## Impact
The syntax AST, semantic representation, compiler, bytecode format, runtime constructor lookup, and VM are affected. Existing static constructors keep their current resolution and execution. No external dependency is added, and the normal crate dependency direction remains unchanged.
