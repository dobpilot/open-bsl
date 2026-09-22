# Support empty lines between multiline string fragments

## Why
Real configuration modules contain empty physical lines between active fragments of a multiline string. The platform accepts them, while open-bsl currently reports `ExpectedContinuationBar`.

## What Changes
- Permit empty physical lines between active multiline-string fragments.
- Normalize LF and CRLF source line endings to one LF in the resulting value.
- Exclude intervening empty lines from the value, preserving one logical line break between active fragments.

## Impact
Only string literal lexing changes. Bytecode, runtime APIs, and crate dependencies remain unchanged.
