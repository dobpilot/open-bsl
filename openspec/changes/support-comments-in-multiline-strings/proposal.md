# Support comments between multiline string fragments

## Why
Real configuration modules place commented-out continuation lines between active fragments of a multiline string. The 1C:Enterprise language permits such comment lines, while open-bsl currently rejects the first `/` with `ExpectedContinuationBar`, so unchanged configuration source cannot be compiled.

## What Changes
- Permit a full-line `//` comment between fragments of a multiline string literal.
- Exclude the comment text and its physical line break from the resulting string value.
- Preserve the existing requirement that the next active fragment starts with `|`.

## Impact
Only string literal lexing changes. Existing string values, bytecode, runtime behavior, and crate dependencies remain unchanged.
