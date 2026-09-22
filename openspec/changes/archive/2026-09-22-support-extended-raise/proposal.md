# Support the extended Raise operator

## Why
Real configuration modules use the platform form `ВызватьИсключение([Описание], [Категория], [Код], [ДополнительнаяИнформация], [Причина])`. open-bsl currently parses only the expression and bare rethrow forms, so an unchanged configuration cannot be compiled.

## What Changes
- Parse the parenthesized five-position form, including omitted positions.
- Evaluate supplied arguments from left to right and raise an immutable error-information value carrying the documented fields and cause.
- Preserve the existing expression form and bare rethrow behavior.
- Extend the versioned bytecode representation for the structured form.

## Impact
The syntax AST, semantic representation, compiler, bytecode format, runtime error-information object, and VM are affected. No external dependency is added, and the normal dependency direction remains unchanged.
