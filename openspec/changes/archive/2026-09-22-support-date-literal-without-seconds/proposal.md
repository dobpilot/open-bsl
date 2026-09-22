# Support date literals without seconds

## Why
Real configuration modules use twelve-digit date literals whose time contains hours and minutes but omits seconds. The 1C:Enterprise Syntax Assistant documents this form, while open-bsl currently rejects it during lexing, so unchanged configuration source cannot be compiled.

## What Changes
- Accept date literals in `YYYYMMDDhhmm` form alongside the existing date-only and full-time forms.
- Interpret an omitted seconds field as zero.
- Ignore non-digit separators inside date literals as the platform does.
- Reject every unsupported digit count and invalid calendar value.

## Impact
The lexer and runtime date parser are affected. Existing unseparated eight- and fourteen-digit literals retain their behavior. The bytecode representation and crate dependency graph do not change.
