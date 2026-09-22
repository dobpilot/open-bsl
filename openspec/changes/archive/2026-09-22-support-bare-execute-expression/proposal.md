# Support the bare Execute expression form

## Why
Real configuration modules use `Выполнить` followed directly by a string expression. The platform accepts this statement form, while open-bsl currently requires an opening parenthesis and rejects unchanged configuration source.

## What Changes
- Accept `Выполнить <Expression>` and `Execute <Expression>` without call parentheses.
- Preserve the existing parenthesized form and dynamic execution behavior.

## Impact
Only statement parsing changes. The AST, semantic model, bytecode, VM, and crate dependencies remain unchanged.
