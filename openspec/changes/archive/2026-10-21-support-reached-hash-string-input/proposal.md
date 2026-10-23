# Support reached hashing string input

## Why
Original demo startup reaches DataHashing.Add with a String. The existing binary-only implementation refuses it.

## What Changes
Hash string input as UTF-8, as measured on native 1C. Preserve binary input and reject other unsupported types.

## Impact
Only the existing crypto method implementation changes; no new dependency or public declaration.
